#![feature(thread_local)]
extern crate lwkt;
use lwkt::{Context, StackSource};

#[thread_local]
static mut ctx_slot: *mut Context<lwkt::os::Stack> = 0 as *mut Context<_>;

fn main() {
  unsafe {
    let stack = lwkt::os::StackSource::get_stack(4 << 20);

    let mut ctx = Context::new(stack, move || {
      println!("it's alive!");
      (*ctx_slot).swap();
    });

    ctx_slot = &mut ctx;

    (*ctx_slot).swap();
  }
}
