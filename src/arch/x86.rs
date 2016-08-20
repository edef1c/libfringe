// This file is part of libfringe, a low-level green threading library.
// Copyright (c) Nathan Zadoks <nathan@nathan7.eu>,
//               whitequark <whitequark@whitequark.org>
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
// A high-level overview of the function of the trampolines is:
// * The 2nd init trampoline puts a controlled value (written in swap to `new_cfa`)
//   into %ebx.
// * The 1st init trampoline tells the unwinder to set %esp to %ebx, thus continuing
//   unwinding at the swap call site instead of falling off the end of context stack.
// * The 1st init trampoline together with the swap trampoline also restore %ebp
//   when unwinding as well as returning normally, because LLVM does not do it for us.
use stack::Stack;

#[derive(Debug, Clone)]
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
        # will use %ebx as the next call frame address, restore return address
        # from CFA-4 and restore %ebp from CFA-8. This mirrors what the second half
        # of `swap_trampoline` does.
        .cfi_def_cfa %ebx, 0
        .cfi_offset %ebp, -8
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
        # will restore %ebx (and thus CFA of the first trampoline) from the stack slot.
        .cfi_offset %ebx, 4
        # Push argument.
        .cfi_def_cfa_offset 8
        pushl   %eax
        # Call the provided function.
        call    *8(%esp)
      "#
      : : : "memory" : "volatile")
  }

  unsafe fn push(sp: &mut StackPointer, val: usize) {
    sp.0 = sp.0.offset(-1);
    *sp.0 = val
  }

  let mut sp = StackPointer(stack.base() as *mut usize);
  push(&mut sp, 0xdead0cfa); // CFA slot
  push(&mut sp, f as usize); // function
  push(&mut sp, trampoline_1 as usize);
  push(&mut sp, 0xdeadbbbb); // saved %ebp
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
        pushl   %ebp

        # Remember stack pointer of the old context, in case %edx==%esi.
        movl    %esp, %ebx
        # Load stack pointer of the new context.
        movl    (%edx), %esp
        # Save stack pointer of the old context.
        movl    %ebx, (%esi)

        # Restore frame pointer of the new context.
        popl    %ebp

        # Return into the new context. Use `pop` and `jmp` instead of a `ret`
        # to avoid return address mispredictions (~8ns per `ret` on Ivy Bridge).
        popl    %ebx
        jmpl    *%ebx
      "#
      : : : "memory" : "volatile")
  }

  let ret: usize;
  asm!(
    r#"
      # Link the call stacks together.
      movl    %esp, (%edi)
      # Push instruction pointer of the old context and switch to
      # the new context.
      call    ${1:c}
    "#
    : "={eax}" (ret)
    : "s" (trampoline as usize)
      "{eax}" (arg)
      "{esi}" (old_sp)
      "{edx}" (new_sp)
      "{edi}" (new_cfa)
    : "eax",  "ebx",  "ecx",  "edx",  "esi",  "edi", //"ebp",  "esp",
      "mmx0", "mmx1", "mmx2", "mmx3", "mmx4", "mmx5", "mmx6", "mmx7",
      "xmm0", "xmm1", "xmm2", "xmm3", "xmm4", "xmm5", "xmm6", "xmm7",
      "cc", "fpsr", "flags", "memory"
    : "volatile");
  ret
}
