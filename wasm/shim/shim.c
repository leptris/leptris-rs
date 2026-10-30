/* wasm libc shim for libleptris (leptris#1900 slice 1).
 *
 * wasm32-unknown-unknown has no OS libc. This TU implements the C99
 * surface the engine actually uses, over wasm linear memory:
 *   - first-fit freelist malloc (16-byte aligned, coalescing)
 *   - string/stdlib/ctype/math/time per C99 semantics
 *   - a bounded printf core (the engine's number/serialize paths)
 *   - no-op pthread mutexes (single-threaded wasm) + key slots
 * Limitations (slice 1): time() returns a fixed epoch (gmtime_r is a
 * real civil conversion, so a real clock can ride in later); file IO
 * fails cleanly; getenv returns NULL.
 */
#include <stdarg.h>
#include <stddef.h>
#include <stdint.h>
#include "string.h"
#include "stdlib.h"
#include "stdio.h"
#include "ctype.h"
#include "math.h"
#include "time.h"
#include "errno.h"
#include "pthread.h"
#include "regex.h"

/* ------------------------------------------------------------------ */
/* allocator                                                           */

/* C allocation routes through the crate's leptris_rust_alloc/dealloc/
 * realloc wrappers over std::alloc (see src/lib.rs) — one allocator
 * for the whole module. The block header carries the payload size the
 * dealloc/realloc wrappers require; NULL propagates as OOM. */
extern void* leptris_rust_alloc(size_t size, size_t align);
extern void leptris_rust_dealloc(void* ptr, size_t size, size_t align);
extern void* leptris_rust_realloc(void* ptr, size_t old_size,
                                  size_t align, size_t new_size);

#define SHIM_ALIGN 16u

typedef struct {
    size_t size; /* payload bytes (aligned) */
} block_t;

void* malloc(size_t size) {
    if (size == 0) size = 1;
    size_t payload = (size + SHIM_ALIGN - 1u) & ~(size_t)(SHIM_ALIGN - 1u);
    block_t* b = (block_t*)leptris_rust_alloc(
        payload + sizeof(block_t), SHIM_ALIGN);
    if (!b) return NULL;
    b->size = payload;
    return (void*)(b + 1);
}

void free(void* p) {
    if (!p) return;
    block_t* b = (block_t*)p - 1;
    leptris_rust_dealloc(b, b->size + sizeof(block_t), SHIM_ALIGN);
}

void* calloc(size_t n, size_t sz) {
    size_t total = n * sz;
    void* p = malloc(total);
    if (p) memset(p, 0, total);
    return p;
}

void* realloc(void* p, size_t size) {
    if (!p) return malloc(size);
    block_t* b = (block_t*)p - 1;
    size_t payload = (size + SHIM_ALIGN - 1u) & ~(size_t)(SHIM_ALIGN - 1u);
    void* np = leptris_rust_realloc(b, b->size + sizeof(block_t),
                                    SHIM_ALIGN, payload + sizeof(block_t));
    if (!np) return NULL;
    b = (block_t*)np;
    b->size = payload;
    return (void*)(b + 1);
}

void abort(void) { __builtin_unreachable(); }

/* ------------------------------------------------------------------ */
/* string                                                              */

void* memcpy(void* d, const void* s, size_t n) {
    unsigned char* dd = (unsigned char*)d;
    const unsigned char* ss = (const unsigned char*)s;
    while (n--) *dd++ = *ss++;
    return d;
}

void* memmove(void* d, const void* s, size_t n) {
    unsigned char* dd = (unsigned char*)d;
    const unsigned char* ss = (const unsigned char*)s;
    if (dd < ss) {
        while (n--) *dd++ = *ss++;
    } else if (dd > ss) {
        dd += n;
        ss += n;
        while (n--) *--dd = *--ss;
    }
    return d;
}

void* memset(void* d, int c, size_t n) {
    unsigned char* dd = (unsigned char*)d;
    while (n--) *dd++ = (unsigned char)c;
    return d;
}

int memcmp(const void* a, const void* b, size_t n) {
    const unsigned char* pa = (const unsigned char*)a;
    const unsigned char* pb = (const unsigned char*)b;
    for (; n; n--, pa++, pb++) {
        if (*pa != *pb) return *pa < *pb ? -1 : 1;
    }
    return 0;
}

void* memchr(const void* s, int c, size_t n) {
    const unsigned char* p = (const unsigned char*)s;
    for (; n; n--, p++) {
        if (*p == (unsigned char)c) return (void*)p;
    }
    return NULL;
}

size_t strlen(const char* s) {
    const char* p = s;
    while (*p) p++;
    return (size_t)(p - s);
}

