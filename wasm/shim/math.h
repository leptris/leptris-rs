#ifndef WASM_SHIM_MATH_H
#define WASM_SHIM_MATH_H
#define HUGE_VAL (__builtin_huge_val())
#define NAN (__builtin_nan(""))
#define INFINITY (__builtin_inf())
#define M_PI 3.14159265358979323846
int isnan(double);
int isinf(double);
int signbit(double);
int isfinite(double);
double fabs(double);
double floor(double);
double ceil(double);
double fmod(double, double);
double pow(double, double);
double sqrt(double);
double modf(double, double *);
double frexp(double, int *);
double ldexp(double, int);
double log10(double);
double log(double);
double exp(double);
double sin(double);
double cos(double);
double tan(double);
double atan(double);
double atan2(double, double);
double asin(double);
double acos(double);
double sinh(double);
double cosh(double);
double tanh(double);
double trunc(double);
double round(double);
double log(double);
long long llrint(double);
long long llround(double);
#endif
