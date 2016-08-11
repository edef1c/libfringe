// This file is part of libfringe, a low-level green threading library.
// Copyright (c) Nathan Zadoks <nathan@nathan7.eu>,
//               whitequark <whitequark@whitequark.org>
// See the LICENSE file included in this distribution.

pub use self::imp::*;

#[allow(unused_attributes)] // rust-lang/rust#35584
#[cfg_attr(target_arch = "x86",    path = "x86.rs")]
#[cfg_attr(target_arch = "x86_64", path = "x86_64.rs")]
mod imp;
