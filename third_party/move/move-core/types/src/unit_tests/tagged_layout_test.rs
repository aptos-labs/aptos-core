// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    account_address::AccountAddress,
    u256,
    value::{
        IdentifierMappingKind, LayoutTag, MoveStruct, MoveStructLayout, MoveTypeLayout, MoveValue,
    },
};

fn assert_same_serialization(
    v: &MoveValue,
    layout: &MoveTypeLayout,
    tagged_layout: &MoveTypeLayout,
) {
    let blob = v.simple_serialize().unwrap();
    let v_with_untagged = MoveValue::simple_deserialize(&blob, layout).unwrap();
    let v_with_tagged = MoveValue::simple_deserialize(&blob, tagged_layout).unwrap();
    assert_eq!(v.clone(), v_with_untagged);
    assert_eq!(v_with_untagged, v_with_tagged);
}

macro_rules! aggregator_mapping {
    ($layout:expr) => {
        // Note: the actual kind is not important fot tests.
        MoveTypeLayout::Tagged(
            LayoutTag::IdentifierMapping(IdentifierMappingKind::Aggregator),
            Box::new($layout),
        )
    };
}

#[test]
fn test_aggregator_mapping_same_serialization_primitive_types() {
    use MoveTypeLayout as L;
    use MoveValue::*;

    assert_same_serialization(&Bool(false), &L::Bool, &aggregator_mapping!(L::Bool));
    assert_same_serialization(&U8(1), &L::U8, &aggregator_mapping!(L::U8));
    assert_same_serialization(&U16(2), &L::U16, &aggregator_mapping!(L::U16));
    assert_same_serialization(&U32(3), &L::U32, &aggregator_mapping!(L::U32));
    assert_same_serialization(&U64(4), &L::U64, &aggregator_mapping!(L::U64));
    assert_same_serialization(&U128(5), &L::U128, &aggregator_mapping!(L::U128));
    assert_same_serialization(
        &U256(u256::U256::one()),
        &L::U256,
        &aggregator_mapping!(L::U256),
    );
    assert_same_serialization(
        &Address(AccountAddress::ONE),
        &L::Address,
        &aggregator_mapping!(L::Address),
    );
    assert_same_serialization(
        &Signer(AccountAddress::TWO),
        &L::Signer,
        &aggregator_mapping!(L::Signer),
    );
}

#[test]
fn test_aggregator_mapping_same_serialization_vector_types() {
    use MoveTypeLayout as L;
    use MoveValue::*;

    let v = Vector(vec![U32(1), U32(2), U32(3)]);
    let layout = L::Vector(Box::new(L::U32));

    let tagged_layout = aggregator_mapping!(layout.clone());
    assert_same_serialization(&v, &layout, &tagged_layout);

    let tagged_layout = L::Vector(Box::new(aggregator_mapping!(L::U32)));
    assert_same_serialization(&v, &layout, &tagged_layout);
}

#[test]
fn test_aggregator_mapping_same_serialization_struct_types() {
    use MoveStructLayout::*;
    use MoveTypeLayout as L;
    use MoveValue::*;

    let a = Struct(MoveStruct::Runtime(vec![U64(1)]));
    let b = Struct(MoveStruct::Runtime(vec![
        U8(2),
        Vector(vec![U32(3), U32(4)]),
        Bool(true),
    ]));
    let c = Struct(MoveStruct::Runtime(vec![a, U128(2), b]));
    let layout = L::Struct(Runtime(vec![
        L::Struct(Runtime(vec![L::U64])),
        L::U128,
        L::Struct(Runtime(vec![L::U8, L::Vector(Box::new(L::U32)), L::Bool])),
    ]));

    let tagged_layout = aggregator_mapping!(layout.clone());
    assert_same_serialization(&c, &layout, &tagged_layout);

    let tagged_layout = L::Struct(Runtime(vec![
        aggregator_mapping!(L::Struct(Runtime(vec![L::U64]))),
        L::U128,
        L::Struct(Runtime(vec![L::U8, L::Vector(Box::new(L::U32)), L::Bool])),
    ]));
    assert_same_serialization(&c, &layout, &tagged_layout);

    let tagged_layout = L::Struct(Runtime(vec![
        aggregator_mapping!(L::Struct(Runtime(vec![L::U64]))),
        L::U128,
        L::Struct(Runtime(vec![
            aggregator_mapping!(L::U8),
            L::Vector(Box::new(aggregator_mapping!(L::U32))),
            L::Bool,
        ])),
    ]));
    assert_same_serialization(&c, &layout, &tagged_layout);
}

#[test]
fn test_nested_aggregator_mapping_same_serialization() {
    use MoveTypeLayout as L;
    use MoveValue::*;

    let v = U32(1);
    let layout = L::U32;
    let tagged_layout = aggregator_mapping!(aggregator_mapping!(aggregator_mapping!(
        aggregator_mapping!(L::U32)
    )));
    assert_same_serialization(&v, &layout, &tagged_layout);
}
