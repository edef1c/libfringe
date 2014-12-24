use libc;
use platform;
use std::ptr;
use std::os::{errno, page_size, MemoryMap, MapReadable, MapWritable,
              MapNonStandardFlags};

const STACK_FLAGS: libc::c_int = libc::MAP_STACK
                               | libc::MAP_PRIVATE
                               | libc::MAP_ANON;

pub enum Stack {
  Native {
    sp_limit: *const u8
  },
  Managed {
    buf: MemoryMap,
    valgrind_id: libc::c_uint
  }
}

impl Stack {
  pub fn new(size: uint) -> Stack {
    let buf = match MemoryMap::new(size, &[MapReadable, MapWritable,
                                   MapNonStandardFlags(STACK_FLAGS)]) {
      Ok(map) => map,
      Err(e) => panic!("mmap for stack of size {} failed: {}", size, e)
    };

    if !protect_last_page(&buf) {
      panic!("Could not memory-protect guard page. stack={}, errno={}",
             buf.data(), errno());
    }

    let valgrind_id = unsafe {
      platform::stack_register(buf.data().offset(buf.len() as int) as *const _,
                               buf.data() as *const _)
    };


    let stk = Stack::Managed {
      buf: buf,
      valgrind_id: valgrind_id
    };

    stk
  }

  pub unsafe fn native(limit: *const u8) -> Stack {
    Stack::Native {
      sp_limit: limit
    }
  }

  pub fn top(&mut self) -> *mut u8 {
    unsafe {
      match *self {
        Stack::Native { .. } => ptr::null_mut(),
        Stack::Managed { ref buf, .. } => {
          buf.data().offset(buf.len() as int)
        }
      }
    }
  }

  pub fn limit(&self) -> *const u8 {
    unsafe {
      match *self {
        Stack::Native { sp_limit, .. } => sp_limit,
        Stack::Managed { ref buf, .. } => {
          buf.data().offset(page_size() as int) as *const _
        }
      }
    }
  }
}

fn protect_last_page(stack: &MemoryMap) -> bool {
  unsafe {
    let last_page = stack.data() as *mut libc::c_void;
    libc::mprotect(last_page, page_size() as libc::size_t,
                   libc::PROT_NONE) != -1
  }
}

impl Drop for Stack {
  fn drop(&mut self) {
    match *self {
      Stack::Native { .. } => {},
      Stack::Managed { valgrind_id, .. } => unsafe {
        platform::stack_deregister(valgrind_id);
      }
    }
  }
}
