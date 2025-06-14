// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

//! Contains tests for serialization

#[cfg(test)]
mod tests {
    use crate::{
        delayed_values::delayed_field_id::DelayedFieldID,
        value_serde::{MockFunctionValueExtension, ValueSerDeContext},
        values::{function_values_impl::mock::MockAbstractFunction, values_impl, Struct, Value},
    };
    use better_any::TidExt;
    use claims::{assert_err, assert_ok, assert_some};
    use move_binary_format::errors::PartialVMResult;
    use move_core_types::{
        ability::AbilitySet,
        account_address::AccountAddress,
        function::{ClosureMask, MoveClosure},
        identifier::Identifier,
        language_storage::{FunctionTag, ModuleId, StructTag, TypeTag},
        u256,
        value::{IdentifierMappingKind, MoveStruct, MoveStructLayout, MoveTypeLayout, MoveValue},
    };
    use serde::{Deserialize, Serialize};
    use std::iter;
    // ==========================================================================
    // Enums

    fn enum_layout() -> MoveTypeLayout {
        MoveTypeLayout::Struct(MoveStructLayout::RuntimeVariants(vec![
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
            let blob = ValueSerDeContext::new()
                .serialize(&value, &layout)
                .unwrap()
                .expect("serialization succeeds");
            let de_value = ValueSerDeContext::new()
                .deserialize(&blob, &layout)
                .expect("deserialization succeeds");
            assert!(
                value.equals(&de_value).unwrap(),
                "roundtrip serialization succeeds"
            )
        }
        let bad_tag_value = Value::struct_(Struct::pack_variant(3, [Value::u64(42)]));
        assert!(
            ValueSerDeContext::new()
                .serialize(&bad_tag_value, &layout)
                .unwrap()
                .is_none(),
            "serialization fails"
        );
        let bad_struct_value = Value::struct_(Struct::pack([Value::u64(42)]));
        assert!(
            ValueSerDeContext::new()
                .serialize(&bad_struct_value, &layout)
                .unwrap()
                .is_none(),
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
            let from_move = ValueSerDeContext::new()
                .serialize(&move_value, &layout)
                .unwrap()
                .expect("from move succeeds");
            let to_rust = bcs::from_bytes::<RustEnum>(&from_move).expect("to rust successful");
            assert_eq!(to_rust, rust_value);

            let from_rust = bcs::to_bytes(&rust_value).expect("from rust succeeds");
            let to_move = ValueSerDeContext::new()
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
                args: vec![TypeTag::Struct(Box::new(StructTag {
                    address: AccountAddress::TEN,
                    module: Identifier::new("mod").unwrap(),
                    name: Identifier::new("st").unwrap(),
                    type_args: vec![TypeTag::Signer],
                }))],
                results: vec![TypeTag::Address],
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
            captured,
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
        fun: MockAbstractFunction,
        captured: Vec<Value>,
    ) -> (Value, PartialVMResult<Value>) {
        let fun_layout = make_fun_layout();
        let mut ext_mock = MockFunctionValueExtension::new();
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
        let blob = assert_ok!(ValueSerDeContext::new()
            .with_func_args_deserialization(&ext_mock)
            .serialize(&value, &fun_layout))
        .expect("serialization result not None");
        let de_value = ValueSerDeContext::new()
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

        let (_, de_value) = round_trip_vm_closure_value(
            MockAbstractFunction::new("f", vec![], ClosureMask::new(0b1), vec![
                MoveTypeLayout::Bool,
            ]),
            vec![],
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
                Struct(Runtime(vec![Bool, Vector(Box::new(U32))])),
            ),
        ];
        for (value, layout) in good_values_layouts_sizes {
            let bytes = assert_some!(assert_ok!(ValueSerDeContext::new()
                .with_delayed_fields_serde()
                .serialize(&value, &layout)));

            let size = assert_ok!(ValueSerDeContext::new()
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
            assert_err!(ValueSerDeContext::new()
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
        let vm_bytes = ValueSerDeContext::new()
            .serialize(&vm_value, &MoveTypeLayout::Signer)
            .unwrap()
            .unwrap();

        // VM Value Roundtrip
        assert!(ValueSerDeContext::new()
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
        let vm_bytes = ValueSerDeContext::new()
            .serialize(&vm_value, &MoveTypeLayout::Signer)
            .unwrap()
            .unwrap();

        // VM Value Roundtrip
        assert!(ValueSerDeContext::new()
            .deserialize(&vm_bytes, &MoveTypeLayout::Signer)
            .unwrap()
            .equals(&vm_value)
            .unwrap());

        // Cannot serialize permissioned signer into bytes with legacy signer
        assert!(ValueSerDeContext::new()
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
        let vm_bytes = ValueSerDeContext::new()
            .with_legacy_signer()
            .serialize(&vm_value, &MoveTypeLayout::Signer)
            .unwrap()
            .unwrap();

        // VM Value Roundtrip
        assert!(ValueSerDeContext::new()
            .with_legacy_signer()
            .deserialize(&vm_bytes, &MoveTypeLayout::Signer)
            .is_none());

        // ser(MoveValue) == ser(VMValue)
        assert_eq!(bytes, vm_bytes);
    }
}
