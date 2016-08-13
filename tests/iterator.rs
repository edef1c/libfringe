// This file is part of libfringe, a low-level green threading library.
// Copyright (c) whitequark <whitequark@whitequark.org>
// See the LICENSE file included in this distribution.
extern crate fringe;

use fringe::OsStack;
use fringe::generator::Generator;

#[test]
fn producer() {
  let stack = OsStack::new(0).unwrap();
  let mut gen = Generator::new(stack, move |yielder, ()| {
    for i in 0.. { yielder.generate(i) }
  });
  assert_eq!(gen.next(), Some(0));
  assert_eq!(gen.next(), Some(1));
  assert_eq!(gen.next(), Some(2));
}
