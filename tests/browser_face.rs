//! Browser runner entry (wasm-pack test --headless --chrome):
//! the #1900 acceptance's "in a browser page" leg.

#![cfg(all(target_arch = "wasm32", feature = "wasm"))]

use wasm_bindgen_test::wasm_bindgen_test;
use wasm_bindgen_test::wasm_bindgen_test_configure;

wasm_bindgen_test_configure!(run_in_browser);

mod common;
crate::wasm_face_tests!();
