// This file is part of libfringe, a low-level green threading library.
// Copyright (c) 2015, edef <edef@edef.eu>
// See the LICENSE file included in this distribution.

//! switch to a new context
//! arguments:
//!  * eax: stack pointer out pointer
//!  * ebx: stack pointer in pointer

// save the frame pointer
pushl %ebp

// save the return address to the stack, control flow continues at label 1
call 1f
// we arrive here once this context is reactivated

// restore the frame pointer
popl %ebp

// and we merrily go on our way, back into Rust-land
jmp 2f

1:
  // retrieve the new stack pointer
  movl (%eax), %edx
  // save the old stack pointer
  movl %esp, (%ebx)
  // switch to the new stack pointer
  movl %edx, %esp

  // jump into the new context (return to the call point)
  // doing this instead of a straight `ret` is 8ns slower,
  // presumably because the branch predictor tries to be clever about it
  popl %eax
  jmpl *%eax

2:
