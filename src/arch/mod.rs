// This file is part of libfringe, a low-level green threading library.
// Copyright (c) edef <edef@edef.eu>,
//               whitequark <whitequark@whitequark.org>
// See the LICENSE file included in this distribution.
pub use self::imp::*;

// rust-lang/rust#25544
// #[cfg_attr(target_arch = "x86",    path = "x86.rs")]
// #[cfg_attr(target_arch = "x86_64", path = "x86_64.rs")]
// mod imp;

#[cfg(target_arch = "x86")]
#[path = "x86.rs"]
mod imp;

#[cfg(target_arch = "x86_64")]
#[path = "x86_64.rs"]
mod imp;
