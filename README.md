[![travis][travis-badge]][travis-url]
[![rustdoc][rustdoc-badge]][rustdoc-url]

[travis-badge]: https://img.shields.io/travis/edef1c/libfringe/master.svg?style=flat-square&label=travis
[travis-url]: https://travis-ci.org/edef1c/libfringe
[rustdoc-badge]: https://img.shields.io/badge/docs-rustdoc-brightgreen.svg?style=flat-square
[rustdoc-url]: https://edef1c.github.io/libfringe

# libfringe

libfringe is a library implementing safe, lightweight context switches,
without relying on kernel services. It can be used in hosted environments
(using `std`) as well as on bare metal (using `core`).

It provides the following safe abstractions:
  * an implementation of generators,
    [Generator](https://edef1c.github.io/libfringe/fringe/generator/struct.Generator.html).

It also provides the necessary low-level building blocks:
  * a trait that can be implemented by stack allocators,
    [Stack](https://edef1c.github.io/libfringe/fringe/struct.Stack.html);
  * a stack allocator based on anonymous memory mappings with guard pages,
    [OsStack](https://edef1c.github.io/libfringe/fringe/struct.OsStack.html).

libfringe emphasizes safety and correctness, and goes to great lengths to never
violate the platform ABI.

## Performance

libfringe does context switches in 3ns flat on x86 and x86_64!

```
test swap ... bench:         6 ns/iter (+/- 0)
```

## Limitations

The only architectures currently supported are x86 and x86_64.
Windows is not supported (see [explanation](#windows-compatibility) below).

## Installation

libfringe is a [Cargo](https://crates.io) package.
It's not stable software yet, so you'll have to use it as a git dependency.
Add this to your `Cargo.toml`:

```toml
[dependencies.fringe]
git = "https://github.com/edef1c/libfringe.git"
```

### Feature flags

[Cargo's feature flags]: http://doc.crates.io/manifest.html#the-[features]-section
libfringe provides some optional features through [Cargo's feature flags].
Currently, all of them are enabled by default.

#### `valgrind`

[Valgrind]: http://valgrind.org
[Valgrind] integration. libfringe will register context stacks with Valgrind.

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
[failed before][tibfail] and it is bound fail again, breaking existing code that uses
boost::context in unexpected and complicated ways. The authors of libfringe consider this
unacceptable.

[mioco]: https://github.com/dpc/mioco
[context]: https://github.com/zonyitoo/context-rs
[boostcontext]: http://www.boost.org/doc/libs/1_60_0/libs/context/doc/html/context/overview.html
[TIB]: https://en.wikipedia.org/wiki/Win32_Thread_Information_Block
[tibfail]: https://svn.boost.org/trac/boost/ticket/8544

The only supported way to implement user-mode context switching on Windows is to use [fibers][].
There are no reasons the safe abstractions provided by libfringe could not be implemented on top
of that; it is simply not yet done. This should be straightforward and an implementation is
welcome.

[fibers]: https://msdn.microsoft.com/en-us/library/windows/desktop/ms682661(v=vs.85).aspx
