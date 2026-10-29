#ifndef WASM_SHIM_STRINGS_H
#define WASM_SHIM_STRINGS_H
#include <string.h>
int strcasecmp(const char *, const char *);
int strncasecmp(const char *, const char *, size_t);
#endif
