// Copyright (c) 2015, Nathan Zadoks <nathan@nathan7.eu>
// See the LICENSE file included in this distribution.
extern crate gcc;
use std::env::var_os;

fn main() {
  if var_os("CARGO_FEATURE_VALGRIND").is_some() {
    gcc::compile_library("libvalgrind.a", &["src/debug/valgrind/native.c"]);
  }
}
