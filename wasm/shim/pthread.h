#ifndef WASM_SHIM_PTHREAD_H
#define WASM_SHIM_PTHREAD_H
/* Single-threaded wasm: mutexes are no-ops. */
typedef int pthread_mutex_t;
typedef int pthread_once_t;
typedef unsigned pthread_key_t;
#define PTHREAD_ONCE_INIT 0
int pthread_key_create(pthread_key_t *, void (*)(void *));
int pthread_key_delete(pthread_key_t);
void *pthread_getspecific(pthread_key_t);
int pthread_setspecific(pthread_key_t, const void *);
#define PTHREAD_MUTEX_INITIALIZER 0
#define PTHREAD_ONCE_INIT 0
int pthread_mutex_lock(pthread_mutex_t *);
int pthread_mutex_unlock(pthread_mutex_t *);
int pthread_mutex_init(pthread_mutex_t *, const void *);
void pthread_once(pthread_once_t *, void (*)(void));
#endif
