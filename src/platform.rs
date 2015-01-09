extern crate libc;
extern crate std;
use self::std::prelude::v1::*;
use self::std::os::{errno, page_size, MemoryMap};
use self::std::os::MapOption::{MapReadable, MapWritable, MapNonStandardFlags};

extern "C" {
  #[link_name = "lwt_stack_register"]
  fn stack_register(start: *const u8, end: *const u8) -> libc::c_uint;
  #[link_name = "lwt_stack_deregister"]
  fn stack_deregister(id: libc::c_uint);
}

pub struct Stack {
  buf: MemoryMap,
  valgrind_id: libc::c_uint
}


const STACK_FLAGS: libc::c_int = libc::MAP_STACK
                               | libc::MAP_PRIVATE
                               | libc::MAP_ANON;

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
      stack_register(buf.data().offset(buf.len() as int) as *const _,
                     buf.data() as *const _)
    };

    Stack {
      buf: buf,
      valgrind_id: valgrind_id
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
    unsafe {
      stack_deregister(self.valgrind_id);
    }
  }
}

impl Stack {
  pub fn top(&mut self) -> *mut u8 {
    unsafe {
      self.buf.data().offset(self.buf.len() as int)
    }
  }

  pub fn limit(&self) -> *const u8 {
    unsafe {
      self.buf.data().offset(page_size() as int) as *const _
    }
  }
}
