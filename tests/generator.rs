// This file is part of libfringe, a low-level green threading library.
// Copyright (c) whitequark <whitequark@whitequark.org>
// See the LICENSE file included in this distribution.
extern crate fringe;

use fringe::OsStack;
use fringe::generator::Generator;

#[test]
fn generator() {
  let stack = OsStack::new(0).unwrap();
  let mut gen = Generator::new(stack, move |yielder| {
    for i in 1..4 {
      yielder.generate(i);
    }
  });
  assert_eq!(gen.next(), Some(1));
  assert_eq!(gen.next(), Some(2));
  assert_eq!(gen.next(), Some(3));
  assert_eq!(gen.next(), None);
}

#[test]
fn move_after_new() {
  let stack = OsStack::new(0).unwrap();
  let mut gen = Generator::new(stack, move |yielder| {
    for i in 1..4 {
      yielder.generate(i);
    }
  });
  assert_eq!(gen.next(), Some(1));

  #[inline(never)]
  fn rest(mut gen: Generator<u32, OsStack>) {
    assert_eq!(gen.next(), Some(2));
    assert_eq!(gen.next(), Some(3));
    assert_eq!(gen.next(), None);
  }
  rest(gen);
}

#[test]
#[should_panic]
fn panic_safety() {
  struct Wrapper {
    gen: Generator<u32, OsStack>
  }

  impl Drop for Wrapper {
    fn drop(&mut self) {
      self.gen.next();
    }
  }

  let stack = OsStack::new(4 << 20).unwrap();
  let gen = Generator::new(stack, move |_yielder| {
    panic!("foo")
  });

  let mut wrapper = Wrapper { gen: gen };
  wrapper.gen.next();
}
