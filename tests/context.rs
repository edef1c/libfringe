// This file is part of libfringe, a low-level green threading library.
// Copyright (c) edef <edef@edef.eu>,
//               whitequark <whitequark@whitequark.org>
// See the LICENSE file included in this distribution.
#![feature(thread_local)]
extern crate simd;
extern crate fringe;

use fringe::Context;

#[thread_local]
static mut ctx_slot: *mut Context<fringe::OsStack> = 0 as *mut Context<_>;

#[test]
fn context() {
  unsafe extern "C" fn adder(arg: usize) -> ! {
    println!("it's alive! arg: {}", arg);
    let arg = Context::swap(ctx_slot, ctx_slot, arg + 1);
    println!("still alive! arg: {}", arg);
    Context::swap(ctx_slot, ctx_slot, arg + 1);
    panic!("i should be dead");
  }

  unsafe {
    let stack = fringe::OsStack::new(4 << 20).unwrap();
    let mut ctx = Context::new(stack, adder);
    ctx_slot = &mut ctx;

    let ret = Context::swap(ctx_slot, ctx_slot, 10);
    assert_eq!(ret, 11);
    let ret = Context::swap(ctx_slot, ctx_slot, 50);
    assert_eq!(ret, 51);
  }
}

#[test]
fn simd() {
  unsafe extern "C" fn permuter(arg: usize) -> ! {
    // This will crash if the stack is not aligned properly.
    let x = simd::i32x4::splat(arg as i32);
    let y = x * x;
    println!("simd result: {:?}", y);
    Context::swap(ctx_slot, ctx_slot, 0);
    // And try again after a context switch.
    let x = simd::i32x4::splat(arg as i32);
    let y = x * x;
    println!("simd result: {:?}", y);
    Context::swap(ctx_slot, ctx_slot, 0);
    panic!("i should be dead");
  }

  unsafe {
    let stack = fringe::OsStack::new(4 << 20).unwrap();
    let mut ctx = Context::new(stack, permuter);
    ctx_slot = &mut ctx;

    Context::swap(ctx_slot, ctx_slot, 10);
    Context::swap(ctx_slot, ctx_slot, 20);
  }
}
