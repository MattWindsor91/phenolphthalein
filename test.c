/* This is an example C 'litmus test' lifted to the phenolphthalein test ABI. */

#include <stdatomic.h>
#include <stdbool.h>
#include "phenol.h"

int atomic_int_initials[2] = {0, 0};
int int_initials[2] = {0, 0};
const char *atomic_int_names[2] = {"x", "y"};
const char *int_names[2] = {"0:r0", "1:r0"};
struct manifest manifest = {
    .n_threads = 2,
    .n_atomic_ints = 2,
    .n_ints = 2,
    .atomic_int_initials = atomic_int_initials,
    .int_initials = int_initials,
    .atomic_int_names = atomic_int_names,
    .int_names = int_names,
};

void
P0(atomic_int *x, atomic_int *y, int *r0)
{
    *r0 = atomic_load_explicit(x, memory_order_relaxed);
    atomic_store_explicit(y, 1, memory_order_relaxed);
}

void
P1(atomic_int *x, atomic_int *y, int *r0)
{
    *r0 = atomic_load_explicit(y, memory_order_relaxed);
    atomic_store_explicit(x, 1, memory_order_relaxed);
}

void
test(size_t tid, struct env *e)
{
    if (tid == 0) P0(e->atomic_ints, e->atomic_ints+1, e->ints);
    if (tid == 1) P1(e->atomic_ints, e->atomic_ints+1, e->ints+1);
}

bool
check(const struct env *e)
{
    int x = e->atomic_ints[0];
    int y = e->atomic_ints[1];
    int t0r0 = e->ints[0];
    int t1r0 = e->ints[1];

    if (x == 1 && y == 1 && t0r0 == 0 && t1r0 == 0) return true;
    if (x == 1 && y == 1 && t0r0 == 0 && t1r0 == 1) return true;
    if (x == 1 && y == 1 && t0r0 == 1 && t1r0 == 0) return true;
    return false;
}
