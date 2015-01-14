pub trait Stack {
  fn top(&mut self) -> *mut u8;
  fn limit(&self) -> *const u8;
}
