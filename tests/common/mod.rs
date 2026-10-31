//! Shared wasm-face test bodies (#1900). Included by the node and
//! browser test binaries via a macro so both runners exercise the
//! identical set.

#[macro_export]
macro_rules! wasm_face_tests {
    () => {
        #[wasm_bindgen_test]
        fn parse_xpath_roundtrip() {
            let doc = leptris::wasmface::WasmDocument::parse(
                "<root><item id=\"1\">alpha</item>\
                 <item id=\"2\">beta</item></root>",
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
                "<w:document xmlns:w=\"u\"><w:r><w:sz w:val=\"24\"/></w:r>\
                 </w:document>",
            )
            .expect("parse");
            let xml = doc.to_xml().unwrap();
            assert!(xml.contains("<w:sz w:val=\"24\"/>"), "xml: {xml}");
        }

        #[wasm_bindgen_test]
        fn parse_error_surfaces_status() {
            assert!(leptris::wasmface::WasmDocument::parse("<r>").is_err());
        }

        #[wasm_bindgen_test]
        fn node_handles_walk_and_write() {
            let doc = leptris::wasmface::WasmDocument::parse(
                "<root><a x='1'>t1</a><b/></root>",
            )
            .unwrap();
            let root = doc.root().expect("root");
            assert_eq!(root.name().as_deref(), Some("root"));
            let kids = root.children();
            assert_eq!(kids.len(), 2, "a + b in document order");
            assert_eq!(kids[0].name().as_deref(), Some("a"));
            assert_eq!(kids[0].text().as_deref(), Some("t1"));
            assert_eq!(
                kids[0].kind(),
                leptris::wasmface::WasmNodeKind::Element
            );

            // write face: build a document from scratch and serialize
            let built = leptris::wasmface::WasmDocument::parse("<w/>").unwrap();
            let top = built.root().expect("w");
            let child = built.create_element("kid").expect("kid");
            child.set_attribute("k", "v").expect("set attr");
            top.add_child(child).expect("attach");
            let text = built.create_text("hello").expect("text");
            top.add_child(text).expect("attach text");
            let xml = built.to_xml().unwrap();
            assert!(xml.contains("<kid k=\"v\"/>"), "xml: {xml}");
            assert!(xml.contains("hello"), "xml: {xml}");

            // xpath nodeset over handles (NODESET=0, NUMBER=2, BOOLEAN=1)
            let res = doc.xpath_full("//a").unwrap();
            assert_eq!(res.kind(), 0, "nodeset");
            assert_eq!(res.nodes().len(), 1);
            assert_eq!(res.nodes()[0].name().as_deref(), Some("a"));
            let count = doc.xpath_full("count(//*)").unwrap();
            assert_eq!(count.kind(), 2, "number");
            assert!((count.number() - 3.0).abs() < 1e-9);
            let flag = doc.xpath_full("count(//a) = 1").unwrap();
            assert_eq!(flag.kind(), 1, "boolean");
            assert!(flag.boolean());
        }

        #[wasm_bindgen_test]
        fn xsd_validate_roundtrip() {
            let schema = leptris::wasmface::compile_xsd(
                "<xs:schema xmlns:xs=\"http://www.w3.org/2001/XMLSchema\">\
                 <xs:element name=\"r\"><xs:complexType><xs:sequence>\
                 <xs:element name=\"e\" type=\"xs:string\"/></xs:sequence>\
                 </xs:complexType></xs:element></xs:schema>",
            )
            .expect("compile");
            let good = leptris::wasmface::WasmDocument::parse("<r><e>x</e></r>").unwrap();
            assert!(schema.validate_doc(&good).unwrap(), "valid instance");

            let bad = leptris::wasmface::WasmDocument::parse("<r><e>x</e><e>y</e></r>").unwrap();
            assert!(!schema.validate_doc(&bad).unwrap(), "invalid instance");
            let rows = schema.last_errors();
            assert!(!rows.is_empty(), "error rows must enumerate");
        }
    };
}
