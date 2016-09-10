// This file is part of libfringe, a low-level green threading library.
// Copyright (c) Nathan Zadoks <nathan@nathan7.eu>
// See the LICENSE file included in this distribution.
#![feature(test)]
extern crate test;
extern crate fringe;


use fringe::SliceStack;
use fringe::session::native_thread_locals;
use fringe::session::cycle::{C1, Cycle};


static mut stack_buf: [u8; 1024] = [0; 1024];

#[bench]
fn context_new(b: &mut test::Bencher) {
  b.iter(|| unsafe {
    let stack = SliceStack(&mut stack_buf);

    let ctx: C1<'static, _, ()> = C1::new(stack, move |tl, (ctx, ())| {
      ctx.unwrap().kontinue(Some(tl), ())
    });

    ctx.swap(native_thread_locals(), ());
  })
}

#[bench]
fn context_new_with_dead_loop(b: &mut test::Bencher) {
  b.iter(|| unsafe {
    let stack = SliceStack(&mut stack_buf);

    let ctx: C1<'static, _, ()> = C1::new(stack, move |tl, (mut ctx, ())| loop {
      ctx = ctx.unwrap().swap(Some(tl), ()).0;
    });

    ctx.swap(native_thread_locals(), ());
  })
}
