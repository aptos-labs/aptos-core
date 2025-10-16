// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{delayed_values::delayed_field_id::DelayedFieldID, values::*};
use claims::{assert_err, assert_ok};
use move_binary_format::errors::*;
use move_core_types::{
    account_address::AccountAddress,
    int256::{I256, U256},
};

#[test]
fn locals() -> PartialVMResult<()> {
    const LEN: usize = 4;
    let mut locals = Locals::new(LEN);
    for i in 0..LEN {
        assert!(locals.copy_loc(i).is_err());
        assert!(locals.move_loc(i).is_err());
        assert!(locals.borrow_loc(i).is_err());
    }
    locals.store_loc(1, Value::u64(42))?;

    assert!(locals.copy_loc(1)?.equals(&Value::u64(42))?);
    let r = locals.borrow_loc(1)?.value_as::<Reference>()?;
    assert!(r.read_ref()?.equals(&Value::u64(42))?);
    assert!(locals.move_loc(1)?.equals(&Value::u64(42))?);

    assert!(locals.copy_loc(1).is_err());
    assert!(locals.move_loc(1).is_err());
    assert!(locals.borrow_loc(1).is_err());

    assert!(locals.copy_loc(LEN + 1).is_err());
    assert!(locals.move_loc(LEN + 1).is_err());
    assert!(locals.borrow_loc(LEN + 1).is_err());

    Ok(())
}

#[test]
fn struct_pack_and_unpack() -> PartialVMResult<()> {
    let vals = vec![
        Value::u8(10),
        Value::u16(12),
        Value::u32(15),
        Value::u64(20),
        Value::u128(30),
        Value::u256(U256::MAX),
    ];
    let s = Struct::pack(vec![
        Value::u8(10),
        Value::u16(12),
        Value::u32(15),
        Value::u64(20),
        Value::u128(30),
        Value::u256(U256::MAX),
    ]);
    let unpacked: Vec<_> = s.unpack()?.collect();

    assert!(vals.len() == unpacked.len());
    for (v1, v2) in vals.iter().zip(unpacked.iter()) {
        assert!(v1.equals(v2)?);
    }

    Ok(())
}

#[test]
fn struct_borrow_field() -> PartialVMResult<()> {
    let mut locals = Locals::new(1);
    locals.store_loc(
        0,
        Value::struct_(Struct::pack(vec![Value::u8(10), Value::bool(false)])),
    )?;
    let r: StructRef = locals.borrow_loc(0)?.value_as()?;

    {
        let f: Reference = r.borrow_field(1)?.value_as()?;
        assert!(f.read_ref()?.equals(&Value::bool(false))?);
    }

    {
        let f: Reference = r.borrow_field(1)?.value_as()?;
        f.write_ref(Value::bool(true))?;
    }

    {
        let f: Reference = r.borrow_field(1)?.value_as()?;
        assert!(f.read_ref()?.equals(&Value::bool(true))?);
    }

    Ok(())
}

#[test]
fn struct_borrow_nested() -> PartialVMResult<()> {
    let mut locals = Locals::new(1);

    fn inner(x: u64) -> Value {
        Value::struct_(Struct::pack(vec![Value::u64(x)]))
    }
    fn outer(x: u64) -> Value {
        Value::struct_(Struct::pack(vec![Value::u8(10), inner(x)]))
    }

    locals.store_loc(0, outer(20))?;
    let r1: StructRef = locals.borrow_loc(0)?.value_as()?;
    let r2: StructRef = r1.borrow_field(1)?.value_as()?;

    {
        let r3: Reference = r2.borrow_field(0)?.value_as()?;
        assert!(r3.read_ref()?.equals(&Value::u64(20))?);
    }

    {
        let r3: Reference = r2.borrow_field(0)?.value_as()?;
        r3.write_ref(Value::u64(30))?;
    }

    {
        let r3: Reference = r2.borrow_field(0)?.value_as()?;
        assert!(r3.read_ref()?.equals(&Value::u64(30))?);
    }

    assert!(r2.read_ref()?.equals(&inner(30))?);
    assert!(r1.read_ref()?.equals(&outer(30))?);

    Ok(())
}

#[test]
fn global_value_non_struct() -> PartialVMResult<()> {
    assert!(GlobalValue::cached(Value::u64(100)).is_err());
    assert!(GlobalValue::cached(Value::bool(false)).is_err());

    let mut locals = Locals::new(1);
    locals.store_loc(0, Value::u8(0))?;
    let r = locals.borrow_loc(0)?;
    assert!(GlobalValue::cached(r).is_err());

    Ok(())
}

