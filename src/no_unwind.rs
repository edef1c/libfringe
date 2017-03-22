// This file is part of libfringe, a low-level green threading library.
// Copyright (c) Amanieu d'Antras <amanieu@gmail.com>,
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use arch::StackPointer;

pub unsafe extern "C" fn unwind_wrapper(arg: usize, sp: StackPointer, _stack_base: *mut u8,
                                        f: unsafe fn(usize, StackPointer)) -> usize {
  f(arg, sp);
  0
}

pub unsafe extern "C" fn start_unwind(_panic: usize) -> ! {
  unreachable!();
}

#[inline]
pub fn unwind_arg(_stack_base: *mut u8) -> usize {
  unreachable!();
}
