// This file is part of libfringe, a low-level green threading library.
// Copyright (c) whitequark <whitequark@whitequark.org>
// See the LICENSE file included in this distribution.

//! Generators.
//!
//! Generators allow repeatedly suspending the execution of a function,
//! returning a value to the caller, and resuming the suspended function
//! afterwards.

use core::marker::PhantomData;
use core::iter::Iterator;
use core::{ptr, mem};

use stack;
use context;

#[derive(Debug, Clone, Copy)]
pub enum State {
  /// Generator can be resumed. This is the initial state.
  Runnable,
  /// Generator cannot be resumed. This is the state of the generator after
  /// the generator function has returned or panicked.
  Unavailable
}

/// Generator wraps a function and allows suspending its execution more than
/// once, return a value each time.
///
/// It implements the Iterator trait. The first time `next()` is called,
/// the function is called as `f(yielder)`; every time `next()` is called afterwards,
/// the function is resumed. In either case, it runs until it suspends its execution
/// through `yielder.generate(val)`), in which case `next()` returns `Some(val)`, or
/// returns, in which case `next()` returns `None`. `next()` will return `None`
/// every time after that.
///
/// After the generator function returns, it is safe to reclaim the generator
/// stack using `unwrap()`.
///
/// `state()` can be used to determine whether the generator function has returned;
/// the state is `State::Runnable` after creation and suspension, and `State::Unavailable`
/// once the generator function returns or panics.
///
/// # Example
///
/// ```
/// use fringe::{OsStack, Generator};
///
/// let stack = OsStack::new(0).unwrap();
/// let mut gen = Generator::new(stack, move |yielder| {
///   for i in 1..4 {
///     yielder.generate(i);
///   }
/// });
/// println!("{:?}", gen.next()); // prints Some(1)
/// println!("{:?}", gen.next()); // prints Some(2)
/// println!("{:?}", gen.next()); // prints Some(3)
/// println!("{:?}", gen.next()); // prints None
/// ```
#[derive(Debug)]
pub struct Generator<Item: Send, Stack: stack::Stack> {
  state:   State,
  context: context::Context<Stack>,
  phantom: PhantomData<Item>
}

impl<Item, Stack> Generator<Item, Stack>
    where Item: Send, Stack: stack::Stack {
  /// Creates a new generator.
  pub fn new<F>(stack: Stack, f: F) -> Generator<Item, Stack>
      where Stack: stack::GuardedStack, F: FnOnce(&mut Yielder<Item, Stack>) + Send {
    unsafe { Generator::unsafe_new(stack, f) }
  }

  /// Same as `new`, but does not require `stack` to have a guard page.
  ///
  /// This function is unsafe because the generator function can easily violate
  /// memory safety by overflowing the stack. It is useful in environments where
  /// guarded stacks do not exist, e.g. in absence of an MMU.
  pub unsafe fn unsafe_new<F>(stack: Stack, f: F) -> Generator<Item, Stack>
      where F: FnOnce(&mut Yielder<Item, Stack>) + Send {
    unsafe extern "C" fn generator_wrapper<Item, Stack, F>(info: usize) -> !
        where Item: Send, Stack: stack::Stack, F: FnOnce(&mut Yielder<Item, Stack>) {
      // Retrieve our environment from the callee and return control to it.
      let (mut yielder, f) = ptr::read(info as *mut (Yielder<Item, Stack>, F));
      let new_context = context::Context::swap(yielder.context, yielder.context, 0);
      // See Yielder::return_.
      yielder.context = new_context as *mut context::Context<Stack>;
      // Run the body of the generator.
      f(&mut yielder);
      // Past this point, the generator has dropped everything it has held.
      loop { yielder.return_(None) }
    }

    let mut generator = Generator {
      state:   State::Runnable,
      context: context::Context::new(stack, generator_wrapper::<Item, Stack, F>),
      phantom: PhantomData
    };

    // Transfer environment to the callee.
    let mut data = (Yielder::new(&mut generator.context), f);
    context::Context::swap(&mut generator.context, &generator.context,
                           &mut data as *mut (Yielder<Item, Stack>, F) as usize);
    mem::forget(data);

    generator
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
pub struct Yielder<Item: Send, Stack: stack::Stack> {
  context: *mut context::Context<Stack>,
  phantom: PhantomData<Item>
}

impl<Item, Stack> Yielder<Item, Stack>
    where Item: Send, Stack: stack::Stack {
  fn new(context: *mut context::Context<Stack>) -> Yielder<Item, Stack> {
    Yielder {
      context: context,
      phantom: PhantomData
    }
  }

  #[inline(always)]
  fn return_(&mut self, mut val: Option<Item>) {
    unsafe {
      let new_context = context::Context::swap(self.context, self.context,
                                               &mut val as *mut Option<Item> as usize);
      // The generator can be moved (and with it, the context).
      // This changes the address of the context.
      // Thus, we update it after each swap.
      self.context = new_context as *mut context::Context<Stack>;
      // However, between this point and the next time we enter return_
      // the generator cannot be moved, as a &mut Generator is necessary
      // to resume the generator function.
    }
  }

  /// Suspends the generator and returns `Some(item)` from the `next()`
  /// invocation that resumed the generator.
  #[inline(always)]
  pub fn generate(&mut self, item: Item) {
    self.return_(Some(item))
  }
}

impl<Item, Stack> Iterator for Generator<Item, Stack>
    where Item: Send, Stack: stack::Stack {
  type Item = Item;

  /// Resumes the generator and return the next value it yields.
  /// If the generator function has returned, returns `None`.
  #[inline]
  fn next(&mut self) -> Option<Self::Item> {
    match self.state {
      State::Runnable => {
        // Set the state to Unavailable. Since we have exclusive access to the generator,
        // the only case where this matters is the generator function panics, after which
        // it must not be invocable again.
        self.state = State::Unavailable;

        // Switch to the generator function.
        let new_context = &mut self.context as *mut context::Context<Stack> as usize;
        let val = unsafe {
          ptr::read(context::Context::swap(&mut self.context, &self.context, new_context)
                    as *mut Option<Item>)
        };

        // Unless the generator function has returned, it can be switched to again, so
        // set the state to Runnable.
        if val.is_some() { self.state = State::Runnable }

        val
      }
      State::Unavailable => None
    }
  }
}
