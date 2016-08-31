// This file is part of libfringe, a low-level green threading library.
// Copyright (c) Nathan Zadoks <nathan@nathan7.eu>,
//               whitequark <whitequark@whitequark.org>
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
//   into r10.
// * The 1st init trampoline tells the unwinder to set sp to r10, thus continuing
//   unwinding at the swap call site instead of falling off the end of context stack.
// * The 1st init trampoline together with the swap trampoline also restore r11 (fp)
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

        # Set up the first part of our ARM EHABI CFI linking stacks together.
        # When unwinding the frame corresponding to this function, a ARM EHABI unwinder
        # will use r10 as the next call frame address, restore return address (lr)
        # from CFA-4 and restore frame pointer (fp) from CFA-8.
        # This mirrors what the second half of `swap_trampoline` does.
      # .setfp  r10, sp, #0
      # .save   {fp, lr}
        # Call the next trampoline.
        b       ${0}

      .Lend:
      .size __morestack, .Lend-__morestack
      "#
      : : "s" (trampoline_2 as usize) : "memory" : "volatile")
  }

  #[naked]
  unsafe extern "C" fn trampoline_2() {
    asm!(
      r#"
        # Set up the second part of our ARM EHABI CFI.
        # When unwinding the frame corresponding to this function, a ARM EHABI unwinder
        # will restore r10 (and thus CFA of the first trampoline) from the stack slot.
      # .setfp  sp, sp, #12
      # .save   {r10}
        # Call the provided function.
        ldr     r8, [sp], #-8
        blx     r8
      "#
      : : : "memory" : "volatile")
  }

  unsafe fn push(sp: &mut StackPointer, val: usize) {
    sp.0 = sp.0.offset(-1);
    *sp.0 = val
  }

  let mut sp = StackPointer(stack.base() as *mut usize);
  push(&mut sp, 0xdead0cfa);            // CFA slot
  push(&mut sp, trampoline_1 as usize); // saved lr in trampoline_2
  push(&mut sp, f as usize);            // function
  push(&mut sp, trampoline_1 as usize); // saved pc
  push(&mut sp, 0xdeadbbbb);            // saved fp
  sp
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
        # Save instruction and frame pointers of the old context.
        push    {fp, lr}

        # Remember stack pointer of the old context, in case r1==r2.
        mov     r10, sp
        # Load stack pointer of the new context.
        ldr     sp, [r2]
        # Save stack pointer of the old context.
        str     r10, [r1]

        # Return into the new context.
        pop     {fp, pc}
      "#
      : : : "memory" : "volatile")
  }

  let ret: usize;
  asm!(
    r#"
      # Link the call stacks together.
      str     sp, [r3]
      # Put instruction pointer of the old context into lr and switch to
      # the new context.
      bl      ${1}
    "#
    : "={r0}" (ret)
    : "s" (trampoline as usize)
      "{r0}" (arg)
      "{r1}" (old_sp)
      "{r2}" (new_sp)
      "{r3}" (new_cfa)
    :/*r0,*/ "r1",  "r2",  "r3",  "r4",  "r5",  "r6",  "r7",
      "r8",  "r9",  "r10",/*r11,*/"r12",/*sp,*/ "lr", /*pc,*/
      "d0",  "d1",  "d2",  "d3",  "d4",  "d5",  "d6",  "d7",
      "d8",  "d9",  "d10", "d11", "d12", "d13", "d14", "d15",
      "d16", "d17", "d18", "d19", "d20", "d21", "d22", "d23",
      "d24", "d25", "d26", "d27", "d28", "d29", "d30", "d31",
      "flags", "memory"
    : "volatile");
  ret
}
