// This file is part of libfringe, a low-level green threading library.
// Copyright (c) Ben Segall <talchas@gmail.com>
// Copyright (c) edef <edef@edef.eu>
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
static mut ctx_slot: *mut Context<fringe::OsStack> = 0 as *mut Context<_>;

const FE_DIVBYZERO: i32 = 0x4;
extern {
  fn feenableexcept(except: i32) -> i32;
}

#[test]
#[ignore]
fn fpe() {
  unsafe extern "C" fn universe_destroyer(_arg: usize) -> ! {
    loop {
        println!("{:?}", 1.0/black_box(0.0));
        Context::swap(ctx_slot, ctx_slot, 0);
    }
  }

  unsafe {
    let stack = fringe::OsStack::new(4 << 20).unwrap();
    let mut ctx = Context::new(stack, universe_destroyer);
    ctx_slot = &mut ctx;

    Context::swap(ctx_slot, ctx_slot, 0);
    feenableexcept(FE_DIVBYZERO);
    Context::swap(ctx_slot, ctx_slot, 0);
  }
}
