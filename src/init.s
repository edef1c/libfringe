/// initialise a new context
/// arguments: rdi: stack pointer,
///            rsi: function pointer,
///            rdx, data pointer
///            rcx, stack limit

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

// crash if it ever returns
ud2

1:
  // save our neatly-setup new stack
  xchg %rsp, %rdi
  // back into Rust-land we go
