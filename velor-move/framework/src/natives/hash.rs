// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use velor_gas_schedule::gas_params::natives::velor_framework::*;
use velor_native_interface::{
    safely_assert_eq, safely_pop_arg, RawSafeNative, SafeNativeBuilder, SafeNativeContext,
    SafeNativeResult,
};
use move_core_types::gas_algebra::{InternalGas, InternalGasPerByte, NumBytes};
use move_vm_runtime::native_functions::NativeFunction;
use move_vm_types::{loaded_data::runtime_types::Type, values::Value};
use ripemd::Digest as OtherDigest;
use sha2::Digest;
use smallvec::{smallvec, SmallVec};
use std::{collections::VecDeque, hash::Hasher};
use tiny_keccak::{Hasher as KeccakHasher, Keccak};

/***************************************************************************************************
 * native fun sip_hash
 *
 *   gas cost: base_cost + unit_cost * data_length
 *
 **************************************************************************************************/
fn native_sip_hash(
    context: &mut SafeNativeContext,
    mut _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert!(_ty_args.is_empty());
    debug_assert!(args.len() == 1);

    let bytes = safely_pop_arg!(args, Vec<u8>);

    let cost = HASH_SIP_HASH_BASE + HASH_SIP_HASH_PER_BYTE * NumBytes::new(bytes.len() as u64);
    context.charge(cost)?;

    // SipHash of the serialized bytes
    let mut hasher = siphasher::sip::SipHasher::new();
    hasher.write(&bytes);
    let hash = hasher.finish();

    Ok(smallvec![Value::u64(hash)])
}

fn native_keccak256(
    context: &mut SafeNativeContext,
    mut _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert!(_ty_args.is_empty());
    debug_assert!(args.len() == 1);

    let bytes = safely_pop_arg!(args, Vec<u8>);

    let cost = HASH_KECCAK256_BASE + HASH_KECCAK256_PER_BYTE * NumBytes::new(bytes.len() as u64);
    context.charge(cost)?;

    let mut hasher = Keccak::v256();
    hasher.update(&bytes);
    let mut output = [0u8; 32];
    hasher.finalize(&mut output);

    Ok(smallvec![Value::vector_u8(output)])
}

fn native_sha2_512(
    context: &mut SafeNativeContext,
    mut _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert!(_ty_args.is_empty());
    debug_assert!(args.len() == 1);

    let bytes = safely_pop_arg!(args, Vec<u8>);

    let cost = HASH_SHA2_512_BASE + HASH_SHA2_512_PER_BYTE * NumBytes::new(bytes.len() as u64);
    context.charge(cost)?;

    let mut hasher = sha2::Sha512::new();
    hasher.update(&bytes);
    let output = hasher.finalize().to_vec();

    Ok(smallvec![Value::vector_u8(output)])
}

fn native_sha3_512(
    context: &mut SafeNativeContext,
    mut _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert!(_ty_args.is_empty());
    debug_assert!(args.len() == 1);

    let bytes = safely_pop_arg!(args, Vec<u8>);

    let cost = HASH_SHA3_512_BASE + HASH_SHA3_512_PER_BYTE * NumBytes::new(bytes.len() as u64);
    context.charge(cost)?;

    let mut hasher = sha3::Sha3_512::new();
    hasher.update(&bytes);
    let output = hasher.finalize().to_vec();

    Ok(smallvec![Value::vector_u8(output)])
}

#[derive(Debug, Clone)]
pub struct Blake2B256HashGasParameters {
    pub base: InternalGas,
    pub per_byte: InternalGasPerByte,
}

fn native_blake2b_256(
    context: &mut SafeNativeContext,
    mut _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    safely_assert_eq!(_ty_args.len(), 0);
    safely_assert_eq!(args.len(), 1);

    let bytes = safely_pop_arg!(args, Vec<u8>);

    context.charge(
        HASH_BLAKE2B_256_BASE + HASH_BLAKE2B_256_PER_BYTE * NumBytes::new(bytes.len() as u64),
    )?;

    let output = blake2_rfc::blake2b::blake2b(32, &[], &bytes)
        .as_bytes()
        .to_vec();

    Ok(smallvec![Value::vector_u8(output)])
}

#[derive(Debug, Clone)]
pub struct Ripemd160HashGasParameters {
    pub base: InternalGas,
    pub per_byte: InternalGasPerByte,
}

fn native_ripemd160(
    context: &mut SafeNativeContext,
    mut _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert!(_ty_args.is_empty());
    debug_assert!(args.len() == 1);

    let bytes = safely_pop_arg!(args, Vec<u8>);

    let cost = HASH_RIPEMD160_BASE + HASH_RIPEMD160_PER_BYTE * NumBytes::new(bytes.len() as u64);
    context.charge(cost)?;

    let mut hasher = ripemd::Ripemd160::new();
    hasher.update(&bytes);
    let output = hasher.finalize().to_vec();

    Ok(smallvec![Value::vector_u8(output)])
}

/***************************************************************************************************
 * module
 *
 **************************************************************************************************/
pub fn make_all(
    builder: &SafeNativeBuilder,
) -> impl Iterator<Item = (String, NativeFunction)> + '_ {
    let natives = [
        ("sip_hash", native_sip_hash as RawSafeNative),
        ("keccak256", native_keccak256),
        ("sha2_512_internal", native_sha2_512),
        ("sha3_512_internal", native_sha3_512),
        ("ripemd160_internal", native_ripemd160),
        ("blake2b_256_internal", native_blake2b_256),
    ];

    builder.make_named_natives(natives)
}
