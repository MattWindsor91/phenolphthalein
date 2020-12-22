#include <stdatomic.h>
#include "phenol.h"

int atomic_int_initials[2] = {0, 0};
int int_initials[2] = {0, 0};
const char *atomic_int_names[2] = {"x", "y"};
const char *int_names[2] = {"0:r0", "1:r0"};
struct manifest MANIFEST = {
    .nthreads = 2,
    .natomic_ints = 2,
    .nints = 2,
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
