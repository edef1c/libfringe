// This file is part of libfringe, a low-level green threading library.
// Copyright (c) edef <edef@edef.eu>
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.
//! Traits for stacks.

/// A trait for objects that hold ownership of a stack.
///
/// To preserve memory safety, an implementation of this trait must fulfill
/// the following contract:
///
///   * The base of the stack must be aligned to a platform-specific boundary.
///   * Every address between the base and the limit must be readable and writable.
///
/// On any platform supported by libfringe, it is safe to align the stack to a 16-byte boundary.
/// The platform ABI document lists the specific alignment requirement.
pub trait Stack {
  /// Returns the base of the stack.
  /// On all modern architectures, the stack grows downwards,
  /// so this is the highest address.
  fn base(&self) -> *mut u8;
  /// Returns the limit of the stack.
  /// On all modern architectures, the stack grows downwards,
  /// so this is the lowest address.
  fn limit(&self) -> *mut u8;
}

/// A marker trait for `Stack` objects with a guard page.
///
/// To preserve memory safety, an implementation of this trait must fulfill
/// the following contract, in addition to the [contract](trait.Stack.html) of `Stack`:
///
///   * Any access of data at addresses `limit()` to `limit().offset(4096)` must
///     abnormally terminate, at least, the thread that performs the access.
pub unsafe trait GuardedStack {}
