// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{loaded_data::runtime_types::Type, values::*, views::*};
use move_binary_format::errors::*;
use move_core_types::{account_address::AccountAddress, u256::U256};

#[test]
fn locals() -> PartialVMResult<()> {
    const LEN: usize = 4;
    let mut locals = Locals::new(LEN);
    for i in 0..LEN {
        assert!(locals.copy_loc(i).is_err());
        assert!(locals.move_loc(i, false).is_err());
        assert!(locals.borrow_loc(i).is_err());
    }
    locals.store_loc(1, Value::u64(42), false)?;

    assert!(locals.copy_loc(1)?.equals(&Value::u64(42))?);
    let r = locals.borrow_loc(1)?.value_as::<Reference>()?;
    assert!(r.read_ref()?.equals(&Value::u64(42))?);
    assert!(locals.move_loc(1, false)?.equals(&Value::u64(42))?);

    assert!(locals.copy_loc(1).is_err());
    assert!(locals.move_loc(1, false).is_err());
    assert!(locals.borrow_loc(1).is_err());

    assert!(locals.copy_loc(LEN + 1).is_err());
    assert!(locals.move_loc(LEN + 1, false).is_err());
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
        Value::u256(U256::max_value()),
    ];
    let s = Struct::pack(vec![
        Value::u8(10),
        Value::u16(12),
        Value::u32(15),
        Value::u64(20),
        Value::u128(30),
        Value::u256(U256::max_value()),
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
        false,
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

    locals.store_loc(0, outer(20), false)?;
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
    locals.store_loc(0, Value::u8(0), false)?;
    let r = locals.borrow_loc(0)?;
    assert!(GlobalValue::cached(r).is_err());

    Ok(())
}

#[test]
fn legacy_ref_abstract_memory_size_consistency() -> PartialVMResult<()> {
    let mut locals = Locals::new(10);

    locals.store_loc(0, Value::u128(0), false)?;
    let r = locals.borrow_loc(0)?;
    assert_eq!(r.legacy_abstract_memory_size(), r.legacy_size());

    locals.store_loc(1, Value::vector_u8([1, 2, 3]), false)?;
    let r = locals.borrow_loc(1)?;
    assert_eq!(r.legacy_abstract_memory_size(), r.legacy_size());

    let r: VectorRef = r.value_as()?;
    let r = r.borrow_elem(0, &Type::U8)?;
    assert_eq!(r.legacy_abstract_memory_size(), r.legacy_size());

    locals.store_loc(2, Value::struct_(Struct::pack([])), false)?;
    let r: Reference = locals.borrow_loc(2)?.value_as()?;
    assert_eq!(r.legacy_abstract_memory_size(), r.legacy_size());

    Ok(())
}

#[test]
fn legacy_struct_abstract_memory_size_consistency() -> PartialVMResult<()> {
    let structs = [
        Struct::pack([]),
        Struct::pack([Value::struct_(Struct::pack([Value::u8(0), Value::u64(0)]))]),
    ];

    for s in &structs {
        assert_eq!(s.legacy_abstract_memory_size(), s.legacy_size());
    }

    Ok(())
}

#[test]
fn legacy_val_abstract_memory_size_consistency() -> PartialVMResult<()> {
    let vals = [
        Value::u8(0),
        Value::u16(0),
        Value::u32(0),
        Value::u64(0),
        Value::u128(0),
        Value::u256(U256::zero()),
        Value::bool(true),
        Value::address(AccountAddress::ZERO),
        Value::vector_u8([0, 1, 2]),
        Value::vector_u16([0, 1, 2]),
        Value::vector_u32([0, 1, 2]),
        Value::vector_u64([]),
        Value::vector_u128([1, 2, 3, 4]),
        Value::vector_u256([1, 2, 3, 4].iter().map(|q| U256::from(*q as u64))),
        Value::struct_(Struct::pack([])),
        Value::struct_(Struct::pack([Value::u8(0), Value::bool(false)])),
        Value::vector_for_testing_only([]),
        Value::vector_for_testing_only([Value::u8(0), Value::u8(1)]),
    ];

    let mut locals = Locals::new(vals.len());
    for (idx, val) in vals.into_iter().enumerate() {
        let val_size_new = val.legacy_abstract_memory_size();
        let val_size_old = val.legacy_size();
        assert_eq!(val_size_new, val_size_old);

        locals.store_loc(idx, val, false)?;

        let val_size_through_ref = locals
            .borrow_loc(idx)?
            .value_as::<Reference>()?
            .value_view()
            .legacy_abstract_memory_size();

        assert_eq!(val_size_through_ref, val_size_old)
    }

    Ok(())
}

#[test]
fn test_vm_value_vector_u64_casting() {
    assert_eq!(
        vec![1, 2, 3],
        Value::vector_u64([1, 2, 3]).value_as::<Vec<u64>>().unwrap()
    );
}

#[cfg(test)]
mod native_values {
    use super::*;
    use crate::delayed_values::sized_id::SizedID;
    use claims::{assert_err, assert_ok};

    #[test]
    fn test_native_value_equality() {
        let v = Value::native_value(SizedID::new(0, 8));

        // Comparing native to all other values results in error.

        assert_err!(Value::bool(false).equals(&v));

        assert_err!(Value::u8(0).equals(&v));
        assert_err!(Value::u16(0).equals(&v));
        assert_err!(Value::u32(0).equals(&v));
        assert_err!(Value::u64(0).equals(&v));
        assert_err!(Value::u128(0).equals(&v));
        assert_err!(Value::u256(U256::zero()).equals(&v));

        assert_err!(Value::address(AccountAddress::ONE).equals(&v));
        assert_err!(Value::signer(AccountAddress::ONE).equals(&v));
        assert_err!(Value::signer_reference(AccountAddress::ONE).equals(&v));

        assert_err!(Value::vector_bool(vec![true, false]).equals(&v));

        assert_err!(Value::vector_u8(vec![0, 1]).equals(&v));
        assert_err!(Value::vector_u16(vec![0, 1]).equals(&v));
        assert_err!(Value::vector_u32(vec![0, 1]).equals(&v));
        assert_err!(Value::vector_u64(vec![0, 1]).equals(&v));
        assert_err!(Value::vector_u128(vec![0, 1]).equals(&v));
        assert_err!(Value::vector_u256(vec![U256::zero(), U256::one()]).equals(&v));

        assert_err!(
            Value::vector_address(vec![AccountAddress::ONE, AccountAddress::TWO]).equals(&v)
        );

        let s = Struct::pack(vec![Value::u32(0), Value::u32(1)]);
        assert_err!(Value::struct_(s).equals(&v));

        // Comparing native values to other native values, even self, results
        // in error.
        assert_err!(Value::native_value(SizedID::new(0, 1)).equals(&v));
        assert_err!(v.equals(&v));
    }

    #[test]
    fn test_native_value_borrow() {
        let delayed_value = Value::native_value(SizedID::new(0, 8));
        let mut locals = Locals::new(1);
        assert_ok!(locals.store_loc(0, delayed_value, false));

        let local = assert_ok!(locals.borrow_loc(0));
        let reference = assert_ok!(local.value_as::<Reference>());
        let v = assert_ok!(reference.read_ref());

        let expected_id = assert_ok!(v.value_as::<SizedID>());
        assert_eq!(expected_id.id(), 0);
        assert_eq!(expected_id.serialized_size(), 8);
    }
}
