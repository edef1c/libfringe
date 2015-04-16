// This file is part of libfringe, a low-level green threading library.
// Copyright (c) 2015, Nathan Zadoks <nathan@nathan7.eu>
// See the LICENSE file included in this distribution.
#![cfg(target_os = "linux")]
#![feature(asm, test)]
extern crate test;
use test::Bencher;

#[bench]
fn kernel_swap(b: &mut Bencher) {
  b.iter(|| unsafe {
    asm!("movq $$102, %rax\n\
          syscall"
         :
         :
         : "rax", "rcx"
         : "volatile");
  });
}
