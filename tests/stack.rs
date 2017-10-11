// This file is part of libfringe, a low-level green threading library.
// Copyright (c) whitequark <whitequark@whitequark.org>
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.
#![feature(alloc, allocator_api)]

extern crate alloc;
extern crate fringe;

use alloc::heap::{Heap, Layout};
use alloc::allocator::Alloc;
use alloc::boxed::Box;
use std::slice;
use fringe::{OsStack, OwnedStack, SliceStack, Stack, STACK_ALIGNMENT};

unsafe fn heap_allocate(size: usize, align: usize) -> *mut u8 {
  Heap.alloc(Layout::from_size_align_unchecked(size, align)).expect("couldn't allocate")
}

#[test]
fn slice_aligned() {
    unsafe {
        let ptr = Heap.alloc(Layout::from_size_align(16384, ::STACK_ALIGNMENT).unwrap())
            .unwrap_or_else(|err| Heap.oom(err));
        let mut slice = Box::from_raw(slice::from_raw_parts_mut(ptr, 16384));
        let stack = SliceStack::new(&mut slice[4096..8192]);
        assert_eq!(stack.base() as usize & (STACK_ALIGNMENT - 1), 0);
        assert_eq!(stack.limit() as usize & (STACK_ALIGNMENT - 1), 0);
    }
}

#[test]
fn slice_unaligned() {
    unsafe {
        let ptr = Heap.alloc(Layout::from_size_align(16384, ::STACK_ALIGNMENT).unwrap())
            .unwrap_or_else(|err| Heap.oom(err));
        let mut slice = Box::from_raw(slice::from_raw_parts_mut(ptr, 16384));
        let stack = SliceStack::new(&mut slice[4097..8193]);
        assert_eq!(stack.base() as usize & (STACK_ALIGNMENT - 1), 0);
        assert_eq!(stack.limit() as usize & (STACK_ALIGNMENT - 1), 0);
    }
}

#[test]
fn slice_too_small() {
    unsafe {
        let ptr = Heap.alloc(Layout::from_size_align(16384, ::STACK_ALIGNMENT).unwrap())
            .unwrap_or_else(|err| Heap.oom(err));
        println!("test");
        let mut slice = Box::from_raw(slice::from_raw_parts_mut(ptr, STACK_ALIGNMENT));
        println!("test");

        let stack = SliceStack::new(&mut slice[0..1]);
        // println!("test");

        assert_eq!(stack.base() as usize & (STACK_ALIGNMENT - 1), 0);
        // println!("test");

        assert_eq!(stack.limit() as usize & (STACK_ALIGNMENT - 1), 0);
        // println!("test");

    }
}

#[test]
#[should_panic(expected = "SliceStack too small")]
fn slice_too_small_unaligned() {
    unsafe {
        let ptr = Heap.alloc(
            Layout::from_size_align(STACK_ALIGNMENT, ::STACK_ALIGNMENT).unwrap(),
        ).unwrap_or_else(|err| Heap.oom(err));
        let mut slice = Box::from_raw(slice::from_raw_parts_mut(ptr, STACK_ALIGNMENT));
        SliceStack::new(&mut slice[1..2]);
    }
}

#[test]
fn slice_stack() {
    let mut memory = [0; 1024];
    let stack = SliceStack::new(&mut memory);
    assert_eq!(stack.base() as usize & (STACK_ALIGNMENT - 1), 0);
    assert_eq!(stack.limit() as usize & (STACK_ALIGNMENT - 1), 0);

    // Size may be a bit smaller due to alignment
    assert!(stack.base() as usize - stack.limit() as usize > 1024 - STACK_ALIGNMENT * 2);
}

#[test]
fn owned_stack() {
    let stack = OwnedStack::new(1024);
    assert_eq!(stack.base() as usize & (STACK_ALIGNMENT - 1), 0);
    assert_eq!(stack.limit() as usize & (STACK_ALIGNMENT - 1), 0);
    assert_eq!(stack.base() as usize - stack.limit() as usize, 1024);
}

#[test]
fn default_os_stack() {
    let stack = OsStack::new(0).unwrap();
    assert_eq!(stack.base() as usize & (STACK_ALIGNMENT - 1), 0);
    assert_eq!(stack.limit() as usize & (STACK_ALIGNMENT - 1), 0);

    // Make sure the topmost page of the stack, at least, is accessible.
    unsafe {
        *(stack.base().offset(-1)) = 0;
    }
}

#[test]
fn one_page_os_stack() {
    let stack = OsStack::new(4096).unwrap();
    assert_eq!(stack.base() as usize & (STACK_ALIGNMENT - 1), 0);
    assert_eq!(stack.limit() as usize & (STACK_ALIGNMENT - 1), 0);

    // Make sure the topmost page of the stack, at least, is accessible.
    unsafe {
        *(stack.base().offset(-1)) = 0;
    }
}
