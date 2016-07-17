// This file is part of libfringe, a low-level green threading library.
// Copyright (c) Nathan Zadoks <nathan@nathan7.eu>
// See the LICENSE file included in this distribution.
extern crate std;
use self::std::io::Error as IoError;
use stack;

mod sys;

/// This object represents a stack from the operating system's
/// anonymous memory mapping facility, usually `mmap`.
/// The stack it provides comes with a guard page, which is not included
/// in the stack limit.
#[derive(Debug)]
pub struct Stack {
  ptr: *mut u8,
  len: usize
}

unsafe impl Send for Stack {}

impl Stack {
  pub fn new(size: usize) -> Result<Stack, IoError> {
    let page_size = sys::page_size();

    // Round the length one page size up, using the fact that the page size
    // is a power of two.
    let len = (size + page_size - 1) & !(page_size - 1);

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
  fn top(&self) -> *mut u8 {
    unsafe {
      self.ptr.offset(self.len as isize)
    }
  }

  fn limit(&self) -> *mut u8 {
    unsafe {
      self.ptr.offset(sys::page_size() as isize)
    }
  }
}

impl Drop for Stack {
  fn drop(&mut self) {
    unsafe { sys::unmap_stack(self.ptr, self.len) }.expect("cannot unmap stack")
  }
}
