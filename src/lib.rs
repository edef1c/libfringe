// This file is part of libfringe, a low-level green threading library.
// Copyright (c) Nathan Zadoks <nathan@nathan7.eu>
// See the LICENSE file included in this distribution.
#![feature(asm)]
#![no_std]

//! libfringe is a low-level green threading library.
//! It provides only a context-swapping mechanism.

pub use context::Context;
pub use stack::Stack;

#[cfg(any(unix, windows))]
pub use os::Stack as OsStack;

mod context;
mod stack;

#[cfg(any(unix, windows))]
mod os;

mod arch;
mod debug;