#[test]
fn test_vm_value_vector_u64_casting() {
    assert_eq!(
        vec![1, 2, 3],
        Value::vector_u64([1, 2, 3]).value_as::<Vec<u64>>().unwrap()
    );
}

#[test]
fn test_mem_swap() -> PartialVMResult<()> {
    let mut locals = Locals::new(20);
    // IndexedRef(Locals)
    locals.store_loc(0, Value::u64(0))?;
    locals.store_loc(1, Value::u64(1))?;
    locals.store_loc(2, Value::address(AccountAddress::ZERO))?;
    locals.store_loc(3, Value::address(AccountAddress::ONE))?;

    // ContainerRef

    // - Specialized
    locals.store_loc(4, Value::vector_u64(vec![1, 2]))?;
    locals.store_loc(5, Value::vector_u64(vec![3, 4, 5]))?;
    locals.store_loc(6, Value::vector_address(vec![AccountAddress::ZERO]))?;
    locals.store_loc(7, Value::vector_address(vec![AccountAddress::ONE]))?;

    // - Generic
    // -- Container of container
    locals.store_loc(8, Value::struct_(Struct::pack(vec![Value::u16(4)])))?;
    locals.store_loc(9, Value::struct_(Struct::pack(vec![Value::u16(5)])))?;
    locals.store_loc(10, Value::master_signer(AccountAddress::ZERO))?;
    locals.store_loc(11, Value::master_signer(AccountAddress::ONE))?;

    // -- Container of vector
    locals.store_loc(
        12,
        Value::vector_unchecked(vec![Value::DelayedFieldID {
            id: DelayedFieldID::from(1),
        }])?,
    )?;
    locals.store_loc(
        13,
        Value::vector_unchecked(vec![Value::DelayedFieldID {
            id: DelayedFieldID::from(2),
        }])?,
    )?;
    locals.store_loc(
        14,
        Value::vector_unchecked(vec![Value::master_signer(AccountAddress::ZERO)]).unwrap(),
    )?;
    locals.store_loc(
        15,
        Value::vector_unchecked(vec![Value::master_signer(AccountAddress::ONE)]).unwrap(),
    )?;

    let mut locals2 = Locals::new(2);
    locals2.store_loc(0, Value::u64(0))?;

    let get_local =
        |ls: &Locals, idx: usize| ls.borrow_loc(idx).unwrap().value_as::<Reference>().unwrap();

    for i in (0..16).step_by(2) {
        assert_ok!(get_local(&locals, i).swap_values(get_local(&locals, i + 1)));
    }

    assert_ok!(get_local(&locals, 0).swap_values(get_local(&locals2, 0)));

    for i in (0..16).step_by(2) {
        for j in ((i + 2)..16).step_by(2) {
            let result = get_local(&locals, i).swap_values(get_local(&locals, j));

            // These would all fail in `call_native` typing checks.
            // But here some do pass:
            if j < 4  // locals are not checked between each other
               || (8 <= i && j < 12) // ContainerRef of containers is not checked between each other
               || (12 <= i && j < 16)
            // ContainerRef of vector is not checked between each other
            //    || i >= 8 // containers are also interchangeable
            {
                assert_ok!(result, "{} and {}", i, j);
            } else {
                assert_err!(result, "{} and {}", i, j);
            }
        }
    }

    Ok(())
}

#[test]
fn test_vector_unchecked() {
    assert_err!(Value::vector_unchecked(vec![Value::bool(true)]));
    assert_err!(Value::vector_unchecked(vec![Value::u8(1)]));
    assert_err!(Value::vector_unchecked(vec![Value::u16(1)]));
    assert_err!(Value::vector_unchecked(vec![Value::u32(1)]));
    assert_err!(Value::vector_unchecked(vec![Value::u64(1)]));
    assert_err!(Value::vector_unchecked(vec![Value::u128(1)]));
    assert_err!(Value::vector_unchecked(vec![Value::u256(U256::ONE)]));
    assert_err!(Value::vector_unchecked(vec![Value::i8(1)]));
    assert_err!(Value::vector_unchecked(vec![Value::i16(1)]));
    assert_err!(Value::vector_unchecked(vec![Value::i32(1)]));
    assert_err!(Value::vector_unchecked(vec![Value::i64(1)]));
    assert_err!(Value::vector_unchecked(vec![Value::i128(1)]));
    assert_err!(Value::vector_unchecked(vec![Value::i256(I256::ONE)]));
    assert_err!(Value::vector_unchecked(vec![Value::address(
        AccountAddress::ONE
    )]));

    assert_ok!(Value::vector_unchecked(vec![Value::delayed_value(
        DelayedFieldID::from(0)
    )]));
    assert_ok!(Value::vector_unchecked(vec![Value::vector_u8(vec![1, 2])]));
    assert_ok!(Value::vector_unchecked(vec![Value::vector_i8(vec![1, 2])]));
    assert_ok!(Value::vector_unchecked(vec![Value::struct_(Struct::pack(
        vec![Value::u128(1), Value::u8(0)]
    ))]));
}

