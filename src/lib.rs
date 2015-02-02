#![feature(asm, core)]
#![feature(os, libc)]
#![no_std]

#[macro_use]
extern crate core;

#[cfg(test)]
#[macro_use]
extern crate std;

pub use context::Context;

#[cfg(not(test))]
mod std { pub use core::*; }

mod context;
mod stack;

mod arch;
mod platform;
