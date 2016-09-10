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

use fringe::session;
use fringe::session::cycle::{C1, Cycle};

use test::black_box;

const FE_DIVBYZERO: i32 = 0x4;
extern {
  fn feenableexcept(except: i32) -> i32;
}

#[test]
#[ignore]
fn fpe() {
  let stack = fringe::OsStack::new(4 << 20).unwrap();

  let mut ctx: C1<'static, fringe::OsStack, ()> = C1::new(stack, move |tl, (mut ctx, ())| loop {
    println!("it's alive!");
    let c = ctx.unwrap();
    assert!(c.0.thread_locals.is_none());
    println!("{:?}", 1.0/black_box(0.0));
    ctx = c.swap(Some(tl), ()).0;
  });

  {
    let (x, ()) = ctx.swap(session::native_thread_locals(), ());
    println!("we're back!");
    ctx = x.unwrap();
  }

  unsafe { feenableexcept(FE_DIVBYZERO) };

  {
    let (_, ()) = ctx.swap(session::native_thread_locals(), ());
    println!("we're back again!");
  }
}
