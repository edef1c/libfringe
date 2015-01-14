extern crate test;
extern crate lwkt;
use lwkt::Context;

static mut ctx_slot: *mut Context = 0 as *mut Context;

#[bench]
fn swap(b: &mut test::Bencher) {
  unsafe {
    let mut ctx = Context::new(move |:| {
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
