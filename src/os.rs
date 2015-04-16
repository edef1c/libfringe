extern crate std;
use core::prelude::*;
use self::std::io::Error as IoError;
use stack;

#[cfg(unix)]
#[path = "os/unix.rs"] mod sys;

#[allow(raw_pointer_derive)]
#[derive(Debug)]
pub struct Stack {
  ptr: *mut u8,
  len: usize
}

pub struct StackSource;

impl stack::StackSource for StackSource {
  type Output = Stack;
  fn get_stack(size: usize) -> Stack {
    Stack::new(size)
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

impl Stack {
  fn new(size: usize) -> Stack {
    let page_size = sys::page_size();

    // round the page size up,
    // using the fact that it is a power of two
    let len = (size + page_size - 1) & !(page_size - 1);

    let stack = unsafe {
      let ptr = match sys::map_stack(size) {
        None => {
          panic!("mmap for stack of size {} failed: {}",
                 len, IoError::last_os_error())
        }
        Some(ptr) => ptr
      };

      Stack { ptr: ptr as *mut u8, len: len }
    };

    unsafe {
      if !sys::protect_stack(stack.ptr) {
        panic!("mprotect for guard page of stack {:p} failed: {}",
               stack.ptr, IoError::last_os_error());
      }
    }

    stack
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