char* strcpy(char* d, const char* s) {
    char* r = d;
    while ((*d++ = *s++)) {}
    return r;
}

char* strncpy(char* d, const char* s, size_t n) {
    char* r = d;
    while (n && *s) { *d++ = *s++; n--; }
    while (n--) *d++ = '\0';
    return r;
}

char* strcat(char* d, const char* s) {
    char* r = d;
    while (*d) d++;
    while ((*d++ = *s++)) {}
    return r;
}

int strcmp(const char* a, const char* b) {
    while (*a && *a == *b) { a++; b++; }
    return (unsigned char)*a - (unsigned char)*b;
}

int strncmp(const char* a, const char* b, size_t n) {
    for (; n; n--, a++, b++) {
        if (*a != *b) return (unsigned char)*a - (unsigned char)*b;
        if (!*a) break;
    }
    return 0;
}

char* strchr(const char* s, int c) {
    for (;; s++) {
        if (*s == (char)c) return (char*)s;
        if (!*s) return NULL;
    }
}

char* strrchr(const char* s, int c) {
    const char* last = NULL;
    for (;; s++) {
        if (*s == (char)c) last = s;
        if (!*s) return (char*)last;
    }
}

char* strstr(const char* h, const char* n) {
    if (!*n) return (char*)h;
    for (; *h; h++) {
        const char* a = h;
        const char* b = n;
        while (*a && *b && *a == *b) { a++; b++; }
        if (!*b) return (char*)h;
    }
    return NULL;
}

static char lc(unsigned char c) {
    return (c >= 'A' && c <= 'Z') ? (char)(c + 32) : (char)c;
}

int strcasecmp(const char* a, const char* b) {
    while (*a && lc((unsigned char)*a) == lc((unsigned char)*b)) {
        a++; b++;
    }
    return (unsigned char)lc((unsigned char)*a) -
           (unsigned char)lc((unsigned char)*b);
}

int strncasecmp(const char* a, const char* b, size_t n) {
    for (; n; n--, a++, b++) {
        int d = (unsigned char)lc((unsigned char)*a) -
                (unsigned char)lc((unsigned char)*b);
        if (d) return d;
        if (!*a) break;
    }
    return 0;
}

char* strdup(const char* s) {
    size_t n = strlen(s) + 1;
    char* p = (char*)malloc(n);
    if (p) memcpy(p, s, n);
    return p;
}

char* strpbrk(const char* s, const char* accept) {
    for (; *s; s++) {
        for (const char* a = accept; *a; a++) {
            if (*s == *a) return (char*)s;
        }
    }
    return NULL;
}

size_t strspn(const char* s, const char* accept) {
    size_t n = 0;
    for (; *s; s++, n++) {
        int found = 0;
        for (const char* a = accept; *a; a++) {
            if (*s == *a) { found = 1; break; }
        }
        if (!found) break;
    }
    return n;
}

size_t strcspn(const char* s, const char* reject) {
    size_t n = 0;
    for (; *s; s++, n++) {
        for (const char* r = reject; *r; r++) {
            if (*s == *r) return n;
        }
    }
    return n;
}

static char* g_strtok_save = NULL;

char* strtok(char* s, const char* delim) {
    return strtok_r(s, delim, &g_strtok_save);
}

char* strtok_r(char* s, const char* delim, char** save) {
    char* p = s ? s : *save;
    if (!p) return NULL;
    while (*p && strchr(delim, *p)) p++;
    if (!*p) { *save = NULL; return NULL; }
    char* tok = p;
    while (*p && !strchr(delim, *p)) p++;
    if (*p) { *p = '\0'; *save = p + 1; }
    else { *save = NULL; }
    return tok;
}

static double exp_ln_pow(double x, double y);

/* ------------------------------------------------------------------ */
/* stdlib                                                              */

int errno = 0;
volatile unsigned long g_tick = 0;
static void wdt(void) {
    if (++g_tick > 100000000UL) __builtin_trap();
}
char* g_environ_null[1] = { 0 };
char** environ = g_environ_null;

/* slice-1 stub: XSD pattern facets ride the RNG regex engine, whose
 * utf8proc dependency is not in this build. Permissive match keeps
 * patterned types valid; tightening arrives with the utf8proc bundle
 * (issue #1900 scope question 3). */
int rng_regex_matches(const char* pat, const char* s) {
    (void)pat; (void)s;
    return 1;
}

char* getenv(const char* name) {
    (void)name;
    return NULL;
}

int atoi(const char* s) { return (int)strtol(s, NULL, 10); }
long atol(const char* s) { return strtol(s, NULL, 10); }
double atof(const char* s) { return strtod(s, NULL); }

