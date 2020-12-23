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
    size_t n_atomic_ints;  /* Number of atomic_ints in this test. */
    size_t n_ints;         /* Number of ints in this test. */

    /* These fields are arrays with size set to the respective number above. */

    int *atomic_int_initials;  /* Initial value for each atomic_int. */
    int *int_initials;         /* Initial value for each int. */

    const char **atomic_int_names;  /* Name of each atomic_int. */
    const char **int_names;         /* Name of each int. */
};

#endif /* PHENOL_H */
