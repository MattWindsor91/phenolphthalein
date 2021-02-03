#ifndef ENV_H
#define ENV_H

#include <stdint.h>

struct env;

// Constructs a new environment with the given number of variable slots.
struct env *alloc_env(size_t n_atomic_int32, size_t n_int32);

// Copies an environment, returning a pointer to the new environment.
// This pointer may or may not be the same as e.
struct env *copy_env(struct env *e);

// Frees the environment e.
// Depending on the implementation of copy_env, this may or may not actually
// de-allocate e's contents on copies; regardless, one should not use e
// after freeing.
void free_env(struct env *e);

/*
 * Reading to and writing from an env outside a test
 *
 * These functions exist mainly for the benefit of test runners written in
 * languages where atomics aren't ABI comparible with those in C.
 */

// Gets the atomic 32-bit int at index c from env e.
// Not guaranteed to be thread-safe.
int32_t get_atomic_int32(const struct env *e, size_t c);

// Gets the 32-bit int at index c from env e.
// Not guaranteed to be thread-safe.
int32_t get_int32(const struct env *e, size_t c);

// Sets the atomic int at index c of env e.
// Not guaranteed to be thread-safe.
void set_atomic_int32(struct env *e, size_t c, int32_t v);

// Sets the int at index c of env e.
// Not guaranteed to be thread-safe.
void set_int32(struct env *e, size_t c, int32_t v);

#endif /* ENV_H */
