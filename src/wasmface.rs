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

/// A compiled XSD schema. Owns its engine-side memory; validate
/// errors enumerate through `last_error_count`/`last_error` on the
/// document's schema handle — surfaced here as the Vec from
/// [`validate`].
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
