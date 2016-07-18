[![travis][travis-badge]][travis-url]
[![appveyor][appveyor-badge]][appveyor-url]
[![rustdoc][rustdoc-badge]][rustdoc-url]

[travis-badge]: https://img.shields.io/travis/nathan7/libfringe/master.svg?style=flat-square&label=travis
[travis-url]: https://travis-ci.org/nathan7/libfringe
[appveyor-badge]: https://img.shields.io/appveyor/ci/nathan7/libfringe/master.svg?style=flat-square&label=appveyor
[appveyor-url]: https://ci.appveyor.com/project/nathan7/libfringe
[rustdoc-badge]: https://img.shields.io/badge/docs-rustdoc-brightgreen.svg?style=flat-square
[rustdoc-url]: https://nathan7.github.io/libfringe

# libfringe

libfringe is a library implementing lightweight context switches,
without relying on kernel services. It can be used in hosted environments
(using `std`) as well as on bare metal (using `core`).

It provides high-level, safe abstractions:
  * an implementation of internal iterators, also known as generators,
    [Generator](https://nathan7.github.io/libfringe/fringe/generator/struct.Generator.html).

It also provides low-level, *very* unsafe building blocks:
  * a flexible, low-level context-swapping mechanism,
    [Context](https://nathan7.github.io/libfringe/fringe/struct.Context.html);
  * a trait that can be implemented by stack allocators,
    [Stack](https://nathan7.github.io/libfringe/fringe/struct.Stack.html);
  * a stack allocator based on anonymous memory mappings with guard pages,
    [OsStack](https://nathan7.github.io/libfringe/fringe/struct.OsStack.html).

## Performance

  libfringe does context switches in 2.5ns flat on x86_64!
```
test swap ... bench:         5 ns/iter (+/- 1)
```

  …and on x86:

```
test swap ... bench:         5 ns/iter (+/- 1)
```

## Limitations

  libfringe currently doesn't work on anything but x86 and x86_64,
  and is untested on anything but Linux.

## Installation

  libfringe is a [Cargo](https://crates.io) package.
  It's not stable software yet, so you'll have to use it as a git dependency.
  Add this to your `Cargo.toml`:
```toml
[dependencies.fringe]
git = "https://github.com/nathan7/libfringe.git"
```

### Feature flags

  [Cargo's feature flags]: http://doc.crates.io/manifest.html#the-[features]-section
  libfringe provides some optional features through [Cargo's feature flags].
  Currently, all of them are enabled by default.

#### `valgrind`

  [Valgrind]: http://valgrind.org
  [Valgrind] integration. libfringe will register context stacks with Valgrind.

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
