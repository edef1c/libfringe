// This file is part of libfringe, a low-level green threading library.
// Copyright (c) Nathan Zadoks <nathan@nathan7.eu>
// See the LICENSE file included in this distribution.
#![feature(test)]
extern crate test;
extern crate fringe;
use fringe::Context;

static mut ctx_slot: *mut Context<'static, SliceStack<'static>> = 0 as *mut Context<_>;
static mut stack_buf: [u8; 1024] = [0; 1024];

#[bench]
fn context_new(b: &mut test::Bencher) {
  b.iter(|| unsafe {
    let stack = SliceStack(&mut stack_buf);

    let mut ctx = Context::new(stack, move || {
      let ctx_ptr = ctx_slot;
      loop {
        Context::swap(ctx_ptr, ctx_ptr);
      }
    });

    ctx_slot = &mut ctx;

    Context::swap(ctx_slot, ctx_slot);
  })
}

struct SliceStack<'a>(&'a mut [u8]);
impl<'a> fringe::Stack for SliceStack<'a> {
  fn top(&mut self) -> *mut u8 {
    unsafe {
      self.0.as_mut_ptr().offset(self.0.len() as isize)
    }
  }

  fn limit(&self) -> *const u8 {
    self.0.as_ptr()
  }
}
