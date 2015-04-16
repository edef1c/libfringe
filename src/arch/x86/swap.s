// This file is part of libfringe, a low-level green threading library.
// Copyright (c) 2015, edef <edef@edef.eu>
// See the LICENSE file included in this distribution.

//! switch to a new context
//! arguments:
//!  * eax: stack pointer pointer

// save the Rust stack limit and the frame pointer, respectively
// TODO: this stack limit location is specific to Linux/FreeBSD.
pushl %gs:0x30
pushl %ebp

// save the return address to the stack, control flow continues at label 1
call 1f
// we arrive here once this context is reactivated

// restore the frame pointer and the Rust stack limit, respectively
popl %ebp
// TODO: this stack limit location is specific to Linux/FreeBSD.
popl %gs:0x30

// and we merrily go on our way, back into Rust-land
jmp 2f

1:
  // retrieve the new stack pointer
  movl (%eax), %ebx
  // save the old stack pointer
  movl %esp, (%eax)
  // switch to the new stack pointer
  movl %ebx, %esp

  // jump into the new context (return to the call point)
  // doing this instead of a straight `ret` is 8ns slower,
  // presumably because the branch predictor tries to be clever about it
  popl %eax
  jmpl *%eax

2:
