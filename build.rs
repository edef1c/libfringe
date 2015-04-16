extern crate gcc;

fn main() {
  gcc::compile_library("libvalgrind.a", &["src/debug/valgrind.c"]);
}
