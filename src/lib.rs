#![feature(default_type_params)]
extern crate libc;
extern crate fn_box;

pub use context::Context;

mod context;
mod stack;
