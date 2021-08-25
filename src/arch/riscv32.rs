// This file is part of libfringe, a low-level green threading library.
// Copyright (c) M-Labs Limited,
//               occheung <dc@m-labs.hk>,
//               edef <edef@edef.eu>,
//               whitequark <whitequark@whitequark.org>
//               Amanieu d'Antras <amanieu@gmail.com>
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

// To understand the machine code in this file, keep in mind these facts:
// * RISCV does not have a red zone.
// * RISCV requires that the stack pointer (sp) should be 16 bytes aligned. It can
//   be overwritten by compiler options, but let's assume it stays 16 bytes aligned.
// * RISCV passes the first argument in a0. We also use a0 to pass a value
//   while swapping context; this is an arbitrary choice
//   (we clobber all registers and could use any of them) but this allows us
//   to reuse the swap function to perform the initial call. We do the same
//   thing with a1 to pass the stack pointer to the new context.
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
//   into fp. This is then used as the CFA for the 1st trampoline.
// * This controlled value points to the bottom of the stack of the parent context,
//   which holds the saved fp and ra from the call to swap().
// * The 1st init trampoline tells the unwinder to restore fp and ra
//   from the stack frame at fp (in the parent stack), thus continuing
//   unwinding at the swap call site instead of falling off the end of context stack.
use core::mem;
use stack::Stack;

pub const STACK_ALIGNMENT: usize = 16;

#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct StackPointer(*mut usize);

pub unsafe fn init(stack: &Stack, f: unsafe extern "C" fn(usize, StackPointer) -> !) -> StackPointer {
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
        # we reach this function from unwinding, fp will be pointing at the bottom
        # of the parent linked stack. This link is set each time swap() is called.
        # When unwinding the frame corresponding to this function, a DWARF unwinder
        # will use fp+16 as the next call frame address, restore fp from CFA-12 and
        # restore return address (ra) from CFA-16. This mirrors what the second half
        # of `swap_trampoline` does.
        .cfi_def_cfa fp, 16
        .cfi_offset fp, -12
        .cfi_offset ra, -16

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

  #[naked]
  unsafe extern "C" fn trampoline_2() {
    asm!(
      r#"
        # Set up the second part of our DWARF CFI.
        # When unwinding the frame corresponding to this function, a DWARF unwinder
        # will restore fp (and thus CFA of the first trampoline) from the stack slot.
        # This stack slot is updated every time swap() is called to point to the bottom
        # of the stack of the context switch just switched from.
        .cfi_def_cfa fp, 16
        .cfi_offset fp, -12
        .cfi_offset ra, -16

        # This nop is here so that the return address of the swap trampoline
        # doesn't point to the start of the symbol. This confuses gdb's backtraces,
        # causing them to think the parent function is trampoline_1 instead of
        # trampoline_2.
        nop

        # Call the provided function.
        # The function address is at offset 8 because sp will increment by 16 at the end
        # of the swap, before calling trampoline_2.
        lw      a2, 8(sp)
        jalr    a2
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
  // followed by the fp value for that frame. This setup supports unwinding
  // using DWARF CFI as well as the frame pointer-based unwinding used by tools
  // such as perf or dtrace.
  let mut sp = StackPointer(stack.base() as *mut usize);

  push(&mut sp, 0);          // Make sure that the stack pointer is 16 bytes aligned
  push(&mut sp, f as usize); // Function that trampoline_2 should call

  // Call frame for trampoline_2. The CFA slot is updated by swap::trampoline
  // each time a context switch is performed.
  push(&mut sp, 0xdead0cfa);                // CFA slot
  push(&mut sp, trampoline_1 as usize + 4); // Return after the nop
  let frame = sp;

  push(&mut sp, 0); // Alignment
  push(&mut sp, 0); // Alignment

  // Call frame for swap::trampoline. We set up the fp value to point to the
  // parent call frame.
  push(&mut sp, frame.0 as usize);          // Pointer to parent call frame
  push(&mut sp, trampoline_2 as usize + 4); // Entry point, skip initial nop
  sp
}

#[inline(always)]
pub unsafe fn swap(arg: usize, new_sp: StackPointer,
                   new_stack: Option<&Stack>) -> (usize, StackPointer) {
  // Address of the topmost CFA stack slot.
  let mut dummy: usize = mem::uninitialized();
  let new_cfa = if let Some(new_stack) = new_stack {
    (new_stack.base() as *mut usize).offset(-3)
  } else {
    // Just pass a dummy pointer if we aren't linking the stack
    &mut dummy
  };

  #[naked]
  unsafe extern "C" fn trampoline() {
    asm!(
      r#"
        # Save the frame pointer and return address by allocating 16 bytes
        # from the stack; unwinder uses them to find the CFA of the caller,
        # and so they have to have the correct value immediately after the
        # call instruction that invoked the trampoline.
        sw      fp, -12(sp)
        sw      ra, -16(sp)
        addi    sp, sp, -16
        .cfi_offset fp, 4
        .cfi_offset ra, 0

        # Link the call stacks together by writing the current stack bottom
        # address to the CFA slot in the new stack.
        sw      sp, 0(a3)

        # Pass the stack pointer of the old context to the new one.
        or      a1, zero, sp
        # Load stack pointer of the new context.
        or      sp, zero, a2
        # Deallocate the 16 bytes
        addi    sp, sp, 16

        # Restore frame pointer and return address of the new context.
        # Load frame and instruction pointers of the new context.
        lw      fp, -12(sp)
        lw      ra, -16(sp)

        # Return into the new context.
        jr      ra
      "#
      : : : : "volatile")
  }

  let ret: usize;
  let ret_sp: *mut usize;
  asm!(
    r#"
      # Call the trampoline to switch to the new context.
      jal     ${2}
    "#
    : "={a0}" (ret)
      "={a1}" (ret_sp)
    : "s" (trampoline as usize)
      "{a0}" (arg)
      "{a2}" (new_sp.0)
      "{a3}" (new_cfa)
    :/*"zero",*/"ra",/*"sp","gp","tp",*/"t0","t1","t2",
    /*"fp",*/"s1",/*"a0", "a1"*/"a2", "a3", "a4", "a5",
      "a6", "a7", "s2",  "s3",  "s4", "s5", "s6", "s7",
      "s8", "s9", "s10", "s11", "t3", "t4", "t5", "t6",
      "memory"
    : "volatile");
  (ret, StackPointer(ret_sp))
}
