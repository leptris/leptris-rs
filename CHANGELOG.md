# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Other

- The XPath result kind enum is 0-based (fixes #19) by @[object]
- Export the XSD validate face (leptris-rs#15 item 4) by @[object]

### Added

- Node-handle walk/write faces + full XPath result shape ([#15](https://github.com/leptris/leptris-rs/pull/15)) by @[object]

### Other

- Cargo fmt by @[object]
- Record the wasm size budget in the release body via the API by @[object]

### Added

- Ns_prefix row fields for plan-path serialization by @[object]
- PLAN_KIND_WILDCARD = 7 by @[object]
- Bootstrap leptris-rs — standalone Rust binding repository by @[object]

### Fixed

- Grant id-token: write for crates.io trusted publishing by @[object]
- The Rust repo uses README.md by @[object]
- Point the readme manifest field at README.adoc by @[object]
- Descriptor_abi literals carry the ns_prefix field by @[object]

### Other

- Attach the wasm bundle + size budget to GitHub releases (leptris#1900 slice 3) by @[object]
- Browser leg — shared test bodies, node + headless-chrome runners by @[object]
- Single-allocator fix — C malloc routes through std::alloc by @[object]
- XSD validate face + test — known-issue on the wasm-pack runner by @[object]
- Rustfmt by @[object]
- Wasm([#1900](https://github.com/leptris/leptris-rs/pull/1900)) slice 2: wasm-bindgen face — engine compiled into the crate by @[object]
- Slice-1 probe kit — engine runs on wasm32-unknown-unknown ([#1900](https://github.com/leptris/leptris-rs/pull/1900)) by @[object]
- Safe descriptor builder + walk lane (leptris-ruby#294) by @[object]
- Convert the readme to Markdown by @[object]
- Release-plz publish pipeline with crates.io trusted publishing by @[object]
- Trusted-publishing workflow for crates.io by @[object]