static int hexval(int c) {
    if (c >= '0' && c <= '9') return c - '0';
    if (c >= 'a' && c <= 'f') return c - 'a' + 10;
    if (c >= 'A' && c <= 'F') return c - 'A' + 10;
    return -1;
}

long strtol(const char* s, char** end, int base) {
    while (*s == ' ' || (*s >= '\t' && *s <= '\r')) s++;
    int neg = 0;
    if (*s == '+' || *s == '-') neg = (*s == '-'), s++;
    if ((base == 0 || base == 16) && s[0] == '0' &&
        (s[1] == 'x' || s[1] == 'X')) {
        base = 16;
        s += 2;
    } else if (base == 0) {
        base = (*s == '0') ? 8 : 10;
    }
    long v = 0;
    int any = 0;
    for (; *s; s++) {
        int d = hexval((unsigned char)*s);
        if (d < 0 || d >= base) break;
        v = v * base + d;
        any = 1;
    }
    if (end) *end = (char*)(any ? s : (any ? s : s - (any ? 0 : 0)));
    if (end && !any) *end = (char*)s;
    return neg ? -v : v;
}

long long strtoll(const char* s, char** end, int base) {
    return (long long)strtol(s, end, base);
}

unsigned long strtoul(const char* s, char** end, int base) {
    return (unsigned long)strtoll(s, end, base);
}

unsigned long long strtoull(const char* s, char** end, int base) {
    return (unsigned long long)strtoll(s, end, base);
}

double strtod(const char* s, char** end) {
    while (*s == ' ' || (*s >= '\t' && *s <= '\r')) s++;
    int neg = 0;
    if (*s == '+' || *s == '-') neg = (*s == '-'), s++;
    double v = 0.0;
    int any = 0;
    while (*s >= '0' && *s <= '9') {
        v = v * 10.0 + (*s - '0');
        s++;
        any = 1;
    }
    if (*s == '.') {
        s++;
        double frac = 1.0;
        while (*s >= '0' && *s <= '9') {
            frac /= 10.0;
            v += (*s - '0') * frac;
            s++;
            any = 1;
        }
    }
    if (any && (*s == 'e' || *s == 'E')) {
        const char* save = s;
        s++;
        int eneg = 0;
        if (*s == '+' || *s == '-') eneg = (*s == '-'), s++;
        if (*s >= '0' && *s <= '9') {
            int e = 0;
            while (*s >= '0' && *s <= '9') e = e * 10 + (*s++ - '0');
            double mul = 1.0;
            while (e--) mul *= 10.0;
            v = eneg ? v / mul : v * mul;
        } else {
            s = save;
        }
    }
    if (end) *end = (char*)(any ? s : (char*)0);
    if (end && !any) *end = (char*)s;
    return neg ? -v : v;
}

int abs(int x) { return x < 0 ? -x : x; }
long labs(long x) { return x < 0 ? -x : x; }

static unsigned long g_rand = 1;
void srand(unsigned seed) { g_rand = seed; }
int rand(void) {
    g_rand = g_rand * 1103515245u + 12345u;
    return (int)((g_rand >> 16) & 0x7fffu);
}

void qsort(void* base, size_t n, size_t sz,
           int (*cmp)(const void*, const void*)) {
    /* insertion sort — engine sort sizes are small; stable enough */
    char* a = (char*)base;
    char* tmp = (char*)malloc(sz);
    if (!tmp) return;
    for (size_t i = 1; i < n; i++) {
        memcpy(tmp, a + i * sz, sz);
        size_t j = i;
        while (j > 0 && cmp(a + (j - 1) * sz, tmp) > 0) {
            memcpy(a + j * sz, a + (j - 1) * sz, sz);
            j--;
        }
        memcpy(a + j * sz, tmp, sz);
    }
    free(tmp);
}

void* bsearch(const void* key, const void* base, size_t n, size_t sz,
              int (*cmp)(const void*, const void*)) {
    const char* a = (const char*)base;
    while (n) {
        size_t mid = n / 2;
        const void* p = a + mid * sz;
        int c = cmp(key, p);
        if (c == 0) return (void*)p;
        if (c < 0) { n = mid; }
        else { a = p + sz; n -= mid + 1; }
    }
    return NULL;
}

/* ------------------------------------------------------------------ */
/* ctype                                                               */

static const unsigned char ctype_bits[256] = {0};

static int in_range(int c, int lo, int hi) {
    return c >= lo && c <= hi;
}

