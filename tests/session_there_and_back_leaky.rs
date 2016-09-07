// This file is part of libfringe, a low-level green threading library.
// Copyright (c) 2015, Nathan Zadoks <nathan@nathan7.eu>
// See the LICENSE file included in this distribution.
extern crate env_logger;
extern crate fringe;

use fringe::session;
use fringe::session::cycle::{C1, Cycle};

#[test]
fn main() {
  env_logger::init().unwrap();
  let stack = fringe::OsStack::new(4 << 20).unwrap();

  let ctx: C1<'static, fringe::OsStack, ()> = C1::new(stack, move |_, (ctx, ())| {
    let c = ctx.unwrap();
    assert!(c.0.thread_locals.is_none());
    println!("it's alive!");
    c.kontinue(session::native_thread_locals(), ());
  });

  let (x, ()) = ctx.swap(session::native_thread_locals(), ());
  println!("we're back!");
  assert!(x.is_none());
  drop(x);
  println!("it's vanquished!");
}
