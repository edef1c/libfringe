// This file is part of libfringe, a low-level green threading library.
// Copyright (c) Nathan Zadoks <nathan@nathan7.eu>
// See the LICENSE file included in this distribution.
#![cfg(target_os = "linux")]
#![feature(asm, test)]
extern crate test;

#[cfg(target_arch = "x86_64")]
#[bench]
fn syscall(b: &mut test::Bencher) {
  b.iter(|| unsafe {
    asm!("movq $$102, %rax\n\
          syscall"
         :
         :
         : "rax", "rcx"
         : "volatile");
  });
}

#[cfg(target_arch = "x86")]
#[bench]
fn syscall(b: &mut test::Bencher) {
  b.iter(|| unsafe {
    asm!("mov $$24, %eax\n\
          int $$0x80"
         :
         :
         : "eax"
         : "volatile");
  });
}
