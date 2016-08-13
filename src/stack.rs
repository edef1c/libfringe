// This file is part of libfringe, a low-level green threading library.
// Copyright (c) Nathan Zadoks <nathan@nathan7.eu>
// See the LICENSE file included in this distribution.
//! Traits for stacks.

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
