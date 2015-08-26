// This file is part of libfringe, a low-level green threading library.
// Copyright (c) 2015, Nathan Zadoks <nathan@nathan7.eu>
// See the LICENSE file included in this distribution.
use core::mem::{size_of, align_of};
use core::cmp::max;
use core::ptr;

use void::{self, Void};

use super::imp::STACK_ALIGN;

pub unsafe extern "C" fn rust_trampoline<F>(f: *const F) -> !
  where F: FnOnce() -> Void {
  void::unreachable(ptr::read(f)())
}

pub unsafe fn push<T>(spp: &mut *mut usize, value: T) -> *mut T {
  let mut sp = *spp as *mut T;
  sp = offset_mut(sp, -1);
  sp = align_down_mut(sp, max(align_of::<T>(), STACK_ALIGN));
  ptr::write(sp, value); // does not attempt to drop old value
  *spp = sp as *mut usize;
  sp
}

pub fn align_down_mut<T>(sp: *mut T, n: usize) -> *mut T {
  let sp = (sp as usize) & !(n - 1);
  sp as *mut T
}

// ptr::offset_mut is positive ints only
pub fn offset_mut<T>(ptr: *mut T, count: isize) -> *mut T {
  (ptr as isize + count * (size_of::<T>() as isize)) as *mut T
}
