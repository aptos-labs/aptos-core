// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    value_serde::{MockFunctionValueExtension, ValueSerDeContext},
    values::{AbstractFunction, GlobalValue, SerializedFunctionData, Struct, StructRef, Value},
};
use better_any::{Tid, TidAble, TidExt};
use claims::{assert_err, assert_none, assert_ok, assert_some};
use move_binary_format::errors::PartialVMResult;
use move_core_types::{
    account_address::AccountAddress,
    function::{ClosureMask, FUNCTION_DATA_SERIALIZATION_FORMAT_V1},
    ident_str,
    language_storage::ModuleId,
    value::{MoveStructLayout, MoveTypeLayout},
    vm_status::StatusCode,
};
use std::{cmp::Ordering, fmt::Debug};

#[derive(Clone, Tid)]
struct MockFunction {
    data: SerializedFunctionData,
}

impl MockFunction {
    fn closure(
        mask: ClosureMask,
        captured: impl IntoIterator<Item = Value>,
        captured_layouts: impl IntoIterator<Item = MoveTypeLayout>,
    ) -> Value {
        let data = SerializedFunctionData {
            format_version: FUNCTION_DATA_SERIALIZATION_FORMAT_V1,
            module_id: ModuleId::new(AccountAddress::ONE, ident_str!("mock").to_owned()),
            fun_id: ident_str!("mock").to_owned(),
            ty_args: vec![],
            mask,
            captured_layouts: captured_layouts.into_iter().collect(),
        };
        Value::closure(Box::new(Self { data }), captured)
    }
}

impl AbstractFunction for MockFunction {
    fn closure_mask(&self) -> ClosureMask {
        self.data.mask
    }

    fn cmp_dyn(&self, _other: &dyn AbstractFunction) -> PartialVMResult<Ordering> {
        Ok(Ordering::Equal)
    }

    fn clone_dyn(&self) -> PartialVMResult<Box<dyn AbstractFunction>> {
        Ok(Box::new(self.clone()))
    }

    fn to_canonical_string(&self) -> String {
        "0x1::mock::mock".to_string()
    }
}

#[test]
fn test_equals() {
    test_binop_with_max_depth(|l, r, max_depth| l.equals_with_depth(r, max_depth));
}

#[test]
fn test_compare() {
    test_binop_with_max_depth(|l, r, max_depth| l.compare_with_depth(r, max_depth));
}

#[test]
fn test_copy_value() {
    test_unop_with_max_depth(|v, max_depth| v.copy_value_with_depth(max_depth));

    // Special-case: reference clone Rcs, so their depth can be larger.
    let v = assert_ok!(GlobalValue::cached(Value::struct_(Struct::pack(vec![
        Value::u8(0)
    ]))));
    let v_ref = assert_ok!(v.borrow_global());
    assert_ok!(v_ref.copy_value_with_depth(3));
    assert_ok!(v_ref.copy_value_with_depth(2));
    assert_ok!(v_ref.copy_value_with_depth(1));
}

#[test]
fn test_read_ref() {
    let v = assert_ok!(GlobalValue::cached(Value::struct_(Struct::pack(vec![
        Value::u8(0)
    ]))));
    let v_ref = assert_ok!(assert_ok!(v.borrow_global()).value_as::<StructRef>());

    // Note: reading a reference will clone the value, so here it is a clone of a struct with 1
    // field of depth 2.
    assert_ok!(v_ref.read_ref_with_depth(2));

    let v_ref = assert_ok!(assert_ok!(v.borrow_global()).value_as::<StructRef>());
    let err = assert_err!(v_ref.read_ref_with_depth(1));
    assert_eq!(err.major_status(), StatusCode::VM_MAX_VALUE_DEPTH_REACHED);
}

