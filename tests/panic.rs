// This file is part of libfringe, a low-level green threading library.
// Copyright (c) whitequark <whitequark@whitequark.org>
// See the LICENSE file included in this distribution.
#![feature(thread_local)]
extern crate fringe;

use fringe::Context;

#[thread_local]
static mut ctx_slot: *mut Context<fringe::OsStack> = 0 as *mut Context<_>;

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
    let stack = fringe::OsStack::new(4 << 20).unwrap();
    let mut ctx = Context::new(stack, do_panic);

    Context::swap(&mut ctx, &ctx, 0);
  }
}

#[test]
#[should_panic="arg=1"]
fn panic_after_swap() {
  unsafe {
    let stack = fringe::OsStack::new(4 << 20).unwrap();
    let mut ctx = Context::new(stack, do_panic);
    ctx_slot = &mut ctx;

    Context::swap(&mut ctx, &ctx, 1);
    Context::swap(&mut ctx, &ctx, 0);
  }
}
