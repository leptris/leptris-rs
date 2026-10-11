//! Shared descriptor-plan ABI for the leptris engine family.
//!
//! One crate per engine (`leptris` XML, `yeptris` YAML, `teptris`
//! TOML/JSON) targets this record layout, so hosts build plan rows
//! once and every engine's compiled-descriptor walk speaks the same
//! shape. The types mirror `src/include/leptris/descriptor.h` in the
//! leptris engine — field order is the ABI; NEVER renumber or
//! reorder. Additive trailing fields only, guarded by an ABI version
//! bump.
//!
//! Layout is verified at runtime against the engine's own
//! introspection (`leptris_plan_*_struct_size`) by the linking
//! binding crate's tests.
//!
//! This crate is `no_std`: types and constants only, plus the FFI
//! declarations behind the `link` feature.

#![no_std]
#![allow(non_camel_case_types, non_snake_case)]

use core::ffi::c_char;

/// Must equal `leptris_plan_abi_version()` reported by the engine.
pub const PLAN_ABI_VERSION: u32 = 1;

/// Child / attribute plan kinds. Append-only.
pub type PlanKind = u8;
pub const PLAN_KIND_SCALAR: PlanKind = 1;
pub const PLAN_KIND_COLLECTION: PlanKind = 2;
pub const PLAN_KIND_NESTED: PlanKind = 3;
pub const PLAN_KIND_RAW: PlanKind = 4;
pub const PLAN_KIND_CONTENT: PlanKind = 5;
pub const PLAN_KIND_CALLBACK: PlanKind = 6;
/// #1552: catch-all row — binds every element child no named
/// sibling row bound; emits one COLLECTION echoing the row
/// wire_name/type_tag; `child_plan_index >= 0` walks members
/// through that plan, else members are RAW serialized subtrees.
pub const PLAN_KIND_WILDCARD: PlanKind = 7;

/// Element plan flags.
pub const PLAN_FLAG_MIXED_CONTENT: u32 = 0x1;
pub const PLAN_FLAG_ORDERED: u32 = 0x2;
pub const PLAN_FLAG_CDATA: u32 = 0x4;
pub const PLAN_FLAG_NS_LENIENT: u32 = 0x8;
pub const PLAN_FLAG_EMIT_ORDER_SPINE: u32 = 0x10;

/// Namespace matching form for binding an element or attribute.
pub type PlanNsForm = u8;
pub const PLAN_NS_NONE: PlanNsForm = 0;
pub const PLAN_NS_EXACT: PlanNsForm = 1;
pub const PLAN_NS_ANY: PlanNsForm = 2;
/// #1560: matches the UNWRITTEN spelling — no written prefix,
/// regardless of the effective namespace URI. An unprefixed child
/// binds under both a namespace-less document and a default-xmlns
/// document; prefixed spellings never bind. Element rows only
/// (unprefixed attributes have no namespace by XML rules).
pub const PLAN_NS_UNQUALIFIED: PlanNsForm = 3;

/// Result node kinds (the value tree a walk returns).
pub type PlanValueKind = i32;
pub const PLAN_VALUE_ELEMENT: PlanValueKind = 0;
pub const PLAN_VALUE_SCALAR: PlanValueKind = 1;
pub const PLAN_VALUE_COLLECTION: PlanValueKind = 2;
pub const PLAN_VALUE_RAW: PlanValueKind = 3;
pub const PLAN_VALUE_CALLBACK: PlanValueKind = 4;

/// Attribute-match row: requires every (name, expected value) pair.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct AttrPredicate {
    /// XML attribute name as it appears on the wire.
    pub wire_name: *const c_char,
    /// String-equal match (non-null); host-owned.
    pub expected_value: *const c_char,
}

/// Attribute binding row.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct AttrPlan {
    /// XML attribute name as it appears on the wire.
    pub wire_name: *const c_char,
    /// `PlanKind`: SCALAR | COLLECTION | CALLBACK.
    pub kind: u8,
    /// Host-defined; echoed back verbatim.
    pub type_tag: u8,
    /// Number of `predicates` rows (0 = historical behavior).
    pub predicate_count: u16,
    pub pad_pred: u16,
    pub predicates: *const AttrPredicate,
    /// Attribute-level namespace form; `wire_name` is the LOCAL
    /// name when set. NONE (0) keeps wire-name lookup.
    pub ns_form: u8,
    pub pad_ns: u8,
    /// `PLAN_NS_EXACT` only.
    pub ns_uri: *const c_char,
    /// #1551: intended wire prefix for serialized output
    /// (`leptris_plan_serialize`); meaningful with EXACT.
    pub ns_prefix: *const c_char,
}

