// This file is part of libfringe, a low-level green threading library.
// Copyright (c) whitequark <whitequark@whitequark.org>
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.
#![feature(test)]
extern crate fringe;
extern crate test;

use fringe::{Generator, OsStack};

#[bench]
fn generate(b: &mut test::Bencher) {
  let stack = OsStack::new(0).unwrap();
  let mut identity = Generator::new(stack, move |yielder, mut input| loop {
    input = yielder.suspend(input)
  });

  b.iter(|| {
    for _ in 0..10 {
      test::black_box(identity.resume(test::black_box(0)));
    }
  });
  unsafe {
    identity.unsafe_unwrap();
  }
}
