// This file is part of libfringe, a low-level green threading library.
// Copyright (c) Nathan Zadoks <nathan@nathan7.eu>
// See the LICENSE file included in this distribution.
#![feature(thread_local)]
extern crate simd;
extern crate rand;

extern crate fringe;

use fringe::SliceStack;
use fringe::session;
use fringe::session::cycle::{C1, Cycle};


#[thread_local]
static mut stack_buf: [u8; 4 << 20] = [0; 4 << 20];

fn stress_alignment(off: u8) {
  use simd::*;
  use rand::Rng;

  // Randomness prevents CSE
  let x = f32x4::splat(rand::thread_rng().gen::<f32>());
  let y = -x;
  let z = y*y;

  println!("Round {} SIMD Time! {:?}", off, z);
}

unsafe fn double_swap(off: u8) {
  let stack = SliceStack(&mut stack_buf[off as usize..]);

  let ctx: C1<'static, _, ()> = C1::new(stack, move |tl, (ctx, ())| {
    stress_alignment(off);
    ctx.unwrap().kontinue(Some(tl), ())
  });

  println!("Round {}: start!", off);
  let (_, ()) = ctx.swap(session::native_thread_locals(), ());
}

macro_rules! offset {
  ($name:ident, $off_m:expr) => {
    #[ignore]
    #[test]
    #[allow(non_snake_case)]
    fn $name() {
      unsafe { double_swap($off_m) }
    }
  }
}

offset! { offset_0x0, 0x0 }
offset! { offset_0x1, 0x1 }
offset! { offset_0x2, 0x2 }
offset! { offset_0x3, 0x3 }
offset! { offset_0x4, 0x4 }
offset! { offset_0x5, 0x5 }
offset! { offset_0x6, 0x6 }
offset! { offset_0x7, 0x7 }
offset! { offset_0x8, 0x8 }
offset! { offset_0x9, 0x9 }
offset! { offset_0xA, 0xA }
offset! { offset_0xB, 0xB }
offset! { offset_0xC, 0xC }
offset! { offset_0xD, 0xD }
offset! { offset_0xE, 0xE }
offset! { offset_0xF, 0xF }
