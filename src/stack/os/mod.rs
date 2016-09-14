// This file is part of libfringe, a low-level green threading library.
// Copyright (c) edef <edef@edef.eu>
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.
extern crate std;

use self::std::io::Error as IoError;
use stack::{Stack, GuardedStack};

mod sys;

/// OsStack holds a guarded stack allocated using the operating system's anonymous
/// memory mapping facility.
#[derive(Debug)]
pub struct OsStack {
  ptr: *mut u8,
  len: usize,
}

unsafe impl Send for OsStack {}

impl OsStack {
  /// Allocates a new stack with at least `size` accessible bytes.
  /// `size` is rounded up to an integral number of pages; `OsStack::new(0)` is legal
  /// and allocates the smallest possible stack, consisting of one data page and
  /// one guard page.
  pub fn new(size: usize) -> Result<OsStack, IoError> {
    let page_size = sys::page_size();

    // Stacks have to be at least 16KB to support unwinding.
    let len = if size == 0 { 16384 } else { size };

    // Round the length one page size up, using the fact that the page size
    // is a power of two.
    let len = (len + page_size - 1) & !(page_size - 1);

    // Increase the length to fit the guard page.
    let len = len + page_size;

    // Allocate a stack.
    let ptr = try!(unsafe { sys::map_stack(len) });
    let stack = OsStack {
      ptr: ptr,
      len: len,
    };

    // Mark the guard page. If this fails, `stack` will be dropped,
    // unmapping it.
    try!(unsafe { sys::protect_stack(stack.ptr) });

    Ok(stack)
  }
}

unsafe impl Stack for OsStack {
  #[inline(always)]
  fn base(&self) -> *mut u8 {
    unsafe {
      self.ptr.offset(self.len as isize)
    }
  }

  #[inline(always)]
  fn limit(&self) -> *mut u8 {
    unsafe {
      self.ptr.offset(sys::page_size() as isize)
    }
  }
}

unsafe impl GuardedStack for OsStack {}

impl Drop for OsStack {
  fn drop(&mut self) {
    unsafe { sys::unmap_stack(self.ptr, self.len) }.expect("cannot unmap stack")
  }
}
