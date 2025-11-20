// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use super::*;
use crate::{
    value_serde::{MockFunctionValueExtension, ValueSerDeContext},
    values::{function_values_impl::mock, prop::layout_and_value_strategy},
};
use better_any::TidExt;
use move_core_types::{value::MoveValue, vm_status::StatusCode};
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(10000))]
    #[test]
    fn serializer_round_trip((layout, value) in layout_and_value_strategy()) {
        // Set up mock function extension for function value serialization
        let mut ext_mock = MockFunctionValueExtension::new();
        ext_mock
            .expect_get_serialization_data()
            .returning(move |af| {
                Ok(af
                    .downcast_ref::<mock::MockAbstractFunction>()
                    .expect("Should be a mock abstract function")
                    .data.clone())
            });
        ext_mock
            .expect_create_from_serialization_data()
            .returning(move |data| Ok(Box::new(mock::MockAbstractFunction::new_from_data(data))));

        let ctx = ValueSerDeContext::new(None).with_func_args_deserialization(&ext_mock);
        let blob = ctx.serialize(&value, &layout).unwrap().expect("must serialize");
        let value_deserialized = ValueSerDeContext::new(None).with_func_args_deserialization(&ext_mock).deserialize(&blob, &layout).expect("must deserialize");
        assert!(value.equals(&value_deserialized).unwrap());

        let move_value = value.as_move_value(&layout);

        let blob2 = move_value.simple_serialize().expect("must serialize");
        assert_eq!(blob, blob2);

        let move_value_deserialized = MoveValue::simple_deserialize(&blob2, &layout).expect("must deserialize.");
        assert_eq!(move_value, move_value_deserialized);
    }
}

#[derive(Clone, Debug)]
struct Mutation {
    change_tag: bool,
    new_tag: u16,
    payload: u64,
}

fn mutation_strategy() -> impl Strategy<Value = Mutation> {
    (any::<bool>(), any::<u16>(), any::<u64>()).prop_map(|(change_tag, new_tag, payload)| {
        Mutation {
            change_tag,
            new_tag,
            payload,
        }
    })
}

#[derive(Clone, Debug)]
enum Step {
    Mut(Mutation),
    Read,
    Write,
    Equals,
    Compare,
}

fn step_strategy() -> impl Strategy<Value = Step> {
    prop_oneof![
        3 => mutation_strategy().prop_map(Step::Mut),
        1 => Just(Step::Read),
        1 => Just(Step::Write),
        1 => Just(Step::Equals),
        1 => Just(Step::Compare),
    ]
}

fn variant_to_str(idx: u16) -> String {
    format!("variant {idx}")
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(10000))]
    #[test]
    fn indexed_ref_tag_guard_interleaved(
        steps in proptest::collection::vec(step_strategy(), 0..10),
        variant_count in 2u16..=8u16,
        stored_tag in any::<u16>(),
        initial_payload in any::<u64>(),
    ) {
        let stored_tag = stored_tag % variant_count;
        // Build enum value and place into locals
        let mut locals = Locals::new(1);
        locals.store_loc(
            0,
            Value::struct_(Struct::pack_variant(stored_tag, vec![Value::u64(initial_payload)])),
        ).expect("store ok");
        let struct_ref: StructRef = locals.borrow_loc(0).expect("borrow_loc ok").value_as().expect("as StructRef");

        // Create stale references/values BEFORE any mutations
        let allowed = [stored_tag];
        let stale_read: Reference = struct_ref
            .borrow_variant_field(&allowed, 0, &variant_to_str)
            .expect("variant borrow ok")
            .value_as()
            .expect("as Reference");
        let stale_write: Reference = struct_ref
            .borrow_variant_field(&allowed, 0, &variant_to_str)
            .expect("variant borrow ok")
            .value_as()
            .expect("as Reference");
        let mut stale_read = Some(stale_read);
        let mut stale_write = Some(stale_write);
        let eq_left = struct_ref.borrow_variant_field(&allowed, 0, &variant_to_str).expect("variant borrow ok");
        let eq_right = struct_ref.borrow_variant_field(&allowed, 0, &variant_to_str).expect("variant borrow ok");
        let cmp_left = struct_ref.borrow_variant_field(&allowed, 0, &variant_to_str).expect("variant borrow ok");
        let cmp_right = struct_ref.borrow_variant_field(&allowed, 0, &variant_to_str).expect("variant borrow ok");

        let mut used_eq = false;
        let mut used_cmp = false;

        let mut current_tag = stored_tag;
        for step in &steps {
            match step {
                Step::Mut(m) => {
                    // Apply mutation via public API. Keep tags within [0, variant_count).
                    let candidate = if m.change_tag {
                        let mut c = m.new_tag % variant_count;
                        if c == current_tag {
                            c = (c + 1) % variant_count;
                        }
                        c
                    } else {
                        current_tag
                    };
                    let tag_ref: Reference = struct_ref.borrow_field(0).expect("borrow tag").value_as().expect("as Reference");
                    prop_assert!(tag_ref.write_ref(Value::u16(candidate)).is_ok());
                    let payload_ref: Reference = struct_ref.borrow_field(1).expect("borrow payload").value_as().expect("as Reference");
                    prop_assert!(payload_ref.write_ref(Value::u64(m.payload)).is_ok());
                    current_tag = candidate;
                },
                Step::Read => {
                    if let Some(r) = stale_read.take() {
                    let res = r.read_ref();
                    if current_tag == stored_tag {
                        prop_assert!(res.is_ok());
                    } else {
                        let err = res.expect_err("tag mismatch should error");
                        prop_assert_eq!(err.major_status(), StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR);
                        prop_assert!(err.message().map(|m| m.starts_with("invalid enum tag")).unwrap_or(false));
                    }
                    }
                },
                Step::Write => {
                    if let Some(r) = stale_write.take() {
                    let res = r.write_ref(Value::u64(12345));
                    if current_tag == stored_tag {
                        prop_assert!(res.is_ok());
                    } else {
                        let err = res.expect_err("tag mismatch should error");
                        prop_assert_eq!(err.major_status(), StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR);
                        prop_assert!(err.message().map(|m| m.starts_with("invalid enum tag")).unwrap_or(false));
                    }
                    }
                },
                Step::Equals if !used_eq => {
                    used_eq = true;
                    let res = eq_left.equals(&eq_right);
                    if current_tag == stored_tag {
                        prop_assert!(res.is_ok());
                    } else {
                        let err = res.expect_err("tag mismatch should error");
                        prop_assert_eq!(err.major_status(), StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR);
                        prop_assert!(err.message().map(|m| m.starts_with("invalid enum tag")).unwrap_or(false));
                    }
                },
                Step::Compare if !used_cmp => {
                    used_cmp = true;
                    let res = cmp_left.compare(&cmp_right);
                    if current_tag == stored_tag {
                        prop_assert!(res.is_ok());
                    } else {
                        let err = res.expect_err("tag mismatch should error");
                        prop_assert_eq!(err.major_status(), StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR);
                        prop_assert!(err.message().map(|m| m.starts_with("invalid enum tag")).unwrap_or(false));
                    }
                },
                _ => {}
            }
        }
    }
}
