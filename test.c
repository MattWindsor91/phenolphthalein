#include <stdatomic.h>
#include "env.h"

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
test(int tid, struct env *e)
{
    if (tid == 0) P0(e->atomic_ints, e->atomic_ints+1, e->ints);
    if (tid == 1) P1(e->atomic_ints, e->atomic_ints+1, e->ints+1);
}
