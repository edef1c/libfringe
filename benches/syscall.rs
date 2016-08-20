// This file is part of libfringe, a low-level green threading library.
// Copyright (c) edef <edef@edef.eu>
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.
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
