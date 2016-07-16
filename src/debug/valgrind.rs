// This file is part of libfringe, a low-level green threading library.
// Copyright (c) edef <edef@edef.eu>
// See the LICENSE file included in this distribution.
extern crate valgrind;

use stack;
use self::valgrind::{stack_register, stack_deregister};

#[derive(Debug)]
pub struct StackId(self::valgrind::Value);

impl StackId {
  #[inline(always)]
  pub fn register<Stack: stack::Stack>(stack: &Stack) -> StackId {
    StackId(stack_register(stack.limit(), stack.top()))
  }
}

impl Drop for StackId {
  #[inline(always)]
  fn drop(&mut self) {
    stack_deregister(self.0)
  }
}
