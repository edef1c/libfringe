// This file is part of libfringe, a low-level green threading library.
// Copyright (c) Nathan Zadoks <nathan@nathan7.eu>,
//               whitequark <whitequark@whitequark.org>
// See the LICENSE file included in this distribution.

// To understand the code in this file, keep in mind these two facts:
// * x86_64 SysV C ABI has a "red zone": 128 bytes under the top of the stack
//   that is defined to be unmolested by signal handlers, interrupts, etc.
//   Leaf functions can use the red zone without adjusting rsp or rbp.
// * x86_64 SysV C ABI requires the stack to be aligned at function entry,
//   so that (%rsp+8) is a multiple of 16. Aligned operands are a requirement
//   of SIMD instructions, and making this the responsibility of the caller
//   avoids having to maintain a frame pointer, which is necessary when
//   a function has to realign the stack from an unknown state.
// * x86_64 SysV C ABI passes the first argument in %rdi. We also use %rdi
//   to pass a value while swapping context; this is an arbitrary choice
//   (we clobber all registers and could use any of them) but this allows us
//   to reuse the swap function to perform the initial call.
//
// To understand the DWARF CFI code in this file, keep in mind these facts:
// * CFI is "call frame information"; a set of instructions to a debugger or
//   an unwinder that allow it to simulate returning from functions. This implies
//   restoring every register to its pre-call state, as well as the stack pointer.
// * CFA is "call frame address"; the value of stack pointer right before the call
//   instruction in the caller. Everything strictly below CFA (and inclusive until
//   the next CFA) is the call frame of the callee. This implies that the return
//   address is the part of callee's call frame.
// * Logically, DWARF CFI is a table where rows are instruction pointer values and
//   columns describe where registers are spilled (mostly using expressions that
//   compute a memory location as CFA+n). A .cfi_offset pseudoinstruction changes
//   the state of a column for all IP numerically larger than the one it's placed
//   after. A .cfi_def_* pseudoinstruction changes the CFA value similarly.
// * Simulating return is as easy as restoring register values from the CFI table
//   and then setting stack pointer to CFA.
use stack::Stack;

#[derive(Debug)]
pub struct StackPointer(*mut usize);

pub unsafe fn init(stack: &Stack, f: unsafe extern "C" fn(usize) -> !) -> StackPointer {
  #[naked]
  unsafe extern "C" fn trampoline_1() {
    asm!(
      r#"
        # gdb has a hardcoded check that rejects backtraces where frame addresses
        # do not monotonically decrease. It is turned off if the function is called
        # "__morestack" and that is hardcoded. So, to make gdb backtraces match
        # the actual unwinder behavior, we call ourselves "__morestack" and mark
        # the symbol as local; it shouldn't interfere with anything.
      __morestack:
      .local __morestack

        # Set up the first part of our DWARF CFI linking stacks together.
        # When unwinding the frame corresponding to this function, a DWARF unwinder
        # will use %rbx as the next call frame address, restore return address
        # from CFA-8 and restore %rbp from CFA-16. This mirrors what the second half
        # of `swap_trampoline` does.
        .cfi_def_cfa %rbx, 0
        .cfi_offset %rbp, -16
        # Call the next trampoline.
        call   ${0:c}

      .Lend:
      .size __morestack, .Lend-__morestack
      "#
      : : "s" (trampoline_2 as usize) : "memory" : "volatile")
  }

  #[naked]
  unsafe extern "C" fn trampoline_2() {
    asm!(
      r#"
        # Set up the second part of our DWARF CFI.
        # When unwinding the frame corresponding to this function, a DWARF unwinder
        # will restore %rbx (and thus CFA of the first trampoline) from the stack slot.
        .cfi_offset %rbx, 16
        # Call the provided function.
        call    *8(%rsp)
      "#
      : : : "memory" : "volatile")
  }

  unsafe fn push(sp: &mut StackPointer, val: usize) {
    sp.0 = sp.0.offset(-1);
    *sp.0 = val
  }

  let mut sp = StackPointer(stack.base() as *mut usize);
  push(&mut sp, 0xdeaddeaddead0cfa); // CFA slot
  push(&mut sp, 0 as usize); // alignment
  push(&mut sp, f as usize); // function
  push(&mut sp, trampoline_1 as usize);
  push(&mut sp, 0xdeaddeaddeadbbbb); // saved %rbp
  sp
}

#[inline(always)]
pub unsafe fn swap(arg: usize, old_sp: &mut StackPointer, new_sp: &StackPointer,
                   new_stack: &Stack) -> usize {
  // Address of the topmost CFA stack slot.
  let new_cfa = (new_stack.base() as *mut usize).offset(-1);

  #[naked]
  unsafe extern "C" fn trampoline() {
    asm!(
      r#"
        # Save frame pointer explicitly; the unwinder uses it to find CFA of
        # the caller, and so it has to have the correct value immediately after
        # the call instruction that invoked the trampoline.
        pushq   %rbp

        # Remember stack pointer of the old context, in case %rdx==%rsi.
        movq    %rsp, %rbx
        # Load stack pointer of the new context.
        movq    (%rdx), %rsp
        # Save stack pointer of the old context.
        movq    %rbx, (%rsi)

        # Restore frame pointer of the new context.
        popq    %rbp

        # Return into the new context. Use `pop` and `jmp` instead of a `ret`
        # to avoid return address mispredictions (~8ns per `ret` on Ivy Bridge).
        popq    %rbx
        jmpq    *%rbx
      "#
      : : : "memory" : "volatile")
  }

  let ret: usize;
  asm!(
    r#"
      # Link the call stacks together.
      movq    %rsp, (%rcx)
      # Push instruction pointer of the old context and switch to
      # the new context.
      call    ${1:c}
    "#
    : "={rdi}" (ret)
    : "s" (trampoline as usize)
      "{rdi}" (arg)
      "{rsi}" (old_sp)
      "{rdx}" (new_sp)
      "{rcx}" (new_cfa)
    : "rax",   "rbx",   "rcx",   "rdx",   "rsi",   "rdi", //"rbp",   "rsp",
      "r8",    "r9",    "r10",   "r11",   "r12",   "r13",   "r14",   "r15",
      "xmm0",  "xmm1",  "xmm2",  "xmm3",  "xmm4",  "xmm5",  "xmm6",  "xmm7",
      "xmm8",  "xmm9",  "xmm10", "xmm11", "xmm12", "xmm13", "xmm14", "xmm15",
      "xmm16", "xmm17", "xmm18", "xmm19", "xmm20", "xmm21", "xmm22", "xmm23",
      "xmm24", "xmm25", "xmm26", "xmm27", "xmm28", "xmm29", "xmm30", "xmm31"
      "cc", "fpsr", "flags", "memory"
      // Ideally, we would set the LLVM "noredzone" attribute on this function
      // (and it would be propagated to the call site). Unfortunately, rustc
      // provides no such functionality. Fortunately, by a lucky coincidence,
      // the "alignstack" LLVM inline assembly option does exactly the same
      // thing on x86_64.
    : "volatile", "alignstack");
  ret
}
