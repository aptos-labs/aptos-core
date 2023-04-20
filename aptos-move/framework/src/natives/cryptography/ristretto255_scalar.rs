// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    natives::{
        cryptography::ristretto255::{
            pop_32_byte_slice, pop_64_byte_slice, pop_scalar_from_bytes, GasParameters,
            SCALAR_NUM_BYTES,
        },
        helpers::{SafeNativeContext, SafeNativeResult},
    },
    safely_assert_eq, safely_pop_arg,
};
use curve25519_dalek::scalar::Scalar;
use move_core_types::gas_algebra::{NumArgs, NumBytes};
use move_vm_types::{loaded_data::runtime_types::Type, values::Value};
use sha2::Sha512;
use smallvec::{smallvec, SmallVec};
use std::{
    collections::VecDeque,
    convert::TryFrom,
    ops::{Add, Mul, Neg, Sub},
};

pub(crate) fn native_scalar_is_canonical(
    gas_params: &GasParameters,
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    safely_assert_eq!(_ty_args.len(), 0);
    safely_assert_eq!(arguments.len(), 1);

    context.charge(gas_params.scalar_is_canonical * NumArgs::one())?;

    let bytes = safely_pop_arg!(arguments, Vec<u8>);
    if bytes.len() != SCALAR_NUM_BYTES {
        return Ok(smallvec![Value::bool(false)]);
    }

    let bytes_slice = <[u8; SCALAR_NUM_BYTES]>::try_from(bytes).unwrap();

    let s = Scalar::from_canonical_bytes(bytes_slice);

    // TODO: Speed up this implementation using bit testing on 'bytes'?
    Ok(smallvec![Value::bool(s.is_some())])
}

pub(crate) fn native_scalar_invert(
    gas_params: &GasParameters,
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    safely_assert_eq!(_ty_args.len(), 0);
    safely_assert_eq!(arguments.len(), 1);

    context.charge(gas_params.scalar_invert * NumArgs::one())?;

    let s = pop_scalar_from_bytes(&mut arguments)?;

    // Invert and return
    Ok(smallvec![Value::vector_u8(s.invert().to_bytes().to_vec())])
}

pub(crate) fn native_scalar_from_sha512(
    gas_params: &GasParameters,
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    safely_assert_eq!(_ty_args.len(), 0);
    safely_assert_eq!(arguments.len(), 1);

    let bytes = safely_pop_arg!(arguments, Vec<u8>);

    context.charge(
        gas_params.scalar_uniform_from_64_bytes * NumArgs::one()
            + gas_params.sha512_per_hash * NumArgs::one()
            + gas_params.sha512_per_byte * NumBytes::new(bytes.len() as u64),
    )?;

    let s = Scalar::hash_from_bytes::<Sha512>(bytes.as_slice());

    Ok(smallvec![Value::vector_u8(s.to_bytes().to_vec())])
}

pub(crate) fn native_scalar_mul(
    gas_params: &GasParameters,
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    safely_assert_eq!(_ty_args.len(), 0);
    safely_assert_eq!(arguments.len(), 2);

    context.charge(gas_params.scalar_mul * NumArgs::one())?;

    let b = pop_scalar_from_bytes(&mut arguments)?;
    let a = pop_scalar_from_bytes(&mut arguments)?;

    let s = a.mul(b);

    Ok(smallvec![Value::vector_u8(s.to_bytes().to_vec())])
}

pub(crate) fn native_scalar_add(
    gas_params: &GasParameters,
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    safely_assert_eq!(_ty_args.len(), 0);
    safely_assert_eq!(arguments.len(), 2);

    context.charge(gas_params.scalar_add * NumArgs::one())?;

    let b = pop_scalar_from_bytes(&mut arguments)?;
    let a = pop_scalar_from_bytes(&mut arguments)?;

    let s = a.add(b);

    Ok(smallvec![Value::vector_u8(s.to_bytes().to_vec())])
}

pub(crate) fn native_scalar_sub(
    gas_params: &GasParameters,
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    safely_assert_eq!(_ty_args.len(), 0);
    safely_assert_eq!(arguments.len(), 2);

    context.charge(gas_params.scalar_sub * NumArgs::one())?;

    let b = pop_scalar_from_bytes(&mut arguments)?;
    let a = pop_scalar_from_bytes(&mut arguments)?;

    let s = a.sub(b);

    Ok(smallvec![Value::vector_u8(s.to_bytes().to_vec())])
}

pub(crate) fn native_scalar_neg(
    gas_params: &GasParameters,
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    safely_assert_eq!(_ty_args.len(), 0);
    safely_assert_eq!(arguments.len(), 1);

    let a = pop_scalar_from_bytes(&mut arguments)?;

    context.charge(gas_params.scalar_neg * NumArgs::one())?;

    let s = a.neg();

    Ok(smallvec![Value::vector_u8(s.to_bytes().to_vec())])
}

pub(crate) fn native_scalar_from_u64(
    gas_params: &GasParameters,
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    safely_assert_eq!(_ty_args.len(), 0);
    safely_assert_eq!(arguments.len(), 1);

    let num = safely_pop_arg!(arguments, u64);

    context.charge(gas_params.scalar_from_u64 * NumArgs::one())?;

    let s = Scalar::from(num);

    Ok(smallvec![Value::vector_u8(s.to_bytes().to_vec())])
}

pub(crate) fn native_scalar_from_u128(
    gas_params: &GasParameters,
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    safely_assert_eq!(_ty_args.len(), 0);
    safely_assert_eq!(arguments.len(), 1);

    let num = safely_pop_arg!(arguments, u128);

    context.charge(gas_params.scalar_from_u128 * NumArgs::one())?;

    let s = Scalar::from(num);

    Ok(smallvec![Value::vector_u8(s.to_bytes().to_vec())])
}

pub(crate) fn native_scalar_reduced_from_32_bytes(
    gas_params: &GasParameters,
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    safely_assert_eq!(_ty_args.len(), 0);
    safely_assert_eq!(arguments.len(), 1);

    let bytes_slice = pop_32_byte_slice(&mut arguments)?;

    context.charge(gas_params.scalar_reduced_from_32_bytes * NumArgs::one())?;

    let s = Scalar::from_bytes_mod_order(bytes_slice);

    Ok(smallvec![Value::vector_u8(s.to_bytes().to_vec())])
}

pub(crate) fn native_scalar_uniform_from_64_bytes(
    gas_params: &GasParameters,
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    safely_assert_eq!(_ty_args.len(), 0);
    safely_assert_eq!(args.len(), 1);

    let bytes_slice = pop_64_byte_slice(&mut args)?;

    context.charge(gas_params.scalar_uniform_from_64_bytes * NumArgs::one())?;

    let s = Scalar::from_bytes_mod_order_wide(&bytes_slice);

    Ok(smallvec![Value::vector_u8(s.to_bytes().to_vec())])
}
