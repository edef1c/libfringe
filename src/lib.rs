#![feature(default_type_params, macro_rules)]
extern crate libc;
extern crate fn_box;

pub use context::Context;

mod context;
mod stack;

mod arch;
mod platform;
