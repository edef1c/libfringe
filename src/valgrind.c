#include <stdint.h>
#include "valgrind/valgrind.h"

typedef uint32_t valgrind_stack_id_t;

valgrind_stack_id_t valgrind_stack_register(const void *start, const void *end) {
  return VALGRIND_STACK_REGISTER(start, end);
}

void valgrind_stack_change(valgrind_stack_id_t id, const void *start, const void *end) {
  VALGRIND_STACK_CHANGE(id, start, end);
}

void valgrind_stack_deregister(valgrind_stack_id_t id) {
  VALGRIND_STACK_DEREGISTER(id);
}
