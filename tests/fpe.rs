// This file is part of libfringe, a low-level green threading library.
// Copyright (c) Ben Segall <talchas@gmail.com>
// Copyright (c) edef <edef@edef.eu>
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.
#![cfg(target_os = "linux")]
#![feature(test)]
#![feature(thread_local)]
#![feature(asm)]
extern crate fringe;
extern crate test;
use fringe::{Generator, OsStack};
use test::black_box;

const FE_DIVBYZERO: i32 = 0x4;
extern "C" {
  fn feenableexcept(except: i32) -> i32;
}

#[test]
#[ignore]
fn fpe() {
  let stack = OsStack::new(0).unwrap();
  let mut gen = Generator::new(stack, move |yielder, ()| {
    yielder.suspend(1.0 / black_box(0.0));
  });

  unsafe {
    feenableexcept(FE_DIVBYZERO);
  }
  println!("{:?}", gen.resume(()));
}
