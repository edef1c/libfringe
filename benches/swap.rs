// Copyright (c) 2015, Nathan Zadoks <nathan@nathan7.eu>
// See the LICENSE file included in this distribution.
#![feature(test)]
extern crate test;
extern crate lwkt;
use lwkt::{Context, StackSource};

static mut ctx_slot: *mut Context<'static, lwkt::os::Stack> = 0 as *mut Context<_>;

#[bench]
fn swap(b: &mut test::Bencher) {
  unsafe {
    let stack = lwkt::os::StackSource::get_stack(4 << 20).unwrap();

    let mut ctx = Context::new(stack, move || {
      let ctx_ptr = ctx_slot;
      loop {
        (*ctx_ptr).swap()
      }
    });

    ctx_slot = &mut ctx;

    ctx.swap();

    b.iter(|| ctx.swap());
  }
}
