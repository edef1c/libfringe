// This file is part of libfringe, a low-level green threading library.
// Copyright (c) whitequark <whitequark@whitequark.org>
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.
extern crate fringe;

use fringe::{OsStack, SliceStack};
use fringe::generator::Generator;

fn new_add_one() -> Generator<i32, i32, OsStack> {
  let stack = OsStack::new(0).unwrap();
  Generator::new(stack, move |yielder, mut input| {
    loop {
      if input == 0 { break }
      input = yielder.suspend(input + 1)
    }
  })
}

#[test]
fn generator() {
  let mut add_one = new_add_one();
  assert_eq!(add_one.resume(1), Some(2));
  assert_eq!(add_one.resume(2), Some(3));
  assert_eq!(add_one.resume(0), None);
}

#[test]
fn move_after_new() {
  let mut add_one = new_add_one();
  assert_eq!(add_one.resume(1), Some(2));

  #[inline(never)]
  fn run_moved(mut add_one: Generator<i32, i32, OsStack>) {
    assert_eq!(add_one.resume(2), Some(3));
    assert_eq!(add_one.resume(3), Some(4));
    assert_eq!(add_one.resume(0), None);
  }
  run_moved(add_one);
}

#[test]
#[should_panic]
fn panic_safety() {
  struct Wrapper {
    gen: Generator<(), (), OsStack>
  }

  impl Drop for Wrapper {
    fn drop(&mut self) {
      self.gen.resume(());
    }
  }

  let stack = OsStack::new(4 << 20).unwrap();
  let gen = Generator::new(stack, move |_yielder, ()| {
    panic!("foo")
  });

  let mut wrapper = Wrapper { gen: gen };
  wrapper.gen.resume(());
}

#[test]
fn with_slice_stack() {
  let mut memory = [0; 1024];
  let stack = SliceStack(&mut memory);
  let mut add_one = unsafe {
    Generator::unsafe_new(stack, move |yielder, mut input| {
      loop {
        if input == 0 { break }
        input = yielder.suspend(input + 1)
      }
    })
  };
  assert_eq!(add_one.resume(1), Some(2));
  assert_eq!(add_one.resume(2), Some(3));
}
