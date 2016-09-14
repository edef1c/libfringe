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
///
/// let stack = OsStack::new(0).unwrap();
/// let mut nat = Generator::new(stack, move |yielder, ()| {
///   for i in 1.. { yielder.suspend(i) }
/// });
/// println!("{:?}", nat.next()); // prints Some(0)
/// println!("{:?}", nat.next()); // prints Some(1)
/// println!("{:?}", nat.next()); // prints Some(2)
/// ```
#[derive(Debug)]
pub struct Generator<'a, Input: 'a, Output: 'a, Stack: stack::Stack> {
  stack:     Stack,
  stack_id:  debug::StackId,
  stack_ptr: Option<arch::StackPointer>,
  phantom:   PhantomData<(&'a (), *mut Input, *const Output)>
}

unsafe impl<'a, Input, Output, Stack> Send for Generator<'a, Input, Output, Stack>
  where Input: Send + 'a, Output: Send + 'a, Stack: stack::Stack + Send {}

impl<'a, Input, Output, Stack> Generator<'a, Input, Output, Stack>
    where Input: 'a, Output: 'a, Stack: stack::Stack {
  /// Creates a new generator.
  ///
  /// See also the [contract](../trait.GuardedStack.html) that needs to be fulfilled by `stack`.
  pub fn new<F>(stack: Stack, f: F) -> Generator<'a, Input, Output, Stack>
      where Stack: stack::GuardedStack,
            F: FnOnce(&Yielder<Input, Output>, Input) + Send + 'a {
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
      where F: FnOnce(&Yielder<Input, Output>, Input) + Send + 'a {
    unsafe extern "C" fn generator_wrapper<Input, Output, Stack, F>(env: usize, stack_ptr: StackPointer)
        where Stack: stack::Stack, F: FnOnce(&Yielder<Input, Output>, Input) {
      // Retrieve our environment from the callee and return control to it.
      let f = ptr::read(env as *const F);
      let (data, stack_ptr) = arch::swap(0, stack_ptr);
      // See the second half of Yielder::suspend_bare.
      let input = ptr::read(data as *const Input);
      // Run the body of the generator.
      let yielder = Yielder::new(stack_ptr);
      f(&yielder, input);
    }

    let stack_id  = debug::StackId::register(&stack);
    let stack_ptr = arch::init(stack.base(), generator_wrapper::<Input, Output, Stack, F>);

    // Transfer environment to the callee.
    let stack_ptr = arch::swap_link(&f as *const F as usize, stack_ptr, stack.base()).1;
    mem::forget(f);

    Generator {
      stack:     stack,
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
        let (data_out, stack_ptr) = arch::swap_link(&input as *const Input as usize, stack_ptr, self.stack.base());
        self.stack_ptr = stack_ptr;
        mem::forget(input);

        // If the generator function has finished, return None, otherwise return the
        // yielded value.
        stack_ptr.map(|_| ptr::read(data_out as *const Output))
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
      State::Runnable    => panic!("Argh! Bastard! Don't touch that!"),
      State::Unavailable => self.stack
    }
  }
}

/// Yielder is an interface provided to every generator through which it
/// returns a value.
#[derive(Debug)]
pub struct Yielder<Input, Output> {
  stack_ptr: Cell<StackPointer>,
  phantom: PhantomData<(*const Input, *mut Output)>
}

impl<Input, Output> Yielder<Input, Output> {
  fn new(stack_ptr: StackPointer) -> Yielder<Input, Output> {
    Yielder {
      stack_ptr: Cell::new(stack_ptr),
      phantom: PhantomData
    }
  }

  /// Suspends the generator and returns `Some(item)` from the `resume()`
  /// invocation that resumed the generator.
  #[inline(always)]
  pub fn suspend(&self, item: Output) -> Input {
    unsafe {
      let (data, stack_ptr) = arch::swap(&item as *const Output as usize, self.stack_ptr.get());
      mem::forget(item);
      self.stack_ptr.set(stack_ptr);
      ptr::read(data as *const Input)
    }
  }
}

impl<'a, Output, Stack> Iterator for Generator<'a, (), Output, Stack>
    where Output: 'a, Stack: stack::Stack {
  type Item = Output;

  fn next(&mut self) -> Option<Self::Item> { self.resume(()) }
}
