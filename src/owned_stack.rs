// This file is part of libfringe, a low-level green threading library.
// Copyright (c) whitequark <whitequark@whitequark.org>
// See the LICENSE file included in this distribution.
extern crate alloc;

use self::alloc::raw_vec::RawVec;
use self::alloc::boxed::Box;

/// OwnedStack allocates on heap and owns a non-guarded stack.
#[derive(Debug)]
pub struct OwnedStack(pub Box<[u8]>);

impl OwnedStack {
    /// Allocates a new stack with exactly `size` accessible bytes using
    /// the default Rust allocator.
    pub fn new(size: usize) -> OwnedStack {
        OwnedStack(unsafe { RawVec::with_capacity(size).into_box() })
    }
}

impl ::stack::Stack for OwnedStack {
    #[inline(always)]
    fn base(&self) -> *mut u8 {
        // The slice cannot wrap around the address space, so the conversion from usize
        // to isize will not wrap either.
        let len: isize = self.0.len() as isize;
        unsafe { self.limit().offset(len) }
    }

    #[inline(always)]
    fn limit(&self) -> *mut u8 {
        self.0.as_ptr() as *mut u8
    }
}
