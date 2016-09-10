// This file is part of libfringe, a low-level green threading library.
// Copyright (c) Nathan Zadoks <nathan@nathan7.eu>,
//               whitequark <whitequark@whitequark.org>
//               Amanieu d'Antras <amanieu@gmail.com>
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

// To understand the code in this file, keep in mind these two facts:
// * The iOS AArch64 ABI has a "red zone": 128 bytes under the top of the stack
//   that is defined to be unmolested by signal handlers, interrupts, etc.
//   Leaf functions can use the red zone without adjusting the stack pointer.
// * The AArch64 ABI requires the stack to always be a multiple of 16 bytes,
//   even in the middle of functions. Aligned operands are a requirement
//   of atomic operations, and making this the responsibility of the caller
//   avoids having to maintain a frame pointer, which is necessary when
//   a function has to realign the stack from an unknown state.
// * The AArch64 ABI passes the first argument in x0. We also use x0
//   to pass a value while swapping context; this is an arbitrary choice
//   (we clobber all registers and could use any of them) but this allows us
//   to reuse the swap function to perform the initial call. We do the same
//   thing with x1 to pass the stack pointer to the new context.
//
// To understand the DWARF CFI code in this file, keep in mind these facts:
// * CFI is "call frame information"; a set of instructions to a debugger or
//   an unwinder that allow it to simulate returning from functions. This implies
//   restoring every register to its pre-call state, as well as the stack pointer.
// * CFA is "call frame address"; the value of stack pointer right before the call
//   instruction in the caller. Everything strictly below CFA (and inclusive until
//   the next CFA) is the call frame of the callee. This implies that the return
//   address is the part of callee's call frame.
// * Logically, DWARF CFI is a table where rows are instruction pointer values and
//   columns describe where registers are spilled (mostly using expressions that
//   compute a memory location as CFA+n). A .cfi_offset pseudoinstruction changes
//   the state of a column for all IP numerically larger than the one it's placed
//   after. A .cfi_def_* pseudoinstruction changes the CFA value similarly.
// * Simulating return is as easy as restoring register values from the CFI table
//   and then setting stack pointer to CFA.
//
// A high-level overview of the function of the trampolines when unwinding is:
// * The 2nd init trampoline puts a controlled value (written in swap to `new_cfa`)
//   into x29. This is then used as the CFA for the 1st trampoline.
// * This controlled value points to the bottom of the stack of the parent context,
//   which holds the saved x29 and x30 from the call to swap().
// * The 1st init trampoline tells the unwinder to restore x29 and x30
//   from the stack frame at x29 (in the parent stack), thus continuing
//   unwinding at the swap call site instead of falling off the end of context stack.
use core::mem;
use stack::Stack;

pub const STACK_ALIGNMENT: usize = 16;

#[derive(Debug, Clone, Copy)]
pub struct StackPointer(*mut usize);

pub unsafe fn init(sp: &mut StackPointer,
                   f: unsafe extern "C" fn(usize, StackPointer) -> !) {
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

        # Set up the first part of our DWARF CFI linking stacks together. When
        # we reach this function from unwinding, x29 will be pointing at the bottom
        # of the parent linked stack. This link is set each time swap() is called.
        # When unwinding the frame corresponding to this function, a DWARF unwinder
        # will use x29+16 as the next call frame address, restore return address (x30)
        # from CFA-8 and restore x29 from CFA-16. This mirrors what the second half
        # of `swap_trampoline` does.
        .cfi_def_cfa x29, 16
        .cfi_offset x30, -8
        .cfi_offset x29, -16

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
        .cfi_def_cfa x29, 16
        .cfi_offset x30, -8
        .cfi_offset x29, -16
        nop
      "#
      : : : : "volatile")
  }

  #[naked]
  unsafe extern "C" fn trampoline_2() {
    asm!(
      r#"
        # Set up the second part of our DWARF CFI.
        # When unwinding the frame corresponding to this function, a DWARF unwinder
        # will restore x29 (and thus CFA of the first trampoline) from the stack slot.
        # This stack slot is updated every time swap() is called to point to the bottom
        # of the stack of the context switch just switched from.
        .cfi_def_cfa x29, 16
        .cfi_offset x30, -8
        .cfi_offset x29, -16

        # This nop is here so that the return address of the swap trampoline
        # doesn't point to the start of the symbol. This confuses gdb's backtraces,
        # causing them to think the parent function is trampoline_1 instead of
        # trampoline_2.
        nop

        # Call the provided function.
        ldr     x2, [sp, #16]
        blr     x2
      "#
      : : : : "volatile")
  }

  unsafe fn push(sp: &mut StackPointer, val: usize) {
    sp.0 = sp.0.offset(-1);
    *sp.0 = val
  }

  // We set up the stack in a somewhat special way so that to the unwinder it
  // looks like trampoline_1 has called trampoline_2, which has in turn called
  // swap::trampoline.
  //
  // There are 2 call frames in this setup, each containing the return address
  // followed by the x29 value for that frame. This setup supports unwinding
  // using DWARF CFI as well as the frame pointer-based unwinding used by tools
  // such as perf or dtrace.
  sp.push(0 as usize); // Padding to ensure the stack is properly aligned
  sp.push(f as usize); // Function that trampoline_2 should call

  // Call frame for trampoline_2. The CFA slot is updated by swap::trampoline
  // each time a context switch is performed.
  sp.push(trampoline_1 as usize + 4); // Return after the nop
  sp.push(0xdeaddeaddead0cfa);        // CFA slot

  // Call frame for swap::trampoline. We set up the x29 value to point to the
  // parent call frame.
  let frame = *sp;
  sp.push(trampoline_2 as usize + 4); // Entry point, skip initial nop
  sp.push(frame as usize);            // Pointer to parent call frame
}

