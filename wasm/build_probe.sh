#!/bin/bash
# leptris#1900 slice-1 probe: compile the engine for wasm32-unknown-unknown
# with the bundled libc shim and link a C probe face. Proven working
# 2026-10-29 (parse + xpath eval + string/number results under Node).
set -euo pipefail
cd "$(dirname "$0")"
ENGINE=${1:-$HOME/src/leptris/wt/1874-ns-bulk}
WASMLD=${WASMLD:-$(ls ~/.rustup/toolchains/*/lib/rustlib/*/bin/gcc-ld/wasm-ld | head -1)}
OUT=.
rm -f "$OUT"/*.o
for f in $(find "$ENGINE/src/leptris" -name "*.c"); do
  case "$f" in
    */encoding/encoding.c|*/unicode/unicode.c|*simd_text_avx2.c|*simd_text_neon.c|*/rng/*) continue;;
  esac
  obj=$(echo "$f" | sed "s|/|_|g; s|src_leptris_||; s|\.c$|.o|")
  clang --target=wasm32-unknown-unknown -O1 -g -ffreestanding \
    -DLEPTRIS_ENABLE_UTF8PROC=0 -DLEPTRIS_ENABLE_ICONV=0 \
    -DLEPTRIS_VERSION='"1.9.389"' \
    -I"$PWD/shim" -I"$ENGINE/src/include" -I"$ENGINE/src" \
    -I"$ENGINE/src/leptris/dom" -I"$ENGINE/src/leptris/parse" \
    -I"$ENGINE/src/leptris/serialize" -I"$ENGINE/src/leptris/xpath" \
    -I"$ENGINE/src/leptris/encoding" -I"$ENGINE/src/leptris/memory" \
    -I"$ENGINE/src/leptris/flat" \
    -c "$f" -o "$OUT/$obj"
done
clang --target=wasm32-unknown-unknown -O1 -g -ffreestanding \
  -I"$PWD/shim" -I"$ENGINE/src/include" -c main.c -o "$OUT/_main.o"
clang --target=wasm32-unknown-unknown -O1 -g -ffreestanding \
  -I"$PWD/shim" -c shim/shim.c -o "$OUT/_shim.o"
"$WASMLD" --no-entry --export-dynamic \
  --export=__heap_base --export=__stack_pointer --export=malloc --export=free \
  -z stack-size=1048576 -o leptris.wasm \
  $(ls "$OUT"/*.o | grep -vE "_(main|shim)\.o" | tr '\n' ' ') "$OUT/_shim.o" "$OUT/_main.o"
echo "leptris.wasm: $(du -h leptris.wasm | cut -f1)"
