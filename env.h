#ifndef ENV_H
#define ENV_H

struct env {
    size_t natomic_ints;
    size_t nints;
    
    atomic_int *atomic_ints;
    int *ints;
};

struct env *alloc_env(size_t atomic_ints, size_t ints);
void free_env (struct env *e);

int get_atomic_int(const struct env *e, size_t c);

int get_int(const struct env *e, size_t c);

#endif /* ENV_H */
