// This file is part of libfringe, a low-level green threading library.
// Copyright (c) edef <edef@edef.eu>
// See the LICENSE file included in this distribution.
pub use self::common::*;

macro_rules! init {
  ($sp:expr, $f_ptr:expr, $tramp:expr) => {
    asm!(include_str!("x86_64/init.s")
         : "={rdi}"($sp)
         : "{rdi}" ($sp),
           "{rsi}" ($tramp),
           "{rdx}" ($f_ptr)
         :
         : "volatile");
  }
}

macro_rules! swap {
  ($out_spp:expr, $in_spp:expr) => {
    asm!(include_str!("x86_64/swap.s")
         :
         : "{rdi}" ($out_spp)
           "{rsi}" ($in_spp)
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

#[path = "../x86_common.rs"]
mod common;
