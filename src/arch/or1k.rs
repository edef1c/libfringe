// This file is part of libfringe, a low-level green threading library.
// Copyright (c) edef <edef@edef.eu>,
//               whitequark <whitequark@whitequark.org>
// See the LICENSE file included in this distribution.

// To understand the machine code in this file, keep in mind these facts:
// * OR1K C ABI has a "red zone": 128 bytes under the top of the stack
//   that is defined to be unmolested by signal handlers, interrupts, etc.
//   Leaf functions can use the red zone without adjusting r1 or r2.
// * OR1K C ABI passes the first argument in r3. We also use r3 to pass a value
//   while swapping context; this is an arbitrary choice
//   (we clobber all registers and could use any of them) but this allows us
//   to reuse the swap function to perform the initial call.
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
// A high-level overview of the function of the trampolines is:
// * The 2nd init trampoline puts a controlled value (written in swap to `new_cfa`)
//   into r13.
// * The 1st init trampoline tells the unwinder to set r1 to r13, thus continuing
//   unwinding at the swap call site instead of falling off the end of context stack.
// * The 1st init trampoline together with the swap trampoline also restore r2
//   when unwinding as well as returning normally, because LLVM does not do it for us.
use stack::Stack;

#[derive(Debug, Clone)]
pub struct StackPointer(*mut usize);

pub unsafe fn init(stack: &Stack, f: unsafe extern "C" fn(usize) -> !) -> StackPointer {
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

        # Set up the first part of our DWARF CFI linking stacks together.
        # When unwinding the frame corresponding to this function, a DWARF unwinder
        # will use r13 as the next call frame address, restore return address (r9)
        # from CFA-4 and restore stack pointer (r2) from CFA-8.
        # This mirrors what the second half of `swap_trampoline` does.
        .cfi_def_cfa r13, 0
        .cfi_offset r2, -8
        .cfi_offset r9, -4
        # Call the next trampoline.
        l.j     ${0}
        l.nop

      .Lend:
      .size __morestack, .Lend-__morestack
      "#
      : : "s" (trampoline_2 as usize) : "memory" : "volatile")
  }

  #[naked]
  unsafe extern "C" fn trampoline_2() {
    asm!(
      r#"
        # Set up the second part of our DWARF CFI.
        # When unwinding the frame corresponding to this function, a DWARF unwinder
        # will restore r13 (and thus CFA of the first trampoline) from the stack slot.
        .cfi_offset r13, 4
        # Call the provided function.
        l.lwz   r9, 0(r1)
        l.jr    r9
        l.nop
      "#
      : : : "memory" : "volatile")
  }

  unsafe fn push(sp: &mut StackPointer, val: usize) {
    sp.0 = sp.0.offset(-1);
    *sp.0 = val
  }

  let mut sp = StackPointer(stack.base() as *mut usize);
  push(&mut sp, 0xdead0cfa);            // CFA slot
  push(&mut sp, f as usize);            // function
  let rsp = sp.clone();
  push(&mut sp, trampoline_1 as usize); // saved r9
  push(&mut sp, 0xdeadbbbb);            // saved r2
  rsp
}

#[inline(always)]
pub unsafe fn swap(arg: usize, old_sp: &mut StackPointer, new_sp: &StackPointer,
                   new_stack: &Stack) -> usize {
  // Address of the topmost CFA stack slot.
  let new_cfa = (new_stack.base() as *mut usize).offset(-1);

  #[naked]
  unsafe extern "C" fn trampoline() {
    asm!(
      r#"
        # Save instruction pointer of the old context.
        l.sw    -4(r1), r9

        # Save frame pointer explicitly; the unwinder uses it to find CFA of
        # the caller, and so it has to have the correct value immediately after
        # the call instruction that invoked the trampoline.
        l.sw    -8(r1), r2

        # Remember stack pointer of the old context, in case r5==r4.
        l.or    r13, r0, r1
        # Load stack pointer of the new context.
        l.lwz   r1, 0(r5)
        # Save stack pointer of the old context.
        l.sw    0(r4), r13

        # Restore frame pointer of the new context.
        l.lwz   r2, -8(r1)

        # Return into the new context.
        l.lwz   r9, -4(r1)
        l.jr    r9
        l.nop
      "#
      : : : "memory" : "volatile")
  }

  let ret: usize;
  asm!(
    r#"
      # Link the call stacks together.
      l.sw    0(r6), r1
      # Put instruction pointer of the old context into r9 and switch to
      # the new context.
      l.jal   ${1}
      l.nop
    "#
    : "={r3}" (ret)
    : "s" (trampoline as usize)
      "{r3}" (arg)
      "{r4}" (old_sp)
      "{r5}" (new_sp)
      "{r6}" (new_cfa)
    :                      "r3",  "r4",  "r5",  "r6",  "r7",
      "r8",  "r9",  "r10", "r11", "r12", "r13", "r14", "r15",
      "r16", "r17", "r18", "r19", "r20", "r21", "r22", "r23",
      "r24", "r25", "r26", "r27", "r28", "r29", "r30", "r31",
      "flags", "memory"
    : "volatile");
  ret
}