/// Child binding row.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ChildPlan {
    /// XML element name as it appears on the wire: the prefixed
    /// form ("w:b") binds that literal prefix+local pair; the bare
    /// local form ("b") binds by local name and lets `ns_form` do
    /// the namespace work.
    pub wire_name: *const c_char,
    pub kind: u8,
    pub type_tag: u8,
    /// NESTED: index into `PlanSpec::plans`; -1 otherwise.
    pub child_plan_index: i32,
    /// Rule-level namespace form, consulted at child-match time.
    pub ns_form: u8,
    pub pad0: u8,
    /// `PLAN_NS_EXACT` only.
    pub ns_uri: *const c_char,
    /// Element-side predicates; same semantics as `AttrPredicate`.
    pub predicate_count: u16,
    pub pad_pred: u16,
    pub predicates: *const AttrPredicate,
    /// #1551: intended wire prefix for serialized output
    /// (`leptris_plan_serialize`); meaningful with EXACT.
    pub ns_prefix: *const c_char,
}

/// One element plan: the attribute and child rows bound to a name.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ElementPlan {
    /// Local name this plan binds; matching uses the CHILD row's
    /// `wire_name`.
    pub element_name: *const c_char,
    pub ns_form: u8,
    pub pad0: u8,
    /// `PLAN_NS_EXACT` only.
    pub ns_uri: *const c_char,
    pub attribute_count: u32,
    pub child_count: u32,
    pub attribute_plans: *const AttrPlan,
    pub child_plans: *const ChildPlan,
    /// `PLAN_FLAG_*`.
    pub flags: u16,
    pub pad1: u16,
    /// #1551: the root plan's intended wire prefix for serialized
    /// output (`leptris_plan_serialize`); meaningful with EXACT.
    pub ns_prefix: *const c_char,
}

/// Top-level plan specification handed to the engine.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct PlanSpec {
    /// Must equal the engine's `leptris_plan_abi_version()`.
    pub abi_version: u32,
    /// `plans[0]` is the root plan applied to the walk context.
    pub plan_count: u32,
    pub plans: *const ElementPlan,
}

#[cfg(feature = "link")]
pub mod ffi {
    //! Engine FFI for the descriptor surface. Enable this feature on
    //! exactly one crate in the graph (the binding that links an
    //! engine).

    use super::PlanSpec;
    use core::ffi::{c_char, c_void};

    /// Opaque engine document handle (leptris: `LeptrisDocument`).
    pub type DocumentHandle = *mut c_void;
    /// Opaque engine element handle (leptris: `LeptrisElement`).
    pub type ElementHandle = *mut c_void;
    /// Status out-parameter (leptris: `LeptrisStatus*`).
    pub type StatusPtr = *mut i32;
    /// Opaque compiled plan.
    pub type Plan = *mut c_void;
    /// Opaque walk result value.
    pub type PlanResult = *const c_void;

