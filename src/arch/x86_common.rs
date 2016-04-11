// This file is part of libfringe, a low-level green threading library.
// Copyright (c) 2015, Nathan Zadoks <nathan@nathan7.eu>
// See the LICENSE file included in this distribution.
use void::Void;

use stack::Stack;
use arch::common::{push, rust_trampoline};

pub const STACK_ALIGN: usize = 16;

#[derive(Debug)]
pub struct Registers {
  stack_pointer: *mut usize
}

impl Registers {
  #[inline]
  pub unsafe fn new<S, F>(stack: &mut S, f: F) -> Registers
    where S: Stack, F: FnOnce() -> Void
  {
    let mut sp = stack.top() as *mut usize;
    let f_ptr = push(&mut sp, f);

    init!(sp, f_ptr, rust_trampoline::<F> as unsafe extern "C" fn(*const F) -> !);

    Registers {
      stack_pointer: sp,
    }
  }

  #[inline(always)]
  pub unsafe fn swap(out_regs: *mut Registers, in_regs: *const Registers) {
    swap!(&mut (*out_regs).stack_pointer, &(*in_regs).stack_pointer);
  }
}
