// This file is part of libfringe, a low-level green threading library.
// Copyright (c) 2015, Nathan Zadoks <nathan@nathan7.eu>
// See the LICENSE file included in this distribution.
use stack::Stack;
use void::Void;
use super::common::{push, rust_trampoline};

pub const STACK_ALIGN: usize = 16;

#[allow(raw_pointer_derive)]
#[derive(Debug)]
pub struct Registers {
  rsp: *mut usize
}

impl Registers {
  #[cfg(windows)]
  #[inline]
  pub unsafe fn new<S, F>(stack: &mut S, f: F) -> Registers
    where S: Stack, F: FnOnce() -> Void {
    let mut sp = stack.top() as *mut usize;
    let f_ptr = push(&mut sp, f);

    asm!(include_str!("init_win.s")
          : "={rdi}"(sp)
          : "{rdi}" (sp),
            "{rsi}" (rust_trampoline::<F>),
            "{rdx}" (f_ptr)
          :
          : "volatile");
    Registers { rsp: sp }
  }

  #[cfg(not(windows))]
  #[inline]
  pub unsafe fn new<S, F>(stack: &mut S, f: F) -> Registers where S: Stack, F: FnOnce() {
    let sp_limit = stack.limit();
    let mut sp = stack.top() as *mut usize;
    let f_ptr = push(&mut sp, f);

    asm!(include_str!("init.s")
          : "={rdi}"(sp)
          : "{rdi}" (sp),
            "{rsi}" (rust_trampoline::<F>),
            "{rdx}" (f_ptr),
            "{rcx}" (sp_limit)
          :
          : "volatile");
    Registers { rsp: sp }
  }

  #[cfg(not(windows))]
  #[inline(always)]
  pub unsafe fn swap(&mut self) {
    asm!(include_str!("swap.s")
          :
          : "{rdi}" (&mut self.rsp)
          : "rax",   "rbx",   "rcx",   "rdx",   "rsi",   "rdi", //"rbp",   "rsp",
            "r8",    "r9",    "r10",   "r11",   "r12",   "r13",   "r14",   "r15",
            "xmm0",  "xmm1",  "xmm2",  "xmm3",  "xmm4",  "xmm5",  "xmm6",  "xmm7",
            "xmm8",  "xmm9",  "xmm10", "xmm11", "xmm12", "xmm13", "xmm14", "xmm15",
            "xmm16", "xmm17", "xmm18", "xmm19", "xmm20", "xmm21", "xmm22", "xmm23", 
            "xmm24", "xmm25", "xmm26", "xmm27", "xmm28", "xmm29", "xmm30", "xmm31"
            "cc"
          : "volatile");
  }

  #[cfg(windows)]
  #[inline(always)]
  pub unsafe fn swap(&mut self) {
    asm!(include_str!("swap_win.s")
          :
          : "{rdi}" (&mut self.rsp)
          : "rax",   "rbx",   "rcx",   "rdx",   "rsi",   "rdi", //"rbp",   "rsp",
            "r8",    "r9",    "r10",   "r11",   "r12",   "r13",   "r14",   "r15",
            "xmm0",  "xmm1",  "xmm2",  "xmm3",  "xmm4",  "xmm5",  "xmm6",  "xmm7",
            "xmm8",  "xmm9",  "xmm10", "xmm11", "xmm12", "xmm13", "xmm14", "xmm15",
            "xmm16", "xmm17", "xmm18", "xmm19", "xmm20", "xmm21", "xmm22", "xmm23", 
            "xmm24", "xmm25", "xmm26", "xmm27", "xmm28", "xmm29", "xmm30", "xmm31"
            "cc"
          : "volatile");
  }
}
