// This file is part of libfringe, a low-level green threading library.
// Copyright (c) Nathan Zadoks <nathan@nathan7.eu>,
//               whitequark <whitequark@whitequark.org>
//               Amanieu d'Antras <amanieu@gmail.com>
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

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
//   to reuse the swap function to perform the initial call. We do the same
//   thing with %rsi to pass the stack pointer to the new context.
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
//
// A high-level overview of the function of the trampolines when unwinding is:
// * The 2nd init trampoline puts a controlled value (written in swap to `new_cfa`)
//   into %rbp. This is then used as the CFA for the 1st trampoline.
// * This controlled value points to the bottom of the stack of the parent context,
//   which holds the saved %rbp and return address from the call to swap().
// * The 1st init trampoline tells the unwinder to restore %rbp and its return
//   address from the stack frame at %rbp (in the parent stack), thus continuing
//   unwinding at the swap call site instead of falling off the end of context stack.
use core::mem;
use stack::Stack;
use stack_pointer::StackPointer;

pub const STACK_ALIGNMENT: usize = 16;

pub unsafe fn init(sp: &mut StackPointer,
                   f: unsafe extern "C" fn(usize, StackPointer) -> !) {
  #[cfg(not(target_vendor = "apple"))]
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

        # Set up the first part of our DWARF CFI linking stacks together. When
        # we reach this function from unwinding, %rbp will be pointing at the bottom
        # of the parent linked stack. This link is set each time swap() is called.
        # When unwinding the frame corresponding to this function, a DWARF unwinder
        # will use %rbp+16 as the next call frame address, restore return address
        # from CFA-8 and restore %rbp from CFA-16. This mirrors what the second half
        # of `swap_trampoline` does.
        .cfi_def_cfa %rbp, 16
        .cfi_offset %rbp, -16

        # This nop is here so that the initial swap doesn't return to the start
        # of the trampoline, which confuses the unwinder since it will look for
        # frame information in the previous symbol rather than this one. It is
        # never actually executed.
        nop

        # Stack unwinding in some versions of libunwind doesn't seem to like
        # 1-byte symbols, so we add a second nop here. This instruction isn't
        # executed either, it is only here to pad the symbol size.
        nop

      .Lend:
      .size __morestack, .Lend-__morestack
      "#
      : : : : "volatile")
  }

  #[cfg(target_vendor = "apple")]
  #[naked]
  unsafe extern "C" fn trampoline_1() {
    asm!(
      r#"
      # Identical to the above, except avoids .local/.size that aren't available on Mach-O.
      __morestack:
      .private_extern __morestack
        .cfi_def_cfa %rbp, 16
        .cfi_offset %rbp, -16
        nop
        nop
      "#
      : : : : "volatile")
  }

  #[naked]
  unsafe extern "C" fn trampoline_2() {
    asm!(
      r#"
        # Set up the second part of our DWARF CFI.
        # When unwinding the frame corresponding to this function, a DWARF unwinder
        # will restore %rbp (and thus CFA of the first trampoline) from the stack slot.
        # This stack slot is updated every time swap() is called to point to the bottom
        # of the stack of the context switch just switched from.
        .cfi_def_cfa %rbp, 16
        .cfi_offset %rbp, -16

        # This nop is here so that the return address of the swap trampoline
        # doesn't point to the start of the symbol. This confuses gdb's backtraces,
        # causing them to think the parent function is trampoline_1 instead of
        # trampoline_2.
        nop

        # Call the provided function.
        call    *16(%rsp)
      "#
      : : : : "volatile")
  }

  // We set up the stack in a somewhat special way so that to the unwinder it
  // looks like trampoline_1 has called trampoline_2, which has in turn called
  // swap::trampoline.
  //
  // There are 2 call frames in this setup, each containing the return address
  // followed by the %rbp value for that frame. This setup supports unwinding
  // using DWARF CFI as well as the frame pointer-based unwinding used by tools
  // such as perf or dtrace.

  sp.push(0 as usize); // Padding to ensure the stack is properly aligned
  sp.push(f as usize); // Function that trampoline_2 should call

  // Call frame for trampoline_2. The CFA slot is updated by swap::trampoline
  // each time a context switch is performed.
  sp.push(trampoline_1 as usize + 2); // Return after the 2 nops
  sp.push(0xdeaddeaddead0cfa);        // CFA slot

  // Call frame for swap::trampoline. We set up the %rbp value to point to the
  // parent call frame.
  let frame = *sp;
  sp.push(trampoline_2 as usize + 1); // Entry point
  sp.push(frame.0 as usize);          // Pointer to parent call frame
}

