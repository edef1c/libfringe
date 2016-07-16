// This file is part of libfringe, a low-level green threading library.
// Copyright (c) edef <edef@edef.eu>,
//               whitequark <whitequark@whitequark.org>
// See the LICENSE file included in this distribution.
#![feature(test)]
extern crate test;
extern crate fringe;

use fringe::Context;

static mut ctx_slot: *mut Context<fringe::OsStack> = 0 as *mut Context<_>;

#[bench]
fn swap(b: &mut test::Bencher) {
  unsafe extern "C" fn loopback(mut arg: usize) -> ! {
    // This deliberately does not ignore arg, to measure the time it takes
    // to move the return value between registers.
    let ctx_ptr = ctx_slot;
    loop { arg = Context::swap(ctx_ptr, ctx_ptr, arg) }
  }

  unsafe {
    let stack = fringe::OsStack::new(4 << 20).unwrap();
    let mut ctx = Context::new(stack, loopback);
    ctx_slot = &mut ctx;

    let ctx_ptr = &mut ctx;
    b.iter(|| Context::swap(ctx_ptr, ctx_ptr, 0));
  }
}
