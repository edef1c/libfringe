#![feature(thread_local)]
extern crate lwkt;
use lwkt::Context;

#[thread_local]
static mut ctx_slot: *mut Context<'static, lwkt::OsStack> = 0 as *mut Context<_>;

fn main() {
  unsafe {
    let stack = lwkt::OsStack::new(4 << 20).unwrap();

    let mut ctx = Context::new(stack, move || {
      println!("it's alive!");
      (*ctx_slot).swap();
    });

    ctx_slot = &mut ctx;

    (*ctx_slot).swap();
  }
}
