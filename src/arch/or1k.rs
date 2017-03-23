// This file is part of libfringe, a low-level green threading library.
// Copyright (c) edef <edef@edef.eu>,
//               whitequark <whitequark@whitequark.org>
//               Amanieu d'Antras <amanieu@gmail.com>
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

// To understand the machine code in this file, keep in mind these facts:
// * OR1K C ABI has a "red zone": 128 bytes under the top of the stack
//   that is defined to be unmolested by signal handlers, interrupts, etc.
//   Leaf functions can use the red zone without adjusting r1 or r2.
// * OR1K C ABI passes the first argument in r3. We also use r3 to pass a value
//   while swapping context; this is an arbitrary choice
//   (we clobber all registers and could use any of them) but this allows us
//   to reuse the swap function to perform the initial call. We do the same
//   thing with r4 to pass the stack pointer to the new context.
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
//   into r2. This is then used as the CFA for the 1st trampoline.
// * This controlled value points to the bottom of the stack of the parent context,
//   which holds the saved r2 and r9 from the call to swap().
// * The 1st init trampoline tells the unwinder to restore r2 and r9
//   from the stack frame at r2 (in the parent stack), thus continuing
//   unwinding at the swap call site instead of falling off the end of context stack.
use core::mem;
use arch::StackPointer;
use unwind;

pub const STACK_ALIGNMENT: usize = 4;

