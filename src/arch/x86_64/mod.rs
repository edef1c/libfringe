// This file is part of libfringe, a low-level green threading library.
// Copyright (c) Nathan Zadoks <nathan@nathan7.eu>
// See the LICENSE file included in this distribution.
use stack::Stack;
use void::Void;
use super::common::{push, rust_trampoline};

pub const STACK_ALIGN: usize = 16;

#[derive(Debug)]
pub struct Registers {
  rsp: *mut usize
}

impl Registers {
  #[inline]
  pub unsafe fn new<S, F>(stack: &mut S, f: F) -> Registers
    where S: Stack, F: FnOnce() -> Void {
    let mut sp = stack.top() as *mut usize;
    let f_ptr = push(&mut sp, f);

    asm!(include_str!("init.s")
          : "={rdi}"(sp)
          : "{rdi}" (sp),
            "{rsi}" (rust_trampoline::<F> as unsafe extern "C" fn(*const F) -> !),
            "{rdx}" (f_ptr)
          :
          : "volatile");

    Registers { rsp: sp }
  }

  #[inline(always)]
  pub unsafe fn swap(out_regs: *mut Registers, in_regs: *const Registers) {
    let out_rspp = &mut (*out_regs).rsp;
    let in_rspp = &(*in_regs).rsp;
    asm!(include_str!("swap.s")
          :
          : "{rdi}" (out_rspp),
            "{rsi}" (in_rspp)
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
