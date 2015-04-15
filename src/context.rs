use core::prelude::*;
use stack::StackSource;
use arch::Registers;
use stack::Stack;
use os;

pub struct Context {
  regs: Registers,
  _stack: os::Stack
}

impl Context {
  #[inline]
  pub unsafe fn new<F>(f: F) -> Context where F: FnOnce() + Send + 'static {
    let mut stack = os::StackSource::get_stack(4 << 20);
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
