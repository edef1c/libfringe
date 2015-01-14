use core::prelude::*;
use platform::Stack;
use arch::{self, Registers};
use platform;

pub struct Context {
  regs: Registers,
  _stack: platform::Stack
}

impl Context {
  #[inline]
  pub unsafe fn new<F>(f: F) -> Context where F: FnOnce() + Send + 'static {
    let mut stack = Stack::new(4 << 20);
    let regs = arch::initialize_call_frame(&mut stack, f);
    Context {
      regs: regs,
      _stack: stack
    }
  }

  #[inline(always)]
  pub unsafe fn swap(&mut self) {
    arch::swap(&mut self.regs)
  }
}
