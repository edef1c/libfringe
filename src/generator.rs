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

use stack;
use context::Context;

#[derive(Debug, Clone, Copy)]
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
/// It runs until it suspends its execution through `yielder.generate(output0)`, after which
/// `resume(input0)` returns `output0`. The function can be resumed again using `resume(input1)`,
/// after which `yielder.generate(output0)` returns `input1`, and so on. Once the function returns,
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
///     input = yielder.generate(input + 1)
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
///   for i in 1.. { yielder.generate(i) }
/// });
/// println!("{:?}", nat.next()); // prints Some(0)
/// println!("{:?}", nat.next()); // prints Some(1)
/// println!("{:?}", nat.next()); // prints Some(2)
/// ```
#[derive(Debug)]
pub struct Generator<Input: Send, Output: Send, Stack: stack::Stack> {
  state:   State,
  context: Context<Stack>,
  phantom: (PhantomData<*const Input>, PhantomData<*const Output>)
}

impl<Input, Output, Stack> Generator<Input, Output, Stack>
    where Input: Send, Output: Send, Stack: stack::Stack {
  /// Creates a new generator.
  pub fn new<F>(stack: Stack, f: F) -> Generator<Input, Output, Stack>
      where Stack: stack::GuardedStack,
            F: FnOnce(&mut Yielder<Input, Output, Stack>, Input) + Send {
    unsafe { Generator::unsafe_new(stack, f) }
  }

  /// Same as `new`, but does not require `stack` to have a guard page.
  ///
  /// This function is unsafe because the generator function can easily violate
  /// memory safety by overflowing the stack. It is useful in environments where
  /// guarded stacks do not exist, e.g. in absence of an MMU.
  pub unsafe fn unsafe_new<F>(stack: Stack, f: F) -> Generator<Input, Output, Stack>
      where F: FnOnce(&mut Yielder<Input, Output, Stack>, Input) + Send {
    unsafe extern "C" fn generator_wrapper<Input, Output, Stack, F>(env: usize) -> !
        where Input: Send, Output: Send, Stack: stack::Stack,
              F: FnOnce(&mut Yielder<Input, Output, Stack>, Input) {
      // Retrieve our environment from the callee and return control to it.
      let (mut yielder, f) = ptr::read(env as *mut (Yielder<Input, Output, Stack>, F));
      let data = Context::swap(yielder.context, yielder.context, 0);
      // See the second half of Yielder::generate_bare.
      let (new_context, input) = ptr::read(data as *mut (*mut Context<Stack>, Input));
      yielder.context = new_context as *mut Context<Stack>;
      // Run the body of the generator.
      f(&mut yielder, input);
      // Past this point, the generator has dropped everything it has held.
      loop { yielder.generate_bare(None); }
    }

    let mut generator = Generator {
      state:   State::Runnable,
      context: Context::new(stack, generator_wrapper::<Input, Output, Stack, F>),
      phantom: (PhantomData, PhantomData)
    };

    // Transfer environment to the callee.
    let mut env = (Yielder::new(&mut generator.context), f);
    Context::swap(&mut generator.context, &generator.context,
                  &mut env as *mut (Yielder<Input, Output, Stack>, F) as usize);
    mem::forget(env);

    generator
  }

  /// Resumes the generator and return the next value it yields.
  /// If the generator function has returned, returns `None`.
  #[inline]
  pub fn resume(&mut self, input: Input) -> Option<Output> {
    match self.state {
      State::Runnable => {
        // Set the state to Unavailable. Since we have exclusive access to the generator,
        // the only case where this matters is the generator function panics, after which
        // it must not be invocable again.
        self.state = State::Unavailable;

        // Switch to the generator function, and retrieve the yielded value.
        let val = unsafe {
          let mut data_in = (&mut self.context as *mut Context<Stack>, input);
          let data_out =
            ptr::read(Context::swap(&mut self.context, &self.context,
                                    &mut data_in as *mut (*mut Context<Stack>, Input)  as usize)
                      as *mut Option<Output>);
          mem::forget(data_in);
          data_out
        };

        // Unless the generator function has returned, it can be switched to again, so
        // set the state to Runnable.
        if val.is_some() { self.state = State::Runnable }

        val
      }
      State::Unavailable => None
    }
  }

  /// Returns the state of the generator.
  #[inline]
  pub fn state(&self) -> State { self.state }

  /// Extracts the stack from a generator when the generator function has returned.
  /// If the generator function has not returned
  /// (i.e. `self.state() == State::Runnable`), panics.
  pub fn unwrap(self) -> Stack {
    match self.state {
      State::Runnable    => panic!("Argh! Bastard! Don't touch that!"),
      State::Unavailable => unsafe { self.context.unwrap() }
    }
  }
}

/// Yielder is an interface provided to every generator through which it
/// returns a value.
#[derive(Debug)]
pub struct Yielder<Input: Send, Output: Send, Stack: stack::Stack> {
  context: *mut Context<Stack>,
  phantom: (PhantomData<*const Input>, PhantomData<*const Output>)
}

impl<Input, Output, Stack> Yielder<Input, Output, Stack>
    where Input: Send, Output: Send, Stack: stack::Stack {
  fn new(context: *mut Context<Stack>) -> Yielder<Input, Output, Stack> {
    Yielder {
      context: context,
      phantom: (PhantomData, PhantomData)
    }
  }

  #[inline(always)]
  fn generate_bare(&mut self, mut val: Option<Output>) -> Input {
    unsafe {
      let data = Context::swap(self.context, self.context,
                               &mut val as *mut Option<Output> as usize);
      let (new_context, input) = ptr::read(data as *mut (*mut Context<Stack>, Input));
      // The generator can be moved (and with it, the context).
      // This changes the address of the context.
      // Thus, we update it after each swap.
      self.context = new_context;
      // However, between this point and the next time we enter generate_bare
      // the generator cannot be moved, as a &mut Generator is necessary
      // to resume the generator function.
      input
    }
  }

  /// Suspends the generator and returns `Some(item)` from the `resume()`
  /// invocation that resumed the generator.
  #[inline(always)]
  pub fn generate(&mut self, item: Output) -> Input {
    self.generate_bare(Some(item))
  }
}

impl<Output, Stack> Iterator for Generator<(), Output, Stack>
    where Output: Send, Stack: stack::Stack {
  type Item = Output;

  fn next(&mut self) -> Option<Self::Item> { self.resume(()) }
}