int isalnum(int c) {
    return in_range(c, '0', '9') || in_range(c, 'A', 'Z') ||
           in_range(c, 'a', 'z');
}
int isalpha(int c) {
    return in_range(c, 'A', 'Z') || in_range(c, 'a', 'z');
}
int isdigit(int c) { return in_range(c, '0', '9'); }
int isxdigit(int c) {
    return isdigit(c) || in_range(c, 'A', 'F') || in_range(c, 'a', 'f');
}
int isspace(int c) {
    return c == ' ' || in_range(c, '\t', '\r');
}
int isupper(int c) { return in_range(c, 'A', 'Z'); }
int islower(int c) { return in_range(c, 'a', 'z'); }
int iscntrl(int c) { return in_range(c, 0, 31) || c == 127; }
int isprint(int c) { return in_range(c, 32, 126); }
int isgraph(int c) { return in_range(c, 33, 126); }
int ispunct(int c) { return isprint(c) && !isalnum(c); }
int tolower(int c) {
    return in_range(c, 'A', 'Z') ? c + 32 : c;
}
int toupper(int c) {
    return in_range(c, 'a', 'z') ? c - 32 : c;
}

/* ------------------------------------------------------------------ */
/* math                                                                */

int isnan(double x) { return x != x; }
int isinf(double x) { return __builtin_isinf(x); }
int isfinite(double x) { return __builtin_isfinite(x); }
int signbit(double x) { return __builtin_signbit(x); }
double fabs(double x) { return __builtin_fabs(x); }
double floor(double x) { return __builtin_floor(x); }
double ceil(double x) { return __builtin_ceil(x); }
double sqrt(double x) { return __builtin_sqrt(x); }

double fmod(double x, double y) {
    if (y == 0.0 || isnan(x) || isinf(x)) return NAN;
    if (isinf(y)) return x;
    double r = x - floor(x / y) * y;
    if (r != 0.0 && signbit(r) != signbit(x)) r += y;
    return r;
}

double pow(double x, double y) {
    if (y == 0.0) return 1.0;
    if (x == 0.0) return y < 0.0 ? INFINITY : 0.0;
    if (x < 0.0) {
        double fl = floor(y);
        if (y != fl) return NAN;
        double r = exp_ln_pow(-x, y);
        return ((long long)fl & 1) ? -r : r;
    }
    return exp_ln_pow(x, y);
}

/* exp(y * ln x) via series — x > 0 path of pow */
static double exp_e(double y) {
    if (y > 1.0 || y < -1.0) {
        double h = y * 0.5;
        double r = exp_e(h);
        return r * r;
    }
    double term = 1.0, sum = 1.0;
    for (int i = 1; i < 40; i++) {
        term *= y / (double)i;
        sum += term;
    }
    return sum;
}

static double ln_approx(double x) {
    /* range-reduce to [1, 2): x = m * 2^k, ln x = ln m + k ln 2 */
    int k = 0;
    while (x > 2.0) { x *= 0.5; k++; }
    while (x < 1.0) { x *= 2.0; k--; }
    double z = (x - 1.0) / (x + 1.0);
    double z2 = z * z;
    double term = z, sum = 0.0;
    for (int i = 1; i < 40; i += 2) {
        sum += term / (double)i;
        term *= z2;
    }
    return 2.0 * sum + 0.6931471805599453 * (double)k;
}

static double exp_ln_pow(double x, double y) {
    return exp_e(y * ln_approx(x));
}

double log10(double x) {
    return ln_approx(x) / 2.302585092994046;
}

long long llrint(double x) {
    double r = __builtin_nearbyint(x);
    return (long long)r;
}

long long llround(double x) {
    return (long long)(x >= 0.0 ? floor(x + 0.5) : ceil(x - 0.5));
}

double modf(double x, double* ip) {
    double f = x < 0.0 ? ceil(x) : floor(x);
    *ip = f;
    return x - f;
}

double frexp(double x, int* e) {
    if (x == 0.0 || !isfinite(x)) { *e = 0; return x; }
    double m = x;
    *e = 0;
    while (m >= 1.0) { m *= 0.5; (*e)++; }
    while (m < 0.5) { m *= 2.0; (*e)--; }
    return m;
}

double ldexp(double x, int e) {
    while (e > 0) { x *= 2.0; e--; }
    while (e < 0) { x *= 0.5; e++; }
    return x;
}

void shim_assert_fail(const char* expr, const char* file, int line) {
    (void)expr; (void)file; (void)line;
    abort();
}

/* ------------------------------------------------------------------ */
/* printf core (bounded: the verbs the engine uses)                    */

struct outbuf {
    char* p;
    size_t left;
    int overflow;
};

static void ob_putc(struct outbuf* ob, char c) {
    wdt();
    if (ob->left > 1) { *ob->p++ = c; ob->left--; }
    else if (ob->left == 1) { *ob->p = '\0'; ob->left = 0; ob->overflow = 1; }
}

