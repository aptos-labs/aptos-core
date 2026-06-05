// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! On-demand annotation of a storable closure's captured arguments by the resource
//! viewer. The captured values are stored with raw (nameless) layouts, but the viewer
//! resolves their real types on-demand from the function signature, so the annotation
//! carries proper struct and field names. These tests cover named structs, multiple
//! captures, generic instantiation, captures from another module, and a non-captured
//! reference parameter.

use crate::{assert_success, MoveHarness};
use aptos_package_builder::PackageBuilder;
use aptos_resource_viewer::{AnnotatedMoveValue, AptosValueAnnotator};
use aptos_types::account_address::AccountAddress;
use claims::assert_ok;
use move_core_types::{
    ability::AbilitySet,
    function::{ClosureMask, MoveClosure},
    identifier::Identifier,
    language_storage::{FunctionParamOrReturnTag, FunctionTag, ModuleId, TypeTag},
    value::{MoveStruct, MoveStructLayout, MoveTypeLayout, MoveValue},
};

const OTHER_SOURCE: &str = r#"
module 0xcafe::other {
    struct Foreign has copy, drop, store { z: u64 }
}
"#;

const M_SOURCE: &str = r#"
module 0xcafe::m {
    use 0xcafe::other::Foreign;

    struct S has copy, drop, store { x: u64, y: bool }
    struct Wrapper<T: copy + drop + store> has copy, drop, store { val: T }

    public fun cap_named(s: S, k: u64): u64 { s.x + k }
    public fun cap_two(a: S, b: u64): u64 { a.x + b }
    public fun cap_generic<T: copy + drop + store>(_w: Wrapper<T>, k: u64): u64 { k }
    public fun cap_foreign(_f: Foreign, k: u64): u64 { k }
    public fun cap_with_ref(s: S, _r: &u64): u64 { s.x }
}
"#;

fn publish() -> (MoveHarness, AccountAddress) {
    let account_address = AccountAddress::from_hex_literal("0xcafe").unwrap();
    let mut builder = PackageBuilder::new("M");
    builder.add_source("other.move", OTHER_SOURCE);
    builder.add_source("m.move", M_SOURCE);
    let path = builder.write_to_temp().unwrap();

    let mut h = MoveHarness::new();
    let acc = h.new_account_at(account_address);
    assert_success!(h.publish_package(&acc, path.path()));
    (h, account_address)
}

/// A function type tag standing for the residual closure type. Its exact shape does
/// not affect captured-argument annotation, which resolves layouts from the function.
fn fun_tag() -> TypeTag {
    TypeTag::Function(Box::new(FunctionTag {
        args: vec![FunctionParamOrReturnTag::Value(TypeTag::U64)],
        results: vec![FunctionParamOrReturnTag::Value(TypeTag::U64)],
        abilities: AbilitySet::PUBLIC_FUNCTIONS,
    }))
}

/// A runtime struct layout + value built from per-field `(layout, value)` pairs. This
/// is the nameless form a closure stores its captured struct as.
fn runtime_struct(fields: Vec<(MoveTypeLayout, MoveValue)>) -> (MoveTypeLayout, MoveValue) {
    let (layouts, values): (Vec<_>, Vec<_>) = fields.into_iter().unzip();
    (
        MoveTypeLayout::new_struct(MoveStructLayout::Runtime(layouts)),
        MoveValue::Struct(MoveStruct::Runtime(values)),
    )
}

/// Serializes a closure over `addr::module::fun<ty_args>` capturing `captured` (each a
/// `(layout, value)` pair, as stored on chain).
fn serialize_closure(
    addr: AccountAddress,
    module: &str,
    fun: &str,
    ty_args: Vec<TypeTag>,
    mask: ClosureMask,
    captured: Vec<(MoveTypeLayout, MoveValue)>,
) -> Vec<u8> {
    MoveValue::Closure(Box::new(MoveClosure {
        module_id: ModuleId::new(addr, Identifier::new(module).unwrap()),
        fun_id: Identifier::new(fun).unwrap(),
        ty_args,
        mask,
        captured,
    }))
    .simple_serialize()
    .unwrap()
}

/// Annotates a closure blob and returns its captured values.
fn annotate_captured(h: &MoveHarness, blob: &[u8]) -> Vec<AnnotatedMoveValue> {
    let annotator = AptosValueAnnotator::new(h.executor.state_store());
    match assert_ok!(annotator.view_value(&fun_tag(), blob)) {
        AnnotatedMoveValue::Closure(c) => c.captured,
        other => panic!("expected a closure, got {:?}", other),
    }
}

fn annotate_single_captured(h: &MoveHarness, blob: &[u8]) -> AnnotatedMoveValue {
    let captured = annotate_captured(h, blob);
    assert_eq!(captured.len(), 1, "expected exactly one captured value");
    captured.into_iter().next().unwrap()
}