    extern "C" {
        pub fn leptris_plan_abi_version() -> u32;
        pub fn leptris_plan_spec_struct_size() -> usize;
        pub fn leptris_plan_element_row_size() -> usize;
        pub fn leptris_plan_child_row_size() -> usize;
        pub fn leptris_plan_attr_row_size() -> usize;
        pub fn leptris_plan_predicate_row_size() -> usize;

        pub fn leptris_plan_build(spec: *const PlanSpec, status: StatusPtr) -> Plan;
        pub fn leptris_plan_free(plan: Plan);

        pub fn leptris_plan_walk(
            doc: DocumentHandle,
            ctx: ElementHandle,
            plan: Plan,
            status: StatusPtr,
        ) -> PlanResult;
        pub fn leptris_plan_materialize(
            source: *const c_char,
            source_len: usize,
            plan: Plan,
            status: StatusPtr,
        ) -> PlanResult;
        pub fn leptris_plan_result_free(result: PlanResult);

        pub fn leptris_plan_value_kind(v: PlanResult) -> i32;
        pub fn leptris_plan_value_name(v: PlanResult) -> *const c_char;
        pub fn leptris_plan_value_type_tag(v: PlanResult) -> u8;
        pub fn leptris_plan_value_string(v: PlanResult) -> *const c_char;
        pub fn leptris_plan_value_length(v: PlanResult) -> usize;
        pub fn leptris_plan_value_position(v: PlanResult) -> usize;
        pub fn leptris_plan_value_node_kind(v: PlanResult) -> u8;
        pub fn leptris_plan_value_order_index(v: PlanResult) -> u32;
        pub fn leptris_plan_value_count(v: PlanResult) -> usize;
        pub fn leptris_plan_value_at(v: PlanResult, i: usize) -> PlanResult;
        pub fn leptris_plan_value_children_snapshot(
            v: PlanResult,
            names_blob: *mut c_char,
            blob_cap: usize,
            name_offsets: *mut usize,
            type_tags: *mut u8,
            child_handles: *mut PlanResult,
        ) -> usize;
        pub fn leptris_plan_value_attribute(
            v: PlanResult,
            wire_name: *const c_char,
        ) -> *const c_char;
        pub fn leptris_plan_value_int(v: PlanResult, out: *mut i64) -> i32;
        pub fn leptris_plan_value_float(v: PlanResult, out: *mut f64) -> i32;
        pub fn leptris_plan_value_bool(v: PlanResult, out: *mut i32) -> i32;
    }
}

/// #294: the safe builder + walk layer — a host declares a plan
/// once, the builder owns every C string until the engine's build
/// returns (and for the descriptor's lifetime), and a walk's result
/// frees on Drop. Value readers borrow the walk, so no returned
/// `&str` can outlive the engine memory behind it.
#[cfg(feature = "std")]
pub mod safe {
    use crate as descriptor;
    use crate::ffi;
    extern crate std;
    use std::ffi::{c_char, CStr, CString};
    use std::string::{String, ToString};
    use std::vec::Vec;

    /// Namespace binding form for a node or attribute row.
    #[derive(Clone, Debug)]
    pub enum Ns {
        /// Wire-name matching only.
        None,
        /// Bind by (uri, optional serialization prefix).
        Exact { uri: String, prefix: Option<String> },
        /// Engine #1560: match the unprefixed spelling only.
        Unqualified,
    }

    impl Ns {
        fn form(&self) -> descriptor::PlanNsForm {
            match self {
                Ns::None => descriptor::PLAN_NS_NONE,
                Ns::Exact { .. } => descriptor::PLAN_NS_EXACT,
                Ns::Unqualified => descriptor::PLAN_NS_UNQUALIFIED,
            }
        }
    }

    /// One attribute binding row.
    #[derive(Clone, Debug)]
    pub struct AttrRow {
        pub name: String,
        pub type_tag: u8,
        /// (name, expected) pairs — every pair must match.
        pub predicates: Vec<(String, String)>,
        pub ns: Ns,
    }

    impl AttrRow {
        pub fn new(name: &str, type_tag: u8) -> AttrRow {
            AttrRow {
                name: name.to_string(),
                type_tag,
                predicates: Vec::new(),
                ns: Ns::None,
            }
        }
        /// Bind only when the attribute's value equals `expected`.
        pub fn predicate(mut self, expected: &str) -> AttrRow {
            self.predicates
                .push((self.name.clone(), expected.to_string()));
            self
        }
        pub fn ns(mut self, ns: Ns) -> AttrRow {
            self.ns = ns;
            self
        }
    }

    /// One child binding row.
    #[derive(Clone, Debug)]
    pub enum ChildSpec {
        Scalar(u8),
        Collection(u8),
        Raw,
        Nested(NodeSpec),
    }

    /// One element plan under construction.
    #[derive(Clone, Debug)]
    pub struct NodeSpec {
        pub name: String,
        /// Host-defined tag echoed back on every value bound by the
        /// CHILD row of this nested node (ElementPlan itself has no
        /// tag field — the root value's tag is 0).
        pub tag: u8,
        pub ns: Ns,
        pub attrs: Vec<AttrRow>,
        pub children: Vec<(String, ChildSpec)>,
    }

