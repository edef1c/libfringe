// This file is part of libfringe, a low-level green threading library.
// Copyright (c) 2015, edef <edef@edef.eu>
// See the LICENSE file included in this distribution.
extern crate libc;
use self::libc::{c_void, c_int, size_t};
use self::libc::{mmap, mprotect, munmap};
use self::libc::MAP_FAILED;
use super::page_size;

use core::ptr;

#[cold]
pub fn sys_page_size() -> usize {
  unsafe {
    libc::sysconf(libc::_SC_PAGESIZE) as usize
  }
}

const GUARD_PROT:  c_int = libc::PROT_NONE;
const STACK_PROT:  c_int = libc::PROT_READ
                         | libc::PROT_WRITE;
#[cfg(not(any(target_os = "freebsd", target_os = "dragonfly")))]
const STACK_FLAGS: c_int = libc::MAP_STACK
                         | libc::MAP_PRIVATE
                         | libc::MAP_ANON;
// workaround for http://lists.freebsd.org/pipermail/freebsd-bugs/2011-July/044840.html
// according to libgreen, DragonFlyBSD suffers from this too
#[cfg(any(target_os = "freebsd", target_os = "dragonfly"))]
const STACK_FLAGS: c_int = libc::MAP_PRIVATE
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
