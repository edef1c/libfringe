// This file is part of libfringe, a low-level green threading library.
// Copyright (c) Nathan Zadoks <nathan@nathan7.eu>
// See the LICENSE file included in this distribution.
extern crate winapi;
extern crate kernel32;
use core::{mem, ptr};
use self::winapi::basetsd::SIZE_T;
use self::winapi::minwindef::{DWORD, LPVOID};
use self::winapi::winnt::{MEM_COMMIT, MEM_RESERVE, MEM_RELEASE, PAGE_READWRITE, PAGE_NOACCESS};
use super::page_size;

#[cfg(windows)]
pub fn sys_page_size() -> usize {
  unsafe {
    let mut info = mem::zeroed();
    kernel32::GetSystemInfo(&mut info);
    info.dwPageSize as usize
  }
}

pub unsafe fn map_stack(len: usize) -> Option<*mut u8> {
  let ptr = kernel32::VirtualAlloc(ptr::null_mut(), len as SIZE_T, MEM_COMMIT | MEM_RESERVE, PAGE_READWRITE);
  if ptr == ptr::null_mut() {
    None
  } else {
    Some(ptr as *mut u8)
  }
}

pub unsafe fn protect_stack(ptr: *mut u8) -> bool {
  let mut old_prot: DWORD = 0;
  kernel32::VirtualProtect(ptr as LPVOID, page_size() as SIZE_T, PAGE_NOACCESS, &mut old_prot) != 0
}

pub unsafe fn unmap_stack(ptr: *mut u8, _len: usize) -> bool {
  kernel32::VirtualFree(ptr as LPVOID, 0, MEM_RELEASE) != 0
}
