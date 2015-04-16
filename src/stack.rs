// Copyright (c) 2015, Nathan Zadoks <nathan@nathan7.eu>
// See the LICENSE file included in this distribution.
//! Traits for stacks.
use core::prelude::*;
use core::fmt::{Debug, Display};

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

/// A trait for objects that provide stacks of arbitrary size.
pub trait StackSource {
  type Output: Stack;
  type Error: Debug + Display = ();
  fn get_stack(&mut self, size: usize) -> Result<Self::Output, Self::Error>;
}
