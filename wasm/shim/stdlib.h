#ifndef WASM_SHIM_STDLIB_H
#define WASM_SHIM_STDLIB_H
#include <stddef.h>
#define EXIT_SUCCESS 0
#define EXIT_FAILURE 1
#define RAND_MAX 2147483647
void *malloc(size_t);
void *calloc(size_t, size_t);
void *realloc(void *, size_t);
void free(void *);
void abort(void);
char *getenv(const char *);
int atoi(const char *);
double atof(const char *);
float strtof(const char *, char **);
char *strndup(const char *, size_t);
long atol(const char *);
long strtol(const char *, char **, int);
long long strtoll(const char *, char **, int);
unsigned long long strtoull(const char *, char **, int);
unsigned long strtoul(const char *, char **, int);
double strtod(const char *, char **);
int abs(int);
long labs(long);
void qsort(void *, size_t, size_t, int (*)(const void *, const void *));
void *bsearch(const void *, const void *, size_t, size_t,
              int (*)(const void *, const void *));
int rand(void);
void srand(unsigned);
#endif
