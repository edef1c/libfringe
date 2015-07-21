// This file is part of libfringe, a low-level green threading library.
// Copyright (c) 2015, Ben Segall <talchas@gmail.com>
// Copyright (c) 2015, Nathan Zadoks <nathan@nathan7.eu>
// See the LICENSE file included in this distribution.
#![cfg(target_os = "linux")]
#![feature(test)]
#![feature(thread_local)]
#![feature(asm)]
extern crate fringe;
extern crate test;
use fringe::Context;
use test::black_box;

#[thread_local]
static mut ctx_slot: *mut Context<'static, fringe::OsStack> = 0 as *mut Context<_>;

const FE_DIVBYZERO: i32 = 0x4;
extern {
  fn feenableexcept(except: i32) -> i32;
}

#[test]
fn fpe() {
  unsafe {
    let stack = fringe::OsStack::new(4 << 20).unwrap();

    let mut ctx = Context::new(stack, move || {
        println!("it's alive!");
        loop {
            println!("{:?}", 1.0/black_box(0.0));
            (*ctx_slot).swap();
        }
    });

    ctx_slot = &mut ctx;

    (*ctx_slot).swap();
    feenableexcept(FE_DIVBYZERO);
    (*ctx_slot).swap();
  }
}
