//! #294's safe crate shape: a builder that owns its CStrings until
//! plan build, a walk whose result frees on Drop, and value readers
//! that borrow the walk — no manual pointer plumbing at the call
//! site (tests/descriptor_abi.rs stays as the raw-ABI gate).

use leptris_descriptor::safe::{AttrRow, NodeSpec};
use std::ffi::c_char;
use std::time::Instant;

// The linking crate: cargo only applies the leptris build script's
// -lleptris directives when this crate is actually used by the
// binary (same anchor as tests/descriptor_abi.rs).
#[allow(dead_code)]
fn link_anchor() {
    let _ = leptris::Document::parse;
}

#[repr(C)]
struct DocH {
    _p: [u8; 0],
}
#[repr(C)]
struct ElemH {
    _p: [u8; 0],
}

extern "C" {
    fn leptris_parse_string(src: *const c_char, len: usize, status: *mut i32) -> *mut DocH;
    fn leptris_document_free(doc: *mut DocH);
    fn leptris_document_root(doc: *mut DocH) -> *mut ElemH;
}

struct Doc {
    raw: *mut DocH,
}

impl Doc {
    fn parse(xml: &[u8]) -> Doc {
        unsafe {
            let mut st: i32 = 0;
            let p = leptris_parse_string(xml.as_ptr() as *const c_char, xml.len(), &mut st);
            assert_eq!(st, 0, "parse status");
            assert!(!p.is_null());
            Doc { raw: p }
        }
    }
    fn root_handle(&self) -> *mut ElemH {
        unsafe { leptris_document_root(self.raw) }
    }
}

impl Drop for Doc {
    fn drop(&mut self) {
        unsafe { leptris_document_free(self.raw) }
    }
}

const TYPE_ID: u8 = 1;
const TYPE_TS: u8 = 2;
const TYPE_NAME: u8 = 3;
const TYPE_TAGS: u8 = 4;
const TYPE_ROW: u8 = 5;

fn spike_spec() -> NodeSpec {
    NodeSpec::new("iso").nested(
        "row",
        NodeSpec::new("row")
            .tag(TYPE_ROW)
            .attr("id", TYPE_ID)
            .attr("ts", TYPE_TS)
            .scalar("name", TYPE_NAME)
            .collection("tags", TYPE_TAGS),
    )
}

#[test]
fn safe_builder_walks_the_spike_shape() {
    let descriptor = spike_spec().build().expect("descriptor build");
    // The #294 spike scenario, scaled: 5000 <row> children under
    // the iso root, each with id/ts attributes, a name scalar, and
    // a tags collection.
    let mut xml = String::from("<iso>");
    for i in 0..5000 {
        xml.push_str(&format!(
            "<row id=\"{}\" ts=\"t{}\"><name>n{}</name><tags>a</tags><tags>b</tags></row>",
            i + 1,
            i,
            i
        ));
    }
    xml.push_str("</iso>");
    let doc = Doc::parse(xml.as_bytes());

    let walk = descriptor
        .walk(doc.raw as _, doc.root_handle() as _)
        .expect("walk");
    // The root value carries no row wire_name or tag (both come
    // from the CHILD row that bound it; the root is plans[0]).
    assert_eq!(walk.len(), 5000);
    assert_eq!(walk.name(), None);
    assert_eq!(walk.type_tag(), 0);

    let mut id_sum: u64 = 0;
    for i in 0..walk.len() {
        let row = walk.at(i).expect("row value");
        assert_eq!(row.type_tag(), TYPE_ROW);
        id_sum += row
            .attribute("id")
            .expect("id attr")
            .parse::<u64>()
            .expect("id numeric");
        assert_eq!(row.attribute("ts"), Some(format!("t{i}").as_str()));
        // len counts BOUND ROWS (name + tags), not XML instances;
        // each <tags> element is a member of the collection value.
        assert_eq!(row.len(), 2);
        let name_v = row.at(0).expect("name value");
        assert_eq!(name_v.name(), Some("name"));
        assert_eq!(name_v.string(), Some(format!("n{i}").as_str()));
        assert_eq!(name_v.type_tag(), TYPE_NAME);
        let tags = row.at(1).expect("tags value");
        assert_eq!(tags.name(), Some("tags"));
        assert_eq!(tags.type_tag(), TYPE_TAGS);
        assert_eq!(tags.len(), 2);
    }
    assert_eq!(id_sum, 5000 * 5001 / 2, "the spike's id-sum invariant");
}

