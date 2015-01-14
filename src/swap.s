sub $$128, %rsp
pushq %fs:0x70
pushq %rbp
call 1f

popq %rbp
popq %fs:0x70
add $$128, %rsp
jmp 2f

1:
  movq (%rdi), %rax
  movq %rsp, (%rdi)
  movq %rax, %rsp
  popq %rax
  jmpq *%rax
2:
