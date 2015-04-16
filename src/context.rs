use core::prelude::*;
use arch::Registers;
use stack;

pub struct Context<Stack: stack::Stack> {
  regs: Registers,
  stack: Stack
}

impl<Stack> Context<Stack> where Stack: stack::Stack {
  #[inline]
  pub unsafe fn new<F>(mut stack: Stack, f: F) -> Context<Stack>
    where F: FnOnce() + Send + 'static {
    let regs = Registers::new(&mut stack, f);
    Context {
      regs: regs,
      stack: stack
    }
  }

  #[inline(always)]
  pub unsafe fn swap(&mut self) {
    self.regs.swap()
  }

  pub unsafe fn destroy(self) -> Stack {
    self.stack
  }
}
