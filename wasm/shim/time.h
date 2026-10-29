#ifndef WASM_SHIM_TIME_H
#define WASM_SHIM_TIME_H
#include <stdint.h>
typedef long long time_t;
typedef long long clock_t;
#define CLOCKS_PER_SEC 1000000
struct tm {
    int tm_sec, tm_min, tm_hour, tm_mday, tm_mon, tm_year, tm_wday,
        tm_yday, tm_isdst;
};
time_t time(time_t *);
struct tm *gmtime_r(const time_t *, struct tm *);
struct tm *gmtime_s(struct tm *, const time_t *);
struct tm *gmtime(const time_t *);
struct tm *localtime(const time_t *);
clock_t clock(void);
size_t strftime(char *, size_t, const char *, const struct tm *);
#endif
