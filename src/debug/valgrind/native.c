// This file is part of libfringe, a low-level green threading library.
// Copyright (c) 2015, Nathan Zadoks <nathan@nathan7.eu>
// See the LICENSE file included in this distribution.
#include <stdint.h>
#include "valgrind.h"

typedef uint32_t valgrind_stack_id_t;

valgrind_stack_id_t valgrind_stack_register(const void *start, const void *end) {
  return VALGRIND_STACK_REGISTER(start, end);
}

void valgrind_stack_deregister(valgrind_stack_id_t id) {
  VALGRIND_STACK_DEREGISTER(id);
}
