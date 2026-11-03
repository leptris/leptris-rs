//! Raw FFI declarations mirroring the public headers
//! (`src/include/leptris/`). Hand-maintained like the Python and
//! Ruby mirrors — the FFI drift gate keeps the surface in check.
//!
//! Strings returned as `*const c_char` from document-taking
//! accessors are document-owned and live until `leptris_document_free`.

#![allow(non_camel_case_types)]
// The mirror declares the full public contract, not just what the
// wrappers currently use — unused decls are intentional.
#![allow(dead_code)]

use std::os::raw::{c_char, c_int};

pub type LeptrisStatus = c_int;

pub const LEPTRIS_OK: c_int = 0;
pub const LEPTRIS_ERROR_MEMORY: c_int = -1;
pub const LEPTRIS_ERROR_PARSE: c_int = -2;
pub const LEPTRIS_ERROR_XPATH: c_int = -3;
pub const LEPTRIS_ERROR_NULL_ARG: c_int = -4;
pub const LEPTRIS_ERROR_INVALID_ARG: c_int = -5;
pub const LEPTRIS_ERROR_NOT_FOUND: c_int = -6;
pub const LEPTRIS_ERROR_IO: c_int = -7;
pub const LEPTRIS_ERROR_NOT_IMPLEMENTED: c_int = -8;

/* Opaque handles */
#[repr(C)]
pub struct LeptrisDocument {
    _private: [u8; 0],
}
#[repr(C)]
pub struct LeptrisElement {
    _private: [u8; 0],
}
#[repr(C)]
pub struct LeptrisNode {
    _private: [u8; 0],
}
#[repr(C)]
pub struct LeptrisAttribute {
    _private: [u8; 0],
}
#[repr(C)]
pub struct LeptrisXPathResult {
    _private: [u8; 0],
}
#[repr(C)]
pub struct LeptrisSAXParser {
    _private: [u8; 0],
}
#[repr(C)]
pub struct LeptrisXsdSchema {
    _private: [u8; 0],
}
#[repr(C)]
pub struct LeptrisSerializeOptions {
    _private: [u8; 0],
}
#[repr(C)]
pub struct LeptrisNamespace {
    _private: [u8; 0],
}
/// The capability descriptor (leptris#1905): `leptris_capabilities()`
/// returns this by value — bits append per the engine's contract.
#[repr(C)]
pub struct LeptrisCapabilities {
    pub caps: u64,
    pub descriptor_version: u32,
}

/* Public enums (values per include/leptris/types.h and xpath.h). */
pub type LeptrisNodeKind = c_int;
pub const LEPTRIS_NODE_TYPE_ELEMENT: c_int = 0;
pub const LEPTRIS_NODE_TYPE_TEXT: c_int = 1;
pub const LEPTRIS_NODE_TYPE_COMMENT: c_int = 2;
pub const LEPTRIS_NODE_TYPE_CDATA: c_int = 3;
pub const LEPTRIS_NODE_TYPE_PI: c_int = 4;
pub const LEPTRIS_NODE_TYPE_DOCTYPE: c_int = 5;
pub const LEPTRIS_NODE_TYPE_ATTRIBUTE: c_int = 6;

pub type LeptrisXPathNodeKind = c_int;
pub const LEPTRIS_XPATH_NODE_ELEMENT: c_int = 0;
pub const LEPTRIS_XPATH_NODE_ATTRIBUTE: c_int = 1;
pub const LEPTRIS_XPATH_NODE_TEXT: c_int = 2;
pub const LEPTRIS_XPATH_NODE_OTHER: c_int = 3;

pub type LeptrisXPathResultType = c_int;
pub const LEPTRIS_XPATH_NODESET: c_int = 0;
pub const LEPTRIS_XPATH_BOOLEAN: c_int = 1;
pub const LEPTRIS_XPATH_NUMBER: c_int = 2;
pub const LEPTRIS_XPATH_STRING: c_int = 3;

extern "C" {
    /* Document / parse */
    pub fn leptris_parse_string(
        xml: *const c_char,
        length: usize,
        status: *mut LeptrisStatus,
    ) -> *mut LeptrisDocument;
    pub fn leptris_document_free(doc: *mut LeptrisDocument);
    pub fn leptris_document_root(doc: *mut LeptrisDocument) -> *mut LeptrisElement;
    pub fn leptris_document_serialize(
        doc: *mut LeptrisDocument,
        options: *mut LeptrisSerializeOptions, /* NULL = defaults */
    ) -> *mut c_char;

    /* Element */
    pub fn leptris_element_name(elem: *mut LeptrisElement) -> *const c_char;
    pub fn leptris_element_text(elem: *mut LeptrisElement) -> *const c_char;
    pub fn leptris_text_node_get_content(node: LeptrisNodeRef) -> *mut c_char;
    pub fn leptris_element_child_count(elem: *mut LeptrisElement) -> usize;
    pub fn leptris_element_first_child(
        elem: *mut LeptrisElement,
        name: *const c_char,
    ) -> *mut LeptrisElement;
    pub fn leptris_element_first_child_any(elem: *mut LeptrisElement) -> *mut LeptrisElement;
    pub fn leptris_element_next_sibling_any(elem: *mut LeptrisElement) -> *mut LeptrisElement;

    /* Attribute iteration (v1.1.0 linked-list face) */
    pub fn leptris_element_first_attribute(elem: *mut LeptrisElement) -> *mut LeptrisAttribute;
    pub fn leptris_attribute_next(attr: *mut LeptrisAttribute) -> *mut LeptrisAttribute;
    pub fn leptris_attribute_get_name(attr: *mut LeptrisAttribute) -> *const c_char;
    pub fn leptris_attribute_get_value(
        elem: *mut LeptrisElement,
        attr: *mut LeptrisAttribute,
    ) -> *const c_char;

    /* XPath */
    pub fn leptris_xpath_eval(
        doc: *mut LeptrisDocument,
        context: *mut LeptrisElement,
        expression: *const c_char,
    ) -> *mut LeptrisXPathResult;
    pub fn leptris_xpath_result_type(result: *mut LeptrisXPathResult) -> LeptrisXPathResultType;
    pub fn leptris_xpath_result_count(result: *mut LeptrisXPathResult) -> usize;
    pub fn leptris_xpath_result_get(
        result: *mut LeptrisXPathResult,
        index: usize,
    ) -> *mut LeptrisElement;
    pub fn leptris_xpath_result_get_node(
        result: *mut LeptrisXPathResult,
        index: usize,
    ) -> *mut LeptrisNode;
    pub fn leptris_xpath_result_node_kind(
        result: *mut LeptrisXPathResult,
        index: usize,
    ) -> LeptrisXPathNodeKind;
    pub fn leptris_xpath_result_node_name(
        result: *mut LeptrisXPathResult,
        index: usize,
    ) -> *const c_char;
    pub fn leptris_xpath_result_node_value(
        result: *mut LeptrisXPathResult,
        index: usize,
    ) -> *const c_char;
    pub fn leptris_xpath_result_number(result: *mut LeptrisXPathResult) -> f64;
    pub fn leptris_xpath_result_string(result: *mut LeptrisXPathResult) -> *mut c_char;
    pub fn leptris_xpath_result_free(result: *mut LeptrisXPathResult);
    pub fn leptris_free_string(s: *mut c_char);

    /* XSD (leptris#1900 wasm face; the full mirror lives in the
     * hand-maintained surface above once native wrappers adopt it). */
    pub fn leptris_xsd_compile(
        xsd: *const c_char,
        len: usize,
        status: *mut LeptrisStatus,
    ) -> *mut LeptrisXsdSchema;
    pub fn leptris_xsd_validate(schema: *mut LeptrisXsdSchema, doc: *mut LeptrisDocument) -> c_int;
    pub fn leptris_xsd_error(schema: *mut LeptrisXsdSchema) -> *const c_char;
    pub fn leptris_xsd_error_count(schema: *mut LeptrisXsdSchema) -> usize;
    pub fn leptris_xsd_error_at(schema: *mut LeptrisXsdSchema, i: usize) -> *const c_char;
    pub fn leptris_xsd_free(schema: *mut LeptrisXsdSchema);

    /* Tier-2 wasm faces (leptris-rs#22): everything below is served
     * by existing engine ABI — this block binds it for the wasm
     * face. The full mirror joins the hand-maintained surface when
     * native wrappers adopt these. */
    pub fn leptris_element_namespace(elem: *mut LeptrisElement) -> *mut LeptrisNamespace;
    pub fn leptris_namespace_uri(ns: *mut LeptrisNamespace) -> *const c_char;
    pub fn leptris_element_prefix(elem: *mut LeptrisElement) -> *const c_char;
    pub fn leptris_element_attribute(
        elem: *mut LeptrisElement,
        name: *const c_char,
    ) -> *const c_char;
    pub fn leptris_element_attribute_count(elem: *mut LeptrisElement) -> usize;
    pub fn leptris_element_attributes_snapshot(
        elem: *mut LeptrisElement,
        names: *mut c_char,
        names_cap: usize,
        name_offsets: *mut usize,
        values: *mut c_char,
        values_cap: usize,
        value_offsets: *mut usize,
        ns_prefixes: *mut c_char,
        prefixes_cap: usize,
        prefix_offsets: *mut usize,
        ns_uris: *mut c_char,
        uris_cap: usize,
        uri_offsets: *mut usize,
    ) -> usize;
    pub fn leptris_xsd_compile_set(
        paths: *const *const c_char,
        count: usize,
        status: *mut LeptrisStatus,
    ) -> *mut LeptrisXsdSchema;
    pub fn leptris_parse_html_string(
        html: *const c_char,
        len: usize,
        status: *mut LeptrisStatus,
    ) -> *mut LeptrisDocument;
    pub fn leptris_capabilities() -> LeptrisCapabilities;

    /* Errors */
    pub fn leptris_error_message(status: LeptrisStatus) -> *const c_char;

    /* SAX */
    pub fn leptris_sax_parse(
        xml: *const c_char,
        len: usize,
        handler: *mut LeptrisSAXHandler,
        user_data: *mut core::ffi::c_void,
    ) -> c_int;
}