    impl NodeSpec {
        pub fn new(name: &str) -> NodeSpec {
            NodeSpec {
                name: name.to_string(),
                tag: 0,
                ns: Ns::None,
                attrs: Vec::new(),
                children: Vec::new(),
            }
        }
        pub fn tag(mut self, tag: u8) -> NodeSpec {
            self.tag = tag;
            self
        }
        pub fn ns(mut self, ns: Ns) -> NodeSpec {
            self.ns = ns;
            self
        }
        pub fn ns_exact(mut self, uri: &str, prefix: Option<&str>) -> NodeSpec {
            self.ns = Ns::Exact {
                uri: uri.to_string(),
                prefix: prefix.map(|p| p.to_string()),
            };
            self
        }
        pub fn attr(mut self, name: &str, type_tag: u8) -> NodeSpec {
            self.attrs.push(AttrRow::new(name, type_tag));
            self
        }
        pub fn attr_row(mut self, row: AttrRow) -> NodeSpec {
            self.attrs.push(row);
            self
        }
        pub fn scalar(mut self, name: &str, type_tag: u8) -> NodeSpec {
            self.children
                .push((name.to_string(), ChildSpec::Scalar(type_tag)));
            self
        }
        pub fn collection(mut self, name: &str, type_tag: u8) -> NodeSpec {
            self.children
                .push((name.to_string(), ChildSpec::Collection(type_tag)));
            self
        }
        pub fn nested(mut self, name: &str, spec: NodeSpec) -> NodeSpec {
            self.children
                .push((name.to_string(), ChildSpec::Nested(spec)));
            self
        }

        /// Compile the spec: build the engine plan. The returned
        /// [`Descriptor`] owns every anchored string and the plan
        /// handle (freed on drop).
        pub fn build(self) -> Result<Descriptor, i32> {
            let mut anchors: Vec<CString> = Vec::new();
            let mut plans: Vec<descriptor::ElementPlan> = Vec::new();
            let mut attr_rows: Vec<Vec<descriptor::AttrPlan>> = Vec::new();
            let mut child_rows: Vec<Vec<descriptor::ChildPlan>> = Vec::new();
            let mut pred_rows: Vec<Vec<descriptor::AttrPredicate>> = Vec::new();
            register_spec(
                self,
                &mut anchors,
                &mut plans,
                &mut attr_rows,
                &mut child_rows,
                &mut pred_rows,
            );
            let spec = descriptor::PlanSpec {
                abi_version: descriptor::PLAN_ABI_VERSION,
                plan_count: plans.len() as u32,
                plans: plans.as_ptr(),
            };
            let mut status: i32 = 0;
            let plan = unsafe { ffi::leptris_plan_build(&spec, &mut status) };
            if plan.is_null() {
                return Err(status);
            }
            Ok(Descriptor {
                plan,
                anchors,
                plans,
                attr_rows,
                child_rows,
                pred_rows,
            })
        }
    }

    fn anchor(anchors: &mut Vec<CString>, s: &str) -> *const c_char {
        // The engine deep-copies at build; the anchors ride the
        // Descriptor's lifetime regardless — every pointer handed
        // to a row stays valid as long as the row does.
        let c = CString::new(s).expect("plan names are C-safe");
        let p = c.as_ptr();
        anchors.push(c);
        p
    }

