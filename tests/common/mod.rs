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
        fn attribute_pairs_no_ns_after_namespaced() {
            // A no-namespace attribute AFTER a namespaced one used to
            // slice a lane to SIZE_MAX and trap (wasm `unreachable`).
            for xml in [
                "<a xmlns:p=\"urn:p\" p:x=\"1\" y=\"2\"/>",
                "<a y=\"2\" xmlns:p=\"urn:p\" p:x=\"1\"/>",
            ] {
                let doc = leptris::wasmface::WasmDocument::parse(xml).unwrap();
                let pairs = doc.root().expect("root").attribute_pairs().expect("pairs");
                let x = pairs.iter().find(|a| a.name() == "p:x").expect("p:x");
                let y = pairs.iter().find(|a| a.name() == "y").expect("y");
                assert_eq!(x.value(), "1", "{xml}");
                assert_eq!(x.prefix().as_deref(), Some("p"), "{xml}");
                assert_eq!(x.uri().as_deref(), Some("urn:p"), "{xml}");
                assert_eq!(y.value(), "2", "{xml}");
                assert_eq!(y.prefix(), None, "{xml}");
                assert_eq!(y.uri(), None, "{xml}");
            }
        }

        #[wasm_bindgen_test]
        fn attribute_pairs_mixed_three() {
            let doc = leptris::wasmface::WasmDocument::parse(
                "<a xmlns:p=\"urn:p\" xmlns:q=\"urn:q\" p:x=\"1\" y=\"2\" q:z=\"3\"/>",
            )
            .unwrap();
            let pairs = doc.root().expect("root").attribute_pairs().expect("pairs");
            let got: Vec<_> = pairs
                .iter()
                .filter(|a| !a.name().starts_with("xmlns"))
                .map(|a| (a.name(), a.value(), a.prefix(), a.uri()))
                .collect();
            let s = |v: &str| Some(v.to_string());
            assert_eq!(
                got,
                vec![
                    ("p:x".to_string(), "1".to_string(), s("p"), s("urn:p")),
                    ("y".to_string(), "2".to_string(), None, None),
                    ("q:z".to_string(), "3".to_string(), s("q"), s("urn:q")),
                ]
            );
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

        #[wasm_bindgen_test]
        fn attribute_single_read() {
            let doc = leptris::wasmface::WasmDocument::parse(
                "<root id=\"r1\" w:val=\"7\" xmlns:w=\"u\"/>",
            )
            .expect("parse");
            let root = doc.root().expect("root");
            assert_eq!(root.attribute("id").as_deref(), Some("r1"));
            assert_eq!(root.attribute("w:val").as_deref(), Some("7"));
            assert_eq!(root.attribute("absent"), None);
        }

        #[wasm_bindgen_test]
        fn element_rows_preorder_batches_and_resume() {
            let doc =
                leptris::wasmface::WasmDocument::parse("<a id=\"top\"><b><c/></b><d>text</d></a>")
                    .expect("parse");
            let root = doc.root().expect("root");
            // whole subtree in one budget
            let all = root.element_rows(0, 10).expect("batch");
            assert!(all.is_done());
            assert_eq!(all.rows().len(), 4); // a, b, c, d
            assert_eq!(all.rows()[0].name(), "a");
            assert_eq!(all.rows()[0].depth(), 0);
            assert_eq!(all.rows()[0].attrs().len(), 1);
            assert_eq!(all.rows()[0].attrs()[0].name(), "id");
            assert_eq!(all.rows()[3].name(), "d");
            assert_eq!(all.rows()[3].text(), "text");
            // budgeted: first batch of 2, resume, exhaust
            let b1 = root.element_rows(0, 2).expect("b1");
            assert_eq!(b1.rows().len(), 2);
            assert!(!b1.is_done());
            assert_eq!(b1.next_cursor(), 2);
            let b2 = root.element_rows(b1.next_cursor(), 2).expect("b2");
            assert_eq!(b2.rows().len(), 2);
            assert!(b2.is_done());
            // a cursor past the end is an empty done batch
            let b3 = root.element_rows(99, 2).expect("b3");
            assert!(b3.rows().is_empty());
            assert!(b3.is_done());
        }

        #[wasm_bindgen_test]
        fn create_comment_cdata_pi_and_add_child() {
            use leptris::wasmface::{WasmDocument, WasmNodeKind};
            let doc = WasmDocument::parse("<r/>").unwrap();
            let root = doc.root().expect("root");
            let c = root
                .add_child(doc.create_comment(" note ").unwrap())
                .unwrap();
            let d = root.add_child(doc.create_cdata("a<b&c").unwrap()).unwrap();
            let p = root
                .add_child(doc.create_pi("xml-stylesheet", "href=\"s.css\"").unwrap())
                .unwrap();
            assert_eq!(c.kind(), WasmNodeKind::Comment);
            assert_eq!(d.kind(), WasmNodeKind::Cdata);
            assert_eq!(p.kind(), WasmNodeKind::Pi);
            assert_eq!(root.children().len(), 3);
            let xml = doc.to_xml().unwrap();
            assert!(xml.contains("<!-- note -->"), "xml: {xml}");
            assert!(xml.contains("<![CDATA[a<b&c]]>"), "xml: {xml}");
            assert!(
                xml.contains("<?xml-stylesheet href=\"s.css\"?>"),
                "xml: {xml}"
            );
        }

        #[wasm_bindgen_test]
        fn remove_attribute_reports_presence() {
            let doc = leptris::wasmface::WasmDocument::parse(
                "<r a=\"1\" b=\"2\" xmlns:n=\"urn:n\" n:c=\"3\"/>",
            )
            .unwrap();
            let root = doc.root().expect("root");
            assert!(root.remove_attribute("a").unwrap());
            assert!(!root.remove_attribute("a").unwrap(), "already gone");
            assert!(!root.remove_attribute("nope").unwrap());
            assert!(root.remove_attribute("n:c").unwrap(), "spelled name");
            let names: Vec<String> = root
                .attribute_pairs()
                .unwrap()
                .iter()
                .map(|a| a.name())
                .collect();
            assert_eq!(names, vec!["b".to_string()]);
            let xml = doc.to_xml().unwrap();
            assert!(
                !xml.contains("a=\"1\"") && !xml.contains("n:c"),
                "xml: {xml}"
            );
        }

        #[wasm_bindgen_test]
        fn parse_error_carries_position() {
            use leptris::wasmface::{last_parse_error, WasmDocument};
            assert!(WasmDocument::parse("<r>\n <a>\n  </b></r>").is_err());
            let e = last_parse_error().expect("engine recorded the failure");
            assert_eq!((e.line(), e.column()), (3, 7), "{}", e.message());
            assert!(e.message().contains("mismatch"), "{}", e.message());
        }

        #[wasm_bindgen_test]
        fn parse_with_options_applies_max_depth() {
            use leptris::wasmface::{WasmDocument, WasmParseOptions};
            let deep = format!("{}{}", "<a>".repeat(20), "</a>".repeat(20));
            assert!(WasmDocument::parse(&deep).is_ok());
            let mut opts = WasmParseOptions::new();
            opts.max_depth = 5;
            assert!(WasmDocument::parse_with_options(&deep, &opts).is_err());
            opts.max_depth = 0;
            let doc = WasmDocument::parse_with_options("<r><x/></r>", &opts).unwrap();
            assert_eq!(doc.xpath_count("//x").unwrap(), 1);
        }

        #[wasm_bindgen_test]
        fn node_xpath_uses_the_element_as_context() {
            let doc = leptris::wasmface::WasmDocument::parse(
                "<r xmlns:n=\"urn:n\"><n:a x=\"1\"/><n:a x=\"2\"/><b/></r>",
            )
            .unwrap();
            let kids = doc.root().expect("root").children();
            assert_eq!(kids[1].xpath("string(@x)").unwrap(), "2");
            assert_eq!(kids[1].xpath("namespace-uri(.)").unwrap(), "urn:n");
            assert_eq!(kids[2].xpath("name(.)").unwrap(), "b");
        }
    };
}