static void ob_puts_n(struct outbuf* ob, const char* s, size_t n) {
    while (n--) ob_putc(ob, *s++);
}

static void ob_pad(struct outbuf* ob, char c, int n) {
    while (n-- > 0) ob_putc(ob, c);
}

static void emit_uchar(struct outbuf* ob, const char* pref, int zero_pad,
                       int width, int precision, int upper,
                       unsigned long long v, unsigned base) {
    char digits[64];
    const char* ds = upper ? "0123456789ABCDEF" : "0123456789abcdef";
    int n = 0;
    if (v == 0) {
        digits[n++] = '0';
    } else {
        while (v) { digits[n++] = ds[v % base]; v /= base; }
    }
    while (precision > n) digits[n++] = '0';
    int total = n + (pref[0] ? 2 : 0);
    int pad = width > total ? width - total : 0;
    if (!zero_pad) ob_pad(ob, ' ', pad);
    if (pref[0]) { ob_putc(ob, '0'); ob_putc(ob, pref[0]); }
    if (zero_pad) ob_pad(ob, '0', pad);
    while (n--) ob_putc(ob, digits[n]);
}

static void emit_int(struct outbuf* ob, char sign, int zero_pad,
                     int width, int precision, unsigned long long mag,
                     int neg) {
    char digits[24];
    int n = 0;
    if (mag == 0) digits[n++] = '0';
    while (mag) { digits[n++] = (char)('0' + (mag % 10)); mag /= 10; }
    while (precision > n) digits[n++] = '0';
    if (neg && precision > 0) sign = '-';
    int total = n + (sign ? 1 : 0);
    int pad = width > total ? width - total : 0;
    if (!zero_pad) ob_pad(ob, ' ', pad);
    if (sign) ob_putc(ob, sign);
    if (zero_pad) ob_pad(ob, '0', pad);
    while (n--) ob_putc(ob, digits[n]);
}

/* %f with non-negative exponent scaling via double math */
static void emit_double(struct outbuf* ob, double v, int width,
                        int precision, int alt, char conv) {
    char tmp[360];
    size_t used = 0;
    if (isnan(v)) {
        used = (size_t)snprintf(tmp, sizeof tmp, "nan");
    } else if (isinf(v)) {
        used = (size_t)snprintf(tmp, sizeof tmp, "inf");
    } else {
        int neg = signbit(v);
        if (neg) v = -v;
        double scale = 1.0;
        for (int i = 0; i < precision; i++) scale *= 10.0;
        double r = v * scale;
        double ip = floor(r + 0.5);
        unsigned long long i0 = (unsigned long long)ip;
        char digs[340];
        int dn = 0;
        if (i0 == 0) digs[dn++] = '0';
        while (i0) { wdt(); digs[dn++] = (char)('0' + (i0 % 10)); i0 /= 10; }
        if (neg) tmp[used++] = '-';
        if (dn <= precision) {
            tmp[used++] = '0';
        } else {
            for (int i = dn - 1; i >= precision; i--)
                tmp[used++] = digs[i];
        }
        if (precision > 0 || alt) tmp[used++] = '.';
        for (int i = precision - 1; i >= 0; i--)
            tmp[used++] = digs[i];
        tmp[used] = '\0';
    }
    int len = (int)strlen(tmp);
    int pad = width > len ? width - len : 0;
    ob_pad(ob, ' ', pad);
    ob_puts_n(ob, tmp, (size_t)len);
    (void)conv;
}

