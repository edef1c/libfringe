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
  let mut identity = Generator::new(stack, move |yielder, mut input| {
    loop { input = yielder.generate(input) }
  });

  b.iter(|| test::black_box(identity.resume(test::black_box(0))));
}
