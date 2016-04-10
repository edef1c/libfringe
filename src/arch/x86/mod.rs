// This file is part of libfringe, a low-level green threading library.
// Copyright (c) Nathan Zadoks <nathan@nathan7.eu>
// See the LICENSE file included in this distribution.
use void::Void;

use stack::Stack;
use super::common::{push, rust_trampoline};

pub const STACK_ALIGN: usize = 16;

#[derive(Debug)]
pub struct Registers {
  esp: *mut usize
}

impl Registers {
  #[inline]
  pub unsafe fn new<S, F>(stack: &mut S, f: F) -> Registers
    where S: Stack, F: FnOnce() -> Void {
    let mut sp = stack.top() as *mut usize;
    let f_ptr = push(&mut sp, f);

    asm!(include_str!("init.s")
          : "={eax}"(sp)
          : "{eax}" (sp),
            "{ebx}" (rust_trampoline::<F> as unsafe extern "C" fn(*const F) -> !),
            "{ecx}" (f_ptr)
          :
          : "volatile");

    Registers { esp: sp }
  }

  #[inline(always)]
  pub unsafe fn swap(out_regs: *mut Registers, in_regs: *const Registers) {
    let out_espp = &mut (*out_regs).esp;
    let in_espp = &(*in_regs).esp;
    asm!(include_str!("swap.s")
          :
          : "{eax}" (out_espp),
            "{ebx}" (in_espp)
          : "eax",  "ebx",  "ecx",  "edx",  "esi",  "edi", //"ebp",  "esp",
            "mmx0", "mmx1", "mmx2", "mmx3", "mmx4", "mmx5", "mmx6", "mmx7",
            "xmm0", "xmm1", "xmm2", "xmm3", "xmm4", "xmm5", "xmm6", "xmm7",
            "cc"
          : "volatile");
  }
}
