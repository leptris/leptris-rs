#ifndef WASM_SHIM_STDIO_H
#define WASM_SHIM_STDIO_H
#include <stdarg.h>
#include <stddef.h>
typedef struct _FILE FILE;
#include <stddef.h>
extern FILE *stderr;
#define EOF (-1)
#define FILENAME_MAX 4096
int snprintf(char *, size_t, const char *, ...);
int vsnprintf(char *, size_t, const char *, va_list);
int sscanf(const char *, const char *, ...);
int fprintf(FILE *, const char *, ...);
int printf(const char *, ...);
int puts(const char *);
int sprintf(char *, const char *, ...);
int vsprintf(char *, const char *, va_list);
FILE *fopen(const char *, const char *);
size_t fread(void *, size_t, size_t, FILE *);
size_t fwrite(const void *, size_t, size_t, FILE *);
int fclose(FILE *);
int fseek(FILE *, long, int);
long ftell(FILE *);
int feof(FILE *);
char *fgets(char *, int, FILE *);
int fflush(FILE *);
#define SEEK_SET 0
#define SEEK_CUR 1
#define SEEK_END 2
#endif
