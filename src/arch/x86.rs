// This file is part of libfringe, a low-level green threading library.
// Copyright (c) edef <edef@edef.eu>,
//               whitequark <whitequark@whitequark.org>
//               Amanieu d'Antras <amanieu@gmail.com>
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

// To understand the machine code in this file, keep in mind these facts:
// * i686 SysV C ABI requires the stack to be aligned at function entry,
//   so that `%esp+4` is a multiple of 16. Aligned operands are a requirement
//   of SIMD instructions, and making this the responsibility of the caller
//   avoids having to maintain a frame pointer, which is necessary when
//   a function has to realign the stack from an unknown state.
// * i686 SysV C ABI passes the first argument on the stack. This is
//   unfortunate, because unlike every other architecture we can't reuse
//   `swap` for the initial call, and so we use a trampoline.
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
//   into %ebp. This is then used as the CFA for the 1st trampoline.
// * This controlled value points to the bottom of the stack of the parent context,
//   which holds the saved %ebp and return address from the call to swap().
// * The 1st init trampoline tells the unwinder to restore %ebp and its return
//   address from the stack frame at %ebp (in the parent stack), thus continuing
//   unwinding at the swap call site instead of falling off the end of context stack.
use core::mem;
use arch::StackPointer;
use unwind;

pub const STACK_ALIGNMENT: usize = 16;

// Rust's fastcall support is currently broken due to #18086, so we use a
// custom wrapper instead. We don't quite follow the normal fastcall ABI since
// we accept the first parameter in %edi rather than the usual %ecx.
#[naked]
unsafe extern "C" fn fastcall_start_unwind() {
  asm!(
    r#"
      subl    $$12, %esp
      .cfi_adjust_cfa_offset 12
      movl    %edi, (%esp)
      call    ${0:c}
    "#
    :
    : "s" (unwind::start_unwind as usize)
    : : "volatile")
}

pub unsafe fn init(stack_base: *mut u8, f: unsafe fn(usize, StackPointer)) -> StackPointer {
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
        # we reach this function from unwinding, %ebp will be pointing at the bottom
        # of the parent linked stack. This link is set each time swap() is called.
        # When unwinding the frame corresponding to this function, a DWARF unwinder
        # will use %ebp+8 as the next call frame address, restore return address
        # from CFA-4 and restore %ebp from CFA-8. This mirrors what the second half
        # of `swap_trampoline` does.
        .cfi_def_cfa %ebp, 8
        .cfi_offset %ebp, -8

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
        .cfi_def_cfa %ebp, 8
        .cfi_offset %ebp, -8
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
        # will restore %ebp (and thus CFA of the first trampoline) from the stack slot.
        # This stack slot is updated every time swap() is called to point to the bottom
        # of the stack of the context switch just switched from.
        .cfi_def_cfa %ebp, 8
        .cfi_offset %ebp, -8

        # This nop is here so that the return address of the swap trampoline
        # doesn't point to the start of the symbol. This confuses gdb's backtraces,
        # causing them to think the parent function is trampoline_1 instead of
        # trampoline_2.
        nop

        # Call unwind_wrapper with the provided function and the CFA address.
        leal    16(%esp), %edx
        pushl   8(%esp)
        pushl   %edx
        pushl   %esi
        pushl   %edi
        call    ${0:c}

        # Restore the stack pointer of the parent context. No CFI adjustments
        # are needed since we have the same stack frame as trampoline_1.
        movl    16(%esp), %esp

        # Restore frame pointer of the parent context.
        popl    %ebp
        .cfi_adjust_cfa_offset -4
        .cfi_restore %ebp

        # If the returned value is nonzero, trigger an unwind in the parent
        # context with the given exception object.
        movl    %eax, %edi
        testl   %eax, %eax
        jnz     ${1:c}

        # Clear the stack pointer. We can't call into this context any more once
        # the function has returned.
        xorl    %esi, %esi

        # Return into the parent context. Use `pop` and `jmp` instead of a `ret`
        # to avoid return address mispredictions (~8ns per `ret` on Ivy Bridge).
        popl    %eax
        .cfi_adjust_cfa_offset -4
        .cfi_register %eip, %eax
        jmpl    *%eax
      "#
      :
      : "s" (unwind::unwind_wrapper as usize)
        "s" (fastcall_start_unwind as usize)
      : : "volatile")
  }

  // We set up the stack in a somewhat special way so that to the unwinder it
  // looks like trampoline_1 has called trampoline_2, which has in turn called
  // swap::trampoline.
  //
  // There are 2 call frames in this setup, each containing the return address
  // followed by the %ebp value for that frame. This setup supports unwinding
  // using DWARF CFI as well as the frame pointer-based unwinding used by tools
  // such as perf or dtrace.
  let mut sp = StackPointer::new(stack_base);

  sp.push(0 as usize); // Padding to ensure the stack is properly aligned
  sp.push(f as usize); // Function that trampoline_2 should call

  // Call frame for trampoline_2. The CFA slot is updated by swap::trampoline
  // each time a context switch is performed.
  sp.push(trampoline_1 as usize + 2); // Return after the 2 nops
  sp.push(0xdead0cfa);                // CFA slot

  // Call frame for swap::trampoline. We set up the %ebp value to point to the
  // parent call frame.
  let frame = sp.offset(0);
  sp.push(trampoline_2 as usize + 1); // Entry point, skip initial nop
  sp.push(frame as usize);            // Pointer to parent call frame

  sp
}

