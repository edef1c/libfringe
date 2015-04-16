// Copyright (c) 2015, Nathan Zadoks <nathan@nathan7.eu>
// See the LICENSE file included in this distribution.

extern crate std;
use core::prelude::*;
use self::std::io::Error as IoError;
use stack;
mod sys;

/// This object represents a stack from the operating system's
/// anonymous memory mapping facility, usually `mmap`.
/// The stack it provides comes with a guard page, which is not included
/// in the stack limit.
#[allow(raw_pointer_derive)]
#[derive(Debug)]
pub struct Stack {
  ptr: *mut u8,
  len: usize
}

unsafe impl Send for Stack {}

impl Stack {
  pub fn new(size: usize) -> Result<Stack, IoError> {
    let page_size = sys::page_size();

    // round the page size up,
    // using the fact that it is a power of two
    let len = (size + page_size - 1) & !(page_size - 1);

    let stack = unsafe {
      let ptr = try!(match sys::map_stack(size) {
        None => Err(IoError::last_os_error()),
        Some(ptr) => Ok(ptr)
      });

      Stack { ptr: ptr as *mut u8, len: len }
    };

    try!(unsafe {
      if sys::protect_stack(stack.ptr) { Ok(()) }
      else { Err(IoError::last_os_error()) }
    });

    Ok(stack)
  }
}

impl stack::Stack for Stack {
  fn top(&mut self) -> *mut u8 {
    unsafe {
      self.ptr.offset(self.len as isize)
    }
  }

  fn limit(&self) -> *const u8 {
    unsafe {
      self.ptr.offset(sys::page_size() as isize)
    }
  }
}

impl Drop for Stack {
  fn drop(&mut self) {
    unsafe {
      if !sys::unmap_stack(self.ptr, self.len) {
        panic!("munmap for stack {:p} of size {} failed: {}",
               self.ptr, self.len, IoError::last_os_error())
      }
    }
  }
}