#[test]
fn walk_value_borrow_and_drop_safety() {
    // Engine memory frees with the walk: values are readable while
    // the walk lives (below) and a `&str` can never outlive it (the
    // borrow checker rejects the escape at compile time — the
    // sanctioned escape is an owned copy).
    let descriptor = NodeSpec::new("r")
        .nested("a", NodeSpec::new("a").tag(6).scalar("t", 7))
        .build()
        .unwrap();
    let doc = Doc::parse(b"<r><a><t>x</t></a></r>");
    let owned: String;
    {
        let walk = descriptor
            .walk(doc.raw as _, doc.root_handle() as _)
            .unwrap();
        let a = walk.at(0).unwrap();
        let t = a.at(0).unwrap();
        assert_eq!(t.string(), Some("x"));
        assert_eq!(t.type_tag(), 7);
        owned = t.string().unwrap().to_string();
        assert_eq!(a.name(), Some("a"));
        assert_eq!(a.type_tag(), 6);
    } // walk dropped here: the result frees, no borrowed &str remains
    assert_eq!(owned, "x");
}

#[test]
fn attr_row_and_ns_forms_pass_through() {
    let spec = NodeSpec::new("catalog")
        .attr_row(AttrRow::new("id", 9).predicate("b1"))
        .nested(
            "book",
            NodeSpec::new("book")
                .tag(8)
                .ns_exact("urn:c", Some("c"))
                .scalar("title", 3),
        );
    let descriptor = spec.build().unwrap();
    let doc = Doc::parse(
        b"<catalog id='b1'><c:book xmlns:c='urn:c'><c:title>T</c:title></c:book></catalog>",
    );
    let walk = descriptor
        .walk(doc.raw as _, doc.root_handle() as _)
        .unwrap_or_else(|st| panic!("walk status {st}"));
    assert_eq!(walk.len(), 1);
    let book = walk.at(0).unwrap();
    assert_eq!(book.name(), Some("book"));
    assert_eq!(book.type_tag(), 8);
    assert_eq!(book.at(0).unwrap().name(), Some("title"));
    assert_eq!(book.at(0).unwrap().string(), Some("T"));
}

#[test]
fn spike_perf_median_is_printed() {
    // #294's headline: the marshalling layer is the gap. Same
    // fixture shape as the spike (688KB, 5000 rows) — ten runs,
    // median printed; correctness is asserted, timing is
    // informational (CI noise makes asserts on it flaky).
    let descriptor = spike_spec().build().unwrap();
    let mut xml = String::from("<iso>");
    for i in 0..10_000 {
        xml.push_str(&format!(
            "<row id=\"{}\" ts=\"t{}\"><name>n{}</name><tags>a</tags></row>",
            i + 1,
            i,
            i
        ));
    }
    xml.push_str("</iso>");
    assert!(xml.len() > 600_000, "fixture must stay ~spike scale");

    let mut samples = Vec::new();
    for _ in 0..10 {
        let doc = Doc::parse(xml.as_bytes());
        let t0 = Instant::now();
        let walk = descriptor
            .walk(doc.raw as _, doc.root_handle() as _)
            .unwrap();
        let mut n = 0usize;
        for i in 0..walk.len() {
            n += walk.at(i).unwrap().attribute("id").unwrap().len();
        }
        let dt = t0.elapsed();
        assert_eq!(n, 38_894, "id column bytes (digit sum of 1..=10000)");
        assert_eq!(walk.len(), 10_000);
        samples.push(dt.as_millis() as u64);
    }
    samples.sort();
    println!("plan walk median over 10 runs: {}ms", samples[5]);
}
