// This file is part of libfringe, a low-level green threading library.
// Copyright (c) Nathan Zadoks <nathan@nathan7.eu>
// See the LICENSE file included in this distribution.
pub use self::imp::*;

#[cfg(feature = "valgrind")]
#[path = "valgrind.rs"]
mod imp;

#[cfg(not(feature = "valgrind"))]
mod imp {
  use stack;
  #[derive(Debug)]
  pub struct StackId;
  /// No-op since no valgrind
  impl StackId {
    pub fn register<Stack: stack::Stack>(_stack: &Stack) -> StackId {
      StackId
    }
  }
}
