// This file is part of libfringe, a low-level green threading library.
// Copyright (c) whitequark <whitequark@whitequark.org>
// See the LICENSE file included in this distribution.
#![feature(test)]
extern crate test;
extern crate fringe;

use fringe::{OsStack, Generator};

#[bench]
fn generate(b: &mut test::Bencher) {
  let stack = OsStack::new(0).unwrap();
  let mut gen = Generator::new(stack, move |yielder| {
    for i in 1.. { yielder.generate(i) }
  });

  b.iter(|| test::black_box(gen.next()));
}
