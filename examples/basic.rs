#![feature(thread_local)]
extern crate lwkt;
use lwkt::Context;

#[thread_local]
static mut ctx_slot: *mut Context = 0 as *mut Context;

fn main() {
  unsafe {
    let mut ctx = Context::new(move || {
      println!("it's alive!");
      (*ctx_slot).swap();
    });

    ctx_slot = &mut ctx;

    (*ctx_slot).swap();
  }
}
