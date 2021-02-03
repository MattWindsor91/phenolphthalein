#include <stdatomic.h>
#include <stdint.h>
#include <stdlib.h>

#include "env.h"
#include "phenol.h"

struct env_priv {
    /* The reference count of the environment; env is dead when this hits 0. */
    atomic_size_t rc;

	/* Space for rent (eg, if we implement an indirect mode) */
};

struct env_priv *
alloc_env_priv()
{
	struct env_priv *p = calloc(1, sizeof(struct env_priv));
	if (p == NULL) return NULL;

	// Setting this to 1 so that the last free doesn't underflow it,
	// though there's perhaps no law against that.
	p->rc = 1;

	return p;
}

void
ref_env_priv(struct env_priv *p)
{
	// No need for this to be especially performant.
	atomic_fetch_add(&p->rc, 1);
}

size_t
unref_env_priv(struct env_priv *p)
{
	size_t nrefs = atomic_fetch_sub(&p->rc, 1);
	// The initial value of p->rc is 1, so freeing happens when we observe it
	// to be 1.  Again, no need for this to be especially performant.
	if (nrefs <= 1) free(p);

	return nrefs;
}

struct env *
alloc_env(size_t natomic_int32, size_t nint32)
{
	struct env *e = calloc(1, sizeof(struct env));
	if (e == NULL) return NULL;

	e->priv = alloc_env_priv();
	if (e->priv == NULL) goto fail;

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

struct env *
copy_env(struct env *e)
{
	ref_env_priv(e->priv);
	return e;
}

void
free_env(struct env *e)
{
	if (e == NULL) return;
	if (e->priv != NULL && (1 < unref_env_priv(e->priv))) return;

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
