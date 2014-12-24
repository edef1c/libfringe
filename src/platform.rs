use libc;

extern "C" {
  #[link_name = "lwut_stack_register"]
  pub fn stack_register(start: *const u8, end: *const u8) -> libc::c_uint;
  #[link_name = "lwut_stack_deregister"]
  pub fn stack_deregister(id: libc::c_uint);
}
