// This file is part of libfringe, a low-level green threading library.
// Copyright (c) edef <edef@edef.eu>,
//               whitequark <whitequark@whitequark.org>
// See the LICENSE file included in this distribution.

//! To understand the code in this file, keep in mind these two facts:
//! * x86_64 SysV C ABI has a "red zone": 128 bytes under the top of the stack
//!   that is defined to be unmolested by signal handlers, interrupts, etc.
//!   Leaf functions can use the red zone without adjusting rsp or rbp.
//! * x86_64 SysV C ABI requires the stack to be aligned at function entry,
//!   so that (%rsp+8) is a multiple of 16. Aligned operands are a requirement
//!   of SIMD instructions, and making this the responsibility of the caller
//!   avoids having to maintain a frame pointer, which is necessary when
//!   a function has to realign the stack from an unknown state.
//! * x86_64 SysV C ABI passes the first argument in %rdi. We also use %rdi
//!   to pass a value while swapping context; this is an arbitrary choice
//!   (we clobber all registers and could use any of them) but this allows us
//!   to reuse the swap function to perform the initial call.

use stack::Stack;

#[derive(Debug)]
pub struct StackPointer(*mut usize);

impl StackPointer {
  unsafe fn new(stack: &Stack) -> StackPointer {
    StackPointer(stack.top() as *mut usize)
  }

  unsafe fn push(&mut self, val: usize) {
    self.0 = self.0.offset(-1);
    *self.0 = val
  }
}

pub unsafe fn init(stack: &Stack, f: unsafe extern "C" fn(usize) -> !) -> StackPointer {
  let mut sp = StackPointer::new(stack);
  sp.push(0); // alignment
  sp.push(f as usize);
  sp
}

#[inline(always)]
pub unsafe fn swap(arg: usize, old_sp: &mut StackPointer, new_sp: &StackPointer) -> usize {
  let ret: usize;
  asm!(
    r#"
      # Save frame pointer explicitly; LLVM doesn't spill it even if it is
      # marked as clobbered.
      pushq   %rbp
      # Push instruction pointer of the old context and switch to
      # the new context.
      call    1f
      # Restore frame pointer.
      popq    %rbp
      # Continue executing old context.
      jmp     2f

    1:
      # Remember stack pointer of the old context, in case %rdx==%rsi.
      movq    %rsp, %rax
      # Load stack pointer of the new context.
      movq    (%rdx), %rsp
      # Save stack pointer of the old context.
      movq    %rax, (%rsi)

      # Pop instruction pointer of the new context (placed onto stack by
      # the call above) and jump there; don't use `ret` to avoid return
      # address mispredictions (~8ns on Ivy Bridge).
      popq    %rax
      jmpq    *%rax
    2:
    "#
    : "={rdi}" (ret)
    : "{rdi}" (arg)
      "{rsi}" (old_sp)
      "{rdx}" (new_sp)
    : "rax",   "rbx",   "rcx",   "rdx",   "rsi",   "rdi", //"rbp",   "rsp",
      "r8",    "r9",    "r10",   "r11",   "r12",   "r13",   "r14",   "r15",
      "xmm0",  "xmm1",  "xmm2",  "xmm3",  "xmm4",  "xmm5",  "xmm6",  "xmm7",
      "xmm8",  "xmm9",  "xmm10", "xmm11", "xmm12", "xmm13", "xmm14", "xmm15",
      "xmm16", "xmm17", "xmm18", "xmm19", "xmm20", "xmm21", "xmm22", "xmm23",
      "xmm24", "xmm25", "xmm26", "xmm27", "xmm28", "xmm29", "xmm30", "xmm31"
      "cc", "fpsr", "flags", "memory"
      // Ideally, we would set the LLVM "noredzone" attribute on this function
      // (and it would be propagated to the call site). Unfortunately, rustc
      // provides no such functionality. Fortunately, by a lucky coincidence,
      // the "alignstack" LLVM inline assembly option does exactly the same
      // thing on x86_64.
    : "volatile", "alignstack");
  ret
}
