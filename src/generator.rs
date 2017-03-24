// This file is part of libfringe, a low-level green threading library.
// Copyright (c) whitequark <whitequark@whitequark.org>
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

//! Generators.
//!
//! Generators allow repeatedly suspending the execution of a function,
//! returning a value to the caller, and resuming the suspended function
//! afterwards.

use core::marker::PhantomData;
use core::{ptr, mem};
use core::cell::Cell;

use stack;
use debug;
use arch::{self, StackPointer};

// Wrapper to prevent the compiler from automatically dropping a value when it
// goes out of scope. This is particularly useful when dealing with unwinding
// since mem::forget won't be executed when unwinding.
#[allow(unions_with_drop_fields)]
union NoDrop<T> {
  inner: T,
}

// Try to pack a value into a usize if it fits, otherwise pass its address as a usize.
unsafe fn encode_usize<T>(val: &NoDrop<T>) -> usize {
  if mem::size_of::<T>() <= mem::size_of::<usize>() &&
     mem::align_of::<T>() <= mem::align_of::<usize>() {
    let mut out = 0;
    ptr::copy_nonoverlapping(&val.inner, &mut out as *mut usize as *mut T, 1);
    out
  } else {
    &val.inner as *const T as usize
  }
}

// Unpack a usize produced by encode_usize.
unsafe fn decode_usize<T>(val: usize) -> T {
  if mem::size_of::<T>() <= mem::size_of::<usize>() &&
     mem::align_of::<T>() <= mem::align_of::<usize>() {
    ptr::read(&val as *const usize as *const T)
  } else {
    ptr::read(val as *const T)
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
  /// Generator can be resumed. This is the initial state.
  Runnable,
  /// Generator cannot be resumed. This is the state of the generator after
  /// the generator function has returned or panicked.
  Unavailable
}

/// Generator wraps a function and allows suspending its execution more than once, returning
/// a value each time.
///
/// The first time `resume(input0)` is called, the function is called as `f(yielder, input0)`.
/// It runs until it suspends its execution through `yielder.suspend(output0)`, after which
/// `resume(input0)` returns `output0`. The function can be resumed again using `resume(input1)`,
/// after which `yielder.suspend(output0)` returns `input1`, and so on. Once the function returns,
/// the `resume()` call will return `None`, and it will return `None` every time it is called
/// after that.
///
/// If the generator function panics, the panic is propagated through the `resume()` call as usual.
///
/// After the generator function returns or panics, it is safe to reclaim the generator stack
/// using `unwrap()`.
///
/// `state()` can be used to determine whether the generator function has returned;
/// the state is `State::Runnable` after creation and suspension, and `State::Unavailable`
/// once the generator function returns or panics.
///
/// When the input type is `()`, a generator implements the Iterator trait.
///
/// # Example
///
/// ```
/// use fringe::{OsStack, Generator};
///
/// let stack = OsStack::new(0).unwrap();
/// let mut add_one = Generator::new(stack, move |yielder, mut input| {
///   loop {
///     if input == 0 { break }
///     input = yielder.suspend(input + 1)
///   }
/// });
/// println!("{:?}", add_one.resume(2)); // prints Some(3)
/// println!("{:?}", add_one.resume(3)); // prints Some(4)
/// println!("{:?}", add_one.resume(0)); // prints None
/// ```
///
/// # Iterator example
///
/// ```
/// use fringe::{OsStack, Generator};
/// use std::mem;
///
/// let stack = OsStack::new(0).unwrap();
/// let mut nat = Generator::new(stack, move |yielder, ()| {
///   for i in 1.. { yielder.suspend(i) }
/// });
/// println!("{:?}", nat.next()); // prints Some(0)
/// println!("{:?}", nat.next()); // prints Some(1)
/// println!("{:?}", nat.next()); // prints Some(2)
/// mem::forget(nat); // we can't drop a running Generator, so we leak it
/// ```
#[derive(Debug)]
pub struct Generator<'a, Input: 'a, Output: 'a, Stack: stack::Stack> {
  stack:     NoDrop<Stack>,
  stack_id:  debug::StackId,
  stack_ptr: Option<arch::StackPointer>,
  phantom:   PhantomData<(&'a (), *mut Input, *const Output)>
}

impl<T: ::core::fmt::Debug> ::core::fmt::Debug for NoDrop<T> {
  fn fmt(&self, w: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
    unsafe {
      // this is safe because we never invoke formatting on the empty variant
      self.inner.fmt(w)
    }
  }
}

impl<'a, Input, Output, Stack> Generator<'a, Input, Output, Stack>
    where Input: 'a, Output: 'a, Stack: stack::Stack {
  /// Creates a new generator.
  ///
  /// See also the [contract](../trait.GuardedStack.html) that needs to be fulfilled by `stack`.
  pub fn new<F>(stack: Stack, f: F) -> Generator<'a, Input, Output, Stack>
      where Stack: stack::GuardedStack + 'static,
            F: FnOnce(&Yielder<Input, Output>, Input) + 'a {
    unsafe { Generator::unsafe_new(stack, f) }
  }

  /// Same as `new`, but does not require `stack` to have a guard page.
  ///
  /// This function is unsafe because the generator function can easily violate
  /// memory safety by overflowing the stack. It is useful in environments where
  /// guarded stacks do not exist, e.g. in absence of an MMU.
  ///
  /// See also the [contract](../trait.Stack.html) that needs to be fulfilled by `stack`.
  pub unsafe fn unsafe_new<F>(stack: Stack, f: F) -> Generator<'a, Input, Output, Stack>
      where F: FnOnce(&Yielder<Input, Output>, Input) + 'a {
    unsafe fn generator_wrapper<Input, Output, Stack, F>(env: usize, stack_ptr: StackPointer)
        where Stack: stack::Stack, F: FnOnce(&Yielder<Input, Output>, Input) {
      // Retrieve our environment from the callee and return control to it.
      let f: F = decode_usize(env);
      let (data, stack_ptr) = arch::swap(0, stack_ptr);
      // See the second half of Yielder::suspend_bare.
      let input = decode_usize(data);
      // Run the body of the generator.
      let yielder = Yielder::new(stack_ptr);
      f(&yielder, input);
    }

    let stack_id  = debug::StackId::register(&stack);
    let stack_ptr = arch::init(stack.base(), generator_wrapper::<Input, Output, Stack, F>);

    // Transfer environment to the callee.
    let f = NoDrop { inner: f };
    let stack_ptr = arch::swap_link(encode_usize(&f), stack_ptr, stack.base()).1;

    Generator {
      stack:     NoDrop { inner: stack },
      stack_id:  stack_id,
      stack_ptr: stack_ptr,
      phantom:   PhantomData
    }
  }

  /// Resumes the generator and return the next value it yields.
  /// If the generator function has returned, returns `None`.
  #[inline]
  pub fn resume(&mut self, input: Input) -> Option<Output> {
    // Return None if we have no stack pointer (generator function already returned).
    self.stack_ptr.and_then(|stack_ptr| {
      // Set the state to Unavailable. Since we have exclusive access to the generator,
      // the only case where this matters is the generator function panics, after which
      // it must not be invocable again.
      self.stack_ptr = None;

      // Switch to the generator function, and retrieve the yielded value.
      unsafe {
        let input = NoDrop { inner: input };
        let (data_out, stack_ptr) = arch::swap_link(encode_usize(&input), stack_ptr, self.stack.inner.base());
        self.stack_ptr = stack_ptr;

        // If the generator function has finished, return None, otherwise return the
        // yielded value.
        stack_ptr.map(|_| decode_usize(data_out))
      }
    })
  }

  /// Returns the state of the generator.
  #[inline]
  pub fn state(&self) -> State {
    if self.stack_ptr.is_some() { State::Runnable } else { State::Unavailable }
  }

  /// Extracts the stack from a generator when the generator function has returned.
  /// If the generator function has not returned
  /// (i.e. `self.state() == State::Runnable`), panics.
  pub fn unwrap(self) -> Stack {
    match self.state() {
      State::Runnable => panic!("Argh! Bastard! Don't touch that!"),
      State::Unavailable => unsafe { self.unsafe_unwrap() }
    }
  }

  /// Extracts the stack from a generator without checking if the generator function has returned.
  /// This will leave any pointers into the generator stack dangling, and won't run destructors.
  pub unsafe fn unsafe_unwrap(mut self) -> Stack {
    if cfg!(feature = "unwind") {
      self.stack_ptr.map(|stack_ptr| arch::unwind(stack_ptr, self.stack.inner.base()));
    }

    // We can't just return self.stack since Generator has a Drop impl
    let stack = ptr::read(&self.stack.inner);
    ptr::drop_in_place(&mut self.stack_id);
    mem::forget(self);
    stack
  }
}

