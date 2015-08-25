// This file is part of libfringe, a low-level green threading library.
// Copyright (c) 2015, Nathan Zadoks <nathan@nathan7.eu>
// See the LICENSE file included in this distribution.
#![feature(no_std)]
#![feature(asm, core)]
#![no_std]

//! libfringe is a low-level green threading library.
//! It provides only a context-swapping mechanism.

#[cfg(test)]
#[macro_use]
extern crate std;

extern crate void;

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
