// This file is part of libfringe, a low-level green threading library.
// Copyright (c) Nathan Zadoks <nathan@nathan7.eu>,
//               whitequark <whitequark@whitequark.org>
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.
use stack;
use debug;
use arch;

/// Context holds a suspended thread of execution along with a stack.
///
/// It can be swapped into and out of with the swap method,
/// and once you're done with it, you can get the stack back through unwrap.
///
/// Every operation is unsafe, because no guarantees can be made about
/// the state of the context.
#[derive(Debug)]
pub struct Context<Stack: stack::Stack> {
  stack:     Stack,
  stack_id:  debug::StackId,
  stack_ptr: arch::StackPointer
}

unsafe impl<Stack> Send for Context<Stack>
  where Stack: stack::Stack + Send {}

impl<Stack> Context<Stack> where Stack: stack::Stack {
  /// Creates a new Context. When it is swapped into, it will call
  /// `f(arg)`, where `arg` is the argument passed to `swap`.
  pub unsafe fn new(stack: Stack, f: unsafe extern "C" fn(usize) -> !) -> Context<Stack> {
    let stack_id  = debug::StackId::register(&stack);
    let stack_ptr = arch::init(&stack, f);
    Context {
      stack:     stack,
      stack_id:  stack_id,
      stack_ptr: stack_ptr
    }
  }

  /// Unwraps the context, returning the stack it contained.
  pub unsafe fn unwrap(self) -> Stack {
    self.stack
  }
}

impl<OldStack> Context<OldStack> where OldStack: stack::Stack {
  /// Switches to `in_ctx`, saving the current thread of execution to `out_ctx`.
  #[inline(always)]
  pub unsafe fn swap<NewStack>(old_ctx: *mut Context<OldStack>,
                               new_ctx: *const Context<NewStack>,
                               arg: usize) -> usize
      where NewStack: stack::Stack {
    arch::swap(arg, &mut (*old_ctx).stack_ptr, (*new_ctx).stack_ptr, &(*new_ctx).stack)
  }
}

#[cfg(test)]
mod test {
  extern crate test;
  extern crate simd;

  use std::ptr;
  use super::Context;
  use ::OsStack;

  #[thread_local]
  static mut ctx_slot: *mut Context<OsStack> = ptr::null_mut();

  #[test]
  fn context() {
    unsafe extern "C" fn adder(arg: usize) -> ! {
      println!("it's alive! arg: {}", arg);
      let arg = Context::swap(ctx_slot, ctx_slot, arg + 1);
      println!("still alive! arg: {}", arg);
      Context::swap(ctx_slot, ctx_slot, arg + 1);
      panic!("i should be dead");
    }

    unsafe {
      let stack = OsStack::new(4 << 20).unwrap();
      let mut ctx = Context::new(stack, adder);
      ctx_slot = &mut ctx;

      let ret = Context::swap(ctx_slot, ctx_slot, 10);
      assert_eq!(ret, 11);
      let ret = Context::swap(ctx_slot, ctx_slot, 50);
      assert_eq!(ret, 51);
    }
  }

  #[test]
  fn context_simd() {
    unsafe extern "C" fn permuter(arg: usize) -> ! {
      // This will crash if the stack is not aligned properly.
      let x = simd::i32x4::splat(arg as i32);
      let y = x * x;
      println!("simd result: {:?}", y);
      Context::swap(ctx_slot, ctx_slot, 0);
      // And try again after a context switch.
      let x = simd::i32x4::splat(arg as i32);
      let y = x * x;
      println!("simd result: {:?}", y);
      Context::swap(ctx_slot, ctx_slot, 0);
      panic!("i should be dead");
    }

    unsafe {
      let stack = OsStack::new(4 << 20).unwrap();
      let mut ctx = Context::new(stack, permuter);
      ctx_slot = &mut ctx;

      Context::swap(ctx_slot, ctx_slot, 10);
      Context::swap(ctx_slot, ctx_slot, 20);
    }
  }

  unsafe extern "C" fn do_panic(arg: usize) -> ! {
    match arg {
      0 => panic!("arg=0"),
      1 => {
        Context::swap(ctx_slot, ctx_slot, 0);
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
      let mut ctx = Context::new(stack, do_panic);

      Context::swap(&mut ctx, &ctx, 0);
    }
  }

  #[test]
  #[should_panic="arg=1"]
  fn panic_after_swap() {
    unsafe {
      let stack = OsStack::new(4 << 20).unwrap();
      let mut ctx = Context::new(stack, do_panic);
      ctx_slot = &mut ctx;

      Context::swap(&mut ctx, &ctx, 1);
      Context::swap(&mut ctx, &ctx, 0);
    }
  }

  #[bench]
  fn swap(b: &mut test::Bencher) {
    unsafe extern "C" fn loopback(mut arg: usize) -> ! {
      // This deliberately does not ignore arg, to measure the time it takes
      // to move the return value between registers.
      let ctx_ptr = ctx_slot;
      loop { arg = Context::swap(ctx_ptr, ctx_ptr, arg) }
    }

    unsafe {
      let stack = OsStack::new(4 << 20).unwrap();
      let mut ctx = Context::new(stack, loopback);
      ctx_slot = &mut ctx;

      let ctx_ptr = &mut ctx;
      b.iter(|| Context::swap(ctx_ptr, ctx_ptr, 0));
    }
  }
}
