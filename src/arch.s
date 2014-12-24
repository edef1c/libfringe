; vim: ft=nasm
BITS 64

;; the structure containing every register that is saved on context switches.
;; this needs to match the struct in arch.rs, or shit will break badly.
struc context
  ctx_rbx resq 1
  ctx_rsp resq 1
  ctx_rbp resq 1
  ctx_rdi resq 1
  ctx_r12 resq 1
  ctx_r13 resq 1
  ctx_r14 resq 1
  ctx_r15 resq 1
  ctx_ip:
    resq 1
    alignb 16
  ctx_xmm0 resq 2
  ctx_xmm1 resq 2
  ctx_xmm2 resq 2
  ctx_xmm3 resq 2
  ctx_xmm4 resq 2
  ctx_xmm5 resq 2
endstruc

global lwt_swapcontext
lwt_swapcontext:
;; this is where the actual context switching takes place. first, save every
;; register in the current context into the leaving context, pointed at by rdi,
;; making sure the return address ends up in the IP slot. then, restore every
;; register from the entering context, pointed at by rsi, and jump to the
;; instruction pointer.
  pop rax

  ; save instruction pointer
  mov [rdi+ctx_ip], rax

  ; save non-volatile integer registers (including rsp)
  mov [rdi+ctx_rbx], rbx
  mov [rdi+ctx_rsp], rsp
  mov [rdi+ctx_rbp], rbp
  mov [rdi+ctx_r12], r12
  mov [rdi+ctx_r13], r13
  mov [rdi+ctx_r14], r14
  mov [rdi+ctx_r15], r15

  ; save 0th argument register
  mov [rdi+ctx_rdi], rdi

  ; save non-volatile XMM registers
  movapd [rdi+ctx_xmm0], xmm0
  movapd [rdi+ctx_xmm1], xmm1
  movapd [rdi+ctx_xmm2], xmm2
  movapd [rdi+ctx_xmm3], xmm3
  movapd [rdi+ctx_xmm4], xmm4
  movapd [rdi+ctx_xmm5], xmm5

  ; restore non-volatile integer registers
  mov rbx, [rsi+ctx_rbx]
  mov rsp, [rsi+ctx_rsp]
  mov rbp, [rsi+ctx_rbp]
  mov r12, [rsi+ctx_r12]
  mov r13, [rsi+ctx_r13]
  mov r14, [rsi+ctx_r14]
  mov r15, [rsi+ctx_r15]

  ; restore 0th argument register
  mov rdi, [rsi+ctx_rdi]

  ; restore non-volatile XMM registers
  movapd xmm0, [rsi+ctx_xmm0]
  movapd xmm1, [rsi+ctx_xmm1]
  movapd xmm2, [rsi+ctx_xmm2]
  movapd xmm3, [rsi+ctx_xmm3]
  movapd xmm4, [rsi+ctx_xmm4]
  movapd xmm5, [rsi+ctx_xmm5]

  jmp [rsi+ctx_ip]

global lwt_bootstrap
lwt_bootstrap:
;; some of the parameter registers aren't saved on context switch, and thus
;; can't be set into the struct directly. thus, initialisation from Rust-land
;; places the parameters in unrelated registers, and we frob them into place
;; out here, in assembly-land. below are the parameter registers in order,
;; along with the alternative register used in parentheses, if there is one.
;; rdi, rsi (r13), rdx (r14), rcx (r15), r8, r9
  mov rsi, r13
  mov rdx, r14
  mov rcx, r15
  jmp r12


;; Rust stores a stack limit at [fs:0x70]. These two functions set and retrieve
;; the limit. They could alternatively be implemented as #[inline(always)] Rust
;; functions, with inline assembly, but I prefer keeping all the assembly-land
;; stuff in here.

global lwt_set_sp_limit
lwt_set_sp_limit:
  mov [fs:0x70], rdi
  ret

global lwt_get_sp_limit
lwt_get_sp_limit:
  mov rax, [fs:0x70]
  ret

global lwt_abort
lwt_abort:
;; when a context is created for a native thread, it should only be switched
;; out of. if it's accidentally switched into, it'll hit this, because that's
;; what we set the initial IP to.
  ud2
