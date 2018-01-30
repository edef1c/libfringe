// This file is part of libfringe, a low-level green threading library.
// Copyright (c) Nathan Zadoks <nathan@nathan7.eu>,
//               whitequark <whitequark@whitequark.org>
//               Amanieu d'Antras <amanieu@gmail.com>
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

// To understand the machine code in this file, keep in mind these facts:
// * ARM AAPCS ABI passes the first argument in r0. We also use r0 to pass a value
//   while swapping context; this is an arbitrary choice
//   (we clobber all registers and could use any of them) but this allows us
//   to reuse the swap function to perform the initial call.
//
// To understand the ARM EHABI CFI code in this file, keep in mind these facts:
// * CFI is "call frame information"; a set of instructions to a debugger or
//   an unwinder that allow it to simulate returning from functions. This implies
//   restoring every register to its pre-call state, as well as the stack pointer.
// * CFA is "call frame address"; the value of stack pointer right before the call
//   instruction in the caller. Everything strictly below CFA (and inclusive until
//   the next CFA) is the call frame of the callee. This implies that the return
//   address is the part of callee's call frame.
// * Logically, ARM EHABI CFI is a table where rows are instruction pointer values and
//   columns describe where registers are spilled (mostly using expressions that
//   compute a memory location as CFA+n). A .save pseudoinstruction changes
//   the state of a column for all IP numerically larger than the one it's placed
//   after. A .pad or .setfp pseudoinstructions change the CFA value similarly.
// * Simulating return is as easy as restoring register values from the CFI table
//   and then setting stack pointer to CFA.
//
// A high-level overview of the function of the trampolines is:
// * The 2nd init trampoline puts a controlled value (written in swap to `new_cfa`)
//   into r11. This is then used as the CFA for the 1st trampoline.
// * This controlled value points to the bottom of the stack of the parent context,
//   which holds the saved r11 and lr from the call to swap().
// * The 1st init trampoline tells the unwinder to restore r11 and lr
//   from the stack frame at r11 (in the parent stack), thus continuing
//   unwinding at the swap call site instead of falling off the end of context stack.
use core::mem;
use arch::StackPointer;
use unwind;

pub const STACK_ALIGNMENT: usize = 8;

