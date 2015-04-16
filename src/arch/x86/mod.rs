// This file is part of libfringe, a low-level green threading library.
// Copyright (c) 2015, edef <edef@edef.eu>
// See the LICENSE file included in this distribution.
use core::prelude::*;

use stack::Stack;
use super::common::{push, rust_trampoline};

pub const STACK_ALIGN: usize = 16;

#[allow(raw_pointer_derive)]
#[derive(Debug)]
pub struct Registers {
  esp: *mut usize
}

impl Registers {
  #[inline]
  pub unsafe fn new<S, F>(stack: &mut S, f: F) -> Registers where S: Stack, F: FnOnce() {
    let sp_limit = stack.limit();
    let mut sp = stack.top() as *mut usize;
    let f_ptr = push(&mut sp, f);

    asm!(include_str!("init.s")
          : "={eax}"(sp)
          : "{eax}" (sp),
            "{ebx}" (rust_trampoline::<F>),
            "{ecx}" (f_ptr),
            "{edx}" (sp_limit)
          :
          : "volatile");

    Registers { esp: sp }
  }

  #[inline(always)]
  pub unsafe fn swap(&mut self) {
    asm!(include_str!("swap.s")
          :
          : "{eax}" (&mut self.esp)
          : "eax",  "ebx",  "ecx",  "edx",  "esi",  "edi", //"ebp",  "esp",
            "mmx0", "mmx1", "mmx2", "mmx3", "mmx4", "mmx5", "mmx6", "mmx7",
            "xmm0", "xmm1", "xmm2", "xmm3", "xmm4", "xmm5", "xmm6", "xmm7",
            "cc"
          : "volatile");
  }
}