impl<'a, Input, Output, Stack> Drop for Generator<'a, Input, Output, Stack>
    where Input: 'a, Output: 'a, Stack: stack::Stack {
  fn drop(&mut self) {
    unsafe {
      match self.stack_ptr {
        Some(stack_ptr) => {
          // If unwinding is not available then we have to leak the stack.
          if cfg!(feature = "unwind") {
            arch::unwind(stack_ptr, self.stack.inner.base());
            ptr::drop_in_place(&mut self.stack.inner);
          }
        }
        None => ptr::drop_in_place(&mut self.stack.inner)
      }
    }
  }
}

/// Yielder is an interface provided to every generator through which it
/// returns a value.
#[derive(Debug)]
pub struct Yielder<Input, Output> {
  stack_ptr: Cell<Option<StackPointer>>,
  phantom: PhantomData<(*const Input, *mut Output)>
}

impl<Input, Output> Yielder<Input, Output> {
  fn new(stack_ptr: StackPointer) -> Yielder<Input, Output> {
    Yielder {
      stack_ptr: Cell::new(Some(stack_ptr)),
      phantom: PhantomData
    }
  }

  /// Suspends the generator and returns `Some(item)` from the `resume()`
  /// invocation that resumed the generator.
  #[inline(always)]
  pub fn suspend(&self, item: Output) -> Input {
    unsafe {
      struct PanicGuard<'a>(&'a Cell<Option<StackPointer>>);
      impl<'a> Drop for PanicGuard<'a> {
        fn drop(&mut self) {
          self.0.set(None);
        }
      }

      let stack_ptr = self.stack_ptr.get().expect("attempted to yield while unwinding");
      let item = NoDrop { inner: item };

      // Use a PanicGuard to set self.stack_ptr to None if unwinding occurs. This
      // is necessary to guarantee safety in case someone tries to call yield
      // while we are unwinding since there is nowhere to yield to.
      let guard = PanicGuard(&self.stack_ptr);
      let (data, stack_ptr) = arch::swap(encode_usize(&item), stack_ptr);
      mem::forget(guard);

      self.stack_ptr.set(Some(stack_ptr));
      decode_usize(data)
    }
  }
}

impl<'a, Output, Stack> Iterator for Generator<'a, (), Output, Stack>
    where Output: 'a, Stack: stack::Stack {
  type Item = Output;

  fn next(&mut self) -> Option<Self::Item> { self.resume(()) }
}
