// This file is part of libfringe, a low-level green threading library.
// Copyright (c) Nathan Zadoks <nathan@nathan7.eu>
// See the LICENSE file included in this distribution.
//! Traits for stacks.

/// A trait for objects that hold ownership of a stack.
pub trait Stack {
  /// Returns the top of the stack.
  /// On all modern architectures, the stack grows downwards,
  /// so this is the highest address.
  fn top(&mut self) -> *mut u8;
  /// Returns the bottom of the stack.
  /// On all modern architectures, the stack grows downwards,
  /// so this is the lowest address.
  fn limit(&self) -> *const u8;
}
