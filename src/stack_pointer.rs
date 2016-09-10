use arch;
use stack::Stack;

#[derive(Debug, Clone, Copy)]
/// The bare-minimum context. It's quite unsafe
pub struct StackPointer(pub *mut usize);

impl StackPointer {
  #[inline(always)]
  pub unsafe fn push(&mut self, val: usize) {
    self.0 = self.0.offset(-1);
    *self.0 = val
  }

  pub unsafe fn init(
    new_stack: &Stack,
    fun: unsafe extern "C" fn(usize, StackPointer) -> !)
    -> StackPointer
  {
    let mut sp = StackPointer(new_stack.base() as _);
    arch::init(&mut sp, fun);
    sp
  }

  #[inline(always)]
  pub unsafe fn swap(arg: usize, new_sp: StackPointer,
                     new_stack: Option<&Stack>) -> (usize, StackPointer)
  {
    arch::swap(arg, new_sp, new_stack)
  }
}


#[cfg(test)]
mod tests {
  extern crate test;
  extern crate simd;

  use super::StackPointer;
  use ::OsStack;

  #[test]
  fn context() {
    unsafe extern "C" fn adder(arg: usize, stack_ptr: StackPointer) -> ! {
      println!("it's alive! arg: {}", arg);
      let (arg, stack_ptr) = StackPointer::swap(arg + 1, stack_ptr, None);
      println!("still alive! arg: {}", arg);
      StackPointer::swap(arg + 1, stack_ptr, None);
      panic!("i should be dead");
    }

    unsafe {
      let stack = OsStack::new(4 << 20).unwrap();
      let stack_ptr = StackPointer::init(&stack, adder);

      let (ret, stack_ptr) = StackPointer::swap(10, stack_ptr, Some(&stack));
      assert_eq!(ret, 11);
      let (ret, _) = StackPointer::swap(50, stack_ptr, Some(&stack));
      assert_eq!(ret, 51);
    }
  }

  #[test]
  fn context_simd() {
    unsafe extern "C" fn permuter(arg: usize, stack_ptr: StackPointer) -> ! {
      // This will crash if the stack is not aligned properly.
      let x = simd::i32x4::splat(arg as i32);
      let y = x * x;
      println!("simd result: {:?}", y);
      let (_, stack_ptr) = StackPointer::swap(0, stack_ptr, None);
      // And try again after a context switch.
      let x = simd::i32x4::splat(arg as i32);
      let y = x * x;
      println!("simd result: {:?}", y);
      StackPointer::swap(0, stack_ptr, None);
      panic!("i should be dead");
    }

    unsafe {
      let stack = OsStack::new(4 << 20).unwrap();
      let stack_ptr = StackPointer::init(&stack, permuter);

      let (_, stack_ptr) = StackPointer::swap(10, stack_ptr, Some(&stack));
      StackPointer::swap(20, stack_ptr, Some(&stack));
    }
  }

  unsafe extern "C" fn do_panic(arg: usize, stack_ptr: StackPointer) -> ! {
    match arg {
      0 => panic!("arg=0"),
      1 => {
        StackPointer::swap(0, stack_ptr, None);
        panic!("arg=1");
      }
      _ => unreachable!()
    }
  }

  #[test]
  #[should_panic="arg=0"]
  fn panic_after_start() {
    unsafe {
      let stack = OsStack::new(4 << 20).unwrap();
      let stack_ptr = StackPointer::init(&stack, do_panic);

      StackPointer::swap(0, stack_ptr, Some(&stack));
    }
  }

  #[test]
  #[should_panic="arg=1"]
  fn panic_after_swap() {
    unsafe {
      let stack = OsStack::new(4 << 20).unwrap();
      let stack_ptr = StackPointer::init(&stack, do_panic);

      let (_, stack_ptr) = StackPointer::swap(1, stack_ptr, Some(&stack));
      StackPointer::swap(0, stack_ptr, Some(&stack));
    }
  }

  #[bench]
  fn swap(b: &mut test::Bencher) {
    unsafe extern "C" fn loopback(mut arg: usize, mut stack_ptr: StackPointer) -> ! {
      // This deliberately does not ignore arg, to measure the time it takes
      // to move the return value between registers.
      loop {
        let data = StackPointer::swap(arg, stack_ptr, None);
        arg = data.0;
        stack_ptr = data.1;
      }
    }

    unsafe {
      let stack = OsStack::new(4 << 20).unwrap();
      let mut stack_ptr = StackPointer::init(&stack, loopback);

      b.iter(|| for _ in 0..10 {
        stack_ptr = StackPointer::swap(0, stack_ptr, Some(&stack)).1;
      });
    }
  }
}
