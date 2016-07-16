// This file is part of libfringe, a low-level green threading library.
// Copyright (c) edef <edef@edef.eu>,
//               whitequark <whitequark@whitequark.org>
// See the LICENSE file included in this distribution.

//! To understand the code in this file, keep in mind this fact:
//! * i686 SysV C ABI requires the stack to be aligned at function entry,
//!   so that `%esp+4` is a multiple of 16. Aligned operands are a requirement
//!   of SIMD instructions, and making this the responsibility of the caller
//!   avoids having to maintain a frame pointer, which is necessary when
//!   a function has to realign the stack from an unknown state.
//! * i686 SysV C ABI passes the first argument on the stack. This is
//!   unfortunate, because unlike every other architecture we can't reuse
//!   `swap` for the initial call, and so we use a trampoline.
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
  #[naked]
  unsafe extern "C" fn trampoline() -> ! {
    asm!(
      r#"
        # Pop function.
        popl    %ebx
        # Push argument.
        pushl   %eax
        # Call it.
        call    *%ebx
      "# ::: "memory" : "volatile");
    ::core::intrinsics::unreachable()
  }

  let mut sp = StackPointer::new(stack);
  sp.push(0); // alignment
  sp.push(0); // alignment
  sp.push(0); // alignment
  sp.push(f as usize); // function
  sp.push(trampoline as usize);
  sp
}

#[inline(always)]
pub unsafe fn swap(arg: usize, old_sp: &mut StackPointer, new_sp: &StackPointer) -> usize {
  let ret: usize;
  asm!(
    r#"
      # Save frame pointer explicitly; LLVM doesn't spill it even if it is
      # marked as clobbered.
      pushl   %ebp
      # Push instruction pointer of the old context and switch to
      # the new context.
      call    1f
      # Restore frame pointer.
      popl    %ebp
      # Continue executing old context.
      jmp     2f

    1:
      # Remember stack pointer of the old context, in case %rdx==%rsi.
      movl    %esp, %ebx
      # Load stack pointer of the new context.
      movl    (%edx), %esp
      # Save stack pointer of the old context.
      movl    %ebx, (%esi)

      # Pop instruction pointer of the new context (placed onto stack by
      # the call above) and jump there; don't use `ret` to avoid return
      # address mispredictions (~8ns on Ivy Bridge).
      popl    %ebx
      jmpl    *%ebx
    2:
    "#
    : "={eax}" (ret)
    : "{eax}" (arg)
      "{esi}" (old_sp)
      "{edx}" (new_sp)
    : "eax",  "ebx",  "ecx",  "edx",  "esi",  "edi", //"ebp",  "esp",
      "mmx0", "mmx1", "mmx2", "mmx3", "mmx4", "mmx5", "mmx6", "mmx7",
      "xmm0", "xmm1", "xmm2", "xmm3", "xmm4", "xmm5", "xmm6", "xmm7",
      "cc", "fpsr", "flags", "memory"
    : "volatile");
  ret
}
