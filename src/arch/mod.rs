// Copyright (c) 2015, edef <edef@edef.eu>
// See the LICENSE file included in this distribution.
use core::prelude::*;
pub use self::imp::Registers;

unsafe impl Send for Registers {}

mod common;

#[cfg(target_arch = "x86_64")]
#[path = "x86_64/mod.rs"]
mod imp;
