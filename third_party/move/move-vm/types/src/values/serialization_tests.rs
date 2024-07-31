// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

//! Contains tests for serialization
//!
use move_core_types::value::{MoveStruct, MoveStructLayout, MoveTypeLayout, MoveValue};

#[test]
fn enum_round_trip() {
    let layout = MoveTypeLayout::Struct(MoveStructLayout::RuntimeVariants(vec![
        vec![MoveTypeLayout::U64],
        vec![],
        vec![MoveTypeLayout::Bool, MoveTypeLayout::U32],
    ]));
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
                e.to_string().contains("invalid value"),
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
                e.to_string().contains("end of input"),
                "unexpected error message: {}",
                e
            );
        })
        .expect_err("bad struct value deserialization fails");
}