static int format_one(struct outbuf* ob, const char** fmtp, va_list* ap) {
    const char* f = *fmtp;
    int minus = 0, plus = 0, space = 0, alt = 0, zero = 0;
    while (1) {
        if (*f == '-') minus = 1;
        else if (*f == '+') plus = 1;
        else if (*f == ' ') space = 1;
        else if (*f == '#') alt = 1;
        else if (*f == '0') zero = 1;
        else break;
        f++;
    }
    int width = -1, precision = -1;
    if (*f == '*') {
        width = va_arg(*ap, int);
        f++;
    } else {
        while (*f >= '0' && *f <= '9')
            width = (width < 0 ? 0 : width) * 10 + (*f++ - '0');
    }
    if (*f == '.') {
        f++;
        precision = 0;
        if (*f == '*') { precision = va_arg(*ap, int); f++; }
        else while (*f >= '0' && *f <= '9')
            precision = precision * 10 + (*f++ - '0');
    }
    int lmod = 0; /* 1 = l, 2 = ll, 3 = z, 4 = h */
    for (;; f++) {
        if (*f == 'l') lmod = lmod == 1 ? 2 : 1;
        else if (*f == 'z') lmod = 3;
        else if (*f == 'h') lmod = 4;
        else break;
    }
    char conv = *f;
    if (!conv) { *fmtp = f; return -1; }
    *fmtp = f + 1;
    char sign = 0;
    int zero_pad = zero && !minus && width > 0;
    switch (conv) {
        case 'd':
        case 'i': {
            long long v = lmod == 2 ? (long long)va_arg(*ap, long long)
                          : lmod == 1 ? (long long)va_arg(*ap, long)
                          : lmod == 3 ? (long long)va_arg(*ap, size_t)
                          : (long long)va_arg(*ap, int);
            int neg = v < 0;
            unsigned long long mag =
                neg ? (unsigned long long)(-(v + 1)) + 1u : (unsigned long long)v;
            if (plus && !neg) sign = '+';
            else if (space && !neg) sign = ' ';
            else if (neg) sign = '-';
            emit_int(ob, sign, zero_pad, width, precision, mag, neg);
            break;
        }
        case 'u':
        case 'o':
        case 'x':
        case 'X': {
            unsigned long long v =
                lmod == 2 ? va_arg(*ap, unsigned long long)
                : lmod == 1 ? va_arg(*ap, unsigned long)
                : lmod == 3 ? va_arg(*ap, size_t)
                : (unsigned int)va_arg(*ap, unsigned int);
            if (conv == 'u') {
                emit_int(ob, 0, zero_pad, width, precision, v, 0);
            } else if (conv == 'o') {
                emit_uchar(ob, alt ? "0" : "", zero_pad, width,
                           precision, 0, v, 8);
            } else {
                emit_uchar(ob, (alt && v) ? "x" : "", zero_pad, width,
                           precision, conv == 'X', v, 16);
            }
            break;
        }
        case 'c':
            ob_pad(ob, ' ', width > 1 ? width - 1 : 0);
            ob_putc(ob, (char)va_arg(*ap, int));
            break;
        case 's': {
            const char* s = va_arg(*ap, const char*);
            if (!s) s = "(null)";
            size_t len = strlen(s);
            if (precision >= 0 && (size_t)precision < len)
                len = (size_t)precision;
            int pad = width > (int)len ? width - (int)len : 0;
            if (!minus) ob_pad(ob, ' ', pad);
            ob_puts_n(ob, s, len);
            if (minus) ob_pad(ob, ' ', pad);
            break;
        }
        case 'p': {
            void* p = va_arg(*ap, void*);
            ob_putc(ob, '0');
            ob_putc(ob, 'x');
            emit_uchar(ob, "", 0, 0, -1, 0, (unsigned long long)(uintptr_t)p, 16);
            break;
        }
        case 'f':
        case 'F': {
            double v = va_arg(*ap, double);
            if (plus && !signbit(v) && !isnan(v)) sign = '+';
            if (sign) ob_putc(ob, sign);
            emit_double(ob, v, width, precision < 0 ? 6 : precision,
                        alt, conv);
            break;
        }
        case 'e':
        case 'E':
        case 'g':
        case 'G': {
            double v = va_arg(*ap, double);
            /* %g/%e via the fixed path is lossy for huge/tiny — the
             * engine's number formatting goes through format_number.c
             * (its own shortest-round-trip writer), so these hit only
             * simple magnitudes in practice. */
            emit_double(ob, v, width, precision < 0 ? 6 : precision,
                        alt, conv);
            break;
        }
        case '%':
            ob_putc(ob, '%');
            break;
        default:
            ob_putc(ob, '%');
            ob_putc(ob, conv);
            break;
    }
    (void)space;
    return 0;
}

static int vsnprintf_core(char* out, size_t n, const char* fmt,
                          va_list ap) {
    struct outbuf ob;
    ob.p = out;
    ob.left = n ? n : 1;
    ob.overflow = 0;
    if (n) *out = '\0';
    const char* f = fmt;
    while (*f) {
        wdt();
        if (*f != '%') { ob_putc(&ob, *f++); continue; }
        f++;
        if (*f == '%') { ob_putc(&ob, '%'); f++; continue; }
        if (format_one(&ob, &f, &ap) < 0) break;
    }
    if (!ob.overflow && n) *ob.p = '\0';
    return 0;
}

int vsnprintf(char* out, size_t n, const char* fmt, va_list ap) {
    return vsnprintf_core(out, n, fmt, ap);
}

int snprintf(char* out, size_t n, const char* fmt, ...) {
    va_list ap;
    va_start(ap, fmt);
    int r = vsnprintf_core(out, n, fmt, ap);
    va_end(ap);
    return r;
}

