// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::types::DelayedFieldValue;
use claims::{assert_err, assert_ok};
use move_core_types::value::{
    IdentifierMappingKind,
    MoveStructLayout::Runtime,
    MoveTypeLayout,
    MoveTypeLayout::{Address, Bool, Struct, Vector, U128, U64, U8},
};
use move_vm_types::{
    delayed_values::{
        delayed_field_id::{DelayedFieldID, TryFromMoveValue, TryIntoMoveValue},
        derived_string_snapshot::bytes_and_width_to_derived_string_struct,
    },
    values::Value,
};
use once_cell::sync::Lazy;
use test_case::test_case;
use DelayedFieldValue as A;
use IdentifierMappingKind as K;

static DERIVED_STRING: Lazy<MoveTypeLayout> = Lazy::new(|| {
    Struct(Runtime(vec![
        // String value
        Struct(Runtime(vec![Vector(Box::new(U8))])),
        // Vec<u8> padding
        Vector(Box::new(U8)),
    ]))
});

#[test_case(&U64, 8)]
#[test_case(&U128, 16)]
#[test_case(&*DERIVED_STRING, 20)]
fn test_aggregator_id_roundtrip_ok(layout: &MoveTypeLayout, width: u32) {
    let input = DelayedFieldID::new_with_width(100, width);
    let value = assert_ok!(input.try_into_move_value(layout));
    let (id, _) = assert_ok!(DelayedFieldID::try_from_move_value(layout, value, &()));
    assert_eq!(id, input);
}

#[test_case(&U8)]
#[test_case(&Bool)]
#[test_case(&Address)]
#[test_case(&Vector(Box::new(U8)))]
fn test_aggregator_id_to_value_err(layout: &MoveTypeLayout) {
    assert_err!(DelayedFieldID::new_with_width(100, 8).try_into_move_value(layout));
}

#[test_case(&U64, Value::u8(1))]
#[test_case(&U8, Value::u8(1))]
#[test_case(&Bool, Value::u8(1))]
#[test_case(&Vector(Box::new(U8)), Value::vector_u8(vec![0, 1]))]
fn test_aggregator_id_from_value_err(layout: &MoveTypeLayout, value: Value) {
    assert_err!(DelayedFieldID::try_from_move_value(layout, value, &()));
}

#[test_case(A::Aggregator(10), &U64, K::Aggregator, 8)]
#[test_case(A::Aggregator(10), &U128, K::Aggregator, 16)]
#[test_case(A::Snapshot(10), &U64, K::Snapshot, 8)]
#[test_case(A::Snapshot(10), &U128, K::Snapshot, 16)]
#[test_case(A::Derived(vec![0, 1]), &*DERIVED_STRING, K::DerivedString, 20)]
fn test_aggregator_value_roundtrip_ok(
    aggregator_value: DelayedFieldValue,
    layout: &MoveTypeLayout,
    kind: IdentifierMappingKind,
    width: u32,
) {
    let value = assert_ok!(aggregator_value.clone().try_into_move_value(layout, width));
    let (a, _) = assert_ok!(DelayedFieldValue::try_from_move_value(layout, value, &kind));
    assert_eq!(a, aggregator_value);
}

#[test_case(&U8, 1)]
#[test_case(&Bool, 1)]
#[test_case(&Address, 20)]
#[test_case(&Vector(Box::new(U8)), 5)]
fn test_aggregator_value_to_value_err(layout: &MoveTypeLayout, width: u32) {
    assert_err!(DelayedFieldValue::Aggregator(0).try_into_move_value(layout, width));
    assert_err!(DelayedFieldValue::Snapshot(1).try_into_move_value(layout, width));
    assert_err!(DelayedFieldValue::Derived(vec![3]).try_into_move_value(layout, width));
}

#[test_case(&U64, Value::u8(1), K::Aggregator)]
#[test_case(&U8, Value::u8(1), K::Aggregator)]
#[test_case(&U8, Value::u8(1), K::Snapshot)]
#[test_case(&Bool, Value::u8(1), K::Snapshot)]
#[test_case(&Vector(Box::new(U8)), Value::vector_u8(vec![0, 1]), K::Snapshot)]
#[test_case(&*DERIVED_STRING, bytes_and_width_to_derived_string_struct(vec![1,2], 20).unwrap(), K::Aggregator)]
fn test_aggregator_value_from_value_err(
    layout: &MoveTypeLayout,
    value: Value,
    kind: IdentifierMappingKind,
) {
    assert_err!(DelayedFieldValue::try_from_move_value(layout, value, &kind));
}
