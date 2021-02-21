#include <stdatomic.h>
#include <stdint.h>
#include <stdlib.h>

#include "env.h"
#include "phenol.h"

struct env *
alloc_env(size_t natomic_int32, size_t nint32)
{
	struct env *e = calloc(1, sizeof(struct env));
	if (e == NULL) return NULL;

	e->natomic_int32 = natomic_int32;
	e->atomic_int32 = calloc(natomic_int32, sizeof(_Atomic int32_t));
	if (e->atomic_int32 == NULL) goto fail;

	e->nint32 = nint32;
	e->int32 = calloc(nint32, sizeof(int32_t));
	if (e->int32 == NULL) goto fail;
	
	return e;
fail:
	free_env(e);
	return NULL;
}

void
free_env(struct env *e)
{
	if (e == NULL) return;
	if (e->atomic_int32 != NULL) free(e->atomic_int32);
	if (e->int32 != NULL) free(e->int32);
	free(e);
}

int32_t
get_int32(const struct env *e, size_t c)
{
	if (e->nint32 < c) return 0;
	return e->int32[c];
}

int32_t
get_atomic_int32(const struct env *e, size_t c)
{
	if (e->natomic_int32 < c) return 0;
	return e->atomic_int32[c];
}

void
set_int32(struct env *e, size_t c, int32_t v)
{
	if (e->nint32 < c) return;
	e->int32[c] = v;
}

void
set_atomic_int32(struct env *e, size_t c, int32_t v)
{
	if (e->natomic_int32 < c) return;
	e->atomic_int32[c] = v;
}
