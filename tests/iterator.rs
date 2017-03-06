// This file is part of libfringe, a low-level green threading library.
// Copyright (c) whitequark <whitequark@whitequark.org>
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.
extern crate fringe;

use fringe::OsStack;
use fringe::generator::Generator;

#[test]
fn producer() {
  let stack = OsStack::new(0).unwrap();
  let mut gen = Generator::new(stack, move |yielder, ()| {
    for i in 0.. { yielder.suspend(i) }
  });
  assert_eq!(gen.next(), Some(0));
  assert_eq!(gen.next(), Some(1));
  assert_eq!(gen.next(), Some(2));
  unsafe { gen.unsafe_unwrap(); }
}
