// Parts of the file are Copyright (c) The Diem Core Contributors
// Parts of the file are Copyright (c) The Move Contributors
// Parts of the file are Copyright (c) Aptos Foundation
// All Aptos Foundation code and content is licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Contains tests for serialization

#[cfg(test)]
mod tests {
    use crate::{
        delayed_values::delayed_field_id::DelayedFieldID,
        value_serde::{MockFunctionValueExtension, ValueSerDeContext},
        values::{
            function_values_impl::{closure_captured_depth, mock::MockAbstractFunction},
            values_impl, Struct, Value,
        },
    };
    use better_any::TidExt;
    use claims::{assert_err, assert_none, assert_ok, assert_some};
    use move_binary_format::errors::PartialVMResult;
    use move_core_types::{
        ability::AbilitySet,
        account_address::AccountAddress,
        function::{ClosureMask, MoveClosure, MoveClosureCapturedArgs},
        identifier::Identifier,
        int256,
        language_storage::{FunctionParamOrReturnTag, FunctionTag, ModuleId, StructTag, TypeTag},
        value::{IdentifierMappingKind, MoveStruct, MoveStructLayout, MoveTypeLayout, MoveValue},
    };
    use serde::{Deserialize, Serialize};
    use std::iter;

    // ==========================================================================
    // Enums

    fn enum_layout() -> MoveTypeLayout {
        MoveTypeLayout::new_struct(MoveStructLayout::RuntimeVariants(vec![
            vec![MoveTypeLayout::U64],
            vec![],
            vec![MoveTypeLayout::Bool, MoveTypeLayout::U32],
        ]))
    }

    // ---------------------------------------------------------------------------
    // Move Values

    #[test]
    fn enum_round_trip_move_value() {
        let layout = enum_layout();
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
        let bad_tag_value =
            MoveValue::Struct(MoveStruct::RuntimeVariant(3, vec![MoveValue::U64(42)]));
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

    // ---------------------------------------------------------------------------
    // VM Values

    #[test]
    fn enum_round_trip_vm_value() {
        let layout = enum_layout();
        let good_values = vec![
            Value::struct_(Struct::pack_variant(0, iter::once(Value::u64(42)))),
            Value::struct_(Struct::pack_variant(1, iter::empty())),
            Value::struct_(Struct::pack_variant(
                2,
                [Value::bool(true), Value::u32(13)].into_iter(),
            )),
        ];
        for value in good_values {
            let blob = ValueSerDeContext::new(None)
                .serialize(&value, &layout)
                .unwrap()
                .expect("serialization succeeds");
            let de_value = ValueSerDeContext::new(None)
                .deserialize(&blob, &layout)
                .expect("deserialization succeeds");
            assert!(
                value.equals(&de_value).unwrap(),
                "roundtrip serialization succeeds"
            )
        }
        let bad_tag_value = Value::struct_(Struct::pack_variant(3, [Value::u64(42)]));
        assert!(
            ValueSerDeContext::new(None)
                .serialize(&bad_tag_value, &layout)
                .unwrap()
                .is_none(),
            "serialization fails"
        );
        let bad_struct_value = Value::struct_(Struct::pack([Value::u64(42)]));
        assert!(
            ValueSerDeContext::new(None)
                .serialize(&bad_struct_value, &layout)
                .unwrap()
                .is_none(),
            "serialization fails"
        );
    }

    #[test]
    fn enum_out_of_range_zero_field_tag_fails_to_serialize() {
        let layout = enum_layout();

        let bad_zero_field_tag = Value::struct_(Struct::pack_variant(3, iter::empty()));
        assert!(
            ValueSerDeContext::new(None)
                .serialize(&bad_zero_field_tag, &layout)
                .unwrap()
                .is_none(),
            "serializing an out-of-range zero-field variant tag must fail"
        );

        let variants_layout = MoveStructLayout::RuntimeVariants(vec![
            vec![MoveTypeLayout::U64],
            vec![],
            vec![MoveTypeLayout::Bool, MoveTypeLayout::U32],
        ]);
        assert!(variants_layout.fields(Some(3)).is_none());
        assert!(variants_layout.fields(Some(1)).is_some());
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
        let layout = enum_layout();
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
            let to_move =
                MoveValue::simple_deserialize(&from_rust, &layout).expect("to move succeeds");
            assert_eq!(to_move, move_value)
        }
    }