/// Field order must match `struct LeptrisSAXHandler` in
/// `include/leptris/sax/sax.h` exactly.
#[repr(C)]
pub struct LeptrisSAXHandler {
    pub start_document: Option<unsafe extern "C" fn(user_data: *mut core::ffi::c_void)>,
    pub end_document: Option<unsafe extern "C" fn(user_data: *mut core::ffi::c_void)>,
    pub start_element: Option<
        unsafe extern "C" fn(
            user_data: *mut core::ffi::c_void,
            name: *const c_char,
            attrs: *const *const c_char,
        ),
    >,
    pub end_element:
        Option<unsafe extern "C" fn(user_data: *mut core::ffi::c_void, name: *const c_char)>,
    pub characters: Option<
        unsafe extern "C" fn(user_data: *mut core::ffi::c_void, text: *const c_char, len: usize),
    >,
    pub comment:
        Option<unsafe extern "C" fn(user_data: *mut core::ffi::c_void, comment: *const c_char)>,
    pub cdata:
        Option<unsafe extern "C" fn(user_data: *mut core::ffi::c_void, cdata: *const c_char)>,
    pub processing_instruction: Option<
        unsafe extern "C" fn(
            user_data: *mut core::ffi::c_void,
            target: *const c_char,
            data: *const c_char,
        ),
    >,
    pub start_prefix_mapping: Option<
        unsafe extern "C" fn(
            user_data: *mut core::ffi::c_void,
            prefix: *const c_char,
            uri: *const c_char,
        ),
    >,
    pub end_prefix_mapping:
        Option<unsafe extern "C" fn(user_data: *mut core::ffi::c_void, prefix: *const c_char)>,
    pub error: Option<
        unsafe extern "C" fn(
            user_data: *mut core::ffi::c_void,
            message: *const c_char,
            line: c_int,
            column: c_int,
        ),
    >,
}
pub type LeptrisNodeRef = *mut LeptrisNode;

