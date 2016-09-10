// This file is part of libfringe, a low-level green threading library.
// Copyright (c) whitequark <whitequark@whitequark.org>
//               John Ericson <Ericson2314@Yahoo.com>
// See the LICENSE file included in this distribution.

//! Adaptor methods for types that are bigger than a CPU Word
use core::marker::PhantomData;
use core::mem;
use core::ptr;

use stack::Stack;
use stack_pointer::StackPointer;

const NUM_REGS: usize = 1;

#[inline(always)]
pub unsafe fn to_regs<T>(data_ptr: *const T) -> [usize; NUM_REGS] {
  let mut regs: [usize; NUM_REGS] = [mem::uninitialized()];
  if mem::size_of::<T>() <= (mem::size_of::<usize>() * NUM_REGS) {
    // in regs
    ptr::write(&mut regs as *mut _ as *mut _,
               ptr::read(data_ptr));
  } else {
    // via pointer
    regs[0] = data_ptr as usize;
  }
  regs
}

#[inline(always)]
pub unsafe fn from_regs<T>(regs: [usize; NUM_REGS]) -> T {
  let ptr = if mem::size_of::<T>() <= mem::size_of::<usize>() * NUM_REGS
    && mem::align_of::<T>() <= mem::align_of::<usize>()
  {
    // in regs
    &regs as *const _ as *const _
  } else {
    // via pointer
    regs[0] as *const _
  };
  ptr::read(ptr)
}

// Adapted from whitquark's generator

/// Initializes a stack with the trampoline for a closure.
///
/// The phantom arguments are used so that `init0` and `init1` can be
/// called with a single closure literal of unnamable type.
#[inline]
pub unsafe fn init0<F>(stack: &Stack) -> (StackPointer, PhantomData<F>)
  where F: FnOnce(StackPointer) -> !
{
  unsafe extern "C" fn closure_wrapper<F>(a0: usize, sp: StackPointer) -> !
    where F: FnOnce(StackPointer) -> !
  {
    let closure: F = from_regs::<F>([a0]);
    closure(sp)
  }

  let sp = StackPointer::init(stack, closure_wrapper::<F>);
  (sp, PhantomData)
}

/// Initialize the stack with the closure environment *and switch*.
///
/// It is the responsibility of the closure to immediately yield `R`
/// if control wishes to be returned to caller immediately. Use a
/// reference if closure is a DST.
///
/// The phantom arguments are used so that `init0` and `init1` can be
/// called with a single closure literal of unnamable type.
#[inline]
pub unsafe fn init1<F, R>((new_sp, _): (StackPointer, PhantomData<F>),
                          new_stack: Option<&Stack>,
                          closure: F)
                          -> (StackPointer, R)
  where F: FnOnce(StackPointer) -> !
{
  let (sp2, ret) = swap(closure, new_sp, new_stack);
  (sp2, ret)
}

/// `I` and `O` can be any size
#[inline]
pub unsafe fn swap<I, O>(args: I, new_sp: StackPointer, new_stack: Option<&Stack>)
                         -> (StackPointer, O)
{
  let [arg0] = to_regs(&args);
  let (param0, old_sp) = StackPointer::swap(arg0, new_sp, new_stack);
  mem::forget(args);
  (old_sp, from_regs([param0]))
}

#[cfg(test)]
mod test {
  extern crate rand;
  extern crate simd;

  use core::fmt::Debug;

  use self::rand::Rng;

  use super::*;

  fn simple_round_trip_test<T: PartialEq + Debug + rand::Rand>() {
    let data = rand::thread_rng().gen::<T>();
    unsafe {
      assert_eq!(data, from_regs(to_regs(&data)));
    }
    ::std::mem::forget(data);
  }

  #[test]
  fn round_trip_u8() {
    simple_round_trip_test::<u8>();
  }

  #[test]
  fn round_trip_u16() {
    simple_round_trip_test::<u16>();
  }

  #[test]
  fn round_trip_u32() {
    simple_round_trip_test::<u32>();
  }

  #[test]
  fn round_trip_u64() {
    simple_round_trip_test::<u32>();
  }

  #[test]
  fn round_trip_max_by_value() {
    simple_round_trip_test::<[usize; 2]>();
  }

  #[test]
  fn round_trip_big_array() {
    simple_round_trip_test::<[u64; 10]>();
  }

  #[test]
  fn simple_init() {
    use OsStack;

    let stack = OsStack::new(4 << 20).unwrap();

    unsafe {
      let rets = init0(&stack);
      init1::<_, ()>(rets, None, move |initializer_sp| {
        debug!("made it!");
        swap::<(), !>((), initializer_sp, None).1
      })
    };
  }
}
