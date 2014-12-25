#![feature(default_type_params, macro_rules, phase, globs, asm)]
#![no_std]

#[phase(plugin, link)]
extern crate core;
extern crate alloc;
extern crate fn_box;

pub use context::Context;

mod std { pub use core::fmt; }

#[macro_escape]
mod macros;

mod context;
mod stack;

mod arch;
mod platform;
