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
            let doc =
                leptris::wasmface::WasmDocument::parse("<root><a x='1'>t1</a><b/></root>").unwrap();
            let root = doc.root().expect("root");
            assert_eq!(root.name().as_deref(), Some("root"));
            let kids = root.children();
            assert_eq!(kids.len(), 2, "a + b in document order");
            assert_eq!(kids[0].name().as_deref(), Some("a"));
            assert_eq!(kids[0].text().as_deref(), Some("t1"));
            assert_eq!(kids[0].kind(), leptris::wasmface::WasmNodeKind::Element);

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

        #[wasm_bindgen_test]
        fn namespace_reads() {
            let doc = leptris::wasmface::WasmDocument::parse(
                "<root xmlns:ns=\"urn:n\"><ns:child>t</ns:child><plain/></root>",
            )
            .unwrap();
            let root = doc.root().expect("root");
            assert!(root.namespace_uri().is_none(), "no-ns root");
            let kids = root.children();
            assert_eq!(kids[0].namespace_uri().as_deref(), Some("urn:n"));
            assert_eq!(kids[0].prefix().as_deref(), Some("ns"));
            assert_eq!(kids[1].namespace_uri(), None);
            assert_eq!(kids[1].prefix(), None);
        }

        #[wasm_bindgen_test]
        fn attribute_pairs_lanes() {
            let doc = leptris::wasmface::WasmDocument::parse(
                "<root a=\"1\" ns:b=\"2\" xmlns:ns=\"urn:n\"/>",
            )
            .unwrap();
            let pairs = doc.root().expect("root").attribute_pairs().expect("pairs");
            assert_eq!(pairs.len(), 2);
            assert_eq!(pairs[0].name(), "a");
            assert_eq!(pairs[0].value(), "1");
            assert_eq!(pairs[0].uri(), None, "no-namespace attr has no uri lane");
            // engine-qualified name; prefix/uri lanes carry the split
            assert_eq!(pairs[1].name(), "ns:b");
            assert_eq!(pairs[1].prefix().as_deref(), Some("ns"));
            assert_eq!(pairs[1].uri().as_deref(), Some("urn:n"));
        }

        #[wasm_bindgen_test]
        fn capabilities_describe_engine() {
            let caps = leptris::wasmface::engine_capabilities();
            assert!(caps.caps() > 0.0, "engine advertises bits");
            assert_eq!(caps.descriptor_version(), 1);
            assert!(caps.serves(1.0), "bit 0 must be served by any engine");
        }

        #[wasm_bindgen_test]
        fn parse_html_whatwg() {
            let doc = leptris::wasmface::parse_html("<p>hello<br>world</p>").unwrap();
            let xml = doc.to_xml().unwrap();
            assert!(xml.to_lowercase().contains("hello"), "text kept: {xml}");
            assert!(doc.root_name().is_some());
        }

        #[wasm_bindgen_test]
        fn schema_set_through_vfs() {
            const A: &str = "<xs:schema xmlns:xs=\"http://www.w3.org/2001/XMLSchema\" \
                 targetNamespace=\"urn:a\" xmlns=\"urn:a\">\
                 <xs:element name=\"ea\" type=\"xs:string\"/></xs:schema>";
            const B: &str = "<xs:schema xmlns:xs=\"http://www.w3.org/2001/XMLSchema\" \
                 targetNamespace=\"urn:b\" xmlns:b=\"urn:b\" xmlns:a=\"urn:a\">\
                 <xs:import namespace=\"urn:a\" schemaLocation=\"set-a.xsd\"/>\
                 <xs:element name=\"eb\"><xs:complexType><xs:sequence>\
                 <xs:element ref=\"a:ea\"/></xs:sequence></xs:complexType>\
                 </xs:element></xs:schema>";
            leptris::wasmface::register_file(
                "/virtual/set-a.xsd".to_string(),
                A.as_bytes().to_vec(),
            )
            .expect("register a");
            leptris::wasmface::register_file(
                "/virtual/set-b.xsd".to_string(),
                B.as_bytes().to_vec(),
            )
            .expect("register b");
            let schema = leptris::wasmface::compile_xsd_set(vec![
                "/virtual/set-b.xsd".to_string(),
                "/virtual/set-a.xsd".to_string(),
            ])
            .expect("set compile through vfs");
            let good = leptris::wasmface::WasmDocument::parse(
                "<b:eb xmlns:b=\"urn:b\" xmlns:a=\"urn:a\"><a:ea>x</a:ea></b:eb>",
            )
            .unwrap();
            assert!(
                schema.validate_doc(&good).unwrap(),
                "imported set validates"
            );
        }

        #[wasm_bindgen_test]
        fn loud_failures() {
            // unregistered path: the face refuses before the engine runs
            assert!(
                leptris::wasmface::compile_xsd_set(vec!["/virtual/nope.xsd".to_string()]).is_err(),
                "unregistered path must fail"
            );
            // broken schema: error-carrying-handle contract — status is
            // authoritative, so compile raises instead of returning a
            // degraded schema
            assert!(leptris::wasmface::compile_xsd(
                "<xs:schema xmlns:xs=\"http://www.w3.org/2001/XMLSchema\">\
                 <xs:element name=\"x\"><xs:complexType><xs:sequence>\
                 <xs:element ref=\"nope\"/></xs:sequence></xs:complexType>\
                 </xs:element></xs:schema>",
            )
            .is_err());
        }
    };
}