/* ---- node reads (#15 face 1) — the mirror already carries
 * element_text; node_get_type + the mixed-children face are new */
extern "C" {
    pub fn leptris_node_get_type(node: LeptrisNodeRef) -> c_int;
    pub fn leptris_node_children(
        parent: LeptrisNodeRef,
        out_nodes: *mut LeptrisNodeRef,
        max_count: usize,
    ) -> usize;
    pub fn leptris_element_first_child_element(elem: *mut LeptrisElement) -> *mut LeptrisElement;
}

/* ---- xpath result shapes (#15 face 2) — get_nodes_ex is the one
 * the mirror lacked (node_kind/name/value/boolean/number exist) */
extern "C" {
    pub fn leptris_xpath_result_get_nodes_ex(
        result: *mut LeptrisXPathResult,
        out_nodes: *mut LeptrisNodeRef,
        out_kinds: *mut c_int,
        max_count: usize,
    ) -> usize;
    pub fn leptris_xpath_result_boolean(result: *mut LeptrisXPathResult) -> c_int;
}

/* ---- write face (#15 face 3) ---- */
extern "C" {
    pub fn leptris_element_new_with_attributes(
        doc: *mut LeptrisDocument,
        name: *const c_char,
        attr_names: *const *const c_char,
        attr_values: *const *const c_char,
        attr_count: usize,
    ) -> *mut LeptrisElement;
    pub fn leptris_text_node_create(
        doc: *mut LeptrisDocument,
        content: *const c_char,
    ) -> LeptrisNodeRef;
    pub fn leptris_element_set_attribute(
        elem: *mut LeptrisElement,
        name: *const c_char,
        value: *const c_char,
    ) -> c_int;
    pub fn leptris_element_add_child_addr_installed(
        parent_addr: usize,
        child_addr: usize,
        installed_addr: *mut usize,
    ) -> c_int;
    pub fn leptris_document_set_root(doc: *mut LeptrisDocument, elem: *mut LeptrisElement)
        -> c_int;
}

/* ---- wasm codec faces (moxml-ts#9): comment/cdata/PI creation,
 * attribute removal, per-call parse options, parse-error position.
 * All served by the engine's public header (src/include/leptris.h,
 * leptris/types.h, leptris/error.h). */
extern "C" {
    pub fn leptris_comment_node_create(
        doc: *mut LeptrisDocument,
        content: *const c_char,
    ) -> LeptrisNodeRef;
    pub fn leptris_cdata_node_create(
        doc: *mut LeptrisDocument,
        content: *const c_char,
    ) -> LeptrisNodeRef;
    pub fn leptris_pi_node_create(
        doc: *mut LeptrisDocument,
        target: *const c_char,
        data: *const c_char,
    ) -> LeptrisNodeRef;
    pub fn leptris_element_remove_attribute(
        elem: *mut LeptrisElement,
        name: *const c_char,
    ) -> LeptrisStatus;
    pub fn leptris_parse_string_ex(
        xml: *const c_char,
        length: usize,
        options: *const LeptrisParseOptions,
        status: *mut LeptrisStatus,
    ) -> *mut LeptrisDocument;
    pub fn leptris_last_error() -> *const c_char;
    pub fn leptris_last_error_position(line: *mut c_int, column: *mut c_int);
}

/// Mirror of `LeptrisParseOptions` (leptris/types.h). `flags` is the
/// `LeptrisParseFlags` bitset (an enum, int-sized in the C ABI).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct LeptrisParseOptions {
    pub flags: c_int,
    /// -1 keep thread default, 0 lenient, 1 strict W3C.
    pub strict_mode: c_int,
    /// 0 = engine default (256), >0 = cap.
    pub max_depth: c_int,
    /// Non-zero: a failed parse yields an empty document.
    pub recover: c_int,
}
