extern crate gcc;

fn main() {
  gcc::compile_library("libcontext.a", &["src/platform.c"]);
}
