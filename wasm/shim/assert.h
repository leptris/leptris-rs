#ifndef WASM_SHIM_ASSERT_H
#define WASM_SHIM_ASSERT_H
void shim_assert_fail(const char *, const char *, int);
#define assert(x) ((x) ? (void)0 : shim_assert_fail(#x, __FILE__, __LINE__))
#endif
