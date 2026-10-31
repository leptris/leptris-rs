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

/// Compile an XSD schema (tier-1 subset; schema-set via one text with
/// relative xsd:import is NOT supported in the wasm face yet — pass
/// the head document).
#[wasm_bindgen]
pub fn compile_xsd(xsd: &str) -> Result<WasmSchema, JsError> {
    let cxsd = CString::new(xsd).map_err(|_| JsError::new("xsd contains a NUL byte"))?;
    let mut st: ffi::LeptrisStatus = ffi::LEPTRIS_OK;
    let raw = unsafe { ffi::leptris_xsd_compile(cxsd.as_ptr(), xsd.len(), &mut st) };
    if raw.is_null() {
        return Err(JsError::new(&format!(
            "leptris xsd compile failed (status {st})"
        )));
    }
    Ok(WasmSchema { raw })
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
            return Err(JsError::new(&format!("leptris parse failed (status {st})")));
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
    /// 1 nodeset, 2 boolean, 3 number, 4 string (engine enum).
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
