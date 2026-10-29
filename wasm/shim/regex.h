#ifndef WASM_SHIM_REGEX_H
#define WASM_SHIM_REGEX_H
/* slice-1 stub: POSIX regex is unavailable — regcomp reports the
 * failure and regexec never matches. fn:matches/matches-type faces
 * degrade; the parse/xpath core is unaffected. */
typedef long regoff_t;
typedef struct { regoff_t rm_so, rm_eo; } regmatch_t;
typedef struct { int re_nsub; } regex_t;
#define REG_EXTENDED 1
#define REG_ICASE 2
#define REG_NOSUB 4
#define REG_NEWLINE 8
#define REG_NOMATCH 1
#define REG_BADPAT 2
#define REG_ENOSYS (-1)
int regcomp(regex_t *, const char *, int);
size_t regerror(int, const regex_t *, char *, size_t);
int regexec(const regex_t *, const char *, size_t, regmatch_t[], int);
void regfree(regex_t *);
#endif