    fn register_spec(
        spec: NodeSpec,
        anchors: &mut Vec<CString>,
        plans: &mut Vec<descriptor::ElementPlan>,
        attr_rows: &mut Vec<Vec<descriptor::AttrPlan>>,
        child_rows: &mut Vec<Vec<descriptor::ChildPlan>>,
        pred_rows: &mut Vec<Vec<descriptor::AttrPredicate>>,
    ) {
        // Depth-first registration: children's NESTED plans get
        // their indices before this element's rows are built.
        let this_index = plans.len();
        // Placeholder overwritten below; zeroed repr(C) = null
        // pointers, safe to hand the engine after the real row
        // lands.
        plans.push(unsafe { core::mem::zeroed() });
        attr_rows.push(Vec::new());
        child_rows.push(Vec::new());

        let mut child_plans: Vec<descriptor::ChildPlan> = Vec::new();
        for (name, cs) in &spec.children {
            let kind;
            let type_tag;
            let mut nested_index: i32 = -1;
            let row_ns: Ns;
            match cs {
                ChildSpec::Scalar(t) => {
                    kind = descriptor::PLAN_KIND_SCALAR;
                    type_tag = *t;
                    row_ns = Ns::None;
                }
                ChildSpec::Collection(t) => {
                    kind = descriptor::PLAN_KIND_COLLECTION;
                    type_tag = *t;
                    row_ns = Ns::None;
                }
                ChildSpec::Raw => {
                    kind = descriptor::PLAN_KIND_RAW;
                    type_tag = 0;
                    row_ns = Ns::None;
                }
                ChildSpec::Nested(n) => {
                    kind = descriptor::PLAN_KIND_NESTED;
                    type_tag = n.tag;
                    row_ns = n.ns.clone();
                    register_spec(n.clone(), anchors, plans, attr_rows, child_rows, pred_rows);
                    nested_index = plans.len() as i32 - 1;
                }
            }
            // The CHILD row is what the engine consults at
            // child-match time — it carries the nested node's
            // namespace form and uri (prefix rides the element plan
            // for serialized output).
            // Standard DSL semantics: a child that declares no ns
            // of its own rides its parent's EXACT ns (declare the
            // namespace once on the subtree) — the engine binds the
            // bare local name against the row's EXACT uri.
            let inherit = matches!((&row_ns, &spec.ns), (Ns::None, Ns::Exact { .. }));
            let row_ns = if inherit { spec.ns.clone() } else { row_ns };
            let (ns_uri, ns_prefix) = match &row_ns {
                Ns::Exact { uri, prefix } => {
                    let u = anchor(anchors, uri);
                    let p = prefix
                        .as_ref()
                        .map(|x| anchor(anchors, x))
                        .unwrap_or(core::ptr::null());
                    (u, p)
                }
                _ => (core::ptr::null(), core::ptr::null()),
            };
            child_plans.push(descriptor::ChildPlan {
                wire_name: anchor(anchors, name),
                kind,
                type_tag,
                child_plan_index: nested_index,
                ns_form: row_ns.form(),
                pad0: 0,
                ns_uri,
                predicate_count: 0,
                pad_pred: 0,
                predicates: core::ptr::null(),
                ns_prefix,
            });
        }

        let mut attr_plans: Vec<descriptor::AttrPlan> = Vec::new();
        for a in &spec.attrs {
            let mut preds: Vec<descriptor::AttrPredicate> = Vec::new();
            for (pname, expected) in &a.predicates {
                preds.push(descriptor::AttrPredicate {
                    wire_name: anchor(anchors, pname),
                    expected_value: anchor(anchors, expected),
                });
            }
            let (uri, prefix) = match &a.ns {
                Ns::Exact { uri, prefix } => {
                    let u = anchor(anchors, uri);
                    let p = prefix
                        .as_ref()
                        .map(|x| anchor(anchors, x))
                        .unwrap_or(core::ptr::null());
                    (u, p)
                }
                _ => (core::ptr::null(), core::ptr::null()),
            };
            attr_plans.push(descriptor::AttrPlan {
                wire_name: anchor(anchors, &a.name),
                kind: descriptor::PLAN_KIND_SCALAR,
                type_tag: a.type_tag,
                predicate_count: preds.len() as u16,
                pad_pred: 0,
                predicates: preds.as_ptr(),
                ns_form: a.ns.form(),
                pad_ns: 0,
                ns_uri: uri,
                ns_prefix: prefix,
            });
            pred_rows.push(preds);
        }

        let (ns_uri, ns_prefix) = match &spec.ns {
            Ns::Exact { uri, prefix } => {
                let u = anchor(anchors, uri);
                let p = prefix
                    .as_ref()
                    .map(|x| anchor(anchors, x))
                    .unwrap_or(core::ptr::null());
                (u, p)
            }
            _ => (core::ptr::null(), core::ptr::null()),
        };

        let plan = descriptor::ElementPlan {
            element_name: anchor(anchors, &spec.name),
            ns_form: spec.ns.form(),
            pad0: 0,
            ns_uri,
            attribute_count: attr_plans.len() as u32,
            child_count: child_plans.len() as u32,
            attribute_plans: attr_plans.as_ptr(),
            child_plans: child_plans.as_ptr(),
            flags: 0,
            pad1: 0,
            ns_prefix,
        };
        attr_rows[this_index] = attr_plans;
        child_rows[this_index] = child_plans;
        plans[this_index] = plan;
    }

    /// A compiled descriptor. Owns the engine plan and every
    /// anchored string; dropping it frees the plan.
    pub struct Descriptor {
        plan: ffi::Plan,
        #[allow(dead_code)]
        anchors: Vec<CString>,
        #[allow(dead_code)]
        plans: Vec<descriptor::ElementPlan>,
        #[allow(dead_code)]
        attr_rows: Vec<Vec<descriptor::AttrPlan>>,
        #[allow(dead_code)]
        child_rows: Vec<Vec<descriptor::ChildPlan>>,
        #[allow(dead_code)]
        pred_rows: Vec<Vec<descriptor::AttrPredicate>>,
    }

