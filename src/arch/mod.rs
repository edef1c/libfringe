// This file is part of libfringe, a low-level green threading library.
// Copyright (c) 2015, Nathan Zadoks <nathan@nathan7.eu>
// See the LICENSE file included in this distribution.
use core::prelude::*;
pub use self::imp::Registers;

unsafe impl Send for Registers {}

mod common;

#[cfg(target_arch = "x86_64")]
#[path = "x86_64/mod.rs"]
mod imp;

#[cfg(target_arch = "x86")]
#[path = "x86/mod.rs"]
mod imp;
