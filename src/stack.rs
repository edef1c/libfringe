// This file is part of libfringe, a low-level green threading library.
// Copyright (c) Nathan Zadoks <nathan@nathan7.eu>
// See the LICENSE file included in this distribution.

/// A trait for objects that hold ownership of a stack.
pub trait Stack {
  /// Returns the base of the stack.
  /// On all modern architectures, the stack grows downwards,
  /// so this is the highest address.
  fn base(&self) -> *mut u8;
  /// Returns the bottom of the stack.
  /// On all modern architectures, the stack grows downwards,
  /// so this is the lowest address.
  fn limit(&self) -> *mut u8;
}

/// A marker trait for `Stack` objects with a guard page.
///
/// A guarded stack must guarantee that any access of data at addresses `limit()` to
/// `limit().offset(4096)` will abnormally terminate the program.
pub unsafe trait GuardedStack {}

/// SliceStack holds a non-guarded stack allocated elsewhere and provided as a mutable
/// slice.
pub struct SliceStack<'a>(pub &'a mut [u8]);

impl<'a> Stack for SliceStack<'a> {
    #[inline(always)]
    fn base(&self) -> *mut u8 {
        // The slice cannot wrap around the address space, so the conversion from usize
        // to isize will not wrap either.
        let len: isize = self.0.len() as isize;
        unsafe { self.limit().offset(len) }
    }

    #[inline(always)]
    fn limit(&self) -> *mut u8 {
        self.0.as_ptr() as *mut u8
    }
}
