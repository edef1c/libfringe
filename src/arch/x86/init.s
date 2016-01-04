// This file is part of libfringe, a low-level green threading library.
// Copyright (c) 2015, Nathan Zadoks <nathan@nathan7.eu>
// See the LICENSE file included in this distribution.

//! initialise a new context
//! arguments:
//!  * eax: stack pointer
//!  * ebx: function pointer
//!  * ecx: data pointer
//!
//! return values:
//!  * eax: new stack pointer

// switch to the fresh stack
xchg %esp, %eax

// save the data pointer and the function pointer, respectively
pushl %ecx
pushl %ebx

// save the return address, control flow continues at label 1
call 1f
// we arrive here once this context is reactivated (see swap.s)

// restore the function pointer (the data pointer is the first argument, which lives at the top of the stack)
popl %eax

// initialise the frame pointer
movl $$0, %ebp

// call the function pointer with the data pointer (top of the stack is the first argument)
call *%eax

// crash if it ever returns
ud2

1:
  // save our neatly-setup new stack
  xchg %esp, %eax
  // back into Rust-land we go
