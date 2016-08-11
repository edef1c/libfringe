// This file is part of libfringe, a low-level green threading library.
// Copyright (c) edef <edef@edef.eu>
// See the LICENSE file included in this distribution.
#![feature(asm)]
#![cfg_attr(test, feature(test, thread_local, const_fn))]
#![cfg_attr(target_arch = "x86",    feature(naked_functions, core_intrinsics))]
#![cfg_attr(target_arch = "x86_64", feature(naked_functions, core_intrinsics))]
#![no_std]

//! libfringe is a library implementing safe, lightweight context switches,
//! without relying on kernel services. It can be used in hosted environments
//! (using `std`) as well as on bare metal (using `core`).
//!
//! It provides the following safe abstractions:
//!
//!   * an implementation of internal iterators, also known as generators,
//!     [Generator](generator/struct.Generator.html).
//!
//! It also provides the necessary low-level building blocks:
//!
//!   * a trait that can be implemented by stack allocators,
//!     [Stack](struct.Stack.html);
//!   * a stack allocator based on anonymous memory mappings with guard pages,
//!     [OsStack](struct.OsStack.html).

#[cfg(test)]
#[macro_use]
extern crate std;

pub use stack::Stack;
pub use stack::GuardedStack;
pub use generator::Generator;

#[cfg(unix)]
pub use os::Stack as OsStack;

mod arch;
mod debug;

mod stack;
mod context;
pub mod generator;

#[cfg(unix)]
mod os;