#[inline(always)]
pub unsafe fn swap(arg: usize, new_sp: StackPointer,
                   new_stack: Option<&Stack>) -> (usize, StackPointer) {
  // Address of the topmost CFA stack slot.
  let mut dummy: usize = mem::uninitialized();
  let new_cfa = if let Some(new_stack) = new_stack {
    (new_stack.base() as *mut usize).offset(-4)
  } else {
    // Just pass a dummy pointer if we aren't linking the stack
    &mut dummy
  };

  #[naked]
  unsafe extern "C" fn trampoline() {
    asm!(
      r#"
        # Save frame pointer explicitly; the unwinder uses it to find CFA of
        # the caller, and so it has to have the correct value immediately after
        # the call instruction that invoked the trampoline.
        pushq   %rbp
        .cfi_adjust_cfa_offset 8
        .cfi_rel_offset %rbp, 0

        # Link the call stacks together by writing the current stack bottom
        # address to the CFA slot in the new stack.
        movq    %rsp, (%rcx)

        # Pass the stack pointer of the old context to the new one.
        movq    %rsp, %rsi
        # Load stack pointer of the new context.
        movq    %rdx, %rsp

        # Restore frame pointer of the new context.
        popq    %rbp
        .cfi_adjust_cfa_offset -8
        .cfi_restore %rbp

        # Return into the new context. Use `pop` and `jmp` instead of a `ret`
        # to avoid return address mispredictions (~8ns per `ret` on Ivy Bridge).
        popq    %rax
        .cfi_adjust_cfa_offset -8
        .cfi_register %rip, %rax
        jmpq    *%rax
      "#
      : : : : "volatile")
  }

  let ret: usize;
  let ret_sp: *mut usize;
  asm!(
    r#"
      # Push instruction pointer of the old context and switch to
      # the new context.
      call    ${2:c}
    "#
    : "={rdi}" (ret)
      "={rsi}" (ret_sp)
    : "s" (trampoline as usize)
      "{rdi}" (arg)
      "{rdx}" (new_sp.0)
      "{rcx}" (new_cfa)
    : "rax",   "rbx",   "rcx",   "rdx", /*"rsi",   "rdi",   "rbp",   "rsp",*/
      "r8",    "r9",    "r10",   "r11",   "r12",   "r13",   "r14",   "r15",
      "mm0",   "mm1",   "mm2",   "mm3",   "mm4",   "mm5",   "mm6",   "mm7",
      "xmm0",  "xmm1",  "xmm2",  "xmm3",  "xmm4",  "xmm5",  "xmm6",  "xmm7",
      "xmm8",  "xmm9",  "xmm10", "xmm11", "xmm12", "xmm13", "xmm14", "xmm15",
      "xmm16", "xmm17", "xmm18", "xmm19", "xmm20", "xmm21", "xmm22", "xmm23",
      "xmm24", "xmm25", "xmm26", "xmm27", "xmm28", "xmm29", "xmm30", "xmm31",
      "cc", "dirflag", "fpsr", "flags", "memory"
      // Ideally, we would set the LLVM "noredzone" attribute on this function
      // (and it would be propagated to the call site). Unfortunately, rustc
      // provides no such functionality. Fortunately, by a lucky coincidence,
      // the "alignstack" LLVM inline assembly option does exactly the same
      // thing on x86_64.
    : "volatile", "alignstack");
  (ret, StackPointer(ret_sp))
}
