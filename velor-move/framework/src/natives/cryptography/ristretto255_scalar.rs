// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::natives::cryptography::ristretto255::{
    pop_32_byte_slice, pop_64_byte_slice, pop_scalar_from_bytes, SCALAR_NUM_BYTES,
};
use velor_gas_schedule::gas_params::natives::velor_framework::*;
use velor_native_interface::{
    safely_assert_eq, safely_pop_arg, SafeNativeContext, SafeNativeResult,
};
use curve25519_dalek::scalar::Scalar;
use move_core_types::gas_algebra::{NumArgs, NumBytes};
use move_vm_types::{loaded_data::runtime_types::Type, values::Value};
#[cfg(feature = "testing")]
use rand::thread_rng;
#[cfg(feature = "testing")]
use rand_core::RngCore;
use sha2::Sha512;
use smallvec::{smallvec, SmallVec};
use std::{
    collections::VecDeque,
    convert::TryFrom,
    ops::{Add, Mul, Neg, Sub},
};

#[cfg(feature = "testing")]
/// This is a test-only native that charges zero gas. It is only exported in testing mode.
pub(crate) fn native_scalar_random(
    _context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert!(_ty_args.is_empty());
    debug_assert!(args.is_empty());

    let mut rng = thread_rng();

    // We do this manually due to curve25519-dalek-ng's `Scalar::random` being incompatible with our
    // `rand-0.7.3` dependency
    let mut scalar_bytes = [0u8; 64];
    rng.fill_bytes(&mut scalar_bytes);

    let scalar = Scalar::from_bytes_mod_order_wide(&scalar_bytes);

    Ok(smallvec![Value::vector_u8(scalar.to_bytes())])
}

pub(crate) fn native_scalar_is_canonical(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    safely_assert_eq!(_ty_args.len(), 0);
    safely_assert_eq!(arguments.len(), 1);

    context.charge(RISTRETTO255_SCALAR_IS_CANONICAL * NumArgs::one())?;

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
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    safely_assert_eq!(_ty_args.len(), 0);
    safely_assert_eq!(arguments.len(), 1);

    context.charge(RISTRETTO255_SCALAR_INVERT * NumArgs::one())?;

    let s = pop_scalar_from_bytes(&mut arguments)?;

    // Invert and return
    Ok(smallvec![Value::vector_u8(s.invert().to_bytes().to_vec())])
}

// NOTE: This was supposed to be more clearly named with *_sha2_512_*.
pub(crate) fn native_scalar_from_sha512(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    safely_assert_eq!(_ty_args.len(), 0);
    safely_assert_eq!(arguments.len(), 1);

    let bytes = safely_pop_arg!(arguments, Vec<u8>);

    context.charge(
        RISTRETTO255_SCALAR_UNIFORM_FROM_64_BYTES * NumArgs::one()
            + RISTRETTO255_SHA512_PER_HASH * NumArgs::one()
            + RISTRETTO255_SHA512_PER_BYTE * NumBytes::new(bytes.len() as u64),
    )?;

    let s = Scalar::hash_from_bytes::<Sha512>(bytes.as_slice());

    Ok(smallvec![Value::vector_u8(s.to_bytes().to_vec())])
}

pub(crate) fn native_scalar_mul(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    safely_assert_eq!(_ty_args.len(), 0);
    safely_assert_eq!(arguments.len(), 2);

    context.charge(RISTRETTO255_SCALAR_MUL * NumArgs::one())?;

    let b = pop_scalar_from_bytes(&mut arguments)?;
    let a = pop_scalar_from_bytes(&mut arguments)?;

    let s = a.mul(b);

    Ok(smallvec![Value::vector_u8(s.to_bytes().to_vec())])
}

pub(crate) fn native_scalar_add(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    safely_assert_eq!(_ty_args.len(), 0);
    safely_assert_eq!(arguments.len(), 2);

    context.charge(RISTRETTO255_SCALAR_ADD * NumArgs::one())?;

    let b = pop_scalar_from_bytes(&mut arguments)?;
    let a = pop_scalar_from_bytes(&mut arguments)?;

    let s = a.add(b);

    Ok(smallvec![Value::vector_u8(s.to_bytes().to_vec())])
}

pub(crate) fn native_scalar_sub(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    safely_assert_eq!(_ty_args.len(), 0);
    safely_assert_eq!(arguments.len(), 2);

    context.charge(RISTRETTO255_SCALAR_SUB * NumArgs::one())?;

    let b = pop_scalar_from_bytes(&mut arguments)?;
    let a = pop_scalar_from_bytes(&mut arguments)?;

    let s = a.sub(b);

    Ok(smallvec![Value::vector_u8(s.to_bytes().to_vec())])
}

pub(crate) fn native_scalar_neg(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    safely_assert_eq!(_ty_args.len(), 0);
    safely_assert_eq!(arguments.len(), 1);

    let a = pop_scalar_from_bytes(&mut arguments)?;

    context.charge(RISTRETTO255_SCALAR_NEG * NumArgs::one())?;

    let s = a.neg();

    Ok(smallvec![Value::vector_u8(s.to_bytes().to_vec())])
}

pub(crate) fn native_scalar_from_u64(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    safely_assert_eq!(_ty_args.len(), 0);
    safely_assert_eq!(arguments.len(), 1);

    let num = safely_pop_arg!(arguments, u64);

    context.charge(RISTRETTO255_SCALAR_FROM_U64 * NumArgs::one())?;

    let s = Scalar::from(num);

    Ok(smallvec![Value::vector_u8(s.to_bytes().to_vec())])
}

pub(crate) fn native_scalar_from_u128(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    safely_assert_eq!(_ty_args.len(), 0);
    safely_assert_eq!(arguments.len(), 1);

    let num = safely_pop_arg!(arguments, u128);

    context.charge(RISTRETTO255_SCALAR_FROM_U128 * NumArgs::one())?;

    let s = Scalar::from(num);

    Ok(smallvec![Value::vector_u8(s.to_bytes().to_vec())])
}

pub(crate) fn native_scalar_reduced_from_32_bytes(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    safely_assert_eq!(_ty_args.len(), 0);
    safely_assert_eq!(arguments.len(), 1);

    let bytes_slice = pop_32_byte_slice(&mut arguments)?;

    context.charge(RISTRETTO255_SCALAR_REDUCED_FROM_32_BYTES * NumArgs::one())?;

    let s = Scalar::from_bytes_mod_order(bytes_slice);

    Ok(smallvec![Value::vector_u8(s.to_bytes().to_vec())])
}

pub(crate) fn native_scalar_uniform_from_64_bytes(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    safely_assert_eq!(_ty_args.len(), 0);
    safely_assert_eq!(args.len(), 1);

    let bytes_slice = pop_64_byte_slice(&mut args)?;

    context.charge(RISTRETTO255_SCALAR_UNIFORM_FROM_64_BYTES * NumArgs::one())?;

    let s = Scalar::from_bytes_mod_order_wide(&bytes_slice);

    Ok(smallvec![Value::vector_u8(s.to_bytes().to_vec())])
}
