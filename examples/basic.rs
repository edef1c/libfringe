// This file is part of libfringe, a low-level green threading library.
// Copyright (c) Nathan Zadoks <nathan@nathan7.eu>
// See the LICENSE file included in this distribution.
#![feature(thread_local)]
extern crate fringe;
use fringe::Context;

#[thread_local]
static mut ctx_slot: *mut Context<'static, fringe::OsStack> = 0 as *mut Context<_>;

fn main() {
  unsafe {
    let stack = fringe::OsStack::new(4 << 20).unwrap();

    let mut ctx = Context::new(stack, move || {
      println!("it's alive!");
      (*ctx_slot).swap();
      panic!("Do not come back!")
    });

    ctx_slot = &mut ctx;

    (*ctx_slot).swap();
  }
}