#[cfg(test)]
mod indexed_ref_tests {
    use crate::{
        delayed_values::delayed_field_id::DelayedFieldID,
        values::{AbstractFunction, Locals, Struct, StructRef, Value, VectorRef},
    };
    use better_any::{Tid, TidAble};
    use claims::{assert_matches, assert_ok};
    use move_binary_format::errors::PartialVMResult;
    use move_core_types::{
        account_address::AccountAddress,
        function::ClosureMask,
        int256::{I256, U256},
    };
    use std::cmp::Ordering;

    #[derive(Clone, Tid)]
    struct MockAbstractFunction;

    impl AbstractFunction for MockAbstractFunction {
        fn closure_mask(&self) -> ClosureMask {
            unreachable!()
        }

        fn cmp_dyn(&self, _other: &dyn AbstractFunction) -> PartialVMResult<Ordering> {
            unreachable!()
        }

        fn clone_dyn(&self) -> PartialVMResult<Box<dyn AbstractFunction>> {
            unreachable!()
        }

        fn to_canonical_string(&self) -> String {
            unreachable!()
        }
    }

    fn test_locals_or_struct_fields() -> Vec<(bool, Value)> {
        vec![
            // Primitives.
            (true, Value::bool(true)),
            (true, Value::u8(1)),
            (true, Value::u16(1)),
            (true, Value::u32(1)),
            (true, Value::u64(1)),
            (true, Value::u128(1)),
            (true, Value::u256(U256::ONE)),
            (true, Value::i8(1)),
            (true, Value::i16(1)),
            (true, Value::i32(1)),
            (true, Value::i64(1)),
            (true, Value::i128(1)),
            (true, Value::i256(I256::ONE)),
            (true, Value::address(AccountAddress::ONE)),
            (true, Value::delayed_value(DelayedFieldID::from(0))),
            (true, Value::closure(Box::new(MockAbstractFunction), vec![])),
            // Non-primitives.
            (false, Value::vector_u8(vec![1, 2, 3])),
            (false, Value::struct_(Struct::pack(vec![Value::bool(true)]))),
        ]
    }

    fn test_vectors() -> Vec<(bool, Value)> {
        vec![
            // Primitives.
            (true, Value::vector_bool(vec![false])),
            (true, Value::vector_u8(vec![1])),
            (true, Value::vector_u16(vec![1])),
            (true, Value::vector_u32(vec![1])),
            (true, Value::vector_u64(vec![1])),
            (true, Value::vector_u128(vec![1])),
            (true, Value::vector_u256(vec![U256::ONE])),
            (true, Value::vector_i8(vec![1])),
            (true, Value::vector_i16(vec![1])),
            (true, Value::vector_i32(vec![1])),
            (true, Value::vector_i64(vec![1])),
            (true, Value::vector_i128(vec![1])),
            (true, Value::vector_i256(vec![I256::ONE])),
            (true, Value::vector_address(vec![AccountAddress::ONE])),
            (
                true,
                Value::vector_unchecked(vec![Value::closure(
                    Box::new(MockAbstractFunction),
                    vec![],
                )])
                .unwrap(),
            ),
            // Non-primitives.
            (
                false,
                Value::vector_unchecked(vec![Value::vector_u8(vec![1, 2, 3])]).unwrap(),
            ),
            (
                false,
                Value::vector_unchecked(vec![Value::vector_i8(vec![1, 2, 3])]).unwrap(),
            ),
            (
                false,
                Value::vector_unchecked(vec![Value::struct_(Struct::pack(vec![Value::bool(
                    true,
                )]))])
                .unwrap(),
            ),
        ]
    }

    #[test]
    fn test_locals_indexed_ref() {
        let values = test_locals_or_struct_fields();

        let mut locals = Locals::new(values.len());
        for (idx, (is_indexed_ref, value)) in values.into_iter().enumerate() {
            assert_ok!(locals.store_loc(idx, value));
            let reference = assert_ok!(locals.borrow_loc(idx));
            if is_indexed_ref {
                assert_matches!(reference, Value::IndexedRef(_));
            } else {
                assert_matches!(reference, Value::ContainerRef(_));
            }
        }
    }

