use libc;
use stack::Stack;
use std::simd::u64x2;
use std::mem::{zeroed, size_of, transmute};
use std::raw;
use fn_box::FnBox;

pub struct Context {
  regs: Registers,
  stack: Stack
}

#[repr(C)]
#[allow(dead_code)]
struct Registers {
  rbx: u64,
  rsp: u64,
  rbp: u64,
  rdi: u64,
  r12: u64,
  r13: u64,
  r14: u64,
  r15: u64,
  ip:  u64,
  xmm0: u64x2,
  xmm1: u64x2,
  xmm2: u64x2,
  xmm3: u64x2,
  xmm4: u64x2,
  xmm5: u64x2,
}

impl Registers {
  fn zeroed() -> Registers { unsafe { zeroed() } }
}

extern "C" {
  fn lwut_bootstrap();
  fn lwut_swapcontext(save: *mut Registers, restore: *mut Registers);
  fn lwut_get_sp_limit() -> *const u8;
  fn lwut_set_sp_limit(limit: *const u8);
  fn lwut_abort() -> !;
}

pub type BoxedFn<Args, Result> = Box<FnBox<Args, Result> + Send + 'static>;
pub type StartFn<T, Args, Result> = fn(data: *mut T, f: BoxedFn<Args, Result>) -> !;

impl Context {
  pub fn new<T, Args, Result>(init: StartFn<T, Args, Result>, data: *mut T,
                              f: BoxedFn<Args, Result>) -> Context {
    let stack = Stack::new(4 << 20);

    let sp = stack.top() as *mut uint;
    let sp = align_down_mut(sp, 16);
    let sp = offset_mut(sp, -1);
    unsafe {
      *sp = 0;
    }

    let f: raw::TraitObject = unsafe { transmute(f) };

    Context {
      regs: Registers {
        rbp: 0,
        rsp: sp as libc::uintptr_t,
        ip: lwut_bootstrap as libc::uintptr_t,
        r12: lwut_init::<T, Args, Result> as libc::uintptr_t,
        rdi: init as libc::uintptr_t,
        r13: data as libc::uintptr_t,
        r14: f.data as libc::uintptr_t,
        r15: f.vtable as libc::uintptr_t,
        // r8: …,
        // r9: …,
        .. Registers::zeroed()
      },
      stack: stack
    }
  }
}

unsafe extern "C" fn lwut_init<T, A, R>(start: StartFn<T, A, R>, data: *mut T,
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
      regs: Registers {
        ip: lwut_abort as libc::uintptr_t,
        .. Registers::zeroed()
      },
      stack: Stack::native(lwut_get_sp_limit())
    }
  }


  #[inline(always)]
  pub unsafe fn swap(out_context: &mut Context, in_context: &mut Context) {
    lwut_set_sp_limit(in_context.stack.limit());
    lwut_swapcontext(&mut out_context.regs, &mut in_context.regs);
  }
}

#[inline]
fn align_down_mut<T>(sp: *mut T, n: uint) -> *mut T {
  let sp = (sp as uint) & !(n - 1);
  sp as *mut T
}

// ptr::offset_mmut is positive ints only
#[inline]
pub fn offset_mut<T>(ptr: *mut T, count: int) -> *mut T {
  (ptr as int + count * (size_of::<T>() as int)) as *mut T
}
