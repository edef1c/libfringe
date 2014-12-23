#![feature(unboxed_closures, default_type_params, asm)]
extern crate test;
extern crate libc;
extern crate lwkt;
extern crate fn_box;
use test::Bencher;
use lwkt::Context;
use fn_box::FnBox;
use std::ptr::null_mut;
use std::mem::{transmute, forget};

#[bench]
fn swap(b: &mut Bencher) {
  let mut native = unsafe { Context::native() };
  let f: Box<FnBox() + Send + 'static> = unsafe { transmute((1u, 1u)) };

  let mut ctx = box { (&mut native as *mut Context, null_mut()) };
  let mut green = Context::new(init, &mut *ctx as *mut _, f);
  ctx.1 = &mut green as *mut Context;

  fn init(ctx: *mut (*mut Context, *mut Context), f: Box<FnBox()>) -> ! {
    unsafe {
      let (native, green) = *ctx;
      forget(f);
      loop { Context::swap(&mut *green, &mut *native); }
    }
  }

  unsafe {
    Context::swap(&mut native, &mut green);
  }

  b.iter(|| unsafe {
    Context::swap(&mut native, &mut green);
  })
}

#[bench]
fn kernel_swap(b: &mut Bencher) {
  b.iter(|| unsafe {
    asm!("movq $$102, %rax\n\
          syscall"
         :
         :
         : "rax", "rcx"
         : "volatile");
  });
}
