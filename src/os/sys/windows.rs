// This file is part of libfringe, a low-level green threading library.
// Copyright (c) 2015, Nathan Zadoks <nathan@nathan7.eu>
// See the LICENSE file included in this distribution.
extern crate libc;
extern crate std;

use core::prelude::*;
//use self::libc::{c_void, c_int, size_t};
//use super::page_size;
use self::std::rt::heap::{allocate, deallocate};

//use core::ptr;

#[cold]
pub fn sys_page_size() -> usize {
  4096 as usize
}

pub unsafe fn map_stack(len: usize) -> Option<*mut u8> {
  let ptr = allocate(len, 8);
  if !ptr.is_null() {
    Some(ptr as *mut u8)
  }
  else {
    None
  }
}

pub unsafe fn protect_stack(_: *mut u8) -> bool {
  true
}

pub unsafe fn unmap_stack(ptr: *mut u8, len: usize) -> bool {
  deallocate(ptr, len, 8);
  true
}
