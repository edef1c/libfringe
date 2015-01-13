#![feature(asm)]
#![no_std]

#[macro_use]
extern crate core;
extern crate alloc;
extern crate fn_box;

pub use context::Context;

mod std { pub use core::fmt; }

#[macro_use]
mod macros;

mod context;
mod stack;

mod arch;
mod platform;
