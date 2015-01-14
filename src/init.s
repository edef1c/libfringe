xchg %rsp, %rdi

pushq %rsi
pushq %rdx
pushq %rcx
call 1f

popq %fs:0x70
popq %rdi
popq %rax

movq $$0, %rbp
call *%rax
ud2

1:
  xchg %rsp, %rdi
