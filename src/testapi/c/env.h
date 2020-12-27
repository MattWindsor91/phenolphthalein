#ifndef ENV_H
#define ENV_H

struct env;

// Constructs a new environment with the given number of variable slots.
struct env *alloc_env(size_t atomic_ints, size_t ints);

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

// Gets the atomic int at index c from env e.
// Not guaranteed to be thread-safe.
int get_atomic_int(const struct env *e, size_t c);

// Gets the int at index c from env e.
// Not guaranteed to be thread-safe.
int get_int(const struct env *e, size_t c);

// Sets the atomic int at index c of env e.
// Not guaranteed to be thread-safe.
void set_atomic_int(struct env *e, size_t c, int v);

// Sets the int at index c of env e.
// Not guaranteed to be thread-safe.
void set_int(struct env *e, size_t c, int v);

#endif /* ENV_H */