#[test]
fn test_serialization() {
    use MoveStructLayout::*;
    use MoveTypeLayout as L;

    let mut extension = MockFunctionValueExtension::new();
    extension
        .expect_get_serialization_data()
        .returning(move |af| Ok(af.downcast_ref::<MockFunction>().unwrap().data.clone()));

    let depth_1_ok = [
        (Value::u64(0), L::U64),
        (Value::vector_u8(vec![0, 1]), L::Vector(Box::new(L::U8))),
        (
            MockFunction::closure(ClosureMask::empty(), vec![], vec![]),
            L::Function,
        ),
    ];
    let depth_2_ok = [
        (
            Value::struct_(Struct::pack(vec![Value::u16(0)])),
            L::Struct(Runtime(vec![L::U16])),
        ),
        (
            Value::vector_for_testing_only(vec![Value::vector_u8(vec![0, 1])]),
            L::Vector(Box::new(L::Vector(Box::new(L::U8)))),
        ),
        (
            // Serialize first variant, so the depth is 2.
            Value::struct_(Struct::pack(vec![Value::u16(0), Value::bool(true)])),
            L::Struct(RuntimeVariants(vec![vec![L::Bool], vec![L::Vector(
                Box::new(L::Vector(Box::new(L::U8))),
            )]])),
        ),
        (
            MockFunction::closure(ClosureMask::empty(), vec![Value::u16(0)], vec![L::U16]),
            L::Function,
        ),
    ];
    let depth_3_ok = [(
        // Serialize second variant, so the depth is 3.
        Value::struct_(Struct::pack(vec![
            Value::u16(1),
            Value::vector_for_testing_only(vec![Value::vector_u8(vec![1, 2])]),
        ])),
        L::Struct(RuntimeVariants(vec![vec![L::Bool], vec![L::Vector(
            Box::new(L::Vector(Box::new(L::U8))),
        )]])),
    )];

    let ctx = |max_depth: u64| {
        ValueSerDeContext::new(Some(max_depth)).with_func_args_deserialization(&extension)
    };

    for (v, l) in &depth_1_ok {
        assert_some!(assert_ok!(ctx(1).serialize(v, l)));
        assert_ok!(ctx(1).serialized_size(v, l));
    }

    for (v, l) in &depth_2_ok {
        assert_some!(assert_ok!(ctx(2).serialize(v, l)));
        assert_ok!(ctx(2).serialized_size(v, l));
        assert_none!(assert_ok!(ctx(1).serialize(v, l)));
        assert_err!(ctx(1).serialized_size(v, l));
    }

    for (v, l) in &depth_3_ok {
        assert_some!(assert_ok!(ctx(3).serialize(v, l)));
        assert_ok!(ctx(3).serialized_size(v, l));
        assert_none!(assert_ok!(ctx(2).serialize(v, l)));
        assert_err!(ctx(2).serialized_size(v, l));
        assert_none!(assert_ok!(ctx(1).serialize(v, l)));
        assert_err!(ctx(1).serialized_size(v, l));
    }
}

fn test_binop_with_max_depth<F, T>(f: F)
where
    T: Debug,
    F: Fn(&Value, &Value, u64) -> PartialVMResult<T>,
{
    let v = Value::u8(0);
    assert_ok!(f(&v, &v, 1));

    let v = Value::vector_u8(vec![0, 1]);
    assert_ok!(f(&v, &v, 1));

    let v = Value::vector_for_testing_only(vec![Value::vector_u8(vec![0, 1])]);
    let err = assert_err!(f(&v, &v, 1));
    assert_eq!(err.major_status(), StatusCode::VM_MAX_VALUE_DEPTH_REACHED);

    let v = Value::struct_(Struct::pack(vec![Value::u8(0)]));
    let err = assert_err!(f(&v, &v, 1));
    assert_eq!(err.major_status(), StatusCode::VM_MAX_VALUE_DEPTH_REACHED);

    let v = MockFunction::closure(ClosureMask::empty(), vec![], vec![]);
    assert_ok!(f(&v, &v, 1));

    let v = MockFunction::closure(ClosureMask::new_for_leading(1), vec![Value::u8(0)], vec![
        MoveTypeLayout::U8,
    ]);
    let err = assert_err!(f(&v, &v, 1));
    assert_eq!(err.major_status(), StatusCode::VM_MAX_VALUE_DEPTH_REACHED);

    // Create a reference to struct with 1 field (3 nodes).
    let v = assert_ok!(GlobalValue::cached(Value::struct_(Struct::pack(vec![
        Value::u8(0)
    ]))));
    let v_ref = assert_ok!(v.borrow_global());
    assert_ok!(f(&v_ref, &v_ref, 3));
    let err = assert_err!(f(&v_ref, &v_ref, 2));
    assert_eq!(err.major_status(), StatusCode::VM_MAX_VALUE_DEPTH_REACHED);
}

fn test_unop_with_max_depth<F, T>(f: F)
where
    T: Debug,
    F: Fn(&Value, u64) -> PartialVMResult<T>,
{
    let v = Value::u8(0);
    assert_ok!(f(&v, 1));

    let v = Value::vector_u8(vec![0, 1]);
    assert_ok!(f(&v, 1));

    let v = Value::vector_for_testing_only(vec![Value::vector_u8(vec![0, 1])]);
    let err = assert_err!(f(&v, 1));
    assert_eq!(err.major_status(), StatusCode::VM_MAX_VALUE_DEPTH_REACHED);

    let v = Value::struct_(Struct::pack(vec![Value::u8(0)]));
    let err = assert_err!(f(&v, 1));
    assert_eq!(err.major_status(), StatusCode::VM_MAX_VALUE_DEPTH_REACHED);

    let v = MockFunction::closure(ClosureMask::empty(), vec![], vec![]);
    assert_ok!(f(&v, 1));

    let v = MockFunction::closure(ClosureMask::new_for_leading(1), vec![Value::u8(0)], vec![
        MoveTypeLayout::U8,
    ]);
    let err = assert_err!(f(&v, 1));
    assert_eq!(err.major_status(), StatusCode::VM_MAX_VALUE_DEPTH_REACHED);
}
