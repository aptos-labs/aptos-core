// Parts of the file are Copyright (c) The Diem Core Contributors
// Parts of the file are Copyright (c) The Move Contributors
// Parts of the file are Copyright (c) Aptos Foundation
// All Aptos Foundation code and content is licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    value_serde::{MockFunctionValueExtension, ValueSerDeContext},
    values::{
        closure_captured_depth, AbstractFunction, Closure, GlobalValue, SerializedFunctionData,
        Struct, StructRef, Value,
    },
};
use better_any::{Tid, TidAble, TidExt};
use claims::{assert_err, assert_none, assert_ok, assert_some};
use move_binary_format::errors::PartialVMResult;
use move_core_types::{
    account_address::AccountAddress,
    function::ClosureMask,
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
            module_id: ModuleId::new(AccountAddress::ONE, ident_str!("mock").to_owned()),
            fun_id: ident_str!("mock").to_owned(),
            ty_args: vec![],
            mask,
            captured_depth: 0,
            captured_layouts: Some(captured_layouts.into_iter().collect()),
        };
        Value::closure(Box::new(Self { data }), captured)
    }

    /// A closure whose captured arguments are still an opaque blob, reporting the
    /// given cached `depth` (as a stored V2 closure would).
    fn serialized_closure(mask: ClosureMask, depth: u16, blob: Vec<u8>) -> Value {
        let data = SerializedFunctionData {
            module_id: ModuleId::new(AccountAddress::ONE, ident_str!("mock").to_owned()),
            fun_id: ident_str!("mock").to_owned(),
            ty_args: vec![],
            mask,
            captured_depth: depth,
            captured_layouts: None,
        };
        Value::ClosureValue(Closure::pack_serialized(Box::new(Self { data }), blob))
    }
}

impl AbstractFunction for MockFunction {
    fn closure_mask(&self) -> ClosureMask {
        self.data.mask
    }

    fn captured_depth(&self) -> u16 {
        self.data.captured_depth
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
fn serialized_closure_depth_sees_through_blob() {
    // A serialized (opaque-blob) closure contributes its cached captured depth to a
    // depth traversal, instead of being counted as a single level. This closes the
    // repack bomb: the true nesting hidden inside the blob is visible in O(1).
    let mask = ClosureMask::new_for_leading(1).expect("one capture");
    let stored_depth = 5u16;
    let value = MockFunction::serialized_closure(mask, stored_depth, vec![0]);

    // The traversal observes the cached depth, not 1.
    assert_eq!(
        assert_ok!(value.check_depth_of_value(u64::MAX)),
        stored_depth as u64
    );

    // The limit is enforced against the cached depth: one below it fails.
    let err = assert_err!(value.check_depth_of_value(stored_depth as u64 - 1));
    assert_eq!(err.major_status(), StatusCode::VM_MAX_VALUE_DEPTH_REACHED);
}

#[test]
fn repack_bomb_rejected_by_captured_depth() {
    // The repack bomb: a serialized closure whose captured arguments already nest
    // near the limit is captured again into a new closure. The pack-time depth
    // computation sees through the blob via the cached depth and rejects it. Before
    // the fix, the opaque blob counted as a single level and the bomb slipped
    // through. This mirrors `Interpreter::compute_and_check_closure_depth`, which
    // calls `closure_captured_depth` with `max_value_nest_depth - 1` as the limit.
    let max_value_nest_depth = 128u64;
    let per_captured_limit = max_value_nest_depth - 1;
    let mask = ClosureMask::new_for_leading(1).expect("one capture");

    // A captured value one below the limit is fine: the new closure sits exactly at
    // the limit.
    let ok_inner = MockFunction::serialized_closure(mask, per_captured_limit as u16, vec![0]);
    assert_eq!(
        assert_ok!(closure_captured_depth(&[ok_inner], per_captured_limit)),
        max_value_nest_depth as u16
    );

    // A captured value at the limit pushes the new closure past it: rejected.
    let bomb_inner = MockFunction::serialized_closure(mask, max_value_nest_depth as u16, vec![0]);
    let err = assert_err!(closure_captured_depth(&[bomb_inner], per_captured_limit));
    assert_eq!(err.major_status(), StatusCode::VM_MAX_VALUE_DEPTH_REACHED);
}

#[test]
fn test_equals() {
    test_binop_with_max_depth(|l, r, max_depth| l.equals_with_depth_for_test(r, max_depth));
}

#[test]
fn test_compare() {
    test_binop_with_max_depth(|l, r, max_depth| l.compare_with_depth_for_test(r, max_depth));
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
        .expect_is_function_data_format_v2_enabled()
        .returning(|| false);
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
            L::new_struct(Runtime(vec![L::U16])),
        ),
        (
            Value::vector_unchecked(vec![Value::vector_u8(vec![0, 1])]).unwrap(),
            L::Vector(Box::new(L::Vector(Box::new(L::U8)))),
        ),
        (
            // Serialize first variant, so the depth is 2.
            Value::struct_(Struct::pack(vec![Value::u16(0), Value::bool(true)])),
            L::new_struct(RuntimeVariants(vec![vec![L::Bool], vec![L::Vector(
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
            Value::vector_unchecked(vec![Value::vector_u8(vec![1, 2])]).unwrap(),
        ])),
        L::new_struct(RuntimeVariants(vec![vec![L::Bool], vec![L::Vector(
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

    let v = Value::vector_unchecked(vec![Value::vector_u8(vec![0, 1])]).unwrap();
    let err = assert_err!(f(&v, &v, 1));
    assert_eq!(err.major_status(), StatusCode::VM_MAX_VALUE_DEPTH_REACHED);

    let v = Value::struct_(Struct::pack(vec![Value::u8(0)]));
    let err = assert_err!(f(&v, &v, 1));
    assert_eq!(err.major_status(), StatusCode::VM_MAX_VALUE_DEPTH_REACHED);

    let v = MockFunction::closure(ClosureMask::empty(), vec![], vec![]);
    assert_ok!(f(&v, &v, 1));

    let v = MockFunction::closure(
        ClosureMask::new_for_leading(1).expect("Capturing one argument should not fail"),
        vec![Value::u8(0)],
        vec![MoveTypeLayout::U8],
    );
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

    let v = Value::vector_unchecked(vec![Value::vector_u8(vec![0, 1])]).unwrap();
    let err = assert_err!(f(&v, 1));
    assert_eq!(err.major_status(), StatusCode::VM_MAX_VALUE_DEPTH_REACHED);

    let v = Value::struct_(Struct::pack(vec![Value::u8(0)]));
    let err = assert_err!(f(&v, 1));
    assert_eq!(err.major_status(), StatusCode::VM_MAX_VALUE_DEPTH_REACHED);

    let v = MockFunction::closure(ClosureMask::empty(), vec![], vec![]);
    assert_ok!(f(&v, 1));

    let v = MockFunction::closure(
        ClosureMask::new_for_leading(1).expect("Capturing one argument should not fail"),
        vec![Value::u8(0)],
        vec![MoveTypeLayout::U8],
    );
    let err = assert_err!(f(&v, 1));
    assert_eq!(err.major_status(), StatusCode::VM_MAX_VALUE_DEPTH_REACHED);
}
