[![travis][travis-badge]][travis-url]
[![rustdoc][rustdoc-badge]][rustdoc-url]

[travis-badge]: https://img.shields.io/travis/edef1c/libfringe/master.svg?style=flat-square
[travis-url]: https://travis-ci.org/edef1c/libfringe
[rustdoc-badge]: https://img.shields.io/badge/docs-rustdoc-brightgreen.svg?style=flat-square
[rustdoc-url]: https://edef1c.github.io/libfringe

# libfringe

  libfringe is a low-level green threading library for Rust.
  It's usable in freestanding environments (like kernels),
  but it can also provide an easy-to-use stack allocator using
  your operating system's memory mapping facility.

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
git = "https://github.com/edef1c/libfringe.git"
```

### Feature flags

  [Cargo's feature flags]: http://doc.crates.io/manifest.html#the-[features]-section
  libfringe provides some optional features through [Cargo's feature flags].
  Currently, all of them are enabled by default.

#### `valgrind`

  [Valgrind]: http://valgrind.org
  [Valgrind] integration. libfringe will register context stacks with Valgrind.
