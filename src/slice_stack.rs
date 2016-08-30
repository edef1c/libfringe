// This file is part of libfringe, a low-level green threading library.
// Copyright (c) whitequark <whitequark@whitequark.org>
// See the LICENSE file included in this distribution.

/// SliceStack holds a non-guarded stack allocated elsewhere and provided as a mutable
/// slice.
#[derive(Debug)]
pub struct SliceStack<'a>(pub &'a mut [u8]);

impl<'a> ::stack::Stack for SliceStack<'a> {
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