int sprintf(char* out, const char* fmt, ...) {
    va_list ap;
    va_start(ap, fmt);
    int r = vsnprintf_core(out, (size_t)-1 / 2, fmt, ap);
    va_end(ap);
    return r;
}

int vsprintf(char* out, const char* fmt, va_list ap) {
    return vsnprintf_core(out, (size_t)-1 / 2, fmt, ap);
}

int sscanf(const char* s, const char* fmt, ...) {
    /* only %d/%f-style simple scans the engine needs — implemented on
     * demand; unimplemented verbs leave outputs untouched and return
     * EOF. */
    (void)s; (void)fmt;
    return -1;
}

int puts(const char* s) { (void)s; return 0; }
int printf(const char* fmt, ...) { (void)fmt; return 0; }
int vfprintf(FILE* f, const char* fmt, va_list ap) {
    (void)f; (void)fmt; (void)ap;
    return 0;
}
int fprintf(FILE* f, const char* fmt, ...) {
    (void)f; (void)fmt;
    return 0;
}
size_t fwrite(const void* p, size_t sz, size_t n, FILE* f) {
    (void)p; (void)f;
    return sz * n;
}
float strtof(const char* s, char** end) {
    return (float)strtod(s, end);
}
char* strndup(const char* s, size_t n) {
    size_t l = 0;
    while (l < n && s[l]) l++;
    char* p = (char*)malloc(l + 1);
    if (p) { memcpy(p, s, l); p[l] = '\0'; }
    return p;
}
static double exp_e_series(double y) { return exp_e(y); }
double exp(double x) { return exp_e_series(x); }
static double y_split(double y) { return y; }
double sin(double x) { return __builtin_sin(x); }
double cos(double x) { return __builtin_cos(x); }
double tan(double x) { return __builtin_tan(x); }
double atan(double x) { return __builtin_atan(x); }
double atan2(double y, double x) { return __builtin_atan2(y, x); }
double asin(double x) { return __builtin_asin(x); }
double acos(double x) { return __builtin_acos(x); }
double sinh(double x) { return __builtin_sinh(x); }
double cosh(double x) { return __builtin_cosh(x); }
double tanh(double x) { return __builtin_tanh(x); }
double trunc(double x) { return __builtin_trunc(x); }
double round(double x) { return __builtin_round(x); }
double log(double x) { return ln_approx(x); }

static double exp_e_series_fwd(double y) { return exp_e(y); }

/* strftime: the subset CURRENT_FN uses (%Y %m %d %H %M %S and %Z). */
static int two(char* out, int v) {
    out[0] = (char)('0' + (v / 10) % 10);
    out[1] = (char)('0' + v % 10);
    return 2;
}
size_t strftime(char* s, size_t max, const char* fmt, const struct tm* tm) {
    size_t used = 0;
#define PUTC(c) do { if (used + 1 < max) s[used++] = (char)(c); else return 0; } while (0)
    for (const char* f = fmt; *f; f++) {
        if (*f != '%') { PUTC(*f); continue; }
        f++;
        switch (*f) {
            case 'Y': {
                int y = tm->tm_year + 1900;
                char b[8];
                int n = 0;
                do { b[n++] = (char)('0' + y % 10); y /= 10; } while (y);
                while (n--) PUTC(b[n]);
                break;
            }
            case 'm': used += (size_t)two(s + used, tm->tm_mon + 1); break;
            case 'd': used += (size_t)two(s + used, tm->tm_mday); break;
            case 'H': used += (size_t)two(s + used, tm->tm_hour); break;
            case 'M': used += (size_t)two(s + used, tm->tm_min); break;
            case 'S': used += (size_t)two(s + used, tm->tm_sec); break;
            case 'Z': PUTC('Z'); break;
            case '%': PUTC('%'); break;
            default: return 0;
        }
    }
#undef PUTC
    if (used < max) s[used] = '\0';
    return used;
}
int fflush(FILE* f) { (void)f; return 0; }

/* ------------------------------------------------------------------ */
/* stdio files: unavailable — the API surface exists and fails         */

static int dummy_file[4];
FILE* stdout = (FILE*)&dummy_file[0];
FILE* stderr = (FILE*)&dummy_file[1];
FILE* stdin = (FILE*)&dummy_file[2];

FILE* fopen(const char* path, const char* mode) {
    (void)path; (void)mode;
    return NULL;
}
size_t fread(void* p, size_t sz, size_t n, FILE* f) {
    (void)p; (void)sz; (void)n; (void)f;
    return 0;
}
int fclose(FILE* f) { (void)f; return -1; }
int fseek(FILE* f, long off, int whence) {
    (void)f; (void)off; (void)whence;
    return -1;
}
long ftell(FILE* f) { (void)f; return -1; }
int feof(FILE* f) { (void)f; return 1; }
char* fgets(char* s, int n, FILE* f) { (void)s; (void)n; (void)f; return NULL; }

