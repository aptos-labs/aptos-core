// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::natives::util::make_native_from_func;

use move_binary_format::errors::PartialVMResult;
use move_core_types::gas_algebra::{InternalGas, InternalGasPerByte, NumBytes};
use move_vm_runtime::native_functions::{NativeContext, NativeFunction};
use move_vm_types::{
    loaded_data::runtime_types::Type, natives::function::NativeResult, pop_arg, values::Value,
};
use ripemd::Digest as OtherDigest;
use sha2::Digest;
use smallvec::smallvec;
use std::{collections::VecDeque, hash::Hasher};
use tiny_keccak::{Hasher as KeccakHasher, Keccak};

/***************************************************************************************************
 * native fun sip_hash
 *
 *   gas cost: base_cost + unit_cost * data_length
 *
 **************************************************************************************************/
#[derive(Debug, Clone)]
pub struct SipHashGasParameters {
    pub base: InternalGas,
    pub per_byte: InternalGasPerByte,
}

/// Feed these bytes into SipHasher. This is not cryptographically secure.
fn native_sip_hash(
    gas_params: &SipHashGasParameters,
    _context: &mut NativeContext,
    mut _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    debug_assert!(_ty_args.is_empty());
    debug_assert!(args.len() == 1);

    let bytes = pop_arg!(args, Vec<u8>);

    let cost = gas_params.base + gas_params.per_byte * NumBytes::new(bytes.len() as u64);

    // SipHash of the serialized bytes
    let mut hasher = siphasher::sip::SipHasher::new();
    hasher.write(&bytes);
    let hash = hasher.finish();

    Ok(NativeResult::ok(cost, smallvec![Value::u64(hash)]))
}

#[derive(Debug, Clone)]
pub struct Keccak256HashGasParameters {
    pub base: InternalGas,
    pub per_byte: InternalGasPerByte,
}

fn native_keccak256(
    gas_params: &Keccak256HashGasParameters,
    _context: &mut NativeContext,
    mut _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    debug_assert!(_ty_args.is_empty());
    debug_assert!(args.len() == 1);

    let bytes = pop_arg!(args, Vec<u8>);

    let cost = gas_params.base + gas_params.per_byte * NumBytes::new(bytes.len() as u64);

    let mut hasher = Keccak::v256();
    hasher.update(&bytes);
    let mut output = [0u8; 32];
    hasher.finalize(&mut output);

    Ok(NativeResult::ok(cost, smallvec![Value::vector_u8(output)]))
}

#[derive(Debug, Clone)]
pub struct Sha2_512HashGasParameters {
    pub base: InternalGas,
    pub per_byte: InternalGasPerByte,
}

fn native_sha2_512(
    gas_params: &Sha2_512HashGasParameters,
    _context: &mut NativeContext,
    mut _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    debug_assert!(_ty_args.is_empty());
    debug_assert!(args.len() == 1);

    let bytes = pop_arg!(args, Vec<u8>);

    let cost = gas_params.base + gas_params.per_byte * NumBytes::new(bytes.len() as u64);

    let mut hasher = sha2::Sha512::new();
    hasher.update(&bytes);
    let output = hasher.finalize().to_vec();

    Ok(NativeResult::ok(cost, smallvec![Value::vector_u8(output)]))
}

#[derive(Debug, Clone)]
pub struct Sha3_512HashGasParameters {
    pub base: InternalGas,
    pub per_byte: InternalGasPerByte,
}

fn native_sha3_512(
    gas_params: &Sha3_512HashGasParameters,
    _context: &mut NativeContext,
    mut _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    debug_assert!(_ty_args.is_empty());
    debug_assert!(args.len() == 1);

    let bytes = pop_arg!(args, Vec<u8>);

    let cost = gas_params.base + gas_params.per_byte * NumBytes::new(bytes.len() as u64);

    let mut hasher = sha3::Sha3_512::new();
    hasher.update(&bytes);
    let output = hasher.finalize().to_vec();

    Ok(NativeResult::ok(cost, smallvec![Value::vector_u8(output)]))
}

#[derive(Debug, Clone)]
pub struct Ripemd160HashGasParameters {
    pub base: InternalGas,
    pub per_byte: InternalGasPerByte,
}

fn native_ripemd160(
    gas_params: &Ripemd160HashGasParameters,
    _context: &mut NativeContext,
    mut _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    debug_assert!(_ty_args.is_empty());
    debug_assert!(args.len() == 1);

    let bytes = pop_arg!(args, Vec<u8>);

    let cost = gas_params.base + gas_params.per_byte * NumBytes::new(bytes.len() as u64);

    let mut hasher = ripemd::Ripemd160::new();
    hasher.update(&bytes);
    let output = hasher.finalize().to_vec();

    Ok(NativeResult::ok(cost, smallvec![Value::vector_u8(output)]))
}

/***************************************************************************************************
 * module
 *
 **************************************************************************************************/
#[derive(Debug, Clone)]
pub struct GasParameters {
    pub sip_hash: SipHashGasParameters,
    pub keccak256: Keccak256HashGasParameters,
    pub sha2_512: Sha2_512HashGasParameters,
    pub sha3_512: Sha3_512HashGasParameters,
    pub ripemd160: Ripemd160HashGasParameters,
}

pub fn make_all(gas_params: GasParameters) -> impl Iterator<Item = (String, NativeFunction)> {
    let natives = [
        (
            "sip_hash",
            make_native_from_func(gas_params.sip_hash, native_sip_hash),
        ),
        (
            "keccak256",
            make_native_from_func(gas_params.keccak256, native_keccak256),
        ),
        (
            "sha2_512_internal",
            make_native_from_func(gas_params.sha2_512, native_sha2_512),
        ),
        (
            "sha3_512_internal",
            make_native_from_func(gas_params.sha3_512, native_sha3_512),
        ),
        (
            "ripemd160_internal",
            make_native_from_func(gas_params.ripemd160, native_ripemd160),
        ),
    ];

    crate::natives::helpers::make_module_natives(natives)
}
