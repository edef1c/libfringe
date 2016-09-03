// This file is part of libfringe, a low-level green threading library.
// Copyright (c) Nathan Zadoks <nathan@nathan7.eu>,
//               whitequark <whitequark@whitequark.org>
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

// To understand the code in this file, keep in mind these two facts:
// * x86_64 SysV C ABI has a "red zone": 128 bytes under the top of the stack
//   that is defined to be unmolested by signal handlers, interrupts, etc.
//   Leaf functions can use the red zone without adjusting rsp or rbp.
// * x86_64 SysV C ABI requires the stack to be aligned at function entry,
//   so that (%rsp+8) is a multiple of 16. Aligned operands are a requirement
//   of SIMD instructions, and making this the responsibility of the caller
//   avoids having to maintain a frame pointer, which is necessary when
//   a function has to realign the stack from an unknown state.
// * x86_64 SysV C ABI passes the first argument in %rdi. We also use %rdi
//   to pass a value while swapping context; this is an arbitrary choice
//   (we clobber all registers and could use any of them) but this allows us
//   to reuse the swap function to perform the initial call.
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
        subq   $$16, %rsp
        .cfi_def_cfa_offset 16
        .cfi_offset %rbp, -16
        movq   %rsp, %rbp
        .cfi_def_cfa_register %rbp
        # Call f.
        callq  *16(%rsp)

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
  push(&mut sp, 0 as usize);          // alignment
  push(&mut sp, f as usize);          // function
  push(&mut sp, trampoline as usize); // trampoline   / linked return address
  push(&mut sp, 0xdeaddeaddead0bbb);  // initial %rbp / linked %rbp
  sp
}

#[inline(always)]
pub unsafe fn swap(arg: usize, old_sp: *mut StackPointer, new_sp: StackPointer,
                   new_stack: &Stack) -> usize {
  // Address of the topmost CFA stack slot.
  let new_cfa = (new_stack.base() as *mut usize).offset(-4);

  #[naked]
  unsafe extern "C" fn trampoline() {
    asm!(
      r#"
        # Remember the frame and instruction pointers in the callee, to link
        # the stacks together later.
        movq    %rbp, %r8
        movq    (%rsp), %r9

        # Save frame pointer explicitly; the unwinder uses it to find CFA of
        # the caller, and so it has to have the correct value immediately after
        # the call instruction that invoked the trampoline.
        pushq   %rbp

        # Save stack pointer of the old context.
        movq    %rsp, (%rsi)
        # Load stack pointer of the new context.
        movq    %rdx, %rsp

        # Load frame and instruction pointers of the new context.
        popq    %rbp
        popq    %rbx

        # Put the frame and instruction pointers into the trampoline stack frame,
        # making it appear to return right after the call instruction that invoked
        # this trampoline. This is done after the loads above, since on the very first
        # swap, the saved %rbp/%rbx intentionally alias 0(%rcx)/8(%rcx).
        movq    %r8, 0(%rcx)
        movq    %r9, 8(%rcx)

        # Return into new context.
        jmpq    *%rbx
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
    : "={rdi}" (ret)
    : "s" (trampoline as usize)
      "{rdi}" (arg)
      "{rsi}" (old_sp)
      "{rdx}" (new_sp.0)
      "{rcx}" (new_cfa)
    : "rax",   "rbx",   "rcx",   "rdx",   "rsi", /*"rdi",   "rbp",   "rsp",*/
      "r8",    "r9",    "r10",   "r11",   "r12",   "r13",   "r14",   "r15",
      "mm0",   "mm1",   "mm2",   "mm3",   "mm4",   "mm5",   "mm6",   "mm7",
      "xmm0",  "xmm1",  "xmm2",  "xmm3",  "xmm4",  "xmm5",  "xmm6",  "xmm7",
      "xmm8",  "xmm9",  "xmm10", "xmm11", "xmm12", "xmm13", "xmm14", "xmm15",
      "xmm16", "xmm17", "xmm18", "xmm19", "xmm20", "xmm21", "xmm22", "xmm23",
      "xmm24", "xmm25", "xmm26", "xmm27", "xmm28", "xmm29", "xmm30", "xmm31",
      "cc", "dirflag", "fpsr", "flags", "memory"
      // Ideally, we would set the LLVM "noredzone" attribute on this function
      // (and it would be propagated to the call site). Unfortunately, rustc
      // provides no such functionality. Fortunately, by a lucky coincidence,
      // the "alignstack" LLVM inline assembly option does exactly the same
      // thing on x86_64.
    : "volatile", "alignstack");
  ret
}