    impl Descriptor {
        /// Walk `root` under `doc` through this plan. The handles
        /// are opaque engine pointers owned by the caller (from the
        /// linking binding crate); the walk borrows them for the
        /// call only.
        #[allow(clippy::not_unsafe_ptr_arg_deref)]
        pub fn walk(
            &self,
            doc: ffi::DocumentHandle,
            root: ffi::ElementHandle,
        ) -> Result<Walk, i32> {
            let mut status: i32 = 0;
            let r = unsafe { ffi::leptris_plan_walk(doc, root, self.plan, &mut status) };
            if r.is_null() {
                return Err(status);
            }
            Ok(Walk { raw: r })
        }
    }

    impl Drop for Descriptor {
        fn drop(&mut self) {
            unsafe { ffi::leptris_plan_free(self.plan) }
        }
    }

    /// A walk's result tree. Frees on drop; every reader borrows,
    /// so engine memory cannot be reached after the free.
    pub struct Walk {
        raw: ffi::PlanResult,
    }

    fn cstr<'a>(p: *const c_char) -> Option<&'a str> {
        if p.is_null() {
            return None;
        }
        unsafe { CStr::from_ptr(p) }.to_str().ok()
    }

    impl Walk {
        pub fn kind(&self) -> i32 {
            unsafe { ffi::leptris_plan_value_kind(self.raw) }
        }
        pub fn name(&self) -> Option<&str> {
            cstr(unsafe { ffi::leptris_plan_value_name(self.raw) })
        }
        pub fn type_tag(&self) -> u8 {
            unsafe { ffi::leptris_plan_value_type_tag(self.raw) }
        }
        pub fn len(&self) -> usize {
            unsafe { ffi::leptris_plan_value_count(self.raw) }
        }
        pub fn is_empty(&self) -> bool {
            self.len() == 0
        }
        pub fn at(&self, i: usize) -> Option<Value<'_>> {
            if i >= self.len() {
                return None;
            }
            let v = unsafe { ffi::leptris_plan_value_at(self.raw, i) };
            if v.is_null() {
                None
            } else {
                Some(Value {
                    raw: v,
                    _life: core::marker::PhantomData,
                })
            }
        }
    }

    impl Drop for Walk {
        fn drop(&mut self) {
            unsafe { ffi::leptris_plan_result_free(self.raw) }
        }
    }

    /// One value in the walk result tree.
    pub struct Value<'a> {
        raw: ffi::PlanResult,
        _life: core::marker::PhantomData<&'a Walk>,
    }

    impl<'a> Value<'a> {
        pub fn kind(&self) -> i32 {
            unsafe { ffi::leptris_plan_value_kind(self.raw) }
        }
        pub fn name(&self) -> Option<&str> {
            cstr(unsafe { ffi::leptris_plan_value_name(self.raw) })
        }
        pub fn type_tag(&self) -> u8 {
            unsafe { ffi::leptris_plan_value_type_tag(self.raw) }
        }
        /// SCALAR/RAW/CALLBACK content (result-owned; borrows self).
        pub fn string(&self) -> Option<&str> {
            cstr(unsafe { ffi::leptris_plan_value_string(self.raw) })
        }
        pub fn len(&self) -> usize {
            unsafe { ffi::leptris_plan_value_count(self.raw) }
        }
        pub fn is_empty(&self) -> bool {
            self.len() == 0
        }
        pub fn at(&self, i: usize) -> Option<Value<'_>> {
            if i >= self.len() {
                return None;
            }
            let v = unsafe { ffi::leptris_plan_value_at(self.raw, i) };
            if v.is_null() {
                None
            } else {
                Some(Value {
                    raw: v,
                    _life: core::marker::PhantomData,
                })
            }
        }
        /// Attribute value by wire name (borrow of engine memory
        /// tied to the walk's lifetime).
        pub fn attribute(&self, name: &str) -> Option<&'a str> {
            let c = CString::new(name).ok()?;
            cstr(unsafe { ffi::leptris_plan_value_attribute(self.raw, c.as_ptr()) })
        }
    }
}
