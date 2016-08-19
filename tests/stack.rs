// This file is part of libfringe, a low-level green threading library.
// Copyright (c) whitequark <whitequark@whitequark.org>
// See the LICENSE file included in this distribution.
extern crate fringe;

use fringe::{Stack, SliceStack, OsStack};

#[test]
fn slice_stack() {
    let mut memory = [0; 1024];
    let stack = SliceStack(&mut memory);
    assert_eq!(stack.base() as isize - stack.limit() as isize, 1024);
}

#[test]
fn default_os_stack() {
  let stack = OsStack::new(0).unwrap();
  // Make sure the topmost page of the stack, at least, is accessible.
  unsafe { *(stack.base().offset(-1)) = 0; }
}

#[test]
fn one_page_os_stack() {
  let stack = OsStack::new(4096).unwrap();
  // Make sure the topmost page of the stack, at least, is accessible.
  unsafe { *(stack.base().offset(-1)) = 0; }
}
