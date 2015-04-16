// Copyright (c) 2015, Nathan Zadoks <nathan@nathan7.eu>
// See the LICENSE file included in this distribution.
use core::prelude::*;
pub use self::imp::{Registers, STACK_ALIGN};

unsafe impl Send for Registers {}

mod common;

#[cfg(target_arch = "x86_64")]
#[path = "x86_64/mod.rs"]
mod imp;
