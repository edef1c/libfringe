extern crate libc;
extern crate std;
use self::std::prelude::v1::*;
use self::std::env;
use self::std::io::Error as IoError;
use self::libc::{c_void, size_t};
use core::ptr;
use stack;
use valgrind;

#[allow(raw_pointer_derive)]
#[derive(Debug)]
pub struct Stack {
  ptr: *mut u8,
  len: usize,
  valgrind_id: valgrind::stack_id_t
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
      self.ptr.offset(env::page_size() as isize)
    }
  }
}

impl Stack {
  fn new(size: usize) -> Stack {
    let page_size = env::page_size();

    // round the page size up,
    // using the fact that it is a power of two
    let len = (size + page_size - 1) & !(page_size - 1);

    const STACK_PROT: libc::c_int = libc::PROT_READ | libc::PROT_WRITE;
    const STACK_FLAGS: libc::c_int = libc::MAP_STACK
                                   | libc::MAP_PRIVATE
                                   | libc::MAP_ANON;

    let stack = unsafe {
      let ptr = libc::mmap(ptr::null_mut(), len as size_t,
                           STACK_PROT, STACK_FLAGS, -1, 0);

      if ptr == libc::MAP_FAILED {
        panic!("mmap for stack of size {} failed: {:?}",
               len, IoError::last_os_error())
      }

      let valgrind_id =
        valgrind::stack_register(ptr.offset(len as isize) as *const _,
                                 ptr as *const _);

      Stack { ptr: ptr as *mut u8, len: len, valgrind_id: valgrind_id }
    };

    stack.protect();

    stack
  }

  fn protect(&self) {
    unsafe {
      if libc::mprotect(self.ptr as *mut c_void,
                        env::page_size() as libc::size_t,
                        libc::PROT_NONE) != 0 {
        panic!("mprotect for guard page of stack {:p} failed: {:?}",
               self.ptr, IoError::last_os_error());
      }
    }
  }
}

impl Drop for Stack {
  fn drop(&mut self) {
    unsafe {
      valgrind::stack_deregister(self.valgrind_id);
      if libc::munmap(self.ptr as *mut c_void, self.len as size_t) != 0 {
        panic!("munmap for stack {:p} of size {} failed: {:?}",
               self.ptr, self.len, IoError::last_os_error())
      }
    }
  }
}