    #[test]
    fn enum_rust_round_trip_vm_value() {
        let layout = enum_layout();
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
            let from_move = ValueSerDeContext::new(None)
                .serialize(&move_value, &layout)
                .unwrap()
                .expect("from move succeeds");
            let to_rust = bcs::from_bytes::<RustEnum>(&from_move).expect("to rust successful");
            assert_eq!(to_rust, rust_value);

            let from_rust = bcs::to_bytes(&rust_value).expect("from rust succeeds");
            let to_move = ValueSerDeContext::new(None)
                .deserialize(&from_rust, &layout)
                .expect("to move succeeds");
            assert!(
                to_move.equals(&move_value).unwrap(),
                "from rust to move failed"
            )
        }
    }

    // ======================================================================================
    // Closures

    fn make_fun_layout() -> MoveTypeLayout {
        MoveTypeLayout::Function
    }

    fn make_type_args() -> Vec<TypeTag> {
        // Just some more complex type instantiation to cover serialization of TypeTag
        vec![
            TypeTag::Address,
            TypeTag::Function(Box::new(FunctionTag {
                args: vec![FunctionParamOrReturnTag::Value(TypeTag::Struct(Box::new(
                    StructTag {
                        address: AccountAddress::TEN,
                        module: Identifier::new("mod").unwrap(),
                        name: Identifier::new("st").unwrap(),
                        type_args: vec![TypeTag::Signer],
                    },
                )))],
                results: vec![FunctionParamOrReturnTag::Value(TypeTag::Address)],
                abilities: AbilitySet::PUBLIC_FUNCTIONS,
            })),
        ]
    }

    // --------------------------------------------------------------------------------------
    // Move Values

    fn make_move_closure(
        fun_name: &str,
        ty_args: Vec<TypeTag>,
        mask: ClosureMask,
        captured: Vec<(MoveTypeLayout, MoveValue)>,
    ) -> MoveValue {
        MoveValue::closure(MoveClosure {
            module_id: ModuleId::new(AccountAddress::TWO, Identifier::new("m").unwrap()),
            fun_id: Identifier::new(fun_name).unwrap(),
            ty_args,
            mask,
            captured: MoveClosureCapturedArgs::Deserialized(captured),
        })
    }

    #[test]
    fn closure_round_trip_move_value_good() {
        let fun_layout = make_fun_layout();
        let ty_args = make_type_args();
        let good_values = vec![
            make_move_closure("f", ty_args, ClosureMask::new(0b101), vec![
                (MoveTypeLayout::Bool, MoveValue::Bool(true)),
                (MoveTypeLayout::U64, MoveValue::U64(22)),
            ]),
            make_move_closure("f", vec![], ClosureMask::new(0b1), vec![(
                MoveTypeLayout::U64,
                MoveValue::U64(22),
            )]),
            make_move_closure("f", vec![], ClosureMask::new(0b0), vec![]),
        ];
        for value in good_values {
            let blob = value.simple_serialize().expect("serialization succeeds");
            let de_value = assert_ok!(MoveValue::simple_deserialize(&blob, &fun_layout));
            assert_eq!(value, de_value, "round trip serialization succeeds")
        }
    }

    #[test]
    fn closure_round_trip_move_value_bad_size() {
        let fun_layout = make_fun_layout();
        let bad_captures_more = make_move_closure("f", vec![], ClosureMask::new(0b1), vec![
            (MoveTypeLayout::Bool, MoveValue::Bool(true)),
            (MoveTypeLayout::U64, MoveValue::U64(22)),
        ]);
        let blob = bad_captures_more
            .simple_serialize()
            .expect("serialization succeeds");
        MoveValue::simple_deserialize(&blob, &fun_layout)
            .inspect_err(|e| {
                assert!(
                    e.to_string().contains("invalid length"),
                    "unexpected error message: {}",
                    e
                );
            })
            .expect_err("bad size value deserialization fails");
        let bad_captures_less = make_move_closure("f", vec![], ClosureMask::new(0b11), vec![(
            MoveTypeLayout::Bool,
            MoveValue::Bool(true),
        )]);
        let blob = bad_captures_less
            .simple_serialize()
            .expect("serialization succeeds");
        MoveValue::simple_deserialize(&blob, &fun_layout)
            .inspect_err(|e| {
                assert!(
                    e.to_string().contains("expected more"),
                    "unexpected error message: {}",
                    e
                );
            })
            .expect_err("bad size value deserialization fails");
    }

    #[test]
    fn closure_round_trip_move_value_bad_layout() {
        let fun_layout = make_fun_layout();
        let bad_layout = make_move_closure("f", vec![], ClosureMask::new(0b11), vec![
            (
                MoveTypeLayout::Vector(Box::new(MoveTypeLayout::U8)),
                MoveValue::Bool(true),
            ),
            (
                MoveTypeLayout::Bool,
                MoveValue::Vector(vec![MoveValue::U64(22), MoveValue::U8(1)]),
            ),
        ]);
        let blob = bad_layout
            .simple_serialize()
            .expect("serialization succeeds");
        MoveValue::simple_deserialize(&blob, &fun_layout)
            .inspect_err(|e| {
                assert!(
                    e.to_string().contains("remaining input"),
                    "unexpected error message: {}",
                    e
                );
            })
            .expect_err("bad layout value deserialization fails");
    }

    // --------------------------------------------------------------------------------------
    // VM Values

    fn round_trip_vm_closure_value(
        mut fun: MockAbstractFunction,
        captured: Vec<Value>,
    ) -> (Value, PartialVMResult<Value>) {
        // A freshly packed closure carries the true depth of its captured values, so
        // it matches the depth a reader recomputes on the way back.
        fun.data.captured_depth = closure_captured_depth(&captured, u64::MAX).unwrap();
        let fun_layout = make_fun_layout();
        let mut ext_mock = MockFunctionValueExtension::new();
        ext_mock
            .expect_is_function_data_format_v2_enabled()
            .returning(|| false);
        ext_mock
            .expect_get_serialization_data()
            .returning(move |af| {
                Ok(af
                    .downcast_ref::<MockAbstractFunction>()
                    .expect("cast")
                    .data
                    .clone())
            });
        ext_mock
            .expect_create_from_serialization_data()
            .returning(move |data| Ok(Box::new(MockAbstractFunction::new_from_data(data))));
        let value = Value::closure(Box::new(fun), captured);
        let blob = assert_ok!(ValueSerDeContext::new(None)
            .with_func_args_deserialization(&ext_mock)
            .serialize(&value, &fun_layout))
        .expect("serialization result not None");
        let de_value = ValueSerDeContext::new(None)
            .with_func_args_deserialization(&ext_mock)
            .deserialize_or_err(&blob, &fun_layout);
        (value, de_value)
    }

    #[test]
    fn closure_round_trip_vm_value_good() {
        let ty_args = make_type_args();
        let good_seeds = vec![
            (
                MockAbstractFunction::new("f", ty_args, ClosureMask::new(0b101), vec![
                    MoveTypeLayout::Bool,
                    MoveTypeLayout::U64,
                ]),
                vec![Value::bool(true), Value::u64(22)],
            ),
            (
                MockAbstractFunction::new("f", vec![TypeTag::Bool], ClosureMask::new(0b1), vec![
                    MoveTypeLayout::U64,
                ]),
                vec![Value::u64(22)],
            ),
            (
                MockAbstractFunction::new("f", vec![TypeTag::U16], ClosureMask::new(0b0), vec![]),
                vec![],
            ),
        ];
        for (fun, captured) in good_seeds {
            let (value, de_value) = round_trip_vm_closure_value(fun, captured);
            assert!(
                value.equals(&assert_ok!(de_value)).unwrap(),
                "round-trip serialization succeeds"
            );
        }
    }

    #[test]
    fn closure_round_trip_vm_value_bad_size() {
        let (_, de_value) = round_trip_vm_closure_value(
            MockAbstractFunction::new("f", vec![], ClosureMask::new(0b1), vec![
                MoveTypeLayout::Bool,
                MoveTypeLayout::U64,
            ]),
            vec![Value::bool(false), Value::u64(22)],
        );
        de_value
            .inspect_err(|e| {
                assert!(
                    e.to_string().contains("invalid length"),
                    "unexpected error message: {}",
                    e
                )
            })
            .expect_err("bad size value deserialization fails");

        let (_, de_value) = round_trip_vm_closure_value(
            MockAbstractFunction::new("f", vec![], ClosureMask::new(0b11), vec![
                MoveTypeLayout::Bool,
            ]),
            vec![Value::bool(false)],
        );
        de_value
            .inspect_err(|e| {
                assert!(
                    e.to_string().contains("expected more"),
                    "unexpected error message: {}",
                    e
                )
            })
            .expect_err("bad size value deserialization fails");

        // A closure whose captured value count does not match its layouts fails
        // already at serialization time.
        let mut ext_mock = MockFunctionValueExtension::new();
        ext_mock
            .expect_is_function_data_format_v2_enabled()
            .returning(|| false);
        ext_mock
            .expect_get_serialization_data()
            .returning(move |af| {
                Ok(af
                    .downcast_ref::<MockAbstractFunction>()
                    .expect("cast")
                    .data
                    .clone())
            });
        let value = Value::closure(
            Box::new(MockAbstractFunction::new(
                "f",
                vec![],
                ClosureMask::new(0b1),
                vec![MoveTypeLayout::Bool],
            )),
            vec![],
        );
        let result = assert_ok!(ValueSerDeContext::new(None)
            .with_func_args_deserialization(&ext_mock)
            .serialize(&value, &make_fun_layout()));
        assert_none!(result);
    }

    fn make_v2_ext_mock(v2_enabled: bool) -> MockFunctionValueExtension {
        let mut ext_mock = MockFunctionValueExtension::new();
        ext_mock
            .expect_is_function_data_format_v2_enabled()
            .returning(move || v2_enabled);
        ext_mock
            .expect_get_serialization_data()
            .returning(move |af| {
                Ok(af
                    .downcast_ref::<MockAbstractFunction>()
                    .expect("cast")
                    .data
                    .clone())
            });
        ext_mock
            .expect_create_from_serialization_data()
            .returning(move |data| Ok(Box::new(MockAbstractFunction::new_from_data(data))));
        ext_mock
    }

    fn make_captured_closure() -> Value {
        let captured = vec![Value::bool(true), Value::u64(22)];
        let mut fun =
            MockAbstractFunction::new("f", vec![TypeTag::Bool], ClosureMask::new(0b101), vec![
                MoveTypeLayout::Bool,
                MoveTypeLayout::U64,
            ]);
        // Captured bool and u64 are leaves, so the closure's captured depth is 1.
        fun.data.captured_depth = closure_captured_depth(&captured, u64::MAX).unwrap();
        Value::closure(Box::new(fun), captured)
    }

    #[test]
    fn closure_v2_round_trip_vm_value() {
        let fun_layout = make_fun_layout();
        let ext_mock = make_v2_ext_mock(true);

        for (value, expected_depth, expected_blob) in [
            // The blob is the concatenation of the BCS encodings of the captured
            // values: bool(true) and u64(22), length-prefixed. Both are leaves, so
            // the cached depth is 1.
            (make_captured_closure(), 1u16, vec![
                9u8, 1, 22, 0, 0, 0, 0, 0, 0, 0,
            ]),
            // Empty captures serialize as an empty blob with cached depth 0.
            (
                Value::closure(
                    Box::new(MockAbstractFunction::new(
                        "f",
                        vec![],
                        ClosureMask::new(0),
                        vec![],
                    )),
                    vec![],
                ),
                0u16,
                vec![0u8],
            ),
        ] {
            let bytes = assert_ok!(ValueSerDeContext::new(None)
                .with_func_args_deserialization(&ext_mock)
                .serialize(&value, &fun_layout))
            .expect("serialization result not None");
            // Version 2 right after the seq length byte, little-endian u16.
            assert_eq!(&bytes[1..3], &[2, 0]);
            // The blob (length-prefixed) is the last element.
            assert!(bytes.ends_with(&expected_blob));
            // The cached depth is the u16 element right before the blob.
            let depth_end = bytes.len() - expected_blob.len();
            assert_eq!(
                &bytes[depth_end - 2..depth_end],
                &expected_depth.to_le_bytes()
            );

            // Deserialization keeps the captured arguments serialized; re-serialization
            // writes them back verbatim.
            let de_value = assert_ok!(ValueSerDeContext::new(None)
                .with_func_args_deserialization(&ext_mock)
                .deserialize_or_err(&bytes, &fun_layout));
            let re_bytes = assert_ok!(ValueSerDeContext::new(None)
                .with_func_args_deserialization(&ext_mock)
                .serialize(&de_value, &fun_layout))
            .expect("serialization result not None");
            assert_eq!(bytes, re_bytes);

            // Two closures deserialized from the same bytes are equal without
            // materialization (equal identity and equal blobs).
            let de_value_2 = assert_ok!(ValueSerDeContext::new(None)
                .with_func_args_deserialization(&ext_mock)
                .deserialize_or_err(&bytes, &fun_layout));
            assert!(assert_ok!(de_value.equals(&de_value_2)));
        }
    }

    #[test]
    fn closure_v1_converts_to_v2_on_read() {
        let fun_layout = make_fun_layout();
        let v1_ext_mock = make_v2_ext_mock(false);
        let v2_ext_mock = make_v2_ext_mock(true);

        // Written as V1 (flag off).
        let v1_bytes = assert_ok!(ValueSerDeContext::new(None)
            .with_func_args_deserialization(&v1_ext_mock)
            .serialize(&make_captured_closure(), &fun_layout))
        .expect("serialization result not None");
        assert_eq!(&v1_bytes[1..3], &[1, 0]);

        // Read with the flag on: converted to V2, and re-serializes exactly as a
        // directly written V2 closure.
        let de_value = assert_ok!(ValueSerDeContext::new(None)
            .with_func_args_deserialization(&v2_ext_mock)
            .deserialize_or_err(&v1_bytes, &fun_layout));
        let re_bytes = assert_ok!(ValueSerDeContext::new(None)
            .with_func_args_deserialization(&v2_ext_mock)
            .serialize(&de_value, &fun_layout))
        .expect("serialization result not None");
        let v2_bytes = assert_ok!(ValueSerDeContext::new(None)
            .with_func_args_deserialization(&v2_ext_mock)
            .serialize(&make_captured_closure(), &fun_layout))
        .expect("serialization result not None");
        assert_eq!(re_bytes, v2_bytes);

        // Read with the flag off: unchanged legacy behavior, byte-stable V1.
        let de_value = assert_ok!(ValueSerDeContext::new(None)
            .with_func_args_deserialization(&v1_ext_mock)
            .deserialize_or_err(&v1_bytes, &fun_layout));
        let re_bytes = assert_ok!(ValueSerDeContext::new(None)
            .with_func_args_deserialization(&v1_ext_mock)
            .serialize(&de_value, &fun_layout))
        .expect("serialization result not None");
        assert_eq!(re_bytes, v1_bytes);
    }

    #[test]
    fn closure_v2_reads_can_be_disallowed() {
        let fun_layout = make_fun_layout();
        let ext_mock = make_v2_ext_mock(true);
        let v2_bytes = assert_ok!(ValueSerDeContext::new(None)
            .with_func_args_deserialization(&ext_mock)
            .serialize(&make_captured_closure(), &fun_layout))
        .expect("serialization result not None");

        assert_err!(ValueSerDeContext::new(None)
            .with_func_args_deserialization(&ext_mock)
            .with_function_values_v2_reads(false)
            .deserialize_or_err(&v2_bytes, &fun_layout));
        assert_ok!(ValueSerDeContext::new(None)
            .with_func_args_deserialization(&ext_mock)
            .with_function_values_v2_reads(true)
            .deserialize_or_err(&v2_bytes, &fun_layout));
    }

    #[test]
    fn closure_serialized_cmp_requires_materializer() {
        let fun_layout = make_fun_layout();
        let ext_mock = make_v2_ext_mock(true);

        // Two closures with equal identity but different captured arguments.
        let a = Value::closure(
            Box::new(MockAbstractFunction::new(
                "f",
                vec![],
                ClosureMask::new(0b1),
                vec![MoveTypeLayout::U64],
            )),
            vec![Value::u64(1)],
        );
        let b = Value::closure(
            Box::new(MockAbstractFunction::new(
                "f",
                vec![],
                ClosureMask::new(0b1),
                vec![MoveTypeLayout::U64],
            )),
            vec![Value::u64(2)],
        );
        let serialize = |v: &Value| {
            assert_ok!(ValueSerDeContext::new(None)
                .with_func_args_deserialization(&ext_mock)
                .serialize(v, &fun_layout))
            .expect("serialization result not None")
        };
        let deserialize = |bytes: &[u8]| {
            assert_ok!(ValueSerDeContext::new(None)
                .with_func_args_deserialization(&ext_mock)
                .deserialize_or_err(bytes, &fun_layout))
        };
        let a = deserialize(&serialize(&a));
        let b = deserialize(&serialize(&b));

        // Unequal blobs with equal identity need materialization, which requires a
        // materializer.
        let err = a.equals(&b).expect_err("materializer is required");
        assert_eq!(
            err.major_status(),
            move_core_types::vm_status::StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR
        );
    }

    #[test]
    fn closure_serialization_disabled() {
        let mut ext_mock = MockFunctionValueExtension::new();
        ext_mock
            .expect_is_function_data_format_v2_enabled()
            .returning(|| false);
        ext_mock
            .expect_get_serialization_data()
            .returning(move |af| {
                Ok(af
                    .downcast_ref::<MockAbstractFunction>()
                    .expect("cast")
                    .data
                    .clone())
            });

        let make_closure = || {
            let fun = MockAbstractFunction::new("f", vec![], ClosureMask::new(0b1), vec![
                MoveTypeLayout::U64,
            ]);
            Value::closure(Box::new(fun), vec![Value::u64(22)])
        };

        // Closure serialization fails when disabled.
        let result = assert_ok!(ValueSerDeContext::new(None)
            .with_func_args_deserialization(&ext_mock)
            .with_closure_serialization_disabled(true)
            .serialize(&make_closure(), &make_fun_layout()));
        assert_none!(result);
        assert_err!(ValueSerDeContext::new(None)
            .with_func_args_deserialization(&ext_mock)
            .with_closure_serialization_disabled(true)
            .serialized_size(&make_closure(), &make_fun_layout()));

        // Values containing a closure fail as well.
        let struct_layout = MoveTypeLayout::new_struct(MoveStructLayout::Runtime(vec![
            make_fun_layout(),
            MoveTypeLayout::U64,
        ]));
        let struct_value = Value::struct_(Struct::pack(vec![make_closure(), Value::u64(7)]));
        let result = assert_ok!(ValueSerDeContext::new(None)
            .with_func_args_deserialization(&ext_mock)
            .with_closure_serialization_disabled(true)
            .serialize(&struct_value, &struct_layout));
        assert_none!(result);

        // Values without closures are unaffected.
        let result = assert_ok!(ValueSerDeContext::new(None)
            .with_func_args_deserialization(&ext_mock)
            .with_closure_serialization_disabled(true)
            .serialize(&Value::u64(7), &MoveTypeLayout::U64));
        assert_some!(result);
    }

    // ======================================================================================
    // Serialization size tests

    #[test]
    fn test_serialized_size() {
        use IdentifierMappingKind::*;
        use MoveStructLayout::*;
        use MoveTypeLayout::*;

        let u64_delayed_value = Value::delayed_value(DelayedFieldID::new_with_width(12, 8));
        let u128_delayed_value = Value::delayed_value(DelayedFieldID::new_with_width(123, 16));
        let derived_string_delayed_value =
            Value::delayed_value(DelayedFieldID::new_with_width(12, 60));

        // First field is a string, second field is a padding to ensure constant size.
        let derived_string_layout = MoveTypeLayout::new_struct(Runtime(vec![
            MoveTypeLayout::new_struct(Runtime(vec![Vector(Box::new(U8))])),
            Vector(Box::new(U8)),
        ]));

        // All these pairs should serialize.
        let good_values_layouts_sizes = [
            (Value::u8(10), U8),
            (Value::u16(10), U16),
            (Value::u32(10), U32),
            (Value::u64(10), U64),
            (Value::u128(10), U128),
            (Value::u256(int256::U256::ONE), U256),
            (Value::bool(true), Bool),
            (Value::address(AccountAddress::ONE), Address),
            (Value::master_signer(AccountAddress::ONE), Signer),
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
                MoveTypeLayout::new_struct(Runtime(vec![Bool, Vector(Box::new(U32))])),
            ),
        ];
        for (value, layout) in good_values_layouts_sizes {
            let bytes = assert_some!(assert_ok!(ValueSerDeContext::new(None)
                .with_delayed_fields_serde()
                .serialize(&value, &layout)));

            let size = assert_ok!(ValueSerDeContext::new(None)
                .with_delayed_fields_serde()
                .serialized_size(&value, &layout));
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
            assert_err!(ValueSerDeContext::new(None)
                .with_delayed_fields_serde()
                .serialized_size(&value, &layout));
        }
    }

    // ======================================================================================
    // Signer

    #[test]
    fn new_signer_round_trip_vm_value() {
        let move_value = MoveValue::Signer(AccountAddress::ZERO);
        let bytes = move_value.simple_serialize().unwrap();

        let vm_value = Value::master_signer(AccountAddress::ZERO);
        let vm_bytes = ValueSerDeContext::new(None)
            .serialize(&vm_value, &MoveTypeLayout::Signer)
            .unwrap()
            .unwrap();

        // VM Value Roundtrip
        assert!(ValueSerDeContext::new(None)
            .deserialize(&vm_bytes, &MoveTypeLayout::Signer)
            .unwrap()
            .equals(&vm_value)
            .unwrap());

        // MoveValue Roundtrip
        assert!(MoveValue::simple_deserialize(&bytes, &MoveTypeLayout::Signer).is_err());

        // ser(MoveValue) == ser(VMValue)
        assert_eq!(bytes, vm_bytes);

        // Permissioned Signer Roundtrip
        let vm_value = Value::permissioned_signer(AccountAddress::ZERO, AccountAddress::ONE);
        let vm_bytes = ValueSerDeContext::new(None)
            .serialize(&vm_value, &MoveTypeLayout::Signer)
            .unwrap()
            .unwrap();

        // VM Value Roundtrip
        assert!(ValueSerDeContext::new(None)
            .deserialize(&vm_bytes, &MoveTypeLayout::Signer)
            .unwrap()
            .equals(&vm_value)
            .unwrap());

        // Cannot serialize permissioned signer into bytes with legacy signer
        assert!(ValueSerDeContext::new(None)
            .with_legacy_signer()
            .serialize(&vm_value, &MoveTypeLayout::Signer)
            .unwrap()
            .is_none());
    }

    #[test]
    fn legacy_signer_round_trip_vm_value() {
        let move_value = MoveValue::Address(AccountAddress::ZERO);
        let bytes = move_value.simple_serialize().unwrap();

        let vm_value = Value::master_signer(AccountAddress::ZERO);
        let vm_bytes = ValueSerDeContext::new(None)
            .with_legacy_signer()
            .serialize(&vm_value, &MoveTypeLayout::Signer)
            .unwrap()
            .unwrap();

        // VM Value Roundtrip
        assert!(ValueSerDeContext::new(None)
            .with_legacy_signer()
            .deserialize(&vm_bytes, &MoveTypeLayout::Signer)
            .is_none());

        // ser(MoveValue) == ser(VMValue)
        assert_eq!(bytes, vm_bytes);
    }
}
