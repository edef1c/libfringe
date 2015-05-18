// This file is part of libfringe, a low-level green threading library.
// Copyright (c) 2015, Nathan Zadoks <nathan@nathan7.eu>
// See the LICENSE file included in this distribution.

//! switch to a new context
//! arguments:
//!  * rdi: stack pointer pointer

// make sure we leave the red zone alone
sub $$128, %rsp

// save the Rust stack limit and the frame pointer, respectively
//pushq %fs:0x70
sub $$8, %rsp
pushq %rbp

// save the return address to the stack, control flow continues at label 1
call 1f
// we arrive here once this context is reactivated

// restore the frame pointer and the Rust stack limit, respectively
popq %rbp
// popq %fs:0x70
add $$8, %rsp

// give back the red zone
add $$128, %rsp

// and we merrily go on our way, back into Rust-land
jmp 2f

1:
  // retrieve the new stack pointer
  movq (%rdi), %rax
  // save the old stack pointer
  movq %rsp, (%rdi)
  // switch to the new stack pointer
  movq %rax, %rsp

  // jump into the new context (return to the call point)
  // doing this instead of a straight `ret` is 8ns faster,
  // presumably because the branch predictor tries
  // to be clever about it otherwise
  popq %rax
  jmpq *%rax

2: