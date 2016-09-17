// This file is part of libfringe, a low-level green threading library.
// Copyright (c) whitequark <whitequark@whitequark.org>
// See the LICENSE file included in this distribution.

use stack::Stack;

/// SliceStack holds a non-guarded stack allocated elsewhere and provided as a mutable slice.
#[derive(Debug)]
pub struct SliceStack<'a>(&'a mut [u8]);

impl<'a> SliceStack<'a> {
    /// Creates a `SliceStack` from an existing slice.
    ///
    /// This function will automatically align the slice to make it suitable for
    /// use as a stack. However this function may panic if the slice is smaller
    /// than `STACK_ALIGNMENT`.
    pub fn new(slice: &'a mut [u8]) -> SliceStack<'a> {
        // Align the given slice so that it matches platform requirements
        let ptr = slice.as_ptr() as usize;
        let adjusted_ptr = (ptr + ::STACK_ALIGNMENT - 1) & !(::STACK_ALIGNMENT - 1);
        let offset = adjusted_ptr - ptr;
        if offset > slice.len() {
            panic!("SliceStack too small");
        }

        let adjusted_len = (slice.len() - offset) & !(::STACK_ALIGNMENT - 1);
        SliceStack(&mut slice[offset..(offset + adjusted_len)])
    }
}

unsafe impl<'a> Stack for SliceStack<'a> {
    #[inline(always)]
    fn base(&self) -> *mut u8 {
        // The slice cannot wrap around the address space, so the conversion from usize
        // to isize will not wrap either.
        let len = self.0.len() as isize;
        unsafe { self.limit().offset(len) }
    }

    #[inline(always)]
    fn limit(&self) -> *mut u8 {
        self.0.as_ptr() as *mut u8
    }
}
