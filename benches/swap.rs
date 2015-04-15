#![feature(test)]
extern crate test;
extern crate lwkt;
use lwkt::{Context, StackSource};

static mut ctx_slot: *mut Context<lwkt::os::Stack> = 0 as *mut Context<_>;

#[bench]
fn swap(b: &mut test::Bencher) {
  unsafe {
    let stack = lwkt::os::StackSource::get_stack(4 << 20);

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
