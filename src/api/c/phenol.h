#ifndef PHENOL_H
#define PHENOL_H

/* This file contains struct definitions for the phenolpthalein C ABI.

   This header should be included by C tests using phenolpthalein; it makes
   various details about the shared state environment (`struct env`) and test
   manifest (`struct manifest`) transparent. */

#include <stdatomic.h>
#include <stdint.h>

/* Private area for parts of the environment the test shouldn't modify. */
struct env_priv;

/* The environment structure.

   The environment contains dynamically allocated arrays that line up with the
   slots defined in `struct manifest`.  Tests should read from and write to
   the appropriate indices in those arrays wherever they would normally read
   from and write to the corresponding variables. */
struct env {
    /* 32-bit atomic integers */
    size_t           natomic_int32;
    _Atomic int32_t *atomic_int32;

    /* 32-bit non-atomic integers */
    size_t           nint32;
    int32_t         *int32;

    struct env_priv *priv;  /* Private area */
};

/* The manifest structure.

   Tests must expose a `struct manifest` as a symbol with the name `manifest`.
*/
struct manifest {
    size_t n_threads;  /* Number of threads in this test. */

    /* For each type, the number of variables of that type followed by arrays
       with size set to the respective number: */

    size_t          n_atomic_int32;         /* Number of atomic int32_ts in this test. */
    const int32_t  *atomic_int32_initials;  /* Initial value for each atomic int32_t. */
    const char    **atomic_int32_names;     /* Name of each atomic int32_t. */

    size_t          n_int32;                /* Number of ints in this test. */
    const int32_t  *int32_initials;         /* Initial value for each int32_t. */
    const char    **int32_names;            /* Name of each int32_t. */
};

#endif /* PHENOL_H */
