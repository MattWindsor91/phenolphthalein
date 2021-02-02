#include <stdatomic.h>
#include <stdlib.h>

#include "env.h"
#include "phenol.h"

struct env *
alloc_env(size_t atomic_ints, size_t ints)
{
	struct env *e = calloc(1, sizeof(struct env));
	if (e == NULL) return NULL;
	
	// Setting this to 1 so that the last free doesn't underflow it,
	// though there's perhaps no law against that.
	e->rc = 1;

	e->atomic_ints = calloc(atomic_ints, sizeof(atomic_int));
	e->ints = calloc(ints, sizeof(int));
	
	return e;
}

struct env *
copy_env(struct env *e)
{
	// No need for this to be especially performant.
	atomic_fetch_add(&e->rc, 1);
	return e;
}

void
free_env(struct env *e)
{
	if (e == NULL) return;

	// The initial value of e->rc is 1, so freeing happens when we observe it
	// to be 1.  Again, no need for this to be especially performant.
	if (1 < atomic_fetch_sub(&e->rc, 1)) return;

	if (e->atomic_ints != NULL) free(e->atomic_ints);
	if (e->ints != NULL) free(e->ints);
	free(e);
}

int
get_atomic_int(const struct env *e, size_t c)
{
	if (c < e->natomic_ints) return 0;
	return e->atomic_ints[c];
}

int
get_int(const struct env *e, size_t c)
{
	if (c < e->nints) return 0;
	return e->ints[c];
}

void
set_atomic_int(struct env *e, size_t c, int v)
{
	if (c < e->natomic_ints) return;
	e->atomic_ints[c] = v;
}

void
set_int(struct env *e, size_t c, int v)
{
	if (c < e->nints) return;
	e->ints[c] = v;
}
