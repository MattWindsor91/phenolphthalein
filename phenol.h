#ifndef PHENOL_H
#define PHENOL_H

struct env {
    size_t natomic_ints;
    size_t nints;
    
    atomic_int *atomic_ints;
    int *ints;

    // The reference count of the environment; env is dead when this hits 0.
    // Tests MUST NOT change this.
    atomic_size_t rc;
};

struct manifest {
    size_t nthreads;
    size_t natomic_ints;
    size_t nints;

    int *atomic_int_initials;
    int *int_initials;
    const char **atomic_int_names;
    const char **int_names;
};

#endif /* PHENOL_H */
