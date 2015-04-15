use core::prelude::*;
use platform::Stack;
use arch::Registers;
use platform;

pub struct Context {
  regs: Registers,
  _stack: platform::Stack
}

impl Context {
  #[inline]
  pub unsafe fn new<F>(f: F) -> Context where F: FnOnce() + Send + 'static {
    let mut stack = Stack::new(4 << 20);
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
