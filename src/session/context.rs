// This file is part of libfringe, a low-level green threading library.
// Copyright (c) Nathan Zadoks <nathan@nathan7.eu>,
//               John Ericson <Ericson2314@Yahoo.com>
// See the LICENSE file included in this distribution.
use core::marker::PhantomData;
use core::ptr::Unique;

use stack::Stack;
use stack_pointer::StackPointer;
use debug::StackId;
use fat_args;


/// The `Context` is a rough equivalent to fringe's main `Context`, but serves
/// only as an implementation aid.
///
/// The context is scoped in that a child context only valid for a
/// lifetime (aka borrows something from its parent) must eventually
/// return to it's parent.
///
/// I *think* the lifetimes here might enforce that

// TODO Higher-kinded-lifetime with `ArgsF::Output`?
//#[derive(Debug)]
pub struct Context<'a, Args, S: ?Sized>
  where Args: /*RebuildRaw<'a> +*/ 'a,
        S: Stack + 'a,
{
  pub(super) stack_pointer: StackPointer,
  pub thread_locals: Option<Unique<ThreadLocals<S>>>,
  pub(super) _ref: PhantomData<&'a mut fn(StackPointer, Args/*::Payload*/) -> !>
}

// TODO require that static is also Send
unsafe impl<'a, Args, S> Send for Context<'a, Args, S>
  where Args: /*RebuildRaw<'a> +*/ 'a + Send,
        S: Stack + 'a + Send,
        //Args::Payload: Send,
{ }

impl<'a, Args, S> Context<'a, Args, S>
  where Context<'a, Args, S>: Send,
        Args: RebuildRaw<'a> + 'a + Send,
        S: Stack + 'a,
{
  /// Create a new Context. When it is swapped into, it will call the passed
  /// closure.
  #[inline(always)]
  pub fn new<F>(mut stack: S, fun: F) -> Self
    where F: FnOnce(&mut ThreadLocals<S>, Args) -> ! + Send + 'a
  {
    let tl = ThreadLocals {
      stack_id: StackId::register(&mut stack),
      stack: stack
    };
    let (sp, tlp): (StackPointer, Unique<ThreadLocals<S>>) = unsafe {
      let rets = fat_args::init0(&tl.stack);
      fat_args::init1(rets, None, move |initializer_sp| -> ! {
        // We explicitly move the thread-locals and closure so they are owned by
        // the new stack
        let mut tl_move = tl;
        let fun_move = fun;
        // Yield back to context, given whoever initialized us nothing and
        // expecting a `ArgsF` to assemble our `ArgsF::Output`
        let (sp, payload) = fat_args::swap(Unique::new(&mut tl_move),
                                           initializer_sp,
                                           None);
        let args = Args::rebuild_raw(sp, payload);
        fun_move(&mut tl_move, args)
      })
    };
    Context {
      stack_pointer: sp,
      thread_locals: Some(tlp),
      _ref: PhantomData
    }
  }
}

impl<'a, TheirArgs, TheirStack> Context<'a, TheirArgs, TheirStack>
  where Context<'a, TheirArgs, TheirStack>: Send + 'a,
        TheirArgs: RebuildRaw<'a> + 'a,
        TheirStack: Stack + 'a,
{
  #[inline(always)]
  pub(super) unsafe fn raw_switch<'b, OurArgs, OurStack>
    (self, their_payload: TheirArgs::PayloadRaw) -> OurArgs
    where 'a: 'b,
          Context<'b, OurArgs, OurStack>: Send + 'b,
          OurArgs: RebuildRaw<'b> + 'b,
          OurStack: Stack + 'b,
  {
    debug!("new stack_pointer: {:?}", self.stack_pointer);
    let (sp, our_payload) =
      fat_args::swap(their_payload, self.stack_pointer, None);
    debug!("old stack_pointer: {:?}", sp);
    ::core::mem::forget(self);
    OurArgs::rebuild_raw(sp, our_payload)
  }
}

/// Session contexts must rebuild the argument from a black-box payload and the
/// old stack pointer before as the last step before handing off control to the
/// new coroutine. This trait describes how to do that.
pub unsafe trait RebuildRaw<'a> {
  /// The extra data sent over in addition to the old stack pointer
  type PayloadRaw: 'a;

  /// The function which actually does the rebuilding.
  unsafe fn rebuild_raw(StackPointer, Self::PayloadRaw) -> Self where Self: 'a;
}

unsafe impl<'a> RebuildRaw<'a> for !
{
  type PayloadRaw = !;
  unsafe fn rebuild_raw(_sp: StackPointer, payload: !) -> ! { payload }
}


unsafe impl<'a> RebuildRaw<'a> for ()
{
  type PayloadRaw = ();
  unsafe fn rebuild_raw(_sp: StackPointer, (): ()) { }
}


impl<'a, Args, S: ?Sized> Drop for Context<'a, Args, S>
  where Args: /*RebuildRaw<'a> +*/ 'a,
        S: Stack + 'a,
{
  /// Abandon the given context, dropping the stack it contained.
  #[inline]
  fn drop(&mut self) {
    if let Some(ptr) = ::core::mem::replace(&mut self.thread_locals, None) {
      unsafe {
        trace!("dropping thread: {:?}", (**ptr).stack_id);
        ::core::intrinsics::drop_in_place(*ptr);
      }
    }
  }
}

/// The stack is owned by itself
#[derive(Debug)]
pub struct ThreadLocals<Stack: ?Sized> {
  stack_id: StackId,
  stack: Stack,
}

pub const fn native_thread_locals<S>() -> Option<&'static mut ThreadLocals<S>> {
  None
}
