use core::prelude::*;
use core::simd::u64x2;
use core::mem::{size_of, zeroed};

use stack::Stack;

extern "C" {
  #[link_name = "lwt_bootstrap"]
  pub fn bootstrap();
  #[link_name = "lwt_swapcontext"]
  pub fn swapcontext(save: *mut Registers, restore: *mut Registers);
  #[link_name = "lwt_abort"]
  pub fn abort() -> !;
}

#[allow(non_camel_case_types)]
pub type uintptr_t = u64;

#[repr(C)]
#[allow(dead_code)]
pub struct Registers {
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
  pub fn new() -> Registers {
    unsafe {
      Registers {
        ip: abort as uintptr_t,
        .. zeroed()
      }
    }
  }
}

pub fn initialise_call_frame(stack: &mut Stack, init: uintptr_t, args: &[uintptr_t]) -> Registers {
  let sp = stack.top() as *mut uintptr_t;
  let sp = align_down_mut(sp, 16);
  let sp = offset_mut(sp, -1);
  unsafe {
    *sp = 0;
  }

  let mut regs = Registers {
    rbp: 0,
    rsp: sp as uintptr_t,
    ip: bootstrap as uintptr_t,
    rbx: init,
    .. Registers::new()
  };

  match into_fields!(regs { rdi, r12, r13, r14, r15 } <- args.iter().cloned()) {
    Some(mut args) => if args.next().is_some() {
      panic!("too many arguments")
    },
    None => {}
  }

  regs
}

// Rust stores a stack limit at [fs:0x70]. These two functions set and retrieve
// the limit. They're marked as #[inline(always)] so that they can be used in
// situations where the stack limit is invalid.

#[inline(always)]
pub unsafe fn get_sp_limit() -> *const u8 {
  let limit;
  asm!("movq %fs:0x70, $0" : "=r"(limit) ::: "volatile");
  limit
}

#[inline(always)]
pub unsafe fn set_sp_limit(limit: *const u8) {
  asm!("movq $0, %fs:0x70" :: "r"(limit) :: "volatile");
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
