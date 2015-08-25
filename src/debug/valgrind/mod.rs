// This file is part of libfringe, a low-level green threading library.
// Copyright (c) 2015, edef <edef@edef.eu>
// See the LICENSE file included in this distribution.
#![allow(non_camel_case_types)]
//! In order for Valgrind to keep track of stack overflows and such, it needs
//! a little help. That help unfortunately comes in the form of a set of C
//! macros. Calling out to un-inlineable C code for this is pointlessly slow,
//! but that's the way it is for now.
use stack;

pub type stack_id_t = u32;
extern "C" {
  #[link_name = "valgrind_stack_register"]
  /// Register a stack with Valgrind. Returns an integer ID that can
  /// be used to deregister the stack when it's deallocated.
  /// `start < end`.
  pub fn stack_register(start: *const u8, end: *const u8) -> stack_id_t;

  #[link_name = "valgrind_stack_deregister"]
  /// Deregister a stack from Valgrind. Takes the integer ID that was returned
  /// on registration.
  pub fn stack_deregister(id: stack_id_t);
}

#[derive(Debug)]
pub struct StackId(stack_id_t);

impl StackId {
  #[inline(always)]
  pub fn register<Stack: stack::Stack>(stack: &mut Stack) -> StackId {
    StackId(unsafe {
      stack_register(stack.limit(), stack.top())
    })
  }
}

impl Drop for StackId {
  #[inline(always)]
  fn drop(&mut self) {
    unsafe {
      stack_deregister(self.0)
    }
  }
}
