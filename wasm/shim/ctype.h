#ifndef WASM_SHIM_CTYPE_H
#define WASM_SHIM_CTYPE_H
#include <stdint.h>
int isalnum(int);
int isalpha(int);
int iscntrl(int);
int isdigit(int);
int isgraph(int);
int islower(int);
int isprint(int);
int ispunct(int);
int isspace(int);
int isupper(int);
int isxdigit(int);
int tolower(int);
int toupper(int);
#endif
