// This file is part of libfringe, a low-level green threading library.
// Copyright (c) 2015, Nathan Zadoks <nathan@nathan7.eu>
// See the LICENSE file included in this distribution.
#![feature(no_std)]
#![feature(asm, core)]
#![feature(alloc)]
#![no_std]

//! libfringe is a low-level green threading library.
//! It provides only a context-swapping mechanism.

#[macro_use]
extern crate core;

#[cfg(test)]
#[cfg(not(windows))]
#[macro_use]
extern crate std;

#[cfg(not(test))]
#[cfg(windows)]
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
#[cfg(not(windows))]
mod std { pub use core::*; }
