// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::super::utils::bytes_to_string;
use crate::types::{DelayedFieldID, DelayedFieldValue, TryFromMoveValue, TryIntoMoveValue};
use claims::{assert_err, assert_ok};
use move_core_types::value::{
    IdentifierMappingKind,
    MoveStructLayout::Runtime,
    MoveTypeLayout,
    MoveTypeLayout::{Address, Bool, Struct, Vector, U128, U64, U8},
};
use move_vm_types::values::Value;
use once_cell::sync::Lazy;
use test_case::test_case;
use DelayedFieldValue as A;
use IdentifierMappingKind as K;

static STRING: Lazy<MoveTypeLayout> = Lazy::new(|| Struct(Runtime(vec![Vector(Box::new(U8))])));

#[test_case(&U64)]
#[test_case(&U128)]
#[test_case(&*STRING)]
fn test_aggregator_id_roundtrip_ok(layout: &MoveTypeLayout) {
    let value = assert_ok!(DelayedFieldID::new(100).try_into_move_value(layout));
    let id = assert_ok!(DelayedFieldID::try_from_move_value(layout, value, &()));
    assert_eq!(id, DelayedFieldID::new(100));
}

#[test_case(&U8)]
#[test_case(&Bool)]
#[test_case(&Address)]
#[test_case(&Vector(Box::new(U8)))]
fn test_aggregator_id_to_value_err(layout: &MoveTypeLayout) {
    assert_err!(DelayedFieldID::new(100).try_into_move_value(layout));
}

#[test_case(&U64, Value::u8(1))]
#[test_case(&U8, Value::u8(1))]
#[test_case(&Bool, Value::u8(1))]
#[test_case(&Vector(Box::new(U8)), Value::vector_u8(vec![0, 1]))]
fn test_aggregator_id_from_value_err(layout: &MoveTypeLayout, value: Value) {
    assert_err!(DelayedFieldID::try_from_move_value(layout, value, &()));
}

#[test_case(A::Aggregator(10), &U64, K::Aggregator)]
#[test_case(A::Aggregator(10), &U128, K::Aggregator)]
#[test_case(A::Snapshot(10), &U64, K::Snapshot)]
#[test_case(A::Snapshot(10), &U128, K::Snapshot)]
#[test_case(A::Derived(vec![0, 1]), &*STRING, K::Snapshot)]
fn test_aggregator_value_roundtrip_ok(
    aggregator_value: DelayedFieldValue,
    layout: &MoveTypeLayout,
    kind: IdentifierMappingKind,
) {
    let value = assert_ok!(aggregator_value.clone().try_into_move_value(layout));
    let a = assert_ok!(DelayedFieldValue::try_from_move_value(layout, value, &kind));
    assert_eq!(a, aggregator_value);
}

#[test_case(&U8)]
#[test_case(&Bool)]
#[test_case(&Address)]
#[test_case(&Vector(Box::new(U8)))]
fn test_aggregator_value_to_value_err(layout: &MoveTypeLayout) {
    assert_err!(DelayedFieldValue::Aggregator(0).try_into_move_value(layout));
    assert_err!(DelayedFieldValue::Snapshot(1).try_into_move_value(layout));
    assert_err!(DelayedFieldValue::Derived(vec![3]).try_into_move_value(layout));
}

#[test_case(&U64, Value::u8(1), K::Aggregator)]
#[test_case(&U8, Value::u8(1), K::Aggregator)]
#[test_case(&U8, Value::u8(1), K::Snapshot)]
#[test_case(&Bool, Value::u8(1), K::Snapshot)]
#[test_case(&Vector(Box::new(U8)), Value::vector_u8(vec![0, 1]), K::Snapshot)]
#[test_case(&*STRING, bytes_to_string(vec![1,2]), K::Aggregator)]
fn test_aggregator_value_from_value_err(
    layout: &MoveTypeLayout,
    value: Value,
    kind: IdentifierMappingKind,
) {
    assert_err!(DelayedFieldValue::try_from_move_value(layout, value, &kind));
}
