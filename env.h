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

// Gets the atomic int at index c from env e.
// Not guaranteed to be thread-safe.
int get_atomic_int(const struct env *e, size_t c);

// Gets the int at index c from env e.
// Not guaranteed to be thread-safe.
int get_int(const struct env *e, size_t c);

// Sets the atomic int at index c of env e.
// Not guaranteed to be thread-safe.
void set_atomic_int(struct env *e, size_t c, int v);

// Sets the int at index c of env e.
// Not guaranteed to be thread-safe.
void set_int(struct env *e, size_t c, int v);

#endif /* ENV_H */
