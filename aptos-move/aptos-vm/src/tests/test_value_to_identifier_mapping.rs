// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::tests::mock_view::MockStateView;
use aptos_types::delayed_fields::DelayedFieldID;
use claims::assert_none;
use move_core_types::value::{
    IdentifierMappingKind, LayoutTag, MoveStructLayout::Runtime, MoveTypeLayout,
};
use move_vm_types::{
    value_transformation::{
        deserialize_and_replace_values_with_ids, serialize_and_replace_ids_with_values,
    },
    values::{Struct, Value},
};

#[test]
fn test_exchange_not_supported() {
    let exchange = MockStateView::default();

    // We cannot exchange non u64/u128 types.
    let layout = MoveTypeLayout::Tagged(
        LayoutTag::IdentifierMapping(IdentifierMappingKind::Aggregator),
        Box::new(MoveTypeLayout::Bool),
    );
    let input_value = Value::bool(false);
    let input_blob = input_value.simple_serialize(&layout).unwrap();
    assert_none!(deserialize_and_replace_values_with_ids(
        &input_blob,
        &layout,
        &exchange
    ));

    // Inner types in vector layouts cannot be tagged.
    let layout = MoveTypeLayout::Vector(Box::new(MoveTypeLayout::Tagged(
        LayoutTag::IdentifierMapping(IdentifierMappingKind::Aggregator),
        Box::new(MoveTypeLayout::U64),
    )));
    let input_value = Value::vector_u64(vec![1, 2, 3]);
    assert_none!(input_value.simple_serialize(&layout));

    // But also tagging all vector is not supported.
    let layout = MoveTypeLayout::Tagged(
        LayoutTag::IdentifierMapping(IdentifierMappingKind::Aggregator),
        Box::new(MoveTypeLayout::Vector(Box::new(MoveTypeLayout::U64))),
    );
    let input_value = Value::vector_u64(vec![1, 2, 3]);
    let input_blob = input_value.simple_serialize(&layout).unwrap();
    assert_none!(deserialize_and_replace_values_with_ids(
        &input_blob,
        &layout,
        &exchange
    ));

    let layout = MoveTypeLayout::Tagged(
        LayoutTag::IdentifierMapping(IdentifierMappingKind::Aggregator),
        Box::new(MoveTypeLayout::Struct(Runtime(vec![]))),
    );
    let input_value = Value::struct_(Struct::pack(vec![]));
    let input_blob = input_value.simple_serialize(&layout).unwrap();
    assert_none!(deserialize_and_replace_values_with_ids(
        &input_blob,
        &layout,
        &exchange
    ));
}

#[test]
fn test_exchange_preserves_value() {
    let exchange = MockStateView::default();

    let layout = MoveTypeLayout::U64;
    let input_value = Value::u64(100);
    let input_blob = input_value.simple_serialize(&layout).unwrap();
    let patched_value =
        deserialize_and_replace_values_with_ids(&input_blob, &layout, &exchange).unwrap();
    let unpatched_value = Value::simple_deserialize(
        &serialize_and_replace_ids_with_values(&patched_value, &layout, &exchange).unwrap(),
        &layout,
    )
    .unwrap();
    assert!(patched_value.equals(&Value::u64(100)).unwrap());
    assert!(unpatched_value.equals(&input_value).unwrap());
}

#[test]
fn test_exchange_u64() {
    let exchange = MockStateView::default();

    let layout = MoveTypeLayout::Tagged(
        LayoutTag::IdentifierMapping(IdentifierMappingKind::Aggregator),
        Box::new(MoveTypeLayout::U64),
    );
    let input_value = Value::u64(200);
    let input_blob = input_value.simple_serialize(&layout).unwrap();
    let patched_value =
        deserialize_and_replace_values_with_ids(&input_blob, &layout, &exchange).unwrap();
    let unpatched_value = Value::simple_deserialize(
        &serialize_and_replace_ids_with_values(&patched_value, &layout, &exchange).unwrap(),
        &layout,
    )
    .unwrap();
    exchange.assert_mapping_equal_at(0, 8, Value::u64(200));
    assert!(patched_value
        .equals(&Value::u64(DelayedFieldID::new_with_width(0, 8).as_u64()))
        .unwrap());
    assert!(unpatched_value.equals(&input_value).unwrap());
}

#[test]
fn test_exchange_u128() {
    let exchange = MockStateView::default();

    let layout = MoveTypeLayout::Tagged(
        LayoutTag::IdentifierMapping(IdentifierMappingKind::Aggregator),
        Box::new(MoveTypeLayout::U128),
    );
    let input_value = Value::u128(300);
    let input_blob = input_value.simple_serialize(&layout).unwrap();
    let patched_value =
        deserialize_and_replace_values_with_ids(&input_blob, &layout, &exchange).unwrap();
    let unpatched_value = Value::simple_deserialize(
        &serialize_and_replace_ids_with_values(&patched_value, &layout, &exchange).unwrap(),
        &layout,
    )
    .unwrap();
    exchange.assert_mapping_equal_at(0, 16, Value::u128(300));
    assert!(patched_value
        .equals(&Value::u128(
            DelayedFieldID::new_with_width(0, 16).as_u64() as u128
        ))
        .unwrap());
    assert!(unpatched_value.equals(&input_value).unwrap());
}

#[test]
fn test_exchange_works_inside_struct() {
    let exchange = MockStateView::default();

    let layout = MoveTypeLayout::Struct(Runtime(vec![
        MoveTypeLayout::U64,
        MoveTypeLayout::Tagged(
            LayoutTag::IdentifierMapping(IdentifierMappingKind::Aggregator),
            Box::new(MoveTypeLayout::U64),
        ),
        MoveTypeLayout::Tagged(
            LayoutTag::IdentifierMapping(IdentifierMappingKind::Aggregator),
            Box::new(MoveTypeLayout::U128),
        ),
    ]));

    let input_value = Value::struct_(Struct::pack(vec![
        Value::u64(400),
        Value::u64(500),
        Value::u128(600),
    ]));
    let input_blob = input_value.simple_serialize(&layout).unwrap();
    let patched_value =
        deserialize_and_replace_values_with_ids(&input_blob, &layout, &exchange).unwrap();
    let unpatched_value = Value::simple_deserialize(
        &serialize_and_replace_ids_with_values(&patched_value, &layout, &exchange).unwrap(),
        &layout,
    )
    .unwrap();
    exchange.assert_mapping_equal_at(0, 8, Value::u64(500));
    exchange.assert_mapping_equal_at(1, 16, Value::u128(600));
    let expected_patched_value = Value::struct_(Struct::pack(vec![
        Value::u64(400),
        Value::u64(DelayedFieldID::new_with_width(0, 8).as_u64()),
        Value::u128(DelayedFieldID::new_with_width(1, 16).as_u64() as u128),
    ]));
    assert!(patched_value.equals(&expected_patched_value).unwrap());
    assert!(unpatched_value.equals(&input_value).unwrap());
}
