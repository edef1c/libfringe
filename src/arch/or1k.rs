// This file is part of libfringe, a low-level green threading library.
// Copyright (c) edef <edef@edef.eu>,
//               whitequark <whitequark@whitequark.org>
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
//   to reuse the swap function to perform the initial call.
use stack::Stack;

pub const STACK_ALIGNMENT: usize = 4;

#[derive(Debug, Clone, Copy)]
pub struct StackPointer(*mut usize);

pub unsafe fn init(stack: &Stack, f: unsafe extern "C" fn(usize) -> !) -> StackPointer {
  #[naked]
  unsafe extern "C" fn trampoline() {
    asm!(
      r#"
        # gdb has a hardcoded check that rejects backtraces where frame addresses
        # do not monotonically decrease. It is turned off if the function is called
        # "__morestack" and that is hardcoded. So, to make gdb backtraces match
        # the actual unwinder behavior, we call ourselves "__morestack" and mark
        # the symbol as local; it shouldn't interfere with anything.
      __morestack:
      .local __morestack

        # When a normal function is entered, the return address is pushed onto the stack,
        # and the first thing it does is pushing the frame pointer. The init trampoline
        # is not a normal function; on entry the stack pointer is one word above the place
        # where the return address should be, and right under it the return address as
        # well as the stack pointer are already pre-filled. So, simply move the stack
        # pointer where it belongs; and add CFI just like in any other function prologue.
        l.addi  r1, r1, -8
        .cfi_def_cfa_offset 8
        .cfi_offset r2, -8
        l.or    r2, r1, r0
        .cfi_def_cfa_register r2
        # Call f.
        l.lwz   r9, 8(r1)
        l.jr    r9
        l.nop

      .Lend:
      .size __morestack, .Lend-__morestack
      "#
      : : : : "volatile")
  }

  unsafe fn push(sp: &mut StackPointer, val: usize) {
    sp.0 = sp.0.offset(-1);
    *sp.0 = val
  }

  let mut sp = StackPointer(stack.base() as *mut usize);
  push(&mut sp, f as usize);          // function
  let rsp = sp;
  push(&mut sp, trampoline as usize); // trampoline   / linked return address
  push(&mut sp, 0xdead0bbb);          // initial %ebp / linked %ebp
  rsp
}

#[inline(always)]
pub unsafe fn swap(arg: usize, old_sp: *mut StackPointer, new_sp: StackPointer,
                   new_stack: &Stack) -> usize {
  // Address of the topmost CFA stack slot.
  let new_cfa = (new_stack.base() as *mut usize).offset(-3);

  #[naked]
  unsafe extern "C" fn trampoline() {
    asm!(
      r#"
        # Remember the frame and instruction pointers in the callee, to link
        # the stacks together later.
        l.or    r18, r2, r0
        l.or    r19, r9, r0

        # Save instruction pointer of the old context.
        l.sw    -4(r1), r9

        # Save frame pointer explicitly; the unwinder uses it to find CFA of
        # the caller, and so it has to have the correct value immediately after
        # the call instruction that invoked the trampoline.
        l.sw    -8(r1), r2

        # Save stack pointer of the old context.
        l.sw    0(r4), r1
        # Load stack pointer of the new context.
        l.or    r1, r0, r5

        # Load frame and instruction pointers of the new context.
        l.lwz   r2, -8(r1)
        l.lwz   r9, -4(r1)

        # Put the frame and instruction pointers into the trampoline stack frame,
        # making it appear to return right after the call instruction that invoked
        # this trampoline. This is done after the loads above, since on the very first
        # swap, the saved r2/r9 intentionally alias 0(r6)/4(r6).
        l.sw    0(r6), r18
        l.sw    4(r6), r19

        # Return into new context.
        l.jr    r9
        l.nop
      "#
      : : : : "volatile")
  }

  let ret: usize;
  asm!(
    r#"
      # Push instruction pointer of the old context and switch to
      # the new context.
      l.jal   ${1}
      l.nop
    "#
    : "={r3}" (ret)
    : "s" (trampoline as usize)
      "{r3}" (arg)
      "{r4}" (old_sp)
      "{r5}" (new_sp.0)
      "{r6}" (new_cfa)
    :/*"r0", "r1",  "r2",  "r3",*/"r4",  "r5",  "r6",  "r7",
      "r8",  "r9",  "r10", "r11", "r12", "r13", "r14", "r15",
      "r16", "r17", "r18", "r19", "r20", "r21", "r22", "r23",
      "r24", "r25", "r26", "r27", "r28", "r29", "r30", "r31",
      "flags", "memory"
    : "volatile");
  ret
}
