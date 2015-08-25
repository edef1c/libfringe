// This file is part of libfringe, a low-level green threading library.
// Copyright (c) 2015, Nathan Zadoks <nathan@nathan7.eu>
// See the LICENSE file included in this distribution.

//! initialise a new context
//! arguments:
//!  * rdi: stack pointer
//!  * rsi: function pointer
//!  * rdx: data pointer
//!  * rcx: stack limit
//!
//! return values:
//!  * rdi: new stack pointer

// switch to the fresh stack
xchg %rsp, %rdi

// save the function pointer, data pointer, and stack limit, respectively
pushq %rsi
pushq %rdx
pushq %rcx

// save the return address, control flow continues at label 1
call 1f
// we arrive here once this context is reactivated (see swap.s)

// restore the stack limit, data pointer, and function pointer, respectively
popq %fs:0x70
popq %rdi
popq %rax

// initialise the frame pointer
movq $$0, %rbp

// call the function pointer with the data pointer (rdi is the first argument)
call *%rax

1:
  // save our neatly-setup new stack
  xchg %rsp, %rdi
  // back into Rust-land we go
