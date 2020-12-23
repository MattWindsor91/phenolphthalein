/* This is an example C 'litmus test' lifted to the phenolphthalein test ABI.
   Ideally, there will be automated tooling to generate the required glue code,
   but this just serves as a reference. */

#include <stdatomic.h>
#include <stdbool.h>
#include "src/c/phenol.h"

/* Here is the Litmus test itself, with all parameters passed by pointers. */

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

/* The next few functions set up endpoints that phenolphthalein will call into
   to interact with the test. */

/* These macros aren't necessary, but make it a bit clearer as to which
   variables we're pulling out of the environment. */
#define _x(e) (e->atomic_ints[0])
#define _y(e) (e->atomic_ints[1])
#define _0_r0(e) (e->ints[0])
#define _1_r0(e) (e->ints[1])

/* phenolphthalein expects a `struct manifest` called `manifest` to be
   exported, with various pieces of information about the test such as the
   names of variables, number of threads, and so on. */
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

/* phenolphthalein doesn't call the threads directly, but instead calls this
   `test` function with the thread ID and shared memory environment.  The
    function should dispatch to the correct thread. */
void
test(size_t tid, struct env *e)
{
    if (tid == 0) P0(&_x(e), &_y(e), &_0_r0(e));
    if (tid == 1) P1(&_x(e), &_y(e), &_1_r0(e));
}

/* Finally, whenever phenolphtalein reads a state from the environment that it
   hasn't yet encountered, it calls `check` to make sure that the state
   satisfies any postconditions the test expects. */
bool
check(const struct env *e)
{
    int x = _x(e);
    int y = _y(e);
    int t0r0 = _0_r0(e);
    int t1r0 = _1_r0(e);

    if (x == 1 && y == 1 && t0r0 == 0 && t1r0 == 0) return true;
    if (x == 1 && y == 1 && t0r0 == 0 && t1r0 == 1) return true;
    if (x == 1 && y == 1 && t0r0 == 1 && t1r0 == 0) return true;
    return false;
}
