#![allow(non_camel_case_types)]

pub type stack_id_t = u32;
extern "C" {
  #[link_name = "valgrind_stack_register"]
  pub fn stack_register(start: *const u8, end: *const u8) -> stack_id_t;
  #[link_name = "valgrind_stack_deregister"]
  pub fn stack_deregister(id: stack_id_t);
}
