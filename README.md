= leptris-rs

Rust bindings for link:https://github.com/leptris/leptris[libleptris], the pure
C99 XML 1.0 / XPath 1.0 / XSLT / XQuery engine with zero required runtime
dependencies.

This repository owns the publishable `leptris` crate (and its companion
`leptris-descriptor` crate). It is the standalone binding repository for the
engine — the same model as the
link:https://github.com/leptris/leptris-ruby[leptris-ruby] and
link:https://github.com/leptris/leptris-py[leptris-py] bindings: one engine,
many binding repositories, each with its own CI and releases.

== Workspace layout

[cols="1,3"]
|===
|Crate |Purpose

|`leptris`
|Safe wrapper over the engine's public C API: documents, elements,
XPath/XQuery evaluation, serialization. `build.rs` links the engine
through the `LEPTRIS_LIB_PATH` environment variable.

|`leptris-descriptor`
|`no_std` `#[repr(C)]` mirror of the engine's
`src/include/leptris/descriptor.h` ABI descriptors. With the `link`
feature it calls the engine directly and checks its layouts against
the engine's own introspection.
|===

== Mapping to the C engine

The crate is a binding, not a reimplementation. Rust types map onto the
engine's opaque handles (`LeptrisDocument`, `LeptrisElement`, ...) and every
operation is a thin safe wrapper over the corresponding `leptris_*` C
function. Memory ownership follows the engine contract: strings returned
from a document are document-owned and live until `leptris_document_free`;
the wrappers encode that lifetime in Rust types.

== Pointing the crate at an engine build

The crate does not build the C engine itself. It links a prebuilt shared
library, resolved in this order:

. `LEPTRIS_LIB_PATH` — full path to the shared library file, or a
  directory containing it.
. Fallback to the system linker search (`-lleptris`).

The simplest engine build for development:

....
cmake -B build-shared -S /path/to/leptris \
  -DCMAKE_BUILD_TYPE=Release \
  -DLEPTRIS_BUILD_SHARED=ON
cmake --build build-shared -j4
....

WARNING: The CMake option is `LEPTRIS_BUILD_SHARED`, *not* `BUILD_SHARED`.

Then run the crate against it:

....
LEPTRIS_LIB_PATH=build-shared/src cargo test
....

On macOS you may also need `DYLD_LIBRARY_PATH` pointing at the same
directory when running the test binaries.

== The descriptor layout gate

`leptris-descriptor` mirrors the `#[repr(C)]` layout of the engine's
descriptor structs. The test suite gates that mirror against the engine
itself: `tests/descriptor_abi.rs` and `descriptor/tests/layout.rs` compare
`core::mem::size_of` on the Rust side with the engine's
`leptris_plan_*_struct_size` introspection functions. If the engine changes
a descriptor layout, these tests fail until the mirror is updated — the
ABI never drifts silently.

== Relationship to bindings/rust in the engine repository

The engine repository keeps an in-tree Rust harness at `bindings/rust/`
for engine-side development. THIS repository (`leptris-rs`) owns the
crate going forward: publishable releases, binding-level issues, and
binding CI. Engine changes that affect the ABI surface are validated
here against real engine builds from `main`.

== CI

`.github/workflows/ci.yml` checks out the engine repository at `main`
into a sibling path, builds the shared library (utf8proc enabled,
`CMAKE_BUILD_TYPE=Release`), then runs `cargo build`, `cargo test` and
`cargo fmt --check` with `LEPTRIS_LIB_PATH` pointing at that build, on
an Ubuntu and macOS matrix.

== License

Same license as the engine — see the LICENSE file.
