// Copyright (c) 2015, edef <edef@edef.eu>
// See the LICENSE file included in this distribution.
#![feature(no_std)]
#![feature(asm, core)]
#![feature(libc)]
#![no_std]

#[macro_use]
extern crate core;

#[cfg(test)]
#[macro_use]
extern crate std;

pub use context::Context;
pub use stack::{Stack, StackSource};

#[cfg(not(test))]
mod std { pub use core::*; }

mod context;
mod stack;

mod debug;

mod arch;

#[cfg(feature = "os")]
pub mod os;
