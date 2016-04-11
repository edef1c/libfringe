// This file is part of libfringe, a low-level green threading library.
// Copyright (c) Nathan Zadoks <nathan@nathan7.eu>
// See the LICENSE file included in this distribution.
pub use self::common::*;

macro_rules! init {
  ($sp:expr, $f_ptr:expr, $tramp:expr) => {
    asm!(include_str!("x86/init.s")
         : "={eax}"($sp)
         : "{eax}" ($sp),
           "{ebx}" ($tramp),
           "{ecx}" ($f_ptr)
         :
         : "volatile")
  };
}

macro_rules! swap {
  ($out_spp:expr, $in_spp:expr) => {
    asm!(include_str!("x86/swap.s")
         :
         : "{eax}" ($out_spp),
           "{ebx}" ($in_spp)
         : "eax",  "ebx",  "ecx",  "edx",  "esi",  "edi", //"ebp",  "esp",
           "mmx0", "mmx1", "mmx2", "mmx3", "mmx4", "mmx5", "mmx6", "mmx7",
           "xmm0", "xmm1", "xmm2", "xmm3", "xmm4", "xmm5", "xmm6", "xmm7",
           "cc"
         : "volatile")
  };
}

#[path = "../x86_common.rs"]
mod common;
