// This file is part of libfringe, a low-level green threading library.
// Copyright (c) Nathan Zadoks <nathan@nathan7.eu>
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.
#![feature(asm, naked_functions, cfg_target_vendor)]
#![feature(never_type, slice_patterns)]
#![cfg_attr(feature = "alloc", feature(alloc, heap_api))]
#![cfg_attr(test, feature(test))]
#![no_std]

//! libfringe is a library implementing safe, lightweight context switches,
//! without relying on kernel services. It can be used in hosted environments
//! (using `std`) as well as on bare metal (using `core`).
//!
//! It provides the following safe abstractions:
//!
//!   * an implementation of generators,
//!     [Generator](generator/struct.Generator.html).
//!
//! It also provides the necessary low-level building blocks:
//!
//!   * a trait that can be implemented by stack allocators,
//!     [Stack](struct.Stack.html);
//!   * a wrapper for using slice references as stacks,
//!     [SliceStack](struct.SliceStack.html);
//!   * a stack allocator based on `Box<[u8]>`,
//!     [OwnedStack](struct.OwnedStack.html);
//!   * a stack allocator based on anonymous memory mappings with guard pages,
//!     [OsStack](struct.OsStack.html).

#[cfg(test)]
#[macro_use]
extern crate std;

pub use stack::Stack;
pub use stack::GuardedStack;
pub use slice_stack::SliceStack;
pub use generator::Generator;

#[cfg(feature = "alloc")]
pub use owned_stack::OwnedStack;

#[cfg(unix)]
pub use os::Stack as OsStack;

mod arch;

/// Minimum alignment of a stack base address on the target platform.
pub const STACK_ALIGNMENT: usize = arch::STACK_ALIGNMENT;

mod debug;
mod stack_pointer;

mod fat_args;
mod stack;
mod slice_stack;
pub mod generator;

#[cfg(feature = "alloc")]
mod owned_stack;

#[cfg(unix)]
mod os;
