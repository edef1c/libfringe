extern crate libc;
use core::prelude::*;
pub use self::libc::{c_void, c_int, size_t};
pub use self::libc::{mmap, mprotect, munmap};
pub use self::libc::MAP_FAILED;

use core::ptr;

pub fn page_size() -> usize {
  unsafe {
    libc::sysconf(libc::_SC_PAGESIZE) as usize
  }
}

pub const GUARD_PROT:  c_int = libc::PROT_NONE;
pub const STACK_PROT:  c_int = libc::PROT_READ
                             | libc::PROT_WRITE;
pub const STACK_FLAGS: c_int = libc::MAP_STACK
                             | libc::MAP_PRIVATE
                             | libc::MAP_ANON;

pub unsafe fn map_stack(len: usize) -> Option<*mut u8> {
  let ptr = mmap(ptr::null_mut(), len as size_t,
                 STACK_PROT, STACK_FLAGS, -1, 0);
  if ptr != MAP_FAILED {
    Some(ptr as *mut u8)
  }
  else {
    None
  }
}

pub unsafe fn protect_stack(ptr: *mut u8) -> bool {
  mprotect(ptr as *mut c_void, page_size() as size_t, GUARD_PROT) == 0
}

pub unsafe fn unmap_stack(ptr: *mut u8, len: usize) -> bool {
  munmap(ptr as *mut c_void, len as size_t) == 0
}
