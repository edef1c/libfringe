// This file is part of libfringe, a low-level green threading library.
// Copyright (c) edef <edef@edef.eu>
// See the LICENSE file included in this distribution.
extern crate std;
use self::std::io::Error as IoError;
use stack;

mod sys;

/// OsStack holds a guarded stack allocated using the operating system's anonymous
/// memory mapping facility.
#[derive(Debug)]
pub struct Stack {
  ptr: *mut u8,
  len: usize
}

unsafe impl Send for Stack {}

impl Stack {
  /// Allocates a new stack with at least `size` accessible bytes.
  /// `size` is rounded up to an integral number of pages; `Stack::new(0)` is legal
  /// and allocates the smallest possible stack, consisting of one data page and
  /// one guard page.
  pub fn new(size: usize) -> Result<Stack, IoError> {
    let page_size = sys::page_size();

    // Stacks have to be at least one page long.
    let len = if size == 0 { page_size } else { size };

    // Round the length one page size up, using the fact that the page size
    // is a power of two.
    let len = (len + page_size - 1) & !(page_size - 1);

    // Increase the length to fit the guard page.
    let len = len + page_size;

    // Allocate a stack.
    let stack = Stack {
      ptr: try!(unsafe { sys::map_stack(len) }),
      len: len
    };

    // Mark the guard page. If this fails, `stack` will be dropped,
    // unmapping it.
    try!(unsafe { sys::protect_stack(stack.ptr) });

    Ok(stack)
  }
}

impl stack::Stack for Stack {
  #[inline(always)]
  fn top(&self) -> *mut u8 {
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

unsafe impl stack::GuardedStack for Stack {}

impl Drop for Stack {
  fn drop(&mut self) {
    unsafe { sys::unmap_stack(self.ptr, self.len) }.expect("cannot unmap stack")
  }
}
