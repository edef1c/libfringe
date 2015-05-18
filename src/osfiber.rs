//! Provides ergonomic usage of context with an OS type stack.
//!
//! The following is an example usage:
//!
//!    #![feature(thread_local)]
//!    #![feature(asm)]
//!    extern crate fringe;
//!
//!    use fringe::Context;
//!    use std::mem::transmute;
//!    use fringe::OsFiber;
//!
//!    fn test() {
//!        println!("it's alive!");
//!        OsFiber::pause();
//!        println!("its alive again!");
//!        OsFiber::pause();
//!    }
//!
//!    fn main() {
//!
//!        let mut v: Vec<OsFiber> = Vec::new();
//!
//!
//!        for x in 0..1 {
//!            let stack = fringe::OsStack::new(4096).unwrap();
//!            println!("making context");
//!            let mut ctx = OsFiber::new(stack, move || {
//!                // There is a crash if we try to call a non-closure
//!                // function directly. Until it is fixed this will
//!                // work for calling non-closure functions.
//!                test();
//!            });
//!            v.push(ctx);
//!        }
//!
//!        loop {
//!            println!("resuming");
//!            for ctx in v.iter_mut() {
//!                ctx.resume();
//!            }
//!        }
//!    }

use core::prelude::*;
use super::Context;
use super::OsStack;
use std::mem::transmute;

#[thread_local]
static mut cur_fiber: *mut Fiber<'static> = 0 as *mut Fiber;

pub struct Fiber<'a> {
    pub dead:       bool,
    pub context:    Context<'a, OsStack>,
}

impl<'a> Fiber<'a> {
    pub fn pause() {
        unsafe {
            if !cur_fiber.is_null() {
                (*cur_fiber).context.swap();
            }
        }
    }

    pub fn resume(&mut self) -> bool {
        unsafe {
            if self.dead {
                // Let the caller know we are done.
                return true;
            } else {
                cur_fiber = transmute(self);
                (*cur_fiber).context.swap();
                // Let the caller know if we have completed.
                (*cur_fiber).dead
            }
        }
    }

    pub fn new<'b, F>(stack: OsStack, f: F) -> Fiber<'b> where F: FnOnce() + Send + 'b {
        unsafe {
            let wf = move || {
                f();
                // Force a swap back to not require the `f` function
                // to have to be explicit about returning control and
                // risking a CPU exception to the whole program, and
                // also mark us as dead.
                (*cur_fiber).dead = true;
                (*cur_fiber).context.swap();
            };
                Fiber {
                dead:           false,
               context:        Context::new(stack, wf),
            }
        }
    }
}