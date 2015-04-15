#![feature(no_std)]
#![feature(asm, core)]
#![feature(libc, page_size)]
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
pub mod stack;

mod arch;
mod platform;
