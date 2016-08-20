// This file is part of libfringe, a low-level green threading library.
// Copyright (c) Nathan Zadoks <nathan@nathan7.eu>
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.
extern crate valgrind;

use stack;
use self::valgrind::{stack_register, stack_deregister};

#[derive(Debug)]
pub struct StackId(self::valgrind::Value);

impl StackId {
  #[inline(always)]
  pub fn register<Stack: stack::Stack>(stack: &Stack) -> StackId {
    StackId(stack_register(stack.limit(), stack.base()))
  }
}

impl Drop for StackId {
  #[inline(always)]
  fn drop(&mut self) {
    stack_deregister(self.0)
  }
}