#[inline(always)]
pub unsafe fn swap(arg: usize, new_sp: StackPointer,
                   new_stack: Option<&Stack>) -> (usize, StackPointer) {
  // Address of the topmost CFA stack slot.
  let mut dummy: usize = mem::uninitialized();
  let new_cfa = if let Some(new_stack) = new_stack {
    (new_stack.base() as *mut usize).offset(-4)
  } else {
    // Just pass a dummy pointer if we aren't linking the stack
    &mut dummy
  };

  #[naked]
  unsafe extern "C" fn trampoline() {
    asm!(
      r#"
        # Save the frame pointer and link register; the unwinder uses them to find
        # the CFA of the caller, and so they have to have the correct value immediately
        # after the call instruction that invoked the trampoline.
        stp     x29, x30, [sp, #-16]!
        .cfi_adjust_cfa_offset 16
        .cfi_rel_offset x30, 8
        .cfi_rel_offset x29, 0

        # Link the call stacks together by writing the current stack bottom
        # address to the CFA slot in the new stack.
        mov     x4, sp
        str     x4, [x3]

        # Pass the stack pointer of the old context to the new one.
        mov     x1, sp
        # Load stack pointer of the new context.
        mov     sp, x2

        # Load frame and instruction pointers of the new context.
        ldp     x29, x30, [sp], #16
        .cfi_adjust_cfa_offset -16
        .cfi_restore x29
        .cfi_restore x30

        # Return into the new context. Use `br` instead of a `ret` to avoid
        # return address mispredictions.
        br      x30
      "#
      : : : : "volatile")
  }

  let ret: usize;
  let ret_sp: *mut usize;
  asm!(
    r#"
      # Call the trampoline to switch to the new context.
      bl      ${2}
    "#
    : "={x0}" (ret)
      "={x1}" (ret_sp)
    : "s" (trampoline as usize)
      "{x0}" (arg)
      "{x2}" (new_sp.0)
      "{x3}" (new_cfa)
    :/*x0,   "x1",*/"x2",  "x3",  "x4",  "x5",  "x6",  "x7",
      "x8",  "x9",  "x10", "x11", "x12", "x13", "x14", "x15",
      "x16", "x17", "x18", "x19", "x20", "x21", "x22", "x23",
      "x24", "x25", "x26", "x27", "x28",/*fp,*/ "lr", /*sp,*/
      "v0",  "v1",  "v2",  "v3",  "v4",  "v5",  "v6",  "v7",
      "v8",  "v9",  "v10", "v11", "v12", "v13", "v14", "v15",
      "v16", "v17", "v18", "v19", "v20", "v21", "v22", "v23",
      "v24", "v25", "v26", "v27", "v28", "v29", "v30", "v31",
      "cc", "memory"
      // Ideally, we would set the LLVM "noredzone" attribute on this function
      // (and it would be propagated to the call site). Unfortunately, rustc
      // provides no such functionality. Fortunately, by a lucky coincidence,
      // the "alignstack" LLVM inline assembly option does exactly the same
      // thing on AArch64.
    : "volatile", "alignstack");
  (ret, StackPointer(ret_sp))
}
