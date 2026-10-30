//! Node runner entry (wasm-pack test --node).

#![cfg(all(target_arch = "wasm32", feature = "wasm"))]

use wasm_bindgen_test::wasm_bindgen_test;

mod common;
crate::wasm_face_tests!();