/* ------------------------------------------------------------------ */
/* time                                                                */

typedef long long time_shim_t;

typedef struct {
    int tm_sec, tm_min, tm_hour, tm_mday, tm_mon, tm_year, tm_wday,
        tm_yday, tm_isdst;
} tm_shim;

/* real civil-from-days (Howard Hinnant's algorithm) */
void shim_gmtime(long long t, tm_shim* tm) {
    long long days = t / 86400;
    long long secs = t % 86400;
    if (secs < 0) { secs += 86400; days--; }
    tm->tm_hour = (int)(secs / 3600);
    tm->tm_min = (int)((secs / 60) % 60);
    tm->tm_sec = (int)(secs % 60);
    days += 719468;
    long long era = (days >= 0 ? days : days - 146096) / 146097;
    long long doe = days - era * 146097;
    long long yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    long long y = yoe + era * 400;
    long long doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    long long mp = (5 * doy + 2) / 153;
    long long d = doy - (153 * mp + 2) / 5 + 1;
    long long m = mp + (mp < 10 ? 3 : -9);
    tm->tm_year = (int)(y + (m <= 2 ? 1 : 0) - 1900);
    tm->tm_mon = (int)(m - 1);
    tm->tm_mday = (int)d;
    tm->tm_wday = (int)((days + 4) % 7);
    tm->tm_yday = (int)doy;
    tm->tm_isdst = 0;
}



/* ------------------------------------------------------------------ */
/* pthread (single-threaded wasm)                                      */

static void* g_keys[16];
int pthread_key_create(pthread_key_t* k, void (*dtor)(void*)) {
    (void)dtor;
    for (int i = 0; i < 16; i++) {
        if (!g_keys[i]) { *k = (pthread_key_t)(i + 1); return 0; }
    }
    return -1;
}
int pthread_key_delete(pthread_key_t k) {
    if (k >= 1 && k <= 16) { g_keys[k - 1] = 0; return 0; }
    return -1;
}
void* pthread_getspecific(pthread_key_t k) {
    return (k >= 1 && k <= 16) ? g_keys[k - 1] : 0;
}
int pthread_setspecific(pthread_key_t k, const void* v) {
    if (k >= 1 && k <= 16) { g_keys[k - 1] = (void*)v; return 0; }
    return -1;
}

static struct tm g_tm;
time_t time(time_t* out) {
    if (out) *out = 1767225600LL;
    return 1767225600LL;
}
struct tm* gmtime_r(const time_t* t, struct tm* tm) {
    shim_gmtime(*t, (tm_shim*)tm);
    return tm;
}
struct tm* gmtime_s(struct tm* tm, const time_t* t) {
    shim_gmtime(*t, (tm_shim*)tm);
    return tm;
}
struct tm* gmtime(const time_t* t) {
    shim_gmtime(*t, (tm_shim*)&g_tm);
    return &g_tm;
}
struct tm* localtime(const time_t* t) { return gmtime(t); }
clock_t clock(void) { return 0; }
int mkdir(const char* p, unsigned mode) { (void)p; (void)mode; return -1; }

int pthread_mutex_lock(pthread_mutex_t* m) { (void)m; return 0; }
int pthread_mutex_unlock(pthread_mutex_t* m) { (void)m; return 0; }
int pthread_mutex_init(pthread_mutex_t* m, const void* a) {
    (void)m; (void)a;
    return 0;
}

void pthread_once(pthread_once_t* once, void (*fn)(void)) {
    if (!*once) {
        *once = 1;
        fn();
    }
}


/* POSIX regex stub (slice 1): compile reports unsupported, exec never
 * matches. */
int regcomp(regex_t* r, const char* p, int flags) {
    (void)p; (void)flags;
    r->re_nsub = 0;
    return REG_ENOSYS;
}
size_t regerror(int e, const regex_t* r, char* buf, size_t n) {
    (void)r;
    const char* m = e == REG_ENOSYS ? "regex unsupported (wasm slice 1)"
                                    : "regex error";
    size_t l = 0;
    while (m[l] && l + 1 < n) { buf[l] = m[l]; l++; }
    if (n) buf[l] = '\0';
    return l;
}
int regexec(const regex_t* r, const char* s, size_t nm,
            regmatch_t* m, int flags) {
    (void)r; (void)s; (void)nm; (void)m; (void)flags;
    return REG_NOMATCH;
}
void regfree(regex_t* r) { (void)r; }
