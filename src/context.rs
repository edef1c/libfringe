// Copyright (c) 2015, edef <edef@edef.eu>
// See the LICENSE file included in this distribution.
use core::prelude::*;
use core::marker::PhantomData;
use arch::Registers;
use stack;
use debug::StackId;

pub struct Context<'a, Stack: stack::Stack> {
  regs: Registers,
  _stack_id: StackId,
  stack: Stack,
  _ref: PhantomData<&'a ()>
}

impl<'a, Stack> Context<'a, Stack> where Stack: stack::Stack {
  #[inline]
  pub unsafe fn new<F>(mut stack: Stack, f: F) -> Context<'a, Stack>
    where F: FnOnce() + Send + 'a {
    let stack_id = StackId::register(&mut stack);
    let regs = Registers::new(&mut stack, f);
    Context {
      regs: regs,
      _stack_id: stack_id,
      stack: stack,
      _ref: PhantomData
    }
  }

  #[inline(always)]
  pub unsafe fn swap(&mut self) {
    self.regs.swap()
  }

  #[inline]
  pub unsafe fn destroy(self) -> Stack {
    self.stack
  }
}
