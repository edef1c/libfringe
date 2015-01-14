#![feature(asm)]
#![no_std]

#[macro_use]
#[allow(unstable)]
extern crate core;

pub use context::Context;

mod std { pub use core::*; }

mod context;
mod stack;

mod arch;
mod platform;
