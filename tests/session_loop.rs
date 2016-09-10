// This file is part of libfringe, a low-level green threading library.
// Copyright (c) 2015, Nathan Zadoks <nathan@nathan7.eu>
// See the LICENSE file included in this distribution.
extern crate env_logger;
extern crate fringe;

use fringe::session;
use fringe::session::cycle::{C1, Cycle};
use fringe::OsStack;

#[test]
fn main() {
  env_logger::init().unwrap();
  let stack = OsStack::new(4 << 20).unwrap();

  let mut ctx: C1<'static, OsStack, ()> = C1::new(stack, |tl, (mut ctx, ())| {
    let mut c = 0;
    while {
      println!("so far: {}", c);
      c < 5
    } {
      c += 1;
      ctx = ctx.unwrap().swap(Some(tl), ()).0;
    }
    assert_eq!(c, 5);
    ctx.unwrap().kontinue(Some(tl), ())
  });

  loop {
    let (c, ()) = ctx.swap(session::native_thread_locals(), ());
    match c {
      None    => break,
      Some(c) => ctx = c,
    }
  }
}
