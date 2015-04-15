#![feature(test)]
extern crate test;
extern crate lwkt;
use lwkt::Context;

static mut ctx_slot: *mut Context = 0 as *mut Context;

#[bench]
fn context_new(b: &mut test::Bencher) {
  b.iter(|| unsafe {
    let mut ctx = Context::new(move || {
      let ctx_ptr = ctx_slot;
      loop {
        (*ctx_ptr).swap()
      }
    });

    ctx_slot = &mut ctx;

    ctx.swap();
  })
}