    #[test]
    fn test_struct_indexed_ref() {
        let values = test_locals_or_struct_fields();

        let mut locals = Locals::new(values.len());
        for (idx, (is_indexed_ref, value)) in values.into_iter().enumerate() {
            assert_ok!(locals.store_loc(idx, Value::struct_(Struct::pack(vec![value]))));

            let reference = assert_ok!(locals.borrow_loc(idx));
            let struct_ref = assert_ok!(reference.value_as::<StructRef>());
            let field = assert_ok!(struct_ref.borrow_field(0));

            if is_indexed_ref {
                assert_matches!(field, Value::IndexedRef(_));
            } else {
                assert_matches!(field, Value::ContainerRef(_));
            }
        }
    }

    #[test]
    fn test_vector_indexed_ref() {
        let values = test_vectors();

        let mut locals = Locals::new(values.len());
        for (idx, (is_indexed_ref, value)) in values.into_iter().enumerate() {
            assert_ok!(locals.store_loc(idx, value));

            let reference = assert_ok!(locals.borrow_loc(idx));
            let vector_ref = assert_ok!(reference.value_as::<VectorRef>());
            let elem = assert_ok!(vector_ref.borrow_elem(0));

            if is_indexed_ref {
                assert_matches!(elem, Value::IndexedRef(_));
            } else {
                assert_matches!(elem, Value::ContainerRef(_));
            }
        }
    }
}

#[cfg(test)]
mod delayed_fields {
    use super::*;
    use crate::delayed_values::delayed_field_id::{
        DelayedFieldID, ExtractUniqueIndex, ExtractWidth,
    };
    use claims::{assert_err, assert_ok};

    #[test]
    fn test_native_value_equality() {
        let v = Value::delayed_value(DelayedFieldID::new_with_width(0, 8));

        // Comparing delayed values to all other values results in error.

        assert_err!(Value::bool(false).equals(&v));

        assert_err!(Value::u8(0).equals(&v));
        assert_err!(Value::u16(0).equals(&v));
        assert_err!(Value::u32(0).equals(&v));
        assert_err!(Value::u64(0).equals(&v));
        assert_err!(Value::u128(0).equals(&v));
        assert_err!(Value::i8(0).equals(&v));
        assert_err!(Value::i16(0).equals(&v));
        assert_err!(Value::i32(0).equals(&v));
        assert_err!(Value::i64(0).equals(&v));
        assert_err!(Value::i128(0).equals(&v));
        assert_err!(Value::i256(I256::ZERO).equals(&v));

        assert_err!(Value::address(AccountAddress::ONE).equals(&v));
        assert_err!(Value::master_signer(AccountAddress::ONE).equals(&v));
        assert_err!(Value::master_signer_reference(AccountAddress::ONE).equals(&v));

        assert_err!(Value::vector_bool(vec![true, false]).equals(&v));

        assert_err!(Value::vector_u8(vec![0, 1]).equals(&v));
        assert_err!(Value::vector_u16(vec![0, 1]).equals(&v));
        assert_err!(Value::vector_u32(vec![0, 1]).equals(&v));
        assert_err!(Value::vector_u64(vec![0, 1]).equals(&v));
        assert_err!(Value::vector_u128(vec![0, 1]).equals(&v));
        assert_err!(Value::vector_u256(vec![U256::ZERO, U256::ONE]).equals(&v));
        assert_err!(Value::vector_i8(vec![0, 1]).equals(&v));
        assert_err!(Value::vector_i16(vec![0, 1]).equals(&v));
        assert_err!(Value::vector_i32(vec![0, 1]).equals(&v));
        assert_err!(Value::vector_i64(vec![0, 1]).equals(&v));
        assert_err!(Value::vector_i128(vec![0, 1]).equals(&v));
        assert_err!(Value::vector_i256(vec![I256::ZERO, I256::ONE]).equals(&v));

        assert_err!(
            Value::vector_address(vec![AccountAddress::ONE, AccountAddress::TWO]).equals(&v)
        );

        let s = Struct::pack(vec![Value::u32(0), Value::u32(1)]);
        assert_err!(Value::struct_(s).equals(&v));

        // Comparing native values to other native values, even self, results
        // in error.
        assert_err!(Value::delayed_value(DelayedFieldID::new_with_width(0, 8)).equals(&v));
        assert_err!(v.equals(&v));
    }

    #[test]
    fn test_native_value_borrow() {
        let delayed_value = Value::delayed_value(DelayedFieldID::new_with_width(0, 8));
        let mut locals = Locals::new(1);
        assert_ok!(locals.store_loc(0, delayed_value));

        let local = assert_ok!(locals.borrow_loc(0));
        let reference = assert_ok!(local.value_as::<Reference>());
        let v = assert_ok!(reference.read_ref());

        let expected_id = assert_ok!(v.value_as::<DelayedFieldID>());
        assert_eq!(expected_id.extract_unique_index(), 0);
        assert_eq!(expected_id.extract_width(), 8);
    }
}
