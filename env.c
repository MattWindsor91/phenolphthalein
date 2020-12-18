#include <stdatomic.h>
#include <stdlib.h>
#include "env.h"

struct env *
alloc_env(size_t atomic_ints, size_t ints)
{
	struct env *e = calloc(1, sizeof(struct env));
	if (e == NULL) return NULL;
	
	e->atomic_ints = calloc(atomic_ints, sizeof(atomic_int));
	e->ints = calloc(ints, sizeof(int));
	
	return e;
}

void
free_env(struct env *env)
{
	if (env == NULL) return;
	if (env->atomic_ints != NULL) free(env->atomic_ints);
	if (env->atomic_ints != NULL) free(env->ints);
	free(env);
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