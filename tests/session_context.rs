// This file is part of libfringe, a low-level green threading library.
// Copyright (c) 2015, Nathan Zadoks <nathan@nathan7.eu>,
//                     John Ericson <John_Ericson@Yahoo.com>
// See the LICENSE file included in this distribution.
#[macro_use]
extern crate log;
extern crate env_logger;
extern crate fringe;

use fringe::session::Context;

#[test]
fn init() {
  env_logger::init().unwrap();
  let stack = fringe::OsStack::new(4 << 20).unwrap();

  let ctx: Context<(), _> = Context::new(stack, move |_, _ctx| unreachable!());
  debug!("created");
  drop(ctx);
  debug!("dropped");
}
