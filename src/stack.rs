use platform;
use core::ptr;

pub enum Stack {
  Native {
    sp_limit: *const u8
  },
  Managed(platform::Stack)
}

impl Stack {
  pub fn new(size: usize) -> Stack {
    Stack::Managed(platform::Stack::new(size))
  }

  pub unsafe fn native(limit: *const u8) -> Stack {
    Stack::Native {
      sp_limit: limit
    }
  }

  pub fn top(&mut self) -> *mut u8 {
    match *self {
      Stack::Native { .. } => ptr::null_mut(),
      Stack::Managed(ref mut stack) => stack.top()
    }
  }

  pub fn limit(&self) -> *const u8 {
    match *self {
      Stack::Native { sp_limit, .. } => sp_limit,
      Stack::Managed(ref stack) => stack.limit()
    }
  }
}
