#![feature(asm, test)]
extern crate test;
use test::Bencher;

#[bench]
fn kernel_swap(b: &mut Bencher) {
  b.iter(|| unsafe {
    asm!("movq $$102, %rax\n\
          syscall"
         :
         :
         : "rax", "rcx"
         : "volatile");
  });
}