pub unsafe fn init(stack_base: *mut u8, f: unsafe fn(usize, StackPointer)) -> StackPointer {
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
        # we reach this function from unwinding, r2 will be pointing at the bottom
        # of the parent linked stack. This link is set each time swap() is called.
        # When unwinding the frame corresponding to this function, a DWARF unwinder
        # will use r2+8 as the next call frame address, restore r2 from CFA-4 and
        # restore return address (r9) from CFA-8. This mirrors what the second half
        # of `swap_trampoline` does.
        .cfi_def_cfa r2, 8
        .cfi_offset r2, -4
        .cfi_offset r9, -8

        # This nop is here so that the initial swap doesn't return to the start
        # of the trampoline, which confuses the unwinder since it will look for
        # frame information in the previous symbol rather than this one. It is
        # never actually executed.
        l.nop

      .Lend:
      .size __morestack, .Lend-__morestack
      "#
      : : : : "volatile")
  }

  #[naked]
  unsafe extern "C" fn trampoline_2() {
    asm!(
      r#"
        # Set up the second part of our DWARF CFI.
        # When unwinding the frame corresponding to this function, a DWARF unwinder
        # will restore r2 (and thus CFA of the first trampoline) from the stack slot.
        # This stack slot is updated every time swap() is called to point to the bottom
        # of the stack of the context switch just switched from.
        .cfi_def_cfa r2, 8
        .cfi_offset r2, -4
        .cfi_offset r9, -8

        # This nop is here so that the return address of the swap trampoline
        # doesn't point to the start of the symbol. This confuses gdb's backtraces,
        # causing them to think the parent function is trampoline_1 instead of
        # trampoline_2.
        l.nop

        # Call unwind_wrapper with the provided function and the stack base address.
        l.addi  r5, r1, 12
        l.lwz   r6, 8(r1)
        l.jal   ${0}
        l.nop

        # Restore the stack pointer of the parent context. No CFI adjustments
        # are needed since we have the same stack frame as trampoline_1.
        l.lwz   r1, 0(r1)

        # Load frame and instruction pointers of the parent context.
        l.lwz   r2, -4(r1)
        l.lwz   r9, -8(r1)

        # If the returned value is nonzero, trigger an unwind in the parent
        # context with the given exception object.
        l.or    r4, r0, r11
        l.sfeq  r11, r0
        l.bf    ${1}

        # Clear the stack pointer. We can't call into this context any more once
        # the function has returned.
        l.or    r4, r0, r0

        # Return into the parent context.
        l.jr    r9
        l.nop
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
  // followed by the r2 value for that frame. This setup supports unwinding
  // using DWARF CFI as well as the frame pointer-based unwinding used by tools
  // such as perf or dtrace.
  let mut sp = StackPointer::new(stack_base);

  sp.push(f as usize); // Function that trampoline_2 should call

  // Call frame for trampoline_2. The CFA slot is updated by swap::trampoline
  // each time a context switch is performed.
  sp.push(0xdead0cfa);                // CFA slot
  sp.push(trampoline_1 as usize + 4); // Return after the nop

  // Call frame for swap::trampoline. We set up the r2 value to point to the
  // parent call frame.
  let frame = sp;
  sp.push(frame.offset(0) as usize);  // Pointer to parent call frame
  sp.push(trampoline_2 as usize + 4); // Entry point, skip initial nop

  // The last two values are read by the swap trampoline and are actually in the
  // red zone and not below the stack pointer.
  frame
}

#[inline(always)]
pub unsafe fn swap_link(arg: usize, new_sp: StackPointer,
                        new_stack_base: *mut u8) -> (usize, Option<StackPointer>) {
  #[naked]
  unsafe extern "C" fn trampoline() {
    asm!(
      r#"
        # Save the frame pointer and link register; the unwinder uses them to find
        # the CFA of the caller, and so they have to have the correct value immediately
        # after the call instruction that invoked the trampoline.
        l.sw    -4(r1), r2
        l.sw    -8(r1), r9
        .cfi_offset r2, -4
        .cfi_offset r9, -8

        # Link the call stacks together by writing the current stack bottom
        # address to the CFA slot in the new stack.
        l.addi  r7, r1, -8
        l.sw    -8(r6), r7

        # Pass the stack pointer of the old context to the new one.
        l.or    r4, r0, r1
        # Load stack pointer of the new context.
        l.or    r1, r0, r5

        # Load frame and instruction pointers of the new context.
        l.lwz   r2, -4(r1)
        l.lwz   r9, -8(r1)

        # Return into the new context.
        l.jr    r9
        l.nop
      "#
      : : : : "volatile")
  }

  let ret: usize;
  let ret_sp: usize;
  asm!(
    r#"
      # Call the trampoline to switch to the new context.
      l.jal   ${2}
      l.nop
    "#
    : "={r3}" (ret)
      "={r4}" (ret_sp)
    : "s" (trampoline as usize)
      "{r3}" (arg)
      "{r5}" (*new_sp.0)
      "{r6}" (new_stack_base)
    :/*"r0", "r1",  "r2",  "r3",  "r4",*/"r5",  "r6",  "r7",
      "r8",  "r9",  "r10", "r11", "r12", "r13", "r14", "r15",
      "r16", "r17", "r18", "r19", "r20", "r21", "r22", "r23",
      "r24", "r25", "r26", "r27", "r28", "r29", "r30", "r31",
      "cc", "memory"
    : "volatile");
  (ret, mem::transmute(ret_sp))
}

#[inline(always)]
pub unsafe fn swap(arg: usize, new_sp: StackPointer) -> (usize, StackPointer) {
  // This is identical to swap_link, but without the write to the CFA slot.
  #[naked]
  unsafe extern "C" fn trampoline() {
    asm!(
      r#"
        l.sw    -4(r1), r2
        l.sw    -8(r1), r9
        .cfi_offset r2, -4
        .cfi_offset r9, -8
        l.or    r4, r0, r1
        l.or    r1, r0, r5
        l.lwz   r2, -4(r1)
        l.lwz   r9, -8(r1)
        l.jr    r9
        l.nop
      "#
      : : : : "volatile")
  }

  let ret: usize;
  let ret_sp: usize;
  asm!(
    r#"
      l.jal   ${2}
      l.nop
    "#
    : "={r3}" (ret)
      "={r4}" (ret_sp)
    : "s" (trampoline as usize)
      "{r3}" (arg)
      "{r5}" (*new_sp.0)
    :/*"r0", "r1",  "r2",  "r3",  "r4",*/"r5",  "r6",  "r7",
      "r8",  "r9",  "r10", "r11", "r12", "r13", "r14", "r15",
      "r16", "r17", "r18", "r19", "r20", "r21", "r22", "r23",
      "r24", "r25", "r26", "r27", "r28", "r29", "r30", "r31",
      "cc", "memory"
    : "volatile");
  (ret, mem::transmute(ret_sp))
}

#[inline(always)]
pub unsafe fn unwind(new_sp: StackPointer, new_stack_base: *mut u8) {
  // Argument to pass to start_unwind, based on the stack base address.
  let arg = unwind::unwind_arg(new_stack_base);

  // This is identical to swap_link, except that it performs a tail call to
  // start_unwind instead of returning into the target context.
  #[naked]
  unsafe extern "C" fn trampoline() {
    asm!(
      r#"
        l.sw    -4(r1), r2
        l.sw    -8(r1), r9
        .cfi_offset r2, -4
        .cfi_offset r9, -8
        l.addi  r7, r1, -8
        l.sw    -8(r6), r7
        l.or    r1, r0, r5
        l.lwz   r2, -4(r1)
        l.lwz   r9, -8(r1)

        # Jump to the start_unwind function, which will force a stack unwind in
        # the target context. This will eventually return to us through the
        # stack link.
        l.j     ${0}
        l.nop
      "#
      :
      : "s" (unwind::start_unwind as usize)
      : : "volatile")
  }

  asm!(
    r#"
      # Call the trampoline to switch to the new context.
      l.jal   ${0}
      l.nop
    "#
    :
    : "s" (trampoline as usize)
      "{r3}" (arg)
      "{r5}" (*new_sp.0)
      "{r6}" (new_stack_base)
    :/*"r0", "r1",  "r2",*/"r3",  "r4",  "r5",  "r6",  "r7",
      "r8",  "r9",  "r10", "r11", "r12", "r13", "r14", "r15",
      "r16", "r17", "r18", "r19", "r20", "r21", "r22", "r23",
      "r24", "r25", "r26", "r27", "r28", "r29", "r30", "r31",
      "cc", "memory"
    : "volatile");
}
