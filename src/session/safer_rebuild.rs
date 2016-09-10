// This file is part of libfringe, a low-level green threading library.
// Copyright (c) Nathan Zadoks <nathan@nathan7.eu>,
//               John Ericson <Ericson2314@Yahoo.com>
// See the LICENSE file included in this distribution.
use core::marker::PhantomData;
use core::ptr::Unique;

use stack::Stack;
use stack_pointer::StackPointer;
use super::context::*;

// Commented to avoid cyclic problems
pub unsafe trait RebuildWithTl<'a>/*: RebuildRaw<b
    'a,
    Payload = (Option<&'a mut ThreadLocals<Self::OldStack>>,
               Self::PayloadRaw)
  >*/
  where Self: 'a + Send,
{
  /// The extra data sent over in addition to the old context
  type Payload: 'a + Send;

  type OldStack: Stack + 'a + Send;

  /// The function which actually does the rebuilding.
  unsafe fn rebuild_with_tl(StackPointer,
                     Option<&'a mut ThreadLocals<Self::OldStack>>,
                     Self::Payload)
                     -> Self;
}

unsafe impl<'a, Args> RebuildRaw<'a> for Args
  where Args: RebuildWithTl<'a>
{
  type PayloadRaw = (Option<&'a mut ThreadLocals<Args::OldStack>>,
                     Args::Payload);

  unsafe fn rebuild_raw(sp: StackPointer,
                        raw_payload: Self::PayloadRaw)
    -> Args
  {
    let (maybe_stack, payload) = raw_payload;
    Args::rebuild_with_tl(sp, maybe_stack, payload)
  }
}

fn transmute_lifetime<'a, S>(maybe_stack: Option<&mut ThreadLocals<S>>)
                         -> Option<&'a mut ThreadLocals<S>>
{
  unsafe { ::core::mem::transmute(maybe_stack) }
}

/// `StackPointer` is an unsafe abstraction, and thus `RebuildRaw` is an unsafe
/// trait to implement. This adapter trait allows one to work from a context for
/// the old coroutine instead, which is safe.
pub trait Rebuild<'a>: RebuildWithTl<'a>
  where Context<'a, Self::OldArgs, Self::OldStack>: Send,
        Self: 'a + Send,
{
  type OldArgs: RebuildRaw<'a> + 'a + Send;

  /// The function which actually does the rebuilding.
  fn rebuild(Context<'a, Self::OldArgs, Self::OldStack>,
             Self::Payload)
             -> Self;
}

unsafe impl<'a, Args: Rebuild<'a>> RebuildWithTl<'a> for Args
{
  default type Payload = Args::Payload;
  default type OldStack = Args::OldStack;

  unsafe fn rebuild_with_tl(sp: StackPointer,
                            maybe_stack: Option<&mut ThreadLocals<Args::OldStack>>,
                            payload: Args::Payload)
                            -> Args
  {
    let ctx = Context {
      stack_pointer: sp,
      thread_locals: maybe_stack.map(|tl| Unique::new(tl as *mut _ as *mut _)),
      _ref: PhantomData,
    };
    Args::rebuild(ctx, payload)
  }
}

/// Their::Old* == Our*
impl<'a, TheirArgs, TheirStack> Context<'a, TheirArgs, TheirStack>
  where TheirArgs: Rebuild<'a>,
        <TheirArgs::OldArgs as RebuildRaw<'a>>::PayloadRaw: Send,
        TheirStack: Stack + 'a,
        Context<'a, TheirArgs, TheirStack>: 'a + Send,
{
  #[inline(always)]
  pub fn switch(
    self,
    maybe_stack: Option<&mut ThreadLocals<TheirArgs::OldStack>>,
    their_payload: TheirArgs::Payload)
    -> TheirArgs::OldArgs
  {
    unsafe {
      self.raw_switch::<TheirArgs::OldArgs, TheirArgs::OldStack>
        ((transmute_lifetime(maybe_stack), their_payload))
    }
  }
}

/// Represents choice in the protocol
pub enum Either<T, U> {
  Left(T),
  Right(U)
}

unsafe impl<'a, A0, A1, S> RebuildWithTl<'a> for Either<A0, A1>
  where A0: Rebuild<'a, OldStack=S> + Send,
        A1: Rebuild<'a, OldStack=S> + Send,
        S: Stack + 'a + Send,
{
  type Payload = Either<A0::Payload,
                        A1::Payload>;
  type OldStack = S;

  unsafe fn rebuild_with_tl(sp: StackPointer,
                            maybe_stack: Option<&'a mut ThreadLocals<Self::OldStack>>,
                            either_payload: Self::Payload)
                            -> Either<A0, A1>
  {
    let thread_locals = maybe_stack.map(|tl| Unique::new(tl as *mut _ as *mut _));
    match either_payload {
      Either::Left(payload) => {
        let ctx = Context {
          stack_pointer: sp,
          thread_locals: thread_locals,
          _ref: PhantomData,
        };
        Either::Left(A0::rebuild(ctx, payload))
      },
      Either::Right(payload) => {
        let ctx = Context {
          stack_pointer: sp,
          thread_locals: thread_locals,
          _ref: PhantomData,
        };
        Either::Right(A1::rebuild(ctx, payload))
      },
    }
  }
}

impl<'a, TheirArgs0, TheirArgs1, TheirStack, OurStack>
  Context<'a, Either<TheirArgs0, TheirArgs1>, TheirStack>
  where TheirArgs0: Rebuild<'a, OldStack=OurStack>,
        TheirArgs1: Rebuild<'a, OldStack=OurStack>,
        <TheirArgs0::OldArgs as RebuildRaw<'a>>::PayloadRaw: Send,
        <TheirArgs1::OldArgs as RebuildRaw<'a>>::PayloadRaw: Send,
        TheirStack: Stack + 'a + Send,
        OurStack: Stack + 'a + Send,
        Context<'a, Either<TheirArgs0, TheirArgs1>, TheirStack>: 'a + Send,
{
  #[inline(always)]
  pub fn switch_left(
    self,
    maybe_stack: Option<&mut ThreadLocals<OurStack>>,
    their_payload: TheirArgs0::Payload)
    -> TheirArgs0::OldArgs
  {
    unsafe {
      self.raw_switch::<TheirArgs0::OldArgs, OurStack>
        ((transmute_lifetime(maybe_stack), Either::Left(their_payload)))
    }
  }

  #[inline(always)]
  pub fn switch_right(
    self,
    maybe_stack: Option<&mut ThreadLocals<OurStack>>,
    their_payload: TheirArgs1::Payload)
    -> TheirArgs1::OldArgs
  {
    unsafe {
      self.raw_switch::<TheirArgs1::OldArgs, OurStack>
        ((transmute_lifetime(maybe_stack), Either::Right(their_payload)))
    }
  }
}
