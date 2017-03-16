[![crates][crates-badge]][crates-url]
[![travis][travis-badge]][travis-url]
[![rustdoc][rustdoc-badge]][rustdoc-url]
[![irccloud][irccloud-badge]][irccloud-url]

[crates-badge]: https://img.shields.io/crates/v/fringe.svg?style=flat-square
[crates-url]: https://crates.io/crates/fringe
[travis-badge]: https://img.shields.io/travis/edef1c/libfringe/master.svg?style=flat-square&label=travis
[travis-url]: https://travis-ci.org/edef1c/libfringe
[rustdoc-badge]: https://img.shields.io/badge/docs-rustdoc-brightgreen.svg?style=flat-square
[rustdoc-url]: https://edef1c.github.io/libfringe
[irccloud-badge]: https://img.shields.io/badge/IRC-%23libfringe-1e72ff.svg?style=flat-square
[irccloud-url]: https://www.irccloud.com/invite?channel=%23libfringe&hostname=irc.mozilla.org&port=6697&ssl=1

# libfringe

libfringe is a library implementing safe, lightweight context switches,
without relying on kernel services. It can be used in hosted environments
(using `std`) as well as on bare metal (using `core`).

It provides the following safe abstractions:
  * an implementation of generators,
    [Generator](https://edef1c.github.io/libfringe/fringe/generator/struct.Generator.html).

It also provides the necessary low-level building blocks:
  * a trait that can be implemented by stack allocators,
    [Stack](https://edef1c.github.io/libfringe/fringe/trait.Stack.html);
  * a wrapper for using slice references as stacks,
    [SliceStack](https://edef1c.github.io/libfringe/fringe/struct.SliceStack.html);
  * a stack allocator based on `Box<[u8]>`,
    [OwnedStack](https://edef1c.github.io/libfringe/fringe/struct.OwnedStack.html);
  * a stack allocator based on anonymous memory mappings with guard pages,
    [OsStack](https://edef1c.github.io/libfringe/fringe/struct.OsStack.html).

libfringe emphasizes safety and correctness, and goes to great lengths to never
violate the platform ABI.

## Usage example

```rust
extern crate fringe;

use fringe::{OsStack, Generator};

fn main() {
  let stack = OsStack::new(1 << 16).unwrap();
  let mut gen = Generator::new(stack, move |yielder, ()| {
    for i in 1..4 { yielder.suspend(i) }
  });

  println!("{:?}", gen.resume(())); // Some(1)
  println!("{:?}", gen.resume(())); // Some(2)
  println!("{:?}", gen.resume(())); // Some(3)
  println!("{:?}", gen.resume(())); // None
}
```

## Performance

libfringe does context switches in 3ns flat on x86 and x86_64!

```
test swap ... bench:         6 ns/iter (+/- 0)
```

## Debuggability

Uniquely among libraries implementing context switching, libfringe ensures that the call stack
does not abruptly end at the boundary of a generator. Let's consider this buggy code:

```rust
extern crate fringe;

use fringe::{OsStack, Generator};

fn main() {
  let stack = OsStack::new(1 << 16).unwrap();
  let mut gen = Generator::new(stack, move |yielder, mut index| {
    let values = [1, 2, 3];
    loop { index = yielder.suspend(values[index]) }
  });

  println!("{:?}", gen.resume(5));
}
```

It crashes with the following backtrace:

```
thread 'main' panicked at 'index out of bounds: the len is 3 but the index is 5', /checkout/src/libcore/slice.rs:658
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
stack backtrace:
   0: <usize as core::slice::SliceIndex<T>>::index
             at /checkout/src/libcore/slice.rs:658
   1: core::slice::<impl core::ops::Index<I> for [T]>::index
             at /checkout/src/libcore/slice.rs:560
   2: crash_test::main::{{closure}}
             at ./src/main.rs:9
   3: <fringe::generator::Generator<'a, Input, Output, Stack>>::unsafe_new::generator_wrapper
             at /home/edef/src/github.com/edef1c/libfringe/src/generator.rs:137
   4: fringe::arch::imp::init::trampoline_2
             at /home/edef/src/github.com/edef1c/libfringe/src/arch/x86_64.rs:116
   5: fringe::arch::imp::init::trampoline_1
             at /home/edef/src/github.com/edef1c/libfringe/src/arch/x86_64.rs:61
   6: <fringe::generator::Generator<'a, Input, Output, Stack>>::resume
             at /home/edef/src/github.com/edef1c/libfringe/src/arch/x86_64.rs:184
             at /home/edef/src/github.com/edef1c/libfringe/src/generator.rs:171
   7: crash_test::main
             at ./src/main.rs:12
```

Similarly, debuggers, profilers, and all other tools using the DWARF debug information have
full insight into the call stacks.

Note that the stack should be deep enough for the panic machinery to store its state—at any point
there should be at least 8 KiB of free stack space, or panicking will result in a segfault.

## Limitations

The architectures currently supported are: x86, x86_64, aarch64, or1k.

The platforms currently supported are: bare metal, Linux (any libc),
FreeBSD, DragonFly BSD, macOS.
Windows is not supported (see [explanation](#windows-compatibility) below).

## Installation

libfringe is a [Cargo](https://crates.io) package.
Add this to your `Cargo.toml`:

```toml
[dependencies.fringe]
version = "1.2.1"
```

To use libfringe on a bare-metal target, add the `no-default-features` key:

```toml
[dependencies.fringe]
version = "1.2.1"
no-default-features = true
```

### Feature flags

[Cargo's feature flags]: http://doc.crates.io/manifest.html#the-[features]-section
libfringe provides some optional features through [Cargo's feature flags].
Currently, all of them are enabled by default.

#### `alloc`

This flag enables dependency on the `alloc` crate, which is required for
the [OwnedStack](https://edef1c.github.io/libfringe/fringe/struct.OwnedStack.html).

#### `valgrind`

This flag enables [Valgrind] integration. libfringe will register context stacks with Valgrind.

[Valgrind]: http://valgrind.org

## Internals

libfringe uses two key implementation techniques.

### Compiler-assisted register spilling

Traditionally, libraries implementing context switches in userspace have to spill all callee-saved
registers. On the other hand, libfringe fully inlines calls to every function that eventually
results in a context switch, and uses an inline assembly statement marking every register as
clobbered to implement the context switching itself.

As a result, only minimal work needs to be performed in the context switching code (LLVM does not
support spilling the frame pointer), which is especially important on architectures with lots
of callee-saved registers.

### Call stack splicing

Non-Windows platforms use [DWARF][] for both stack unwinding and debugging. DWARF call frame
information is very generic to be ABI-agnostic—it defines a bytecode that describes the actions
that need to be performed to simulate returning from a function. libfringe uses this bytecode
to specify that, after the generator function has returned, execution continues at the point
where the generator function was resumed the last time.

[dwarf]: http://dwarfstd.org

## Windows compatibility

As was said, libfringe emphasizes following the platform ABI. On Windows, the platform ABI
does not allow moving the stack pointer from the range designated by the OS during thread creation.
Therefore, the technique used by libfringe on *nix platforms is not applicable, and libfringe
does not provide Windows support.

You might ask, "but what about [mioco][]?" The mioco library uses the [context][] library to
implement context switches, which is little more than a wrapper of [boost::context][boostcontext].
The boost::context library changes undocumented fields in the [TIB][] during every context switch
to try and work around the restrictions placed by the Windows platform ABI. This has
[failed before][tibfail] and it is bound to fail again, breaking existing code that uses
boost::context in unexpected and complicated ways. The authors of libfringe consider this
unacceptable.

[mioco]: https://github.com/dpc/mioco
[context]: https://github.com/zonyitoo/context-rs
[boostcontext]: http://www.boost.org/doc/libs/1_60_0/libs/context/doc/html/context/overview.html
[TIB]: https://en.wikipedia.org/wiki/Win32_Thread_Information_Block
[tibfail]: https://svn.boost.org/trac/boost/ticket/8544

The only supported way to implement user-mode context switching on Windows is [fibers][].
There are no reasons the safe abstractions provided by libfringe could not be implemented on top
of that; it is simply not yet done. This should be straightforward and an implementation is
welcome. Note that while [UMS threads][] are capable of providing user-mode context switching,
they involve managing scheduler threads to run UMS threads on, which is incompatible with
libfringe's design.

[fibers]: https://msdn.microsoft.com/en-us/library/windows/desktop/ms682661(v=vs.85).aspx
[UMS threads]: https://msdn.microsoft.com/en-us/library/windows/desktop/dd627187(v=vs.85).aspx

## License

Licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you, as defined in the Apache-2.0
license, shall be dual licensed as above, without any additional terms or
conditions.
