// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

//! Contains tests for serialization

use crate::{
    delayed_values::delayed_field_id::DelayedFieldID,
    value_serde::{serialize_and_allow_delayed_values, serialized_size_allowing_delayed_values},
    values::{values_impl, Struct, Value},
};
use claims::{assert_err, assert_ok, assert_some};
use move_core_types::{
    account_address::AccountAddress,
    u256,
    value::{IdentifierMappingKind, MoveStruct, MoveStructLayout, MoveTypeLayout, MoveValue},
};
use serde::{Deserialize, Serialize};
use std::iter;

fn test_layout() -> MoveTypeLayout {
    MoveTypeLayout::Struct(MoveStructLayout::RuntimeVariants(vec![
        vec![MoveTypeLayout::U64],
        vec![],
        vec![MoveTypeLayout::Bool, MoveTypeLayout::U32],
    ]))
}

// ---------------------------------------------------------------------------
// Serialization round trip tests

#[test]
fn enum_round_trip_move_value() {
    let layout = test_layout();
    let good_values = vec![
        MoveValue::Struct(MoveStruct::RuntimeVariant(0, vec![MoveValue::U64(42)])),
        MoveValue::Struct(MoveStruct::RuntimeVariant(1, vec![])),
        MoveValue::Struct(MoveStruct::RuntimeVariant(2, vec![
            MoveValue::Bool(true),
            MoveValue::U32(13),
        ])),
    ];
    for value in good_values {
        let blob = value.simple_serialize().expect("serialization succeeds");
        let de_value =
            MoveValue::simple_deserialize(&blob, &layout).expect("deserialization succeeds");
        assert_eq!(value, de_value, "roundtrip serialization succeeds")
    }
    let bad_tag_value = MoveValue::Struct(MoveStruct::RuntimeVariant(3, vec![MoveValue::U64(42)]));
    let blob = bad_tag_value
        .simple_serialize()
        .expect("serialization succeeds");
    MoveValue::simple_deserialize(&blob, &layout)
        .inspect_err(|e| {
            assert!(
                e.to_string().contains("invalid length"),
                "unexpected error message: {}",
                e
            );
        })
        .expect_err("bad tag value deserialization fails");
    let bad_struct_value = MoveValue::Struct(MoveStruct::Runtime(vec![MoveValue::U64(42)]));
    let blob = bad_struct_value
        .simple_serialize()
        .expect("serialization succeeds");
    MoveValue::simple_deserialize(&blob, &layout)
        .inspect_err(|e| {
            assert!(
                e.to_string().contains("invalid length"),
                "unexpected error message: {}",
                e
            );
        })
        .expect_err("bad struct value deserialization fails");
}

#[test]
fn enum_round_trip_vm_value() {
    let layout = test_layout();
    let good_values = vec![
        Value::struct_(Struct::pack_variant(0, iter::once(Value::u64(42)))),
        Value::struct_(Struct::pack_variant(1, iter::empty())),
        Value::struct_(Struct::pack_variant(
            2,
            [Value::bool(true), Value::u32(13)].into_iter(),
        )),
    ];
    for value in good_values {
        let blob = value
            .simple_serialize(&layout)
            .expect("serialization succeeds");
        let de_value = Value::simple_deserialize(&blob, &layout).expect("deserialization succeeds");
        assert!(
            value.equals(&de_value).unwrap(),
            "roundtrip serialization succeeds"
        )
    }
    let bad_tag_value = Value::struct_(Struct::pack_variant(3, [Value::u64(42)]));
    assert!(
        bad_tag_value.simple_serialize(&layout).is_none(),
        "serialization fails"
    );
    let bad_struct_value = Value::struct_(Struct::pack([Value::u64(42)]));
    assert!(
        bad_struct_value.simple_serialize(&layout).is_none(),
        "serialization fails"
    );
}

// ---------------------------------------------------------------------------
// Rust cross-serialization tests

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub enum RustEnum {
    Number(u64),
    Empty,
    BoolNumber(bool, u32),
}

#[test]
fn enum_rust_round_trip_move_value() {
    let layout = test_layout();
    let move_values = vec![
        MoveValue::Struct(MoveStruct::RuntimeVariant(0, vec![MoveValue::U64(42)])),
        MoveValue::Struct(MoveStruct::RuntimeVariant(1, vec![])),
        MoveValue::Struct(MoveStruct::RuntimeVariant(2, vec![
            MoveValue::Bool(true),
            MoveValue::U32(13),
        ])),
    ];
    let rust_values = vec![
        RustEnum::Number(42),
        RustEnum::Empty,
        RustEnum::BoolNumber(true, 13),
    ];
    for (move_value, rust_value) in move_values.into_iter().zip(rust_values) {
        let from_move = move_value.simple_serialize().expect("from move succeeds");
        let to_rust = bcs::from_bytes::<RustEnum>(&from_move).expect("to rust successful");
        assert_eq!(to_rust, rust_value);

        let from_rust = bcs::to_bytes(&rust_value).expect("from rust succeeds");
        let to_move = MoveValue::simple_deserialize(&from_rust, &layout).expect("to move succeeds");
        assert_eq!(to_move, move_value)
    }
}

