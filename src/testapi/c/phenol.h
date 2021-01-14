#ifndef PHENOL_H
#define PHENOL_H

struct env {
    size_t natomic_ints;
    size_t nints;
    
    atomic_int *atomic_ints;
    int *ints;

    /* The reference count of the environment; env is dead when this hits 0.
       Tests MUST NOT change this. */
    atomic_size_t rc;
};

struct manifest {
    size_t n_threads;      /* Number of threads in this test. */

    /* For each type, the number of variables of that type followed by arrays
       with size set to the respective number. */

    size_t n_atomic_ints;            /* Number of atomic_ints in this test. */
    const int *atomic_int_initials;  /* Initial value for each atomic_int. */
    const char **atomic_int_names;   /* Name of each atomic_int. */

    size_t n_ints;            /* Number of ints in this test. */
    const int *int_initials;  /* Initial value for each int. */
    const char **int_names;   /* Name of each int. */
};

#endif /* PHENOL_H */
