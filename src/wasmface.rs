//! wasm-bindgen face over the compiled-in engine (leptris#1900 slice 2).
//!
//! Only compiled for wasm32 + the `wasm` feature: the engine is linked
//! statically by build.rs (wasm/shim/ supplies the freestanding libc),
//! so the raw C ABI is callable directly — no dlopen layer.

#![allow(non_snake_case)]

use crate::ffi;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};
use wasm_bindgen::prelude::*;

extern "C" {
    fn shim_fs_register(path: *const c_char, data: *const u8, len: usize) -> c_int;
    fn shim_fs_has(path: *const c_char) -> c_int;
    fn shim_fs_clear();
}

/// Register file contents under a virtual path so the engine's
/// path-based APIs (xsd compile set, schemaLocation imports) can read
/// them inside the freestanding wasm environment. Re-registering the
/// same path replaces its contents.
#[wasm_bindgen]
pub fn register_file(path: String, bytes: Vec<u8>) -> Result<(), JsError> {
    let cpath =
        CString::new(path.as_str()).map_err(|_| JsError::new("path contains a NUL byte"))?;
    let rc = unsafe { shim_fs_register(cpath.as_ptr(), bytes.as_ptr(), bytes.len()) };
    if rc != 0 {
        return Err(JsError::new("register_file failed (out of memory)"));
    }
    Ok(())
}

/// Drop every registered virtual file and reset any open handles.
#[wasm_bindgen]
pub fn clear_registered_files() {
    unsafe { shim_fs_clear() };
}

/// One attribute row of the batched snapshot: name and value always
/// carry; prefix/uri are undefined for no-namespace attributes.
#[wasm_bindgen]
#[derive(Clone)]
pub struct WasmAttribute {
    name: String,
    value: String,
    prefix: Option<String>,
    uri: Option<String>,
}