#[test]
fn enum_rust_round_trip_vm_value() {
    let layout = test_layout();
    let move_values = vec![
        Value::struct_(Struct::pack_variant(0, iter::once(Value::u64(42)))),
        Value::struct_(Struct::pack_variant(1, iter::empty())),
        Value::struct_(Struct::pack_variant(
            2,
            [Value::bool(true), Value::u32(13)].into_iter(),
        )),
    ];
    let rust_values = vec![
        RustEnum::Number(42),
        RustEnum::Empty,
        RustEnum::BoolNumber(true, 13),
    ];
    for (move_value, rust_value) in move_values.into_iter().zip(rust_values) {
        let from_move = move_value
            .simple_serialize(&layout)
            .expect("from move succeeds");
        let to_rust = bcs::from_bytes::<RustEnum>(&from_move).expect("to rust successful");
        assert_eq!(to_rust, rust_value);

        let from_rust = bcs::to_bytes(&rust_value).expect("from rust succeeds");
        let to_move = Value::simple_deserialize(&from_rust, &layout).expect("to move succeeds");
        assert!(
            to_move.equals(&move_value).unwrap(),
            "from rust to move failed"
        )
    }
}

// --------------------------------------------------------------------------
// Serialization size tests

#[test]
fn test_serialized_size() {
    use IdentifierMappingKind::*;
    use MoveStructLayout::*;
    use MoveTypeLayout::*;

    let u64_delayed_value = Value::delayed_value(DelayedFieldID::new_with_width(12, 8));
    let u128_delayed_value = Value::delayed_value(DelayedFieldID::new_with_width(123, 16));
    let derived_string_delayed_value = Value::delayed_value(DelayedFieldID::new_with_width(12, 60));

    // First field is a string, second field is a padding to ensure constant size.
    let derived_string_layout = Struct(Runtime(vec![
        Struct(Runtime(vec![Vector(Box::new(U8))])),
        Vector(Box::new(U8)),
    ]));

    // All these pairs should serialize.
    let good_values_layouts_sizes = [
        (Value::u8(10), U8),
        (Value::u16(10), U16),
        (Value::u32(10), U32),
        (Value::u64(10), U64),
        (Value::u128(10), U128),
        (Value::u256(u256::U256::one()), U256),
        (Value::bool(true), Bool),
        (Value::address(AccountAddress::ONE), Address),
        (Value::signer(AccountAddress::ONE), Signer),
        (u64_delayed_value, Native(Aggregator, Box::new(U64))),
        (u128_delayed_value, Native(Snapshot, Box::new(U128))),
        (
            derived_string_delayed_value,
            Native(DerivedString, Box::new(derived_string_layout)),
        ),
        (
            Value::vector_address(vec![AccountAddress::ONE]),
            Vector(Box::new(Address)),
        ),
        (
            Value::struct_(values_impl::Struct::pack(vec![
                Value::bool(true),
                Value::vector_u32(vec![1, 2, 3, 4, 5]),
            ])),
            Struct(Runtime(vec![Bool, Vector(Box::new(U32))])),
        ),
    ];
    for (value, layout) in good_values_layouts_sizes {
        let bytes = assert_some!(assert_ok!(serialize_and_allow_delayed_values(
            &value, &layout
        )));
        let size = assert_ok!(serialized_size_allowing_delayed_values(&value, &layout));
        assert_eq!(size, bytes.len());
    }

    // Also test unhappy path, mostly mismatches in value-layout.
    let u64_delayed_value = Value::delayed_value(DelayedFieldID::new_with_width(0, 8));
    let malformed_delayed_value = Value::delayed_value(DelayedFieldID::new_with_width(1, 7));
    let bad_values_layouts_sizes = [
        (Value::u8(10), U16),
        (u64_delayed_value, U64),
        (malformed_delayed_value, U64),
        (Value::u64(12), Native(Aggregator, Box::new(U64))),
    ];
    for (value, layout) in bad_values_layouts_sizes {
        assert_err!(serialized_size_allowing_delayed_values(&value, &layout));
    }
}

#[test]
fn signer_round_trip_vm_value() {
    let v = MoveValue::Signer(AccountAddress::ZERO);
    let bytes = v.simple_serialize().unwrap();
    let vm_value = Value::simple_deserialize(&bytes, &MoveTypeLayout::Signer).unwrap();
    let vm_bytes = serialize_and_allow_delayed_values(&vm_value, &MoveTypeLayout::Signer)
        .unwrap()
        .unwrap();
    assert_eq!(
        v,
        MoveValue::simple_deserialize(&vm_bytes, &MoveTypeLayout::Signer).unwrap()
    );

    let permissioned_signer = Value::permissioned_signer(AccountAddress::ZERO, AccountAddress::ONE);
    let bytes = permissioned_signer
        .simple_serialize(&MoveTypeLayout::Signer)
        .unwrap();
    assert_eq!(
        v,
        MoveValue::simple_deserialize(&bytes, &MoveTypeLayout::Signer).unwrap(),
    );
}
