// Copyright (c) 2015, edef <edef@edef.eu>
// See the LICENSE file included in this distribution.
use core::prelude::*;
use stack;

mod valgrind;

pub struct StackId(valgrind::stack_id_t);

impl StackId {
  #[inline(always)]
  pub fn register<Stack: stack::Stack>(stack: &mut Stack) -> StackId {
    StackId(unsafe {
      valgrind::stack_register(stack.limit(), stack.top())
    })
  }
}

impl Drop for StackId {
  #[inline(always)]
  fn drop(&mut self) {
    unsafe {
      valgrind::stack_deregister(self.0)
    }
  }
}
