// This file is part of libfringe, a low-level green threading library.
// Copyright (c) edef <edef@edef.eu>
// See the LICENSE file included in this distribution.
#![feature(test)]
#![cfg(feature = "os")]
extern crate test;
extern crate fringe;
use fringe::Context;

static mut ctx_slot: *mut Context<'static, fringe::OsStack> = 0 as *mut Context<_>;

#[bench]
fn swap(b: &mut test::Bencher) {
  unsafe {
    let stack = fringe::OsStack::new(4 << 20).unwrap();

    let mut ctx = Context::new(stack, move || {
      let ctx_ptr = ctx_slot;
      loop {
        Context::swap(ctx_ptr, ctx_ptr);
      }
    });

    let ctx_ptr = &mut ctx;
    ctx_slot = ctx_ptr;

    Context::swap(ctx_ptr, ctx_ptr);

    b.iter(|| Context::swap(ctx_ptr, ctx_ptr));
  }
}