pub unsafe fn init(stack_base: *mut u8, f: unsafe fn(usize, StackPointer)) -> StackPointer {
  #[cfg(not(target_vendor = "apple"))]
  #[naked]
  unsafe extern "C" fn trampoline_1() {
    asm!(
      r#"
        # gdb has a hardcoded check that rejects backtraces where frame addresses
        # do not monotonically decrease. It is turned off if the function is called
        # "__morestack" and that is hardcoded. So, to make gdb backtraces match
        # the actual unwinder behavior, we call ourselves "__morestack" and mark
        # the symbol as local; it shouldn't interfere with anything.
      __morestack:
      .local __morestack

        # Set up the first part of our ARM EHABI CFI linking stacks together. When
        # we reach this function from unwinding, r11 will be pointing at the bottom
        # of the parent linked stack. This link is set each time swap() is called.
        # When unwinding the frame corresponding to this function, a ARM EHABI unwinder
        # will use r11+16 as the next call frame address, restore return address (lr)
        # from CFA-8 and restore r11 from CFA-16. This mirrors what the second half
        # of `swap_trampoline` does.
      # .setfp  fp, sp
      # .save   {fp, lr}

        # This nop is here so that the initial swap doesn't return to the start
        # of the trampoline, which confuses the unwinder since it will look for
        # frame information in the previous symbol rather than this one. It is
        # never actually executed.
        nop

      .Lend:
      .size __morestack, .Lend-__morestack
      "#
      : : : : "volatile")
  }

  #[cfg(target_vendor = "apple")]
  #[naked]
  unsafe extern "C" fn trampoline_1() {
    asm!(
      r#"
      # Identical to the above, except avoids .local/.size that aren't available on Mach-O.
      __morestack:
      .private_extern __morestack
      # .setfp  fp, sp
      # .save   {fp, lr}
        nop
      "#
      : : : : "volatile")
  }

  #[naked]
  unsafe extern "C" fn trampoline_2() {
    asm!(
      r#"
        # Set up the second part of our ARM EHABI CFI.
        # When unwinding the frame corresponding to this function, a DWARF unwinder
        # will restore r11 (and thus CFA of the first trampoline) from the stack slot.
        # This stack slot is updated every time swap() is called to point to the bottom
        # of the stack of the context switch just switched from.
      # .setfp  fp, sp
      # .save   {fp, lr}

        # This nop is here so that the return address of the swap trampoline
        # doesn't point to the start of the symbol. This confuses gdb's backtraces,
        # causing them to think the parent function is trampoline_1 instead of
        # trampoline_2.
        nop

        # Call unwind_wrapper with the provided function and the stack base address.
        add     r2, sp, #16
        ldr     r3, [sp, #8]
        bl      ${0}

        # Restore the stack pointer of the parent context. No CFI adjustments
        # are needed since we have the same stack frame as trampoline_1.
        ldr     sp, [sp]

        # Load frame and instruction pointers of the parent context.
        pop     {fp, lr}

        # If the returned value is nonzero, trigger an unwind in the parent
        # context with the given exception object.
        cmp     r0, #0
        bne     ${1}

        # Clear the stack pointer. We can't call into this context any more once
        # the function has returned.
        mov     r1, #0

        # Return into the new context. Use `r12` instead of `lr` to avoid
        # return address mispredictions.
        mov     r12, lr
        bx      r12
      "#
      :
      : "s" (unwind::unwind_wrapper as usize)
        "s" (unwind::start_unwind as usize)
      : : "volatile")
  }

  // We set up the stack in a somewhat special way so that to the unwinder it
  // looks like trampoline_1 has called trampoline_2, which has in turn called
  // swap::trampoline.
  //
  // There are 2 call frames in this setup, each containing the return address
  // followed by the r11 value for that frame. This setup supports unwinding
  // using DWARF CFI as well as the frame pointer-based unwinding used by tools
  // such as perf or dtrace.
  let mut sp = StackPointer::new(stack_base);

  sp.push(0 as usize); // Padding to ensure the stack is properly aligned
  sp.push(f as usize); // Function that trampoline_2 should call

  // Call frame for trampoline_2. The CFA slot is updated by swap::trampoline
  // each time a context switch is performed.
  sp.push(trampoline_1 as usize + 4); // Return after the nop
  sp.push(0xdead0cfa);                // CFA slot

  // Call frame for swap::trampoline. We set up the r11 value to point to the
  // parent call frame.
  let frame = sp.offset(0);
  sp.push(trampoline_2 as usize + 4); // Entry point, skip initial nop
  sp.push(frame as usize);            // Pointer to parent call frame

  sp
}

#[inline(always)]
pub unsafe fn swap_link(arg: usize, new_sp: StackPointer,
                        new_stack_base: *mut u8) -> (usize, Option<StackPointer>) {
  let ret: usize;
  let ret_sp: usize;
  asm!(
    r#"
        # Set up the link register
        adr     lr, 0f

        # Save the frame pointer and link register; the unwinder uses them to find
        # the CFA of the caller, and so they have to have the correct value immediately
        # after the call instruction that invoked the trampoline.
        push    {fp, lr}

        # Pass the stack pointer of the old context to the new one.
        mov     r1, sp

        # Link the call stacks together by writing the current stack bottom
        # address to the CFA slot in the new stack.
        str     sp, [r3, #-16]

        # Load stack pointer of the new context.
        mov     sp, r2

        # Load frame and instruction pointers of the new context.
        pop     {fp, r12}

        # Return into the new context. Use `r12` instead of `lr` to avoid
        # return address mispredictions.
        bx      r12

      0:
    "#
    : "={r0}" (ret)
      "={r1}" (ret_sp)
    : "{r0}" (arg)
      "{r2}" (new_sp.offset(0))
      "{r3}" (new_stack_base)
    :/*r0,    r1,*/ "r2",  "r3",  "r4",  "r5",  "r6",  "r7",
      "r8",  "r9",  "r10",/*r11,*/"r12",/*sp,*/ "lr", /*pc,*/
      "d0",  "d1",  "d2",  "d3",  "d4",  "d5",  "d6",  "d7",
      "d8",  "d9",  "d10", "d11", "d12", "d13", "d14", "d15",
      "d16", "d17", "d18", "d19", "d20", "d21", "d22", "d23",
      "d24", "d25", "d26", "d27", "d28", "d29", "d30", "d31",
      "cc", "memory"
    : "volatile");
  (ret, mem::transmute(ret_sp))
}

#[inline(always)]
pub unsafe fn swap(arg: usize, new_sp: StackPointer) -> (usize, StackPointer) {
  // This is identical to swap_link, but without the write to the CFA slot.
  let ret: usize;
  let ret_sp: usize;
  asm!(
    r#"
        adr     lr, 0f
        push    {fp, lr}
        mov     r1, sp
        mov     sp, r2
        pop     {fp, r12}
        bx      r12
      0:
    "#
    : "={r0}" (ret)
      "={r1}" (ret_sp)
    : "{r0}" (arg)
      "{r2}" (new_sp.offset(0))
    :/*r0,    r1,*/ "r2",  "r3",  "r4",  "r5",  "r6",  "r7",
      "r8",  "r9",  "r10",/*r11,*/"r12",/*sp,*/ "lr", /*pc,*/
      "d0",  "d1",  "d2",  "d3",  "d4",  "d5",  "d6",  "d7",
      "d8",  "d9",  "d10", "d11", "d12", "d13", "d14", "d15",
      "d16", "d17", "d18", "d19", "d20", "d21", "d22", "d23",
      "d24", "d25", "d26", "d27", "d28", "d29", "d30", "d31",
      "cc", "memory"
      // We need the "alignstack" attribute here to ensure that the stack is
      // properly aligned if a call to start_unwind needs to be injected into
      // our stack context.
    : "volatile", "alignstack");
  (ret, mem::transmute(ret_sp))
}

#[inline(always)]
pub unsafe fn unwind(new_sp: StackPointer, new_stack_base: *mut u8) {
  // Argument to pass to start_unwind, based on the stack base address.
  let arg = unwind::unwind_arg(new_stack_base);

  // This is identical to swap_link, except that it performs a tail call to
  // start_unwind instead of returning into the target context.
  asm!(
    r#"
        adr     lr, 0f
        push    {fp, lr}
        str     sp, [r3, #-16]
        mov     sp, r2
        pop     {fp, r12}

        # Jump to the start_unwind function, which will force a stack unwind in
        # the target context. This will eventually return to us through the
        # stack link.
        b       ${0}

      0:
    "#
    :
    : "s" (unwind::start_unwind as usize)
      "{r0}" (arg)
      "{r2}" (new_sp.offset(0))
      "{r3}" (new_stack_base)
    : "r0",  "r1",  "r2",  "r3",  "r4",  "r5",  "r6",  "r7",
      "r8",  "r9",  "r10",/*r11,*/"r12",/*sp,*/ "lr", /*pc,*/
      "d0",  "d1",  "d2",  "d3",  "d4",  "d5",  "d6",  "d7",
      "d8",  "d9",  "d10", "d11", "d12", "d13", "d14", "d15",
      "d16", "d17", "d18", "d19", "d20", "d21", "d22", "d23",
      "d24", "d25", "d26", "d27", "d28", "d29", "d30", "d31",
      "cc", "memory"
    : "volatile");
}
