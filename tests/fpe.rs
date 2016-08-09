// This file is part of libfringe, a low-level green threading library.
// Copyright (c) Ben Segall <talchas@gmail.com>
// Copyright (c) Nathan Zadoks <nathan@nathan7.eu>
// See the LICENSE file included in this distribution.
#![cfg(target_os = "linux")]
#![feature(test)]
#![feature(thread_local)]
#![feature(asm)]
extern crate fringe;
extern crate test;
use fringe::{OsStack, Generator};
use test::black_box;

const FE_DIVBYZERO: i32 = 0x4;
extern {
  fn feenableexcept(except: i32) -> i32;
}

#[test]
#[ignore]
fn fpe() {
  let stack = OsStack::new(0).unwrap();
  let mut gen = Generator::new(stack, move |yielder| {
    yielder.generate(1.0 / black_box(0.0));
  });

  unsafe { feenableexcept(FE_DIVBYZERO); }
  println!("{:?}", gen.next());
}
