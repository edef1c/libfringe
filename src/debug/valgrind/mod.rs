// Copyright (c) 2015, edef <edef@edef.eu>
// See the LICENSE file included in this distribution.
pub use self::imp::*;

#[cfg(feature = "valgrind")]
#[path = "native.rs"]
mod imp;

#[cfg(not(feature = "valgrind"))]
mod imp {
  //! Stub for the Valgrind functions
  #![allow(non_camel_case_types)]
  pub type stack_id_t = ();
  pub unsafe fn stack_register(_start: *const u8, _end: *const u8) -> stack_id_t {}
  pub unsafe fn stack_deregister(_id: stack_id_t) {}
}
