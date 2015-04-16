// Copyright (c) 2015, edef <edef@edef.eu>
// See the LICENSE file included in this distribution.
pub use self::imp::{Registers, STACK_ALIGN};

mod common;

#[cfg(target_arch = "x86_64")]
#[path = "x86_64/mod.rs"]
mod imp;