#[inline(always)]
pub unsafe fn swap_link(arg: usize, new_sp: StackPointer,
                        new_stack_base: *mut u8) -> (usize, Option<StackPointer>) {
  #[naked]
  unsafe extern "C" fn trampoline() {
    asm!(
      r#"
        # Save frame pointer explicitly; the unwinder uses it to find CFA of
        # the caller, and so it has to have the correct value immediately after
        # the call instruction that invoked the trampoline.
        pushl   %ebp
        .cfi_adjust_cfa_offset 4
        .cfi_rel_offset %ebp, 0

        # Link the call stacks together by writing the current stack bottom
        # address to the CFA slot in the new stack.
        movl    %esp, -16(%ecx)

        # Pass the stack pointer of the old context to the new one.
        movl    %esp, %esi
        # Load stack pointer of the new context.
        movl    %edx, %esp

        # Restore frame pointer of the new context.
        popl    %ebp
        .cfi_adjust_cfa_offset -4
        .cfi_restore %ebp

        # Return into the new context. Use `pop` and `jmp` instead of a `ret`
        # to avoid return address mispredictions (~8ns per `ret` on Ivy Bridge).
        popl    %eax
        .cfi_adjust_cfa_offset -4
        .cfi_register %eip, %eax
        jmpl    *%eax
      "#
      : : : : "volatile")
  }

  let ret: usize;
  let ret_sp: usize;
  asm!(
    r#"
      # Push instruction pointer of the old context and switch to
      # the new context.
      call    ${2:c}
    "#
    : "={edi}" (ret)
      "={esi}" (ret_sp)
    : "s" (trampoline as usize)
      "{edi}" (arg)
      "{edx}" (new_sp.offset(0))
      "{ecx}" (new_stack_base)
    : "eax",  "ebx",  "ecx",  "edx",/*"esi",  "edi",  "ebp",  "esp",*/
      "mm0",  "mm1",  "mm2",  "mm3",  "mm4",  "mm5",  "mm6",  "mm7",
      "xmm0", "xmm1", "xmm2", "xmm3", "xmm4", "xmm5", "xmm6", "xmm7",
      "cc", "dirflag", "fpsr", "flags", "memory"
    : "volatile");
  (ret, mem::transmute(ret_sp))
}

#[inline(always)]
pub unsafe fn swap(arg: usize, new_sp: StackPointer) -> (usize, StackPointer) {
  // This is identical to swap_link, but without the write to the CFA slot.
  #[naked]
  unsafe extern "C" fn trampoline() {
    asm!(
      r#"
        pushl   %ebp
        .cfi_adjust_cfa_offset 4
        .cfi_rel_offset %ebp, 0
        movl    %esp, %esi
        movl    %edx, %esp
        popl    %ebp
        .cfi_adjust_cfa_offset -4
        .cfi_restore %ebp
        popl    %eax
        .cfi_adjust_cfa_offset -4
        .cfi_register %eip, %eax
        jmpl    *%eax
      "#
      : : : : "volatile")
  }

  let ret: usize;
  let ret_sp: usize;
  asm!(
    r#"
      call    ${2:c}
    "#
    : "={edi}" (ret)
      "={esi}" (ret_sp)
    : "s" (trampoline as usize)
      "{edi}" (arg)
      "{edx}" (new_sp.offset(0))
    : "eax",  "ebx",  "ecx",  "edx",/*"esi",  "edi",  "ebp",  "esp",*/
      "mm0",  "mm1",  "mm2",  "mm3",  "mm4",  "mm5",  "mm6",  "mm7",
      "xmm0", "xmm1", "xmm2", "xmm3", "xmm4", "xmm5", "xmm6", "xmm7",
      "cc", "dirflag", "fpsr", "flags", "memory"
      // We need the "alignstack" attribute here to ensure that the stack is
      // properly aligned if a call to start_unwind needs to be injected into
      // our stack context.
    : "volatile", "alignstack");
  (ret, mem::transmute(ret_sp))
}

#[inline(always)]
pub unsafe fn unwind(new_sp: StackPointer, new_stack_base: *mut u8) {
  // Argument to pass to start_unwind, based on the stack base address.
  let arg = unwind::unwind_arg(new_stack_base);

  // This is identical to swap_link, except that it performs a tail call to
  // start_unwind instead of returning into the target context.
  #[naked]
  unsafe extern "C" fn trampoline() {
    asm!(
      r#"
        pushl   %ebp
        .cfi_adjust_cfa_offset 4
        .cfi_rel_offset %ebp, 0
        movl    %esp, -16(%ecx)
        movl    %edx, %esp
        popl    %ebp
        .cfi_adjust_cfa_offset -4
        .cfi_restore %ebp

        # Jump to the start_unwind function, which will force a stack unwind in
        # the target context. This will eventually return to us through the
        # stack link.
        jmp     ${0:c}
      "#
      :
      : "s" (fastcall_start_unwind as usize)
      : : "volatile")
  }

  asm!(
    r#"
      call    ${0:c}
    "#
    :
    : "s" (trampoline as usize)
      "{edi}" (arg)
      "{edx}" (new_sp.offset(0))
      "{ecx}" (new_stack_base)
    : "eax",  "ebx",  "ecx",  "edx",  "esi",  "edi",/*"ebp",  "esp",*/
      "mm0",  "mm1",  "mm2",  "mm3",  "mm4",  "mm5",  "mm6",  "mm7",
      "xmm0", "xmm1", "xmm2", "xmm3", "xmm4", "xmm5", "xmm6", "xmm7",
      "cc", "dirflag", "fpsr", "flags", "memory"
    : "volatile");
}
