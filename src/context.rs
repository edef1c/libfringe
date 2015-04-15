use core::prelude::*;
use arch::Registers;
use os;

pub struct Context {
  regs: Registers,
  _stack: os::Stack
}

impl Context {
  #[inline]
  pub unsafe fn new<F>(mut stack: os::Stack, f: F) -> Context where F: FnOnce() + Send + 'static {
    let regs = Registers::new(&mut stack, f);
    Context {
      regs: regs,
      _stack: stack
    }
  }

  #[inline(always)]
  pub unsafe fn swap(&mut self) {
    self.regs.swap()
  }
}