/// Returns `(struct_name, [(field_name, debug_value)])` for a captured struct.
fn struct_fields(value: &AnnotatedMoveValue) -> (String, Vec<(String, String)>) {
    match value {
        AnnotatedMoveValue::Struct(s) => (
            s.ty_tag.name.as_str().to_string(),
            s.value
                .iter()
                .map(|(f, v)| (f.as_str().to_string(), format!("{:?}", v)))
                .collect(),
        ),
        other => panic!("expected a struct, got {:?}", other),
    }
}

/// The captured struct `S { x: 42, y: true }` annotates with its real field names.
#[test]
fn named_struct_captured_arg() {
    let (h, addr) = publish();

    let captured = runtime_struct(vec![
        (MoveTypeLayout::U64, MoveValue::U64(42)),
        (MoveTypeLayout::Bool, MoveValue::Bool(true)),
    ]);
    let blob = serialize_closure(addr, "m", "cap_named", vec![], ClosureMask::new(0b1), vec![
        captured,
    ]);

    let (name, fields) = struct_fields(&annotate_single_captured(&h, &blob));
    assert_eq!(name, "S");
    assert_eq!(fields, vec![
        ("x".to_string(), "U64(42)".to_string()),
        ("y".to_string(), "Bool(true)".to_string()),
    ]);
}

/// Two captured arguments of different types (`S { x: 42, y: true }` and `7u64`) are
/// annotated, named, and in order.
#[test]
fn multiple_captured_args() {
    let (h, addr) = publish();

    let s = runtime_struct(vec![
        (MoveTypeLayout::U64, MoveValue::U64(42)),
        (MoveTypeLayout::Bool, MoveValue::Bool(true)),
    ]);
    let blob = serialize_closure(addr, "m", "cap_two", vec![], ClosureMask::new(0b11), vec![
        s,
        (MoveTypeLayout::U64, MoveValue::U64(7)),
    ]);

    let captured = annotate_captured(&h, &blob);
    assert_eq!(captured.len(), 2);
    let (name, fields) = struct_fields(&captured[0]);
    assert_eq!(name, "S");
    assert_eq!(fields, vec![
        ("x".to_string(), "U64(42)".to_string()),
        ("y".to_string(), "Bool(true)".to_string()),
    ]);
    assert!(matches!(&captured[1], AnnotatedMoveValue::U64(v) if *v == 7));
}

/// A generic function's captured argument is resolved with the closure's type
/// arguments substituted in: `Wrapper<u64> { val: 7 }`.
#[test]
fn generic_capture_substitutes_type_arguments() {
    let (h, addr) = publish();

    let captured = runtime_struct(vec![(MoveTypeLayout::U64, MoveValue::U64(7))]);
    let blob = serialize_closure(
        addr,
        "m",
        "cap_generic",
        vec![TypeTag::U64],
        ClosureMask::new(0b1),
        vec![captured],
    );

    match annotate_single_captured(&h, &blob) {
        AnnotatedMoveValue::Struct(s) => {
            assert_eq!(s.ty_tag.name.as_str(), "Wrapper");
            assert_eq!(s.ty_tag.type_args, vec![TypeTag::U64]);
            let (field, field_value) = &s.value[0];
            assert_eq!(field.as_str(), "val");
            assert!(matches!(field_value, AnnotatedMoveValue::U64(v) if *v == 7));
        },
        other => panic!("expected named struct, got {:?}", other),
    }
}

/// Only the captured parameter is resolved. A non-captured reference parameter
/// (`_r: &u64`) must be skipped, not resolved (references cannot be annotated).
#[test]
fn capture_skips_non_captured_reference_param() {
    let (h, addr) = publish();

    let captured = runtime_struct(vec![
        (MoveTypeLayout::U64, MoveValue::U64(5)),
        (MoveTypeLayout::Bool, MoveValue::Bool(false)),
    ]);
    let blob = serialize_closure(
        addr,
        "m",
        "cap_with_ref",
        vec![],
        ClosureMask::new(0b1),
        vec![captured],
    );

    let (name, fields) = struct_fields(&annotate_single_captured(&h, &blob));
    assert_eq!(name, "S");
    assert_eq!(fields[0].0, "x");
}

/// A captured struct defined in a different module than the function is resolved
/// across the module boundary: `other::Foreign { z: 9 }`.
#[test]
fn captured_struct_from_other_module() {
    let (h, addr) = publish();

    let captured = runtime_struct(vec![(MoveTypeLayout::U64, MoveValue::U64(9))]);
    let blob = serialize_closure(
        addr,
        "m",
        "cap_foreign",
        vec![],
        ClosureMask::new(0b1),
        vec![captured],
    );

    match annotate_single_captured(&h, &blob) {
        AnnotatedMoveValue::Struct(s) => {
            assert_eq!(s.ty_tag.module.as_str(), "other");
            assert_eq!(s.ty_tag.name.as_str(), "Foreign");
            let (field, field_value) = &s.value[0];
            assert_eq!(field.as_str(), "z");
            assert!(matches!(field_value, AnnotatedMoveValue::U64(v) if *v == 9));
        },
        other => panic!("expected named struct, got {:?}", other),
    }
}
