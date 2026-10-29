# wasm32 probe kit (leptris#1900 slice 1)

The engine compiles and RUNS under `wasm32-unknown-unknown` with this
bundled freestanding libc shim. Proven 2026-10-29: parse, XPath eval,
and string/number result materialization under Node 24 —
`name(/*)` -> "root", `count(//item)` -> "2", `1+1` -> "2",
`concat(//item[1], "-", //item[2])` -> "alpha-beta".

Build (ENGINE points at any engine tree, e.g. a release-tarball
unpack):

    ENGINE=~/src/leptris/wt/1874-ns-bulk ./build_probe.sh
    node run11.js

Excluded from the wasm build (slice-1 scope): iconv, utf8proc/unicode,
the AVX2/NEON SIMD TUs (scalar paths serve), rng/ (pattern facets run
permissive — `rng_regex_matches` stub in the shim). POSIX regex is a
stub (regcomp reports unsupported). time() returns a fixed epoch.
File IO fails cleanly. getenv returns NULL.

## The three wasm-bare-metal traps this kit encodes

1. **Allocator must bump from `__heap_base`** — wasm-ld reserves the
   `-z stack-size` region BELOW `__heap_base`; allocating from the
   current memory top hands out stack pages and corrupts every frame.
2. **Coalescing requires ADDRESS adjacency** — the freelist chain is
   not address-ordered (grown pages prepend), so merging chain
   neighbors without physical adjacency corrupts the heap.
3. **`__stack_pointer` is uninitialized** without a crt0 — set it to
   `__heap_base` from the host before the first export call.

Debugging law: `node --cpu-prof` attributes wasm frames to the calling
JS function; `sample(1)` on a hung Node only shows JIT blobs. Watchdog
traps (`__builtin_trap` past a tick bound) produce named wasm
RuntimeErrors.