#[wasm_bindgen]
impl WasmAttribute {
    #[wasm_bindgen(getter)]
    pub fn name(&self) -> String {
        self.name.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn value(&self) -> String {
        self.value.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn prefix(&self) -> Option<String> {
        self.prefix.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn uri(&self) -> Option<String> {
        self.uri.clone()
    }
}

/// The engine's capability descriptor (leptris#1905): the raw bitset
/// plus the descriptor version (bits append per the engine's
/// contract; the descriptor version gates shape changes).
#[wasm_bindgen]
pub struct WasmCapabilities {
    caps: u64,
    descriptor_version: u32,
}

#[wasm_bindgen]
impl WasmCapabilities {
    #[wasm_bindgen(getter)]
    pub fn caps(&self) -> f64 {
        self.caps as f64
    }

    #[wasm_bindgen(getter)]
    pub fn descriptor_version(&self) -> u32 {
        self.descriptor_version
    }

    /// Whether the engine serves the given LEPTRIS_CAP_* bit value.
    pub fn serves(&self, bit: f64) -> bool {
        self.caps & (bit as u64) != 0
    }
}

/// A parsed document living in engine (wasm linear) memory.
#[wasm_bindgen]
pub struct WasmDocument {
    raw: *mut ffi::LeptrisDocument,
}

/// A compiled XSD schema. Owns its engine-side memory.
/// [`WasmSchema::validate_doc`] answers validity;
/// [`WasmSchema::last_errors`] returns the rows of the most recent
/// run.
#[wasm_bindgen]
pub struct WasmSchema {
    raw: *mut ffi::LeptrisXsdSchema,
}

impl Drop for WasmSchema {
    fn drop(&mut self) {
        if !self.raw.is_null() {
            unsafe { ffi::leptris_xsd_free(self.raw) };
        }
    }
}

/// The engine's capability descriptor (leptris#1905): what THIS
/// installed engine serves, as a versioned bitset. Bits append per
/// the engine's contract; decode against the LEPTRIS_CAP_* layout of
/// the reported descriptor version.
#[wasm_bindgen]
pub fn engine_capabilities() -> WasmCapabilities {
    let raw = unsafe { ffi::leptris_capabilities() };
    WasmCapabilities {
        caps: raw.caps,
        descriptor_version: raw.descriptor_version,
    }
}

/// Compile a schema SET: every listed document's declarations merge
/// into ONE schema (imports resolve across the set; the first
/// document's targetNamespace is the schema's). Paths resolve
/// relative to each document's own directory — register every file
/// the set will touch (heads and imported members) under the exact
/// virtual path first via register_file(); unregistered paths fail.
#[wasm_bindgen]
pub fn compile_xsd_set(paths: Vec<String>) -> Result<WasmSchema, JsError> {
    if paths.is_empty() {
        return Err(JsError::new("schema set needs at least one path"));
    }
    let cpaths: Vec<CString> = paths
        .iter()
        .map(|p| CString::new(p.as_str()))
        .collect::<Result<_, _>>()
        .map_err(|_| JsError::new("schema path contains a NUL byte"))?;
    for (p, cp) in paths.iter().zip(&cpaths) {
        if unsafe { shim_fs_has(cp.as_ptr()) } == 0 {
            return Err(JsError::new(&format!(
                "schema path not registered: {p} — call register_file() for every \
                 document the set will read (heads and imported members) first"
            )));
        }
    }
    let ptrs: Vec<*const std::os::raw::c_char> = cpaths.iter().map(|p| p.as_ptr()).collect();
    let mut st = ffi::LEPTRIS_OK;
    let raw = unsafe { ffi::leptris_xsd_compile_set(ptrs.as_ptr(), ptrs.len(), &mut st) };
    finish_schema(raw, st, "leptris xsd compile_set")
}

/// The engine's XSD compile faces return an ERROR-CARRYING HANDLE on
/// failure (non-NULL with status set) — the status is authoritative.
/// Surface the engine's error text and drop the handle.
fn finish_schema(
    raw: *mut ffi::LeptrisXsdSchema,
    st: ffi::LeptrisStatus,
    what: &str,
) -> Result<WasmSchema, JsError> {
    if st != ffi::LEPTRIS_OK {
        let mut msg = String::new();
        if !raw.is_null() {
            let p = unsafe { ffi::leptris_xsd_error(raw) };
            if !p.is_null() {
                msg = unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned();
            }
            unsafe { ffi::leptris_xsd_free(raw) };
        }
        return Err(JsError::new(&format!("{what} failed (status {st}): {msg}")));
    }
    if raw.is_null() {
        return Err(JsError::new(&format!("{what} failed (status {st})")));
    }
    Ok(WasmSchema { raw })
}

/// Parse an HTML string through the engine's WHATWG-conformant HTML
/// mode (implied head/body, void elements, case-normalized names).
#[wasm_bindgen]
pub fn parse_html(html: &str) -> Result<WasmDocument, JsError> {
    let chtml = CString::new(html).map_err(|_| JsError::new("html contains a NUL byte"))?;
    let mut st = ffi::LEPTRIS_OK;
    let raw = unsafe { ffi::leptris_parse_html_string(chtml.as_ptr(), html.len(), &mut st) };
    if raw.is_null() {
        return Err(JsError::new(&format!(
            "leptris html parse failed (status {st})"
        )));
    }
    Ok(WasmDocument { raw })
}

/// Compile an XSD schema (tier-1 subset). For multi-document schemas
/// (xsd:import / xsd:include across files) use register_file() +
/// compile_xsd_set().
#[wasm_bindgen]
pub fn compile_xsd(xsd: &str) -> Result<WasmSchema, JsError> {
    let cxsd = CString::new(xsd).map_err(|_| JsError::new("xsd contains a NUL byte"))?;
    let mut st: ffi::LeptrisStatus = ffi::LEPTRIS_OK;
    let raw = unsafe { ffi::leptris_xsd_compile(cxsd.as_ptr(), xsd.len(), &mut st) };
    finish_schema(raw, st, "leptris xsd compile")
}

#[wasm_bindgen]
impl WasmSchema {
    /// Validate a document: Ok(true) valid, Ok(false) invalid (fetch
    /// the rows via `last_errors`), Err on bad args.
    pub fn validate_doc(&self, doc: &WasmDocument) -> Result<bool, JsError> {
        let r = unsafe { ffi::leptris_xsd_validate(self.raw, doc.raw) };
        if r < 0 {
            return Err(JsError::new("leptris xsd validation failed"));
        }
        Ok(r == 1)
    }

    /// The error rows of the most recent validate run (schema-owned;
    /// replaced by the next run).
    pub fn last_errors(&self) -> Vec<String> {
        unsafe {
            let n = ffi::leptris_xsd_error_count(self.raw);
            (0..n)
                .filter_map(|i| {
                    let p = ffi::leptris_xsd_error_at(self.raw, i);
                    if p.is_null() {
                        None
                    } else {
                        Some(CStr::from_ptr(p).to_string_lossy().into_owned())
                    }
                })
                .collect()
        }
    }
}

impl Drop for WasmDocument {
    fn drop(&mut self) {
        if !self.raw.is_null() {
            unsafe { ffi::leptris_document_free(self.raw) };
        }
    }
}

#[wasm_bindgen]
impl WasmDocument {
    /// Parse an XML string. Errors carry the engine's LeptrisStatus.
    #[wasm_bindgen(constructor)]
    pub fn parse(xml: &str) -> Result<WasmDocument, JsError> {
        let cxml = CString::new(xml).map_err(|_| JsError::new("xml contains a NUL byte"))?;
        let mut st: ffi::LeptrisStatus = ffi::LEPTRIS_OK;
        let raw = unsafe { ffi::leptris_parse_string(cxml.as_ptr(), xml.len(), &mut st) };
        if raw.is_null() {
            return Err(parse_failure(st));
        }
        Ok(WasmDocument { raw })
    }

    /// Parse with per-call options. The engine offers: a nesting cap
    /// (`max_depth`), strict/lenient mode, recover mode, and the
    /// `LeptrisParseFlags` bitset. It has NO entity-expansion cap
    /// (see [`WasmParseOptions`]).
    pub fn parse_with_options(
        xml: &str,
        options: &WasmParseOptions,
    ) -> Result<WasmDocument, JsError> {
        let cxml = CString::new(xml).map_err(|_| JsError::new("xml contains a NUL byte"))?;
        let opts = ffi::LeptrisParseOptions {
            flags: crate::guards::checked_flags(options.flags).map_err(|m| JsError::new(&m))?,
            strict_mode: options.strict_mode,
            max_depth: crate::guards::checked_max_depth(options.max_depth)
                .map_err(|m| JsError::new(&m))?,
            recover: options.recover as c_int,
        };
        let mut st: ffi::LeptrisStatus = ffi::LEPTRIS_OK;
        let raw = unsafe { ffi::leptris_parse_string_ex(cxml.as_ptr(), xml.len(), &opts, &mut st) };
        if raw.is_null() {
            return Err(parse_failure(st));
        }
        Ok(WasmDocument { raw })
    }

    /// Evaluate an XPath expression; the string-value of the result.
    pub fn xpath(&self, expression: &str) -> Result<String, JsError> {
        let cexpr =
            CString::new(expression).map_err(|_| JsError::new("xpath contains a NUL byte"))?;
        let result =
            unsafe { ffi::leptris_xpath_eval(self.raw, std::ptr::null_mut(), cexpr.as_ptr()) };
        if result.is_null() {
            return Err(JsError::new("leptris xpath evaluation failed"));
        }
        let s = unsafe { take_string(ffi::leptris_xpath_result_string(result)) };
        unsafe { ffi::leptris_xpath_result_free(result) };
        s.map_err(|_| JsError::new("result string materialization failed"))
    }

    /// Count nodes matching the expression.
    pub fn xpath_count(&self, expression: &str) -> Result<usize, JsError> {
        let cexpr =
            CString::new(expression).map_err(|_| JsError::new("xpath contains a NUL byte"))?;
        let result =
            unsafe { ffi::leptris_xpath_eval(self.raw, std::ptr::null_mut(), cexpr.as_ptr()) };
        if result.is_null() {
            return Err(JsError::new("leptris xpath evaluation failed"));
        }
        let n = unsafe { ffi::leptris_xpath_result_count(result) };
        unsafe { ffi::leptris_xpath_result_free(result) };
        Ok(n)
    }

    /// Serialize the document back to XML.
    pub fn to_xml(&self) -> Result<String, JsError> {
        unsafe {
            take_string(ffi::leptris_document_serialize(
                self.raw,
                std::ptr::null_mut(),
            ))
        }
        .map_err(|_| JsError::new("serialization failed"))
    }

    /// Root element name.
    pub fn root_name(&self) -> Option<String> {
        let root = unsafe { ffi::leptris_document_root(self.raw) };
        if root.is_null() {
            return None;
        }
        unsafe {
            Some(
                CStr::from_ptr(ffi::leptris_element_name(root))
                    .to_string_lossy()
                    .into_owned(),
            )
        }
    }
}

/// Per-call parse options (the engine's `LeptrisParseOptions`).
///
/// The engine has NO entity-expansion limit: DTD entities expand
/// fully (a 10^6x nested-entity document yields a 3 MB text node;
/// only nesting depth is capped, at 10 levels). There is therefore
/// no `max_entity_expansion` field; adding one needs an engine
/// change (a cap on total expanded bytes/count in the entity
/// resolver, surfaced as a `LeptrisParseOptions` field).
#[wasm_bindgen]
#[derive(Clone, Copy)]
pub struct WasmParseOptions {
    /// 0 = engine default (256), >0 = nesting cap. Values above
    /// `i32::MAX` are refused by `parse_with_options` (no wraparound).
    pub max_depth: u32,
    /// -1 = keep thread default (recommended), 0 = lenient, 1 = strict.
    pub strict_mode: i32,
    /// A failed parse yields an empty document instead of an error.
    pub recover: bool,
    /// `LeptrisParseFlags` bitset (values above `i32::MAX` are refused,
    /// no wraparound): 1 drop whitespace-only text,
    /// 2 DTD attribute defaults, 4 keep entity refs, 8 skip duplicate
    /// detection, 16 skip source positions, 32 recover.
    pub flags: u32,
}

#[wasm_bindgen]
impl WasmParseOptions {
    /// Engine defaults.
    #[wasm_bindgen(constructor)]
    pub fn new() -> WasmParseOptions {
        WasmParseOptions {
            max_depth: 0,
            strict_mode: -1,
            recover: false,
            flags: 0,
        }
    }
}

impl Default for WasmParseOptions {
    fn default() -> Self {
        Self::new()
    }
}

/// The position and message of the most recent failed parse.
#[wasm_bindgen]
pub struct WasmParseError {
    message: String,
    line: u32,
    column: u32,
}

#[wasm_bindgen]
impl WasmParseError {
    #[wasm_bindgen(getter)]
    pub fn message(&self) -> String {
        self.message.clone()
    }

    /// 1-based line (0 when the engine recorded none).
    #[wasm_bindgen(getter)]
    pub fn line(&self) -> u32 {
        self.line
    }

    /// 1-based byte column (0 when the engine recorded none).
    #[wasm_bindgen(getter)]
    pub fn column(&self) -> u32 {
        self.column
    }
}

/// The engine's record of the most recent failed parse on this
/// thread, or undefined when none was recorded. Read it right after a
/// parse throws; a later parse overwrites it.
#[wasm_bindgen]
pub fn last_parse_error() -> Option<WasmParseError> {
    unsafe {
        let p = ffi::leptris_last_error();
        if p.is_null() {
            return None;
        }
        let (mut line, mut column): (c_int, c_int) = (0, 0);
        ffi::leptris_last_error_position(&mut line, &mut column);
        Some(WasmParseError {
            message: CStr::from_ptr(p).to_string_lossy().into_owned(),
            line: line.max(0) as u32,
            column: column.max(0) as u32,
        })
    }
}

/// The message of the error thrown for a failed parse: status,
/// engine message, and the 1-based position in a stable
/// `(line L, column C)` suffix.
pub(crate) fn parse_failure_message(st: ffi::LeptrisStatus) -> String {
    match last_parse_error() {
        Some(e) if e.line > 0 => format!(
            "leptris parse failed (status {st}): {} (line {}, column {})",
            e.message, e.line, e.column
        ),
        Some(e) => format!("leptris parse failed (status {st}): {}", e.message),
        None => format!("leptris parse failed (status {st})"),
    }
}

fn parse_failure(st: ffi::LeptrisStatus) -> JsError {
    JsError::new(&parse_failure_message(st))
}

/// Take ownership of an engine-allocated C string (leptris_free_string
/// contract) and convert to a Rust String.
unsafe fn take_string(p: *mut c_char) -> Result<String, ()> {
    if p.is_null() {
        return Err(());
    }
    let s = CStr::from_ptr(p as *const c_char)
        .to_string_lossy()
        .into_owned();
    // The mirror's free face (same name as the header's).
    extern "C" {
        fn leptris_free_string(str: *mut c_char);
    }
    leptris_free_string(p);
    let _ = c_int::MAX; // keep the c_int import honest when unused
    Ok(s)
}

/* ---- #15 faces: node handles, result shapes, write face ---------- */

/// Node kinds for mixed children/nodeset rows (engine LeptrisNodeKind).
#[wasm_bindgen]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WasmNodeKind {
    Element = 1,
    Text = 2,
    Comment = 3,
    Cdata = 4,
    Pi = 5,
    DocType = 6,
    Attribute = 7,
    Other = 0,
}

/// A node handle into a document living in engine memory. Reads are
/// bounds-checked by the engine; a handle stays valid for its
/// document's lifetime.
#[wasm_bindgen]
#[derive(Clone)]
pub struct WasmNode {
    doc: *mut ffi::LeptrisDocument,
    node: ffi::LeptrisNodeRef,
}

#[wasm_bindgen]
impl WasmNode {
    /// Element name (element-kind nodes only).
    pub fn name(&self) -> Option<String> {
        unsafe {
            let p = ffi::leptris_element_name(self.node as *mut ffi::LeptrisElement);
            if p.is_null() {
                return None;
            }
            /* document-owned: BORROW, never free */
            Some(CStr::from_ptr(p).to_string_lossy().into_owned())
        }
    }

    /// Text content (text-kind nodes read their data; element-kind
    /// read their concatenated text). Document-owned: borrowed.
    pub fn text(&self) -> Option<String> {
        unsafe {
            let p = match self.kind() {
                WasmNodeKind::Text => ffi::leptris_text_node_get_content(self.node),
                _ => ffi::leptris_element_text(self.node as *mut ffi::LeptrisElement),
            };
            if p.is_null() {
                return None;
            }
            Some(CStr::from_ptr(p).to_string_lossy().into_owned())
        }
    }

    /// Engine node kind.
    pub fn kind(&self) -> WasmNodeKind {
        match unsafe { ffi::leptris_node_get_type(self.node) } {
            0 => WasmNodeKind::Element,
            1 => WasmNodeKind::Text,
            2 => WasmNodeKind::Comment,
            3 => WasmNodeKind::Cdata,
            4 => WasmNodeKind::Pi,
            5 => WasmNodeKind::DocType,
            6 => WasmNodeKind::Attribute,
            _ => WasmNodeKind::Other,
        }
    }

    /// Children across every kind the engine serves, document order.
    pub fn children(&self) -> Vec<WasmNode> {
        unsafe {
            let n = ffi::leptris_node_children(self.node, std::ptr::null_mut(), 0);
            if n == 0 {
                return Vec::new();
            }
            let mut nodes: Vec<ffi::LeptrisNodeRef> = vec![std::ptr::null_mut(); n];
            let got = ffi::leptris_node_children(self.node, nodes.as_mut_ptr(), n);
            nodes.truncate(got);
            nodes
                .into_iter()
                .map(|node| WasmNode {
                    doc: self.doc,
                    node,
                })
                .collect()
        }
    }

    /// Set an attribute (element-kind nodes).
    pub fn set_attribute(&self, name: &str, value: &str) -> Result<(), JsError> {
        let cname =
            CString::new(name).map_err(|_| JsError::new("attribute name contains a NUL byte"))?;
        let cvalue =
            CString::new(value).map_err(|_| JsError::new("attribute value contains a NUL byte"))?;
        let st = unsafe {
            ffi::leptris_element_set_attribute(
                self.node as *mut ffi::LeptrisElement,
                cname.as_ptr(),
                cvalue.as_ptr(),
            )
        };
        if st != ffi::LEPTRIS_OK {
            return Err(JsError::new("leptris set_attribute failed"));
        }
        Ok(())
    }

    /// Evaluate an XPath with THIS element as the context node (the
    /// engine's `leptris_xpath_eval` context argument): `.`/`@*`/
    /// `namespace-uri(.)` answer about the element itself in O(1)
    /// lookups, with no document-root positional path to walk.
    /// Returns the string-value of the result.
    pub fn xpath(&self, expression: &str) -> Result<String, JsError> {
        let cexpr =
            CString::new(expression).map_err(|_| JsError::new("xpath contains a NUL byte"))?;
        let result = unsafe {
            ffi::leptris_xpath_eval(
                self.doc,
                self.node as *mut ffi::LeptrisElement,
                cexpr.as_ptr(),
            )
        };
        if result.is_null() {
            return Err(JsError::new("leptris xpath evaluation failed"));
        }
        let s = unsafe { take_string(ffi::leptris_xpath_result_string(result)) };
        unsafe { ffi::leptris_xpath_result_free(result) };
        s.map_err(|_| JsError::new("result string materialization failed"))
    }

    /// Remove an attribute by its name as spelled (element-kind
    /// nodes). Ok(true) removed, Ok(false) no such attribute. The
    /// engine has no namespace-aware removal (no
    /// `leptris_element_remove_attribute_ns`); resolve the spelled
    /// name from `attribute_pairs()` (prefix + uri) and pass that.
    pub fn remove_attribute(&self, name: &str) -> Result<bool, JsError> {
        let cname =
            CString::new(name).map_err(|_| JsError::new("attribute name contains a NUL byte"))?;
        let st = unsafe {
            ffi::leptris_element_remove_attribute(
                self.node as *mut ffi::LeptrisElement,
                cname.as_ptr(),
            )
        };
        match st {
            ffi::LEPTRIS_OK => Ok(true),
            ffi::LEPTRIS_ERROR_NOT_FOUND => Ok(false),
            _ => Err(JsError::new(&format!(
                "leptris remove_attribute failed (status {st})"
            ))),
        }
    }

    /// The element's namespace URI (element-kind nodes), or undefined
    /// for a no-namespace element.
    pub fn namespace_uri(&self) -> Option<String> {
        unsafe {
            let ns = ffi::leptris_element_namespace(self.node as *mut ffi::LeptrisElement);
            if ns.is_null() {
                return None;
            }
            let p = ffi::leptris_namespace_uri(ns);
            if p.is_null() {
                return None;
            }
            /* document-owned: BORROW, never free */
            Some(CStr::from_ptr(p).to_string_lossy().into_owned())
        }
    }

    /// The element's namespace prefix as spelled, or undefined.
    pub fn prefix(&self) -> Option<String> {
        unsafe {
            let p = ffi::leptris_element_prefix(self.node as *mut ffi::LeptrisElement);
            if p.is_null() {
                return None;
            }
            Some(CStr::from_ptr(p).to_string_lossy().into_owned())
        }
    }

    /// ALL attribute names, values, and namespace data in ONE
    /// crossing (the engine's batched snapshot, leptris#1689): an
    /// array of {name, value, prefix, uri} — prefix/uri undefined for
    /// no-namespace attributes.
    pub fn attribute_pairs(&self) -> Result<Vec<WasmAttribute>, JsError> {
        let elem = self.node as *mut ffi::LeptrisElement;
        let count = unsafe { ffi::leptris_element_attribute_count(elem) };
        if count == 0 {
            return Ok(Vec::new());
        }
        /* Phase 1: sizing (NULL blobs). */
        let cap = unsafe {
            ffi::leptris_element_attributes_snapshot(
                elem,
                std::ptr::null_mut(),
                0,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                0,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                0,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                0,
                std::ptr::null_mut(),
            )
        };
        if cap == usize::MAX {
            return Err(JsError::new("leptris attribute snapshot sizing failed"));
        }
        /* Phase 2: read. One retry at the phase-1 capacity (the
         * engine's contract for mid-call mutation). */
        for _ in 0..2 {
            let mut names = vec![0u8; cap];
            let mut values = vec![0u8; cap];
            let mut prefixes = vec![0u8; cap];
            let mut uris = vec![0u8; cap];
            let mut name_offsets = vec![0usize; count + 1];
            let mut value_offsets = vec![0usize; count + 1];
            let mut prefix_offsets = vec![0usize; count + 1];
            let mut uri_offsets = vec![0usize; count + 1];
            let st = unsafe {
                ffi::leptris_element_attributes_snapshot(
                    elem,
                    names.as_mut_ptr() as *mut c_char,
                    cap,
                    name_offsets.as_mut_ptr(),
                    values.as_mut_ptr() as *mut c_char,
                    cap,
                    value_offsets.as_mut_ptr(),
                    prefixes.as_mut_ptr() as *mut c_char,
                    cap,
                    prefix_offsets.as_mut_ptr(),
                    uris.as_mut_ptr() as *mut c_char,
                    cap,
                    uri_offsets.as_mut_ptr(),
                )
            };
            if st == 0 {
                /* Lanes are NUL-terminated blobs. A no-namespace
                 * attribute stores SIZE_MAX in its prefix/uri slot
                 * (leptris.h), so offsets[i + 1] is NOT a usable end
                 * bound: read from offsets[i] to the first NUL. */
                let lane = |blob: &[u8], offsets: &[usize], i: usize| -> Result<String, JsError> {
                    crate::guards::lane_str(blob, offsets[i]).map_err(|m| JsError::new(&m))
                };
                let mut out = Vec::with_capacity(count);
                for i in 0..count {
                    let no_ns = uri_offsets[i] == usize::MAX;
                    out.push(WasmAttribute {
                        name: lane(&names, &name_offsets, i)?,
                        value: lane(&values, &value_offsets, i)?,
                        prefix: if no_ns {
                            None
                        } else {
                            Some(lane(&prefixes, &prefix_offsets, i)?)
                        },
                        uri: if no_ns {
                            None
                        } else {
                            Some(lane(&uris, &uri_offsets, i)?)
                        },
                    });
                }
                return Ok(out);
            }
        }
        Err(JsError::new("leptris attribute snapshot failed"))
    }

    /// Append an owned node (created via WasmDocument::create_* or
    /// moved from another parent of the SAME document). Returns the
    /// installed handle.
    pub fn add_child(&self, child: WasmNode) -> Result<WasmNode, JsError> {
        if !child.doc.eq(&self.doc) {
            return Err(JsError::new(
                "add_child: cross-document adoption is not served by this face",
            ));
        }
        let mut installed: usize = 0;
        let st = unsafe {
            ffi::leptris_element_add_child_addr_installed(
                self.node as usize,
                child.node as usize,
                &mut installed,
            )
        };
        if st != ffi::LEPTRIS_OK || installed == 0 {
            return Err(JsError::new("leptris add_child failed"));
        }
        Ok(WasmNode {
            doc: self.doc,
            node: installed as ffi::LeptrisNodeRef,
        })
    }

    pub fn ptr(&self) -> usize {
        self.node as usize
    }
}

/// One XPath result: the full processing-model shape — the string
/// value plus the typed view, and for node-sets the node handles in
/// document order.
#[wasm_bindgen]
pub struct WasmXPathResult {
    kind: i32,
    string_value: String,
    boolean: bool,
    number: f64,
    nodes: Vec<WasmNode>,
}

#[wasm_bindgen]
impl WasmXPathResult {
    /// 0 nodeset, 1 boolean, 2 number, 3 string, 4 function item
    /// (the engine's LeptrisXPathResultType — 0-based; #19).
    pub fn kind(&self) -> i32 {
        self.kind
    }
    /// The string-value of the result (every kind).
    #[wasm_bindgen(js_name = stringValue)]
    pub fn string_value(&self) -> String {
        self.string_value.clone()
    }
    /// The boolean value (boolean kind; EBV otherwise per spec).
    pub fn boolean(&self) -> bool {
        self.boolean
    }
    /// The numeric value (number kind; NaN outside the numeric space).
    pub fn number(&self) -> f64 {
        self.number
    }
    /// Node handles for node-set results (document order).
    pub fn nodes(&self) -> Vec<WasmNode> {
        self.nodes.clone()
    }
}

#[wasm_bindgen]
impl WasmDocument {
    /// Evaluate an XPath with the FULL result shape.
    pub fn xpath_full(&self, expression: &str) -> Result<WasmXPathResult, JsError> {
        let cexpr =
            CString::new(expression).map_err(|_| JsError::new("xpath contains a NUL byte"))?;
        let result =
            unsafe { ffi::leptris_xpath_eval(self.raw, std::ptr::null_mut(), cexpr.as_ptr()) };
        if result.is_null() {
            return Err(JsError::new("leptris xpath evaluation failed"));
        }
        unsafe {
            let kind = ffi::leptris_xpath_result_type(result);
            let sv = take_string(ffi::leptris_xpath_result_string(result)).unwrap_or_default();
            let boolean = ffi::leptris_xpath_result_boolean(result) != 0;
            let number = ffi::leptris_xpath_result_number(result);
            let mut nodes = Vec::new();
            if kind == ffi::LEPTRIS_XPATH_NODESET {
                let n = ffi::leptris_xpath_result_count(result);
                let mut handles: Vec<ffi::LeptrisNodeRef> = vec![std::ptr::null_mut(); n];
                let got = ffi::leptris_xpath_result_get_nodes_ex(
                    result,
                    handles.as_mut_ptr(),
                    std::ptr::null_mut(),
                    n,
                );
                handles.truncate(got);
                nodes = handles
                    .into_iter()
                    .map(|node| WasmNode {
                        doc: self.raw,
                        node,
                    })
                    .collect();
            }
            ffi::leptris_xpath_result_free(result);
            Ok(WasmXPathResult {
                kind,
                string_value: sv,
                boolean,
                number,
                nodes,
            })
        }
    }

    /// The root element as a node handle.
    pub fn root(&self) -> Option<WasmNode> {
        let root = unsafe { ffi::leptris_document_root(self.raw) };
        if root.is_null() {
            return None;
        }
        Some(WasmNode {
            doc: self.raw,
            node: root as ffi::LeptrisNodeRef,
        })
    }

    /// Create a detached element owned by this document.
    pub fn create_element(&self, name: &str) -> Result<WasmNode, JsError> {
        let cname =
            CString::new(name).map_err(|_| JsError::new("element name contains a NUL byte"))?;
        let elem = unsafe {
            ffi::leptris_element_new_with_attributes(
                self.raw,
                cname.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                0,
            )
        };
        if elem.is_null() {
            return Err(JsError::new("leptris create_element failed"));
        }
        Ok(WasmNode {
            doc: self.raw,
            node: elem as ffi::LeptrisNodeRef,
        })
    }

    /// Create a detached text node owned by this document.
    pub fn create_text(&self, text: &str) -> Result<WasmNode, JsError> {
        let ctext = CString::new(text).map_err(|_| JsError::new("text contains a NUL byte"))?;
        let node = unsafe { ffi::leptris_text_node_create(self.raw, ctext.as_ptr()) };
        if node.is_null() {
            return Err(JsError::new("leptris create_text failed"));
        }
        Ok(WasmNode {
            doc: self.raw,
            node,
        })
    }

    /// Create a detached comment node owned by this document.
    pub fn create_comment(&self, text: &str) -> Result<WasmNode, JsError> {
        let ctext = CString::new(text).map_err(|_| JsError::new("comment contains a NUL byte"))?;
        let node = unsafe { ffi::leptris_comment_node_create(self.raw, ctext.as_ptr()) };
        if node.is_null() {
            return Err(JsError::new("leptris create_comment failed"));
        }
        Ok(WasmNode {
            doc: self.raw,
            node,
        })
    }

    /// Create a detached CDATA section node owned by this document.
    pub fn create_cdata(&self, text: &str) -> Result<WasmNode, JsError> {
        let ctext = CString::new(text).map_err(|_| JsError::new("cdata contains a NUL byte"))?;
        let node = unsafe { ffi::leptris_cdata_node_create(self.raw, ctext.as_ptr()) };
        if node.is_null() {
            return Err(JsError::new("leptris create_cdata failed"));
        }
        Ok(WasmNode {
            doc: self.raw,
            node,
        })
    }

    /// Create a detached processing-instruction node owned by this
    /// document. `data` may be empty.
    pub fn create_pi(&self, target: &str, data: &str) -> Result<WasmNode, JsError> {
        let ctarget =
            CString::new(target).map_err(|_| JsError::new("pi target contains a NUL byte"))?;
        let cdata = CString::new(data).map_err(|_| JsError::new("pi data contains a NUL byte"))?;
        let node =
            unsafe { ffi::leptris_pi_node_create(self.raw, ctarget.as_ptr(), cdata.as_ptr()) };
        if node.is_null() {
            return Err(JsError::new("leptris create_pi failed"));
        }
        Ok(WasmNode {
            doc: self.raw,
            node,
        })
    }

    /// Set the document's root element (attaches a detached element).
    pub fn set_root(&self, root: WasmNode) -> Result<(), JsError> {
        let st = unsafe {
            ffi::leptris_document_set_root(self.raw, root.node as *mut ffi::LeptrisElement)
        };
        if st != ffi::LEPTRIS_OK {
            return Err(JsError::new("leptris set_root failed"));
        }
        Ok(())
    }
}

/* ---- #22 slice 2: the data-plane faces ----------------------------
 * Bulk-first rule (moxml-rs#2): cross the boundary once per batch,
 * never per node. `element_rows` walks the subtree RUST-SIDE and
 * returns pre-order element rows in ONE crossing; the cursor is
 * stateless (the caller re-walks skipping `next_cursor` elements),
 * so a budgeted traversal never holds engine-side state. */

/// One pre-order element row. Row contract (pinned by the
/// conformance fixtures, not just the answers): elements only,
/// document pre-order, `depth` 0 = the walked root; `text` is the
/// element's concatenated text ("" when none); `attrs` carries every
/// attribute's {name, value, prefix, uri} in document order.
#[wasm_bindgen]
#[derive(Clone)]
pub struct WasmElementRow {
    depth: usize,
    name: String,
    prefix: Option<String>,
    uri: Option<String>,
    text: String,
    attrs: Vec<WasmAttribute>,
}

#[wasm_bindgen]
impl WasmElementRow {
    #[wasm_bindgen(getter)]
    pub fn depth(&self) -> usize {
        self.depth
    }
    #[wasm_bindgen(getter)]
    pub fn name(&self) -> String {
        self.name.clone()
    }
    #[wasm_bindgen(getter)]
    pub fn prefix(&self) -> Option<String> {
        self.prefix.clone()
    }
    #[wasm_bindgen(getter)]
    pub fn uri(&self) -> Option<String> {
        self.uri.clone()
    }
    #[wasm_bindgen(getter)]
    pub fn text(&self) -> String {
        self.text.clone()
    }
    #[wasm_bindgen(getter)]
    pub fn attrs(&self) -> Vec<WasmAttribute> {
        self.attrs.clone()
    }
}

/// One budgeted batch: the rows, the cursor to resume from, and
/// whether the subtree is exhausted.
#[wasm_bindgen]
#[derive(Clone)]
pub struct WasmRowsBatch {
    rows: Vec<WasmElementRow>,
    next_cursor: usize,
    done: bool,
}

#[wasm_bindgen]
impl WasmRowsBatch {
    #[wasm_bindgen(getter)]
    pub fn rows(&self) -> Vec<WasmElementRow> {
        self.rows.clone()
    }
    #[wasm_bindgen(getter)]
    pub fn next_cursor(&self) -> usize {
        self.next_cursor
    }
    #[wasm_bindgen(getter)]
    pub fn is_done(&self) -> bool {
        self.done
    }
}

impl WasmNode {
    /// Single attribute read (element-kind nodes). Convenience
    /// accessor over the engine's borrow — batched reads go through
    /// `attribute_pairs` / `element_rows` per the data-plane rule.
    pub fn attribute(&self, name: &str) -> Option<String> {
        let cname = CString::new(name).ok()?;
        unsafe {
            let p = ffi::leptris_element_attribute(
                self.node as *mut ffi::LeptrisElement,
                cname.as_ptr(),
            );
            if p.is_null() {
                return None;
            }
            /* document-owned: BORROW, never free */
            Some(CStr::from_ptr(p).to_string_lossy().into_owned())
        }
    }

    /// Pre-order element rows for the subtree, resuming at
    /// `cursor`, collecting at most `budget` rows — ONE boundary
    /// crossing per batch. `cursor` 0 starts at this node; the
    /// batch's `next_cursor` feeds the next call until `is_done`.
    pub fn element_rows(&self, cursor: usize, budget: usize) -> Result<WasmRowsBatch, JsError> {
        if budget == 0 {
            return Err(JsError::new("budget must be > 0"));
        }
        let mut skipped = 0usize;
        let mut rows = Vec::new();
        let mut done = true;
        self.collect_rows(cursor, budget, &mut skipped, &mut rows, &mut done);
        let next_cursor = cursor + rows.len();
        Ok(WasmRowsBatch {
            rows,
            next_cursor,
            done,
        })
    }

    fn collect_rows(
        &self,
        cursor: usize,
        budget: usize,
        skipped: &mut usize,
        rows: &mut Vec<WasmElementRow>,
        done: &mut bool,
    ) {
        /* stateless pre-order: skip `cursor` elements, take `budget`.
         * When the budget fills, the walk CONTINUES in look-ahead
         * mode: `done` stays false only if a further element
         * actually exists (an exact-fit batch reports done=true —
         * the next call yields an empty done batch). */
        let mut stack = vec![(self.clone(), 0usize)];
        let mut remaining = budget;
        while let Some((node, depth)) = stack.pop() {
            if node.kind() == WasmNodeKind::Element {
                if *skipped < cursor {
                    *skipped += 1;
                } else if remaining > 0 {
                    let name = node.name().unwrap_or_default();
                    let prefix = node.prefix();
                    let uri = node.namespace_uri();
                    let text = node.text().unwrap_or_default();
                    let attrs = node.attribute_pairs().unwrap_or_default();
                    rows.push(WasmElementRow {
                        depth,
                        name,
                        prefix,
                        uri,
                        text,
                        attrs,
                    });
                    remaining -= 1;
                } else {
                    /* budget spent; a further element exists */
                    *done = false;
                    return;
                }
            }
            let kids = node.children();
            for kid in kids.iter().rev() {
                stack.push((kid.clone(), depth + 1));
            }
        }
        *done = true;
    }
}
