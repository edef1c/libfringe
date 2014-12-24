use std::mem::transmute;
use std::raw;
use fn_box::FnBox;

use stack::Stack;
use arch::{mod, Registers};

pub struct Context {
  regs: Registers,
  stack: Stack
}

pub type BoxedFn<Args, Result> = Box<FnBox<Args, Result> + Send + 'static>;
pub type StartFn<T, Args, Result> = fn(data: *mut T, f: BoxedFn<Args, Result>) -> !;

impl Context {
  pub fn new<T, Args, Result>(init: StartFn<T, Args, Result>, data: *mut T,
                              f: BoxedFn<Args, Result>) -> Context {
    let mut stack = Stack::new(4 << 20);
    let f: raw::TraitObject = unsafe { transmute(f) };

    Context {
      regs: arch::initialise_call_frame(&mut stack,
        init_ctx::<T, Args, Result> as arch::uintptr_t,
        &[init as arch::uintptr_t,
          data as arch::uintptr_t,
          f.data as arch::uintptr_t,
          f.vtable as arch::uintptr_t]),
      stack: stack
    }
  }
}

unsafe extern "C" fn init_ctx<T, A, R>(start: StartFn<T, A, R>, data: *mut T,
                                       f_data: *mut (), f_vtable: *mut ()) -> ! {
  let f: BoxedFn<A, R> = transmute(raw::TraitObject {
    data: f_data,
    vtable: f_vtable
  });

  start(data, f)
}

impl Context {
  pub unsafe fn native() -> Context {
    Context {
      regs: Registers::new(),
      stack: Stack::native(arch::get_sp_limit())
    }
  }


  #[inline(always)]
  pub unsafe fn swap(out_context: &mut Context, in_context: &mut Context) {
    arch::set_sp_limit(in_context.stack.limit());
    arch::swapcontext(&mut out_context.regs, &mut in_context.regs);
  }
}
