// This file is part of libfringe, a low-level green threading library.
// Copyright (c) Nathan Zadoks <nathan@nathan7.eu>,
//               whitequark <whitequark@whitequark.org>
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

pub use self::imp::*;
use core::nonzero::NonZero;
use stack;

#[allow(unused_attributes)] // rust-lang/rust#35584
#[cfg_attr(target_arch = "x86",     path = "x86.rs")]
#[cfg_attr(target_arch = "x86_64",  path = "x86_64.rs")]
#[cfg_attr(target_arch = "aarch64", path = "aarch64.rs")]
#[cfg_attr(target_arch = "or1k",    path = "or1k.rs")]
mod imp;

#[derive(Debug, Clone, Copy)]
pub struct StackPointer(NonZero<*mut usize>);

impl StackPointer {
  #[inline(always)]
  pub unsafe fn push(&mut self, val: usize) {
    self.0 = NonZero::new(self.0.offset(-1));
    **self.0 = val;
  }

  #[inline(always)]
  pub unsafe fn stack_base<Stack: stack::Stack>(stack: &Stack) -> StackPointer {
    StackPointer(NonZero::new(stack.base() as *mut usize))
  }

  #[inline(always)]
  pub unsafe fn offset(&self, count: isize) -> *mut usize {
    self.0.offset(count)
  }
}

#[cfg(test)]
mod tests {
  extern crate test;
  extern crate simd;

  use arch::{self, StackPointer};
  use ::OsStack;

  #[test]
  fn context() {
    unsafe extern "C" fn adder(arg: usize, stack_ptr: StackPointer) {
      println!("it's alive! arg: {}", arg);
      let (arg, stack_ptr) = arch::swap(arg + 1, stack_ptr);
      println!("still alive! arg: {}", arg);
      arch::swap(arg + 1, stack_ptr);
      panic!("i should be dead");
    }

    unsafe {
      let stack = OsStack::new(4 << 20).unwrap();
      let stack_ptr = arch::init(&stack, adder);

      let (ret, stack_ptr) = arch::swap_link(10, stack_ptr, &stack);
      assert_eq!(ret, 11);
      let (ret, _) = arch::swap_link(50, stack_ptr.unwrap(), &stack);
      assert_eq!(ret, 51);
    }
  }

  #[test]
  fn context_simd() {
    unsafe extern "C" fn permuter(arg: usize, stack_ptr: StackPointer) {
      // This will crash if the stack is not aligned properly.
      let x = simd::i32x4::splat(arg as i32);
      let y = x * x;
      println!("simd result: {:?}", y);
      let (_, stack_ptr) = arch::swap(0, stack_ptr);
      // And try again after a context switch.
      let x = simd::i32x4::splat(arg as i32);
      let y = x * x;
      println!("simd result: {:?}", y);
      arch::swap(0, stack_ptr);
      panic!("i should be dead");
    }

    unsafe {
      let stack = OsStack::new(4 << 20).unwrap();
      let stack_ptr = arch::init(&stack, permuter);

      let (_, stack_ptr) = arch::swap_link(10, stack_ptr, &stack);
      arch::swap_link(20, stack_ptr.unwrap(), &stack);
    }
  }

  unsafe extern "C" fn do_panic(arg: usize, stack_ptr: StackPointer) {
    match arg {
      0 => panic!("arg=0"),
      1 => {
        arch::swap(0, stack_ptr);
        panic!("arg=1");
      }
      _ => unreachable!()
    }
  }

  #[test]
  #[should_panic="arg=0"]
  fn panic_after_start() {
    unsafe {
      let stack = OsStack::new(4 << 20).unwrap();
      let stack_ptr = arch::init(&stack, do_panic);

      arch::swap_link(0, stack_ptr, &stack);
    }
  }

  #[test]
  #[should_panic="arg=1"]
  fn panic_after_swap() {
    unsafe {
      let stack = OsStack::new(4 << 20).unwrap();
      let stack_ptr = arch::init(&stack, do_panic);

      let (_, stack_ptr) = arch::swap_link(1, stack_ptr, &stack);
      arch::swap_link(0, stack_ptr.unwrap(), &stack);
    }
  }

  #[test]
  fn ret() {
    unsafe extern "C" fn ret2(_: usize, _: StackPointer) {}

    unsafe {
      let stack = OsStack::new(4 << 20).unwrap();
      let stack_ptr = arch::init(&stack, ret2);

      let (_, stack_ptr) = arch::swap_link(0, stack_ptr, &stack);
      assert!(stack_ptr.is_none());
    }
  }

  #[bench]
  fn swap(b: &mut test::Bencher) {
    unsafe extern "C" fn loopback(mut arg: usize, mut stack_ptr: StackPointer) {
      // This deliberately does not ignore arg, to measure the time it takes
      // to move the return value between registers.
      loop {
        let data = arch::swap(arg, stack_ptr);
        arg = data.0;
        stack_ptr = data.1;
      }
    }

    unsafe {
      let stack = OsStack::new(4 << 20).unwrap();
      let mut stack_ptr = arch::init(&stack, loopback);

      b.iter(|| for _ in 0..10 {
        stack_ptr = arch::swap_link(0, stack_ptr, &stack).1.unwrap();
      });
    }
  }
}
