// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::natives::cryptography::ristretto255::{
    pop_32_byte_slice, pop_64_byte_slice, pop_scalar_from_bytes, GasParameters,
};
use curve25519_dalek::scalar::Scalar;
use move_deps::move_core_types::gas_algebra::{NumArgs, NumBytes};
use move_deps::{
    move_binary_format::errors::PartialVMResult,
    move_vm_runtime::native_functions::NativeContext,
    move_vm_types::{
        loaded_data::runtime_types::Type, natives::function::NativeResult, pop_arg, values::Value,
    },
};
use sha2::Sha512;
use smallvec::smallvec;
use std::ops::{Add, Mul, Neg, Sub};
use std::{collections::VecDeque, convert::TryFrom};

pub(crate) fn native_scalar_is_canonical(
    gas_params: &GasParameters,
    _context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    debug_assert!(_ty_args.is_empty());
    debug_assert!(arguments.len() == 1);

    let cost = gas_params.scalar_is_canonical * NumArgs::one();

    let bytes = pop_arg!(arguments, Vec<u8>);
    if bytes.len() != 32 {
        return Ok(NativeResult::ok(cost, smallvec![Value::bool(false)]));
    }

    let bytes_slice = <[u8; 32]>::try_from(bytes).unwrap();

    let s = Scalar::from_canonical_bytes(bytes_slice);

    // TODO: Speed up this implementation using bit testing on 'bytes'?
    Ok(NativeResult::ok(cost, smallvec![Value::bool(s.is_some())]))
}

pub(crate) fn native_scalar_invert(
    gas_params: &GasParameters,
    _context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    debug_assert!(_ty_args.is_empty());
    debug_assert!(arguments.len() == 1);

    let cost = gas_params.scalar_invert * NumArgs::one();

    let s = pop_scalar_from_bytes(&mut arguments)?;

    // Invert and return
    Ok(NativeResult::ok(
        cost,
        smallvec![Value::vector_u8(s.invert().to_bytes().to_vec())],
    ))
}

pub(crate) fn native_scalar_from_sha512(
    gas_params: &GasParameters,
    _context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    debug_assert!(_ty_args.is_empty());
    debug_assert!(arguments.len() == 1);

    let bytes = pop_arg!(arguments, Vec<u8>);
    let cost = gas_params.scalar_uniform_from_64_bytes * NumArgs::one()
        + gas_params.sha512_per_hash * NumArgs::one()
        + gas_params.sha512_per_byte * NumBytes::new(bytes.len() as u64);
    let s = Scalar::hash_from_bytes::<Sha512>(bytes.as_slice());

    Ok(NativeResult::ok(
        cost,
        smallvec![Value::vector_u8(s.to_bytes().to_vec())],
    ))
}

pub(crate) fn native_scalar_mul(
    gas_params: &GasParameters,
    _context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    debug_assert!(_ty_args.is_empty());
    debug_assert!(arguments.len() == 2);

    let cost = gas_params.scalar_mul * NumArgs::one();

    let b = pop_scalar_from_bytes(&mut arguments)?;
    let a = pop_scalar_from_bytes(&mut arguments)?;

    let s = a.mul(b);

    Ok(NativeResult::ok(
        cost,
        smallvec![Value::vector_u8(s.to_bytes().to_vec())],
    ))
}

pub(crate) fn native_scalar_add(
    gas_params: &GasParameters,
    _context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    debug_assert!(_ty_args.is_empty());
    debug_assert!(arguments.len() == 2);

    let cost = gas_params.scalar_add * NumArgs::one();

    let b = pop_scalar_from_bytes(&mut arguments)?;
    let a = pop_scalar_from_bytes(&mut arguments)?;

    let s = a.add(b);

    Ok(NativeResult::ok(
        cost,
        smallvec![Value::vector_u8(s.to_bytes().to_vec())],
    ))
}

pub(crate) fn native_scalar_sub(
    gas_params: &GasParameters,
    _context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    debug_assert!(_ty_args.is_empty());
    debug_assert!(arguments.len() == 2);

    let cost = gas_params.scalar_sub * NumArgs::one();

    let b = pop_scalar_from_bytes(&mut arguments)?;
    let a = pop_scalar_from_bytes(&mut arguments)?;

    let s = a.sub(b);

    Ok(NativeResult::ok(
        cost,
        smallvec![Value::vector_u8(s.to_bytes().to_vec())],
    ))
}

pub(crate) fn native_scalar_neg(
    gas_params: &GasParameters,
    _context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    debug_assert!(_ty_args.is_empty());
    debug_assert!(arguments.len() == 1);

    let a = pop_scalar_from_bytes(&mut arguments)?;

    let cost = gas_params.scalar_neg * NumArgs::one();
    let s = a.neg();

    Ok(NativeResult::ok(
        cost,
        smallvec![Value::vector_u8(s.to_bytes().to_vec())],
    ))
}

pub(crate) fn native_scalar_from_u64(
    gas_params: &GasParameters,
    _context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    debug_assert!(_ty_args.is_empty());
    debug_assert!(arguments.len() == 1);

    let num = pop_arg!(arguments, u64);
    let cost = gas_params.scalar_from_u64 * NumArgs::one();
    let s = Scalar::from(num);

    Ok(NativeResult::ok(
        cost,
        smallvec![Value::vector_u8(s.to_bytes().to_vec())],
    ))
}

pub(crate) fn native_scalar_from_u128(
    gas_params: &GasParameters,
    _context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    debug_assert!(_ty_args.is_empty());
    debug_assert!(arguments.len() == 1);

    let num = pop_arg!(arguments, u128);
    let cost = gas_params.scalar_from_u128 * NumArgs::one();
    let s = Scalar::from(num);

    Ok(NativeResult::ok(
        cost,
        smallvec![Value::vector_u8(s.to_bytes().to_vec())],
    ))
}

pub(crate) fn native_scalar_reduced_from_32_bytes(
    gas_params: &GasParameters,
    _context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    debug_assert!(_ty_args.is_empty());
    debug_assert!(arguments.len() == 1);

    let bytes_slice = pop_32_byte_slice(&mut arguments)?;
    let cost = gas_params.scalar_reduced_from_32_bytes * NumArgs::one();
    let s = Scalar::from_bytes_mod_order(bytes_slice);

    Ok(NativeResult::ok(
        cost,
        smallvec![Value::vector_u8(s.to_bytes().to_vec())],
    ))
}

pub(crate) fn native_scalar_uniform_from_64_bytes(
    gas_params: &GasParameters,
    _context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    debug_assert!(_ty_args.is_empty());
    debug_assert!(args.len() == 1);

    let bytes_slice = pop_64_byte_slice(&mut args)?;
    let cost = gas_params.scalar_uniform_from_64_bytes * NumArgs::one();
    let s = Scalar::from_bytes_mod_order_wide(&bytes_slice);

    Ok(NativeResult::ok(
        cost,
        smallvec![Value::vector_u8(s.to_bytes().to_vec())],
    ))
}
