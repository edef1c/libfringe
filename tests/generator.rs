// This file is part of libfringe, a low-level green threading library.
// Copyright (c) whitequark <whitequark@whitequark.org>,
//               Nathan Zadoks <nathan@nathan7.eu>
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.
extern crate fringe;

use fringe::{SliceStack, OwnedStack, OsStack};
use fringe::generator::{Generator, Yielder};

fn add_one_fn(yielder: &mut Yielder<i32, i32>, mut input: i32) {
  loop {
    if input == 0 { break }
    input = yielder.suspend(input + 1)
  }
}

fn new_add_one() -> Generator<i32, i32, OsStack> {
  let stack = OsStack::new(0).unwrap();
  Generator::new(stack, add_one_fn)
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
  let mut add_one = unsafe { Generator::unsafe_new(stack, add_one_fn) };
  assert_eq!(add_one.resume(1), Some(2));
  assert_eq!(add_one.resume(2), Some(3));
}

#[test]
fn with_owned_stack() {
  let stack = OwnedStack::new(1024);
  let mut add_one = unsafe { Generator::unsafe_new(stack, add_one_fn) };
  assert_eq!(add_one.resume(1), Some(2));
  assert_eq!(add_one.resume(2), Some(3));
}

#[test]
fn forget_yielded() {
  struct Dropper(*mut bool);

  unsafe impl Send for Dropper {}

  impl Drop for Dropper {
    fn drop(&mut self) {
      unsafe {
        if *self.0 {
          panic!("double drop!")
        }
        *self.0 = true;
      }
    }
  }

  let stack = fringe::OsStack::new(1<<16).unwrap();
  let mut generator = Generator::new(stack, |yielder, ()| {
    let mut flag = false;
    yielder.suspend(Dropper(&mut flag as *mut bool));
  });
  generator.resume(());
  generator.resume(());
}
