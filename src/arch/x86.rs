// This file is part of libfringe, a low-level green threading library.
// Copyright (c) Nathan Zadoks <nathan@nathan7.eu>,
//               whitequark <whitequark@whitequark.org>
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

// To understand the machine code in this file, keep in mind these facts:
// * i686 SysV C ABI requires the stack to be aligned at function entry,
//   so that `%esp+4` is a multiple of 16. Aligned operands are a requirement
//   of SIMD instructions, and making this the responsibility of the caller
//   avoids having to maintain a frame pointer, which is necessary when
//   a function has to realign the stack from an unknown state.
// * i686 SysV C ABI passes the first argument on the stack. This is
//   unfortunate, because unlike every other architecture we can't reuse
//   `swap` for the initial call, and so we use a trampoline.
use stack::Stack;

pub const STACK_ALIGNMENT: usize = 16;

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
        subl   $$8, %esp
        .cfi_def_cfa_offset 8
        .cfi_offset %ebp, -8
        movl   %esp, %ebp
        .cfi_def_cfa_register %ebp
        # Call f.
        pushl  %eax
        calll  *12(%esp)

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
  push(&mut sp, trampoline as usize); // trampoline   / linked return address
  push(&mut sp, 0xdead0bbb);          // initial %ebp / linked %ebp
  sp
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
        # the stacks together later. We put them on stack because x86 doesn't
        # have enough registers.
        movl    %ebp, -8(%edx)
        movl    (%esp), %ebx
        movl    %ebx, -12(%edx)

        # Save frame pointer explicitly; the unwinder uses it to find CFA of
        # the caller, and so it has to have the correct value immediately after
        # the call instruction that invoked the trampoline.
        pushl   %ebp

        # Save stack pointer of the old context.
        movl    %esp, (%esi)
        # Load stack pointer of the new context.
        movl    %edx, %esp

        # Load frame and instruction pointers of the new context.
        popl    %ebp
        popl    %ebx

        # Put the frame and instruction pointers into the trampoline stack frame,
        # making it appear to return right after the call instruction that invoked
        # this trampoline. This is done after the loads above, since on the very first
        # swap, the saved %ebp/%ebx intentionally alias 0(%edi)/4(%edi).
        movl    -8(%edx), %esi
        movl    %esi, 0(%edi)
        movl    -12(%edx), %esi
        movl    %esi, 4(%edi)

        # Return into new context.
        jmpl    *%ebx
      "#
      : : : : "volatile")
  }

  let ret: usize;
  asm!(
    r#"
      # Push instruction pointer of the old context and switch to
      # the new context.
      call    ${1:c}
    "#
    : "={eax}" (ret)
    : "s" (trampoline as usize)
      "{eax}" (arg)
      "{esi}" (old_sp)
      "{edx}" (new_sp.0)
      "{edi}" (new_cfa)
    :/*"eax",*/"ebx", "ecx",  "edx",  "esi",  "edi",/*"ebp",  "esp",*/
      "mm0",  "mm1",  "mm2",  "mm3",  "mm4",  "mm5",  "mm6",  "mm7",
      "xmm0", "xmm1", "xmm2", "xmm3", "xmm4", "xmm5", "xmm6", "xmm7",
      "cc", "dirflag", "fpsr", "flags", "memory"
    : "volatile");
  ret
}
