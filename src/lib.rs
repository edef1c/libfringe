// This file is part of libfringe, a low-level green threading library.
// Copyright (c) edef <edef@edef.eu>
// See the LICENSE file included in this distribution.
#![feature(asm)]
#![cfg_attr(target_arch = "x86", feature(naked_functions, core_intrinsics))]
#![no_std]

//! libfringe is a library implementing lightweight context switches,
//! without relying on kernel services. It can be used in hosted environments
//! (using `std`) as well as on bare metal (using `core`).
//!
//! It provides high-level, safe abstractions:
//!
//!   * an implementation of internal iterators, also known as generators,
//!     [Generator](generator/struct.Generator.html).
//!
//! It also provides low-level, *very* unsafe building blocks:
//!
//!   * a flexible, low-level context-swapping mechanism,
//!     [Context](struct.Context.html);
//!   * a trait that can be implemented by stack allocators,
//!     [Stack](struct.Stack.html);
//!   * a stack allocator based on anonymous memory mappings with guard pages,
//!     [OsStack](struct.OsStack.html).
//!
//! **FIXME:** not actually safe yet in presence of unwinding

#[cfg(test)]
#[macro_use]
extern crate std;

pub use stack::Stack;
pub use stack::GuardedStack;
pub use context::Context;
pub use generator::Generator;

#[cfg(any(unix, windows))]
pub use os::Stack as OsStack;

mod arch;
mod debug;

mod stack;
mod context;
pub mod generator;

#[cfg(any(unix, windows))]
mod os;
