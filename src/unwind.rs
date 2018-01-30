// This file is part of libfringe, a low-level green threading library.
// Copyright (c) Amanieu d'Antras <amanieu@gmail.com>,
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.
extern crate std;

use self::std::panic;
use self::std::boxed::Box;
use core::any::Any;
use arch::StackPointer;

// Marker object that is passed through the stack unwinding
struct UnwindMarker {
  // We use the stack base as an identifier so that nested generators are handled
  // correctly. When unwinding, we will want to continue through any number of
  // nested generators until we reach the one with a matching identifier.
  stack_base: *mut u8,
}
unsafe impl Send for UnwindMarker {}

// Whether the current platform support unwinding across multiple stacks.
#[inline]
fn have_cross_stack_unwind() -> bool {
  // - Windows uses SEH for unwinding instead of libunwind. While it may be
  //   possible to munge it so support cross-stack unwinding, we stay conservative
  //   for now.
  // - iOS on ARM uses setjmp/longjmp instead of DWARF-2 unwinding, which needs
  //   to be explicitly saved/restored when switching contexts.
  // - LLVM doesn't currently support ARM EHABI directives in inline assembly so
  //   we instead need to propagate exceptions manually across contexts.
  !(cfg!(windows) || cfg!(target_arch = "arm"))
}

// Wrapper around the root function of a generator which handles unwinding.
#[unwind(allowed)]
pub unsafe extern "C" fn unwind_wrapper(arg: usize, sp: StackPointer, stack_base: *mut u8,
                                        f: unsafe fn(usize, StackPointer)) -> Option<Box<Box<Any + Send>>> {
  // Catch any attempts to unwind out of the context.
  match panic::catch_unwind(move || f(arg, sp)) {
    Ok(_) => None,
    Err(err) => {
      // If the unwinding is due to an UnwindMarker, check whether it is intended
      // for us by comparing the stack base of the caller with ours. If it is the
      // same then we can swallow the exception and return to the caller normally.
      if let Some(marker) = err.downcast_ref::<UnwindMarker>() {
        if marker.stack_base == stack_base {
          return None;
        }
      }

      // Otherwise, propagate the panic to the parent context.
      if have_cross_stack_unwind() {
        panic::resume_unwind(err)
      } else {
        // The assembly code will call start_unwind in the parent context and
        // pass it this Box as parameter.
        Some(Box::new(err))
      }
    }
  }
}

// Called by asm to start unwinding in the current context with the given
// exception object.
#[unwind(allowed)]
pub unsafe extern "C" fn start_unwind(panic: Box<Box<Any + Send>>) -> ! {
  // Use resume_unwind instead of panic! to avoid printing a message.
  panic::resume_unwind(*panic)
}

// Get the initial argument to pass to start_unwind, keyed to the base address
// of the generator stack that is going to be unwound.
#[inline]
pub fn unwind_arg(stack_base: *mut u8) -> usize {
  let marker = UnwindMarker {
    stack_base: stack_base
  };
  Box::into_raw(Box::new(Box::new(marker) as Box<Any + Send>)) as usize
}
