extern crate gcc;
use std::env::var_os;

fn main() {
  if var_os("CARGO_FEATURE_VALGRIND").is_some() {
    gcc::compile_library("libvalgrind.a", &["src/debug/valgrind/native.c"]);
  }
}
