// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    value_serde::{
        deserialize_and_replace_values_with_ids, serialize_and_replace_ids_with_values,
        TransformationResult, ValueToIdentifierMapping,
    },
    values::{SizedID, Struct, Value},
};
use move_core_types::value::{
    IdentifierMappingKind, LayoutTag, MoveStructLayout::Runtime, MoveTypeLayout,
};
use std::{cell::RefCell, collections::BTreeMap};

#[derive(Debug, Default)]
struct DelayedValueMapping {
    delayed_values: RefCell<BTreeMap<u64, Value>>,
}

impl DelayedValueMapping {
    fn contains_value_at(&self, value: Value, identifier: u64) -> bool {
        self.delayed_values
            .borrow()
            .get(&identifier)
            .is_some_and(|v| v.equals(&value).unwrap())
    }
}

impl ValueToIdentifierMapping for DelayedValueMapping {
    fn value_to_identifier(
        &self,
        _kind: &IdentifierMappingKind,
        value: Value,
    ) -> TransformationResult<Value> {
        let mut delayed_values = self.delayed_values.borrow_mut();

        let identifier = delayed_values.len() as u64;
        // fixme
        let identifier_value = Value::delayed_value(identifier as u32, 0);

        delayed_values.insert(identifier, value);
        Ok(identifier_value)
    }

    fn identifier_to_value(
        &self,
        _layout: &MoveTypeLayout,
        _identifier: Value,
    ) -> TransformationResult<Value> {
        todo!()
        // let identifier = identifier.value_as::<DelayedFieldID>()?.unique_index as u64;
        //
        // let delayed_values = self.delayed_values.borrow();
        // Ok(delayed_values
        //     .get(&identifier)
        //     .expect("Identifiers must always exist for delayed values")
        //     .copy_value()
        //     .expect("Copying extracted delayed values should never fail"))
    }
}

#[test]
fn test_no_delayed_values() {
    let mapping = DelayedValueMapping::default();

    let layout = MoveTypeLayout::U64;
    let input = Value::u64(100);

    let output = deserialize_and_replace_values_with_ids(
        &input.simple_serialize(&layout).unwrap(),
        &layout,
        &mapping,
    )
    .unwrap();
    assert!(output.equals(&input).unwrap());

    let output = Value::simple_deserialize(
        &serialize_and_replace_ids_with_values(&input, &layout, &mapping).unwrap(),
        &layout,
    )
    .unwrap();
    assert!(output.equals(&input).unwrap());
}

#[test]
fn test_delayed_u64_value() {
    let mapping = DelayedValueMapping::default();

    let layout = MoveTypeLayout::Native(
        LayoutTag::IdentifierMapping(IdentifierMappingKind::Aggregator),
        Box::new(MoveTypeLayout::U64),
    );
    let input = Value::u64(200);
    let input_blob = input.simple_serialize(&layout).unwrap();

    // Test roundtrip.
    let delayed_value =
        deserialize_and_replace_values_with_ids(&input_blob, &layout, &mapping).unwrap();
    let output = Value::simple_deserialize(
        &serialize_and_replace_ids_with_values(&delayed_value, &layout, &mapping).unwrap(),
        &layout,
    )
    .unwrap();
    assert!(output.equals(&input).unwrap());

    // Test handle is inserted and value extracted.
    assert!(mapping.contains_value_at(Value::u64(200), 0));
    assert!(delayed_value
        .value_as::<SizedID>()
        .is_ok_and(|id| id.unique_index == 0));
}

#[test]
fn test_delayed_value_inside_struct() {
    let mapping = DelayedValueMapping::default();

    let layout = MoveTypeLayout::Struct(Runtime(vec![
        MoveTypeLayout::U64,
        MoveTypeLayout::Native(
            LayoutTag::IdentifierMapping(IdentifierMappingKind::Aggregator),
            Box::new(MoveTypeLayout::U64),
        ),
        MoveTypeLayout::Native(
            LayoutTag::IdentifierMapping(IdentifierMappingKind::Aggregator),
            Box::new(MoveTypeLayout::U128),
        ),
    ]));

    let input = Value::struct_(Struct::pack(vec![
        Value::u64(400),
        Value::u64(500),
        Value::u128(600),
    ]));
    let input_blob = input.simple_serialize(&layout).unwrap();

    let struct_with_delayed_value =
        deserialize_and_replace_values_with_ids(&input_blob, &layout, &mapping).unwrap();
    let output = Value::simple_deserialize(
        &serialize_and_replace_ids_with_values(&struct_with_delayed_value, &layout, &mapping)
            .unwrap(),
        &layout,
    )
    .unwrap();
    assert!(output.equals(&input).unwrap());

    assert!(mapping.contains_value_at(Value::u64(500), 0));
    assert!(mapping.contains_value_at(Value::u128(600), 1));

    let mut fields: Vec<Value> = struct_with_delayed_value
        .value_as::<Struct>()
        .unwrap()
        .unpack()
        .unwrap()
        .collect();
    assert!(fields
        .pop()
        .unwrap()
        .value_as::<SizedID>()
        .is_ok_and(|h| h.unique_index == 1));
    assert!(fields
        .pop()
        .unwrap()
        .value_as::<SizedID>()
        .is_ok_and(|h| h.unique_index == 0));
    assert!(fields.pop().unwrap().equals(&Value::u64(400)).unwrap());
    assert!(fields.is_empty())
}
