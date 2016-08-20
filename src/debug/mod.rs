// This file is part of libfringe, a low-level green threading library.
// Copyright (c) Nathan Zadoks <nathan@nathan7.eu>
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.
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
