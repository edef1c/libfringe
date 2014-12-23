#![feature(unboxed_closures, default_type_params)]
extern crate lwkt;
extern crate fn_box;

use std::ptr::null_mut;
use std::intrinsics::abort;
use lwkt::Context;
use fn_box::FnBox;

fn main() {
  let f = box move |:| {
    println!("Hello, world!")
  };

  let mut native = unsafe { Context::native() };

  fn init(ctx: *mut (*mut Context, *mut Context), f: Box<FnBox()>) -> ! {
    unsafe {
      let (native, green) = *ctx;

      f();

      Context::swap(&mut *green, &mut *native);
      abort();
    }
  }

  let mut ctx = box { (&mut native as *mut Context, null_mut()) };
  let mut green = Context::new(init, &mut *ctx as *mut _, f);
  ctx.1 = &mut green as *mut Context;

  unsafe {
    Context::swap(&mut native, &mut green);
  }

  println!("size_of::<Context>() == {}", std::mem::size_of::<Context>());
}
