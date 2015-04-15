pub trait Stack {
  fn top(&mut self) -> *mut u8;
  fn limit(&self) -> *const u8;
}

pub trait StackSource {
  type Output: Stack;
  fn get_stack(size: usize) -> Self::Output;
}
