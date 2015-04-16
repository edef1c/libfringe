// Copyright (c) 2015, Nathan Zadoks <nathan@nathan7.eu>
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
pub use stack::Stack;

#[cfg(feature = "os")]
pub use os::Stack as OsStack;

mod context;
mod stack;

#[cfg(feature = "os")]
mod os;

mod arch;
mod debug;

#[cfg(not(test))]
mod std { pub use core::*; }
