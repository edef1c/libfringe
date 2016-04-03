// This file is part of libfringe, a low-level green threading library.
// Copyright (c) edef <edef@edef.eu>
// See the LICENSE file included in this distribution.
use core::marker::PhantomData;

use void::Void;

use arch::Registers;
use stack;
use debug::StackId;

/// Context is the heart of libfringe.
/// A context represents a saved thread of execution, along with a stack.
/// It can be swapped into and out of with the swap method,
/// and once you're done with it, you can get the stack back through unwrap.
///
/// Every operation is unsafe, because libfringe can't make any guarantees
/// about the state of the context.
#[derive(Debug)]
pub struct Context<'a, Stack: stack::Stack> {
  regs: Registers,
  _stack_id: StackId,
  stack: Stack,
  _ref: PhantomData<&'a ()>
}

unsafe impl<'a, Stack> Send for Context<'a, Stack>
  where Stack: stack::Stack + Send {}

impl<'a, Stack> Context<'a, Stack> where Stack: stack::Stack {
  /// Create a new Context. When it is swapped into,
  /// it will call the passed closure.
  #[inline]
  pub unsafe fn new<F>(mut stack: Stack, f: F) -> Context<'a, Stack>
    where F: FnOnce() -> Void + Send + 'a {
    let stack_id = StackId::register(&mut stack);
    let regs = Registers::new(&mut stack, f);
    Context {
      regs: regs,
      _stack_id: stack_id,
      stack: stack,
      _ref: PhantomData
    }
  }

  /// Unwrap the context, returning the stack it contained.
  #[inline]
  pub unsafe fn unwrap(self) -> Stack {
    self.stack
  }
}

impl<'i, InStack> Context<'i, InStack> where InStack: stack::Stack {
  /// Switch to in_ctx, saving the current thread of execution to out_ctx.
  #[inline(always)]
  pub unsafe fn swap<'o, OutStack>(out_ctx: *mut Context<'o, OutStack>, in_ctx: *const Context<'i, InStack>)
    where OutStack: stack::Stack {
    Registers::swap(&mut (*out_ctx).regs, &(*in_ctx).regs)
  }
}
