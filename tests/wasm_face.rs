//! #1900 slice 2: wasm-bindgen face round trip (wasm-pack test).

#![cfg(all(target_arch = "wasm32", feature = "wasm"))]

use wasm_bindgen_test::wasm_bindgen_test;

#[wasm_bindgen_test]
fn parse_xpath_roundtrip() {
    let doc = leptris::wasmface::WasmDocument::parse(
        "<root><item id=\"1\">alpha</item><item id=\"2\">beta</item></root>",
    )
    .expect("parse");
    assert_eq!(doc.root_name().as_deref(), Some("root"));
    assert_eq!(doc.xpath_count("//item").unwrap(), 2);
    assert_eq!(doc.xpath("//item[1]").unwrap(), "alpha");
    assert_eq!(doc.xpath("count(//item)").unwrap(), "2");
    assert_eq!(doc.xpath("1+1").unwrap(), "2");
}

#[wasm_bindgen_test]
fn serialize_roundtrip_keeps_attributes() {
    let doc = leptris::wasmface::WasmDocument::parse(
        "<w:document xmlns:w=\"u\"><w:r><w:sz w:val=\"24\"/></w:r></w:document>",
    )
    .expect("parse");
    let xml = doc.to_xml().unwrap();
    assert!(xml.contains("<w:sz w:val=\"24\"/>"), "xml: {xml}");
}

#[wasm_bindgen_test]
fn parse_error_surfaces_status() {
    assert!(leptris::wasmface::WasmDocument::parse("<r>").is_err());
}
