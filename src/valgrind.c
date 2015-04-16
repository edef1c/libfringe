#include <stdint.h>
#include "valgrind/valgrind.h"

// In order for Valgrind to keep track of stack overflows and such, it needs
// a little help. That help unfortunately comes in the form of a pair of C
// macros. Calling out to un-inlineable C code for this is pointlessly slow,
// but that's the way it is for now.

typedef uint32_t valgrind_stack_id_t;

// Register a stack with Valgrind. start < end. Returns an integer ID that can
// be used to deregister the stack when it's deallocated.
valgrind_stack_id_t lwt_stack_register(const void *start, const void *end) {
  return VALGRIND_STACK_REGISTER(start, end);
}

// Deregister a stack from Valgrind. Takes the integer ID that was returned
// on registration.
void lwt_stack_deregister(valgrind_stack_id_t id) {
  VALGRIND_STACK_DEREGISTER(id);
}
