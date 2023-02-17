// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::natives::helpers::make_safe_native;
use crate::natives::helpers::SafeNativeContext;
use crate::natives::helpers::SafeNativeResult;
use crate::safely_pop_arg;

use aptos_types::on_chain_config::TimedFeatures;
use move_core_types::gas_algebra::{InternalGas, InternalGasPerByte, NumBytes};
use move_vm_runtime::native_functions::NativeFunction;
use move_vm_types::{loaded_data::runtime_types::Type, values::Value};
use ripemd::Digest as OtherDigest;
use sha2::Digest;
use smallvec::smallvec;
use smallvec::SmallVec;
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

// See: https://github.com/aptos-labs/aptos-core/security/advisories/GHSA-x43p-vm4h-r828
fn native_sip_hash(
    gas_params: &SipHashGasParameters,
    context: &mut SafeNativeContext,
    mut _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert!(_ty_args.is_empty());
    debug_assert!(args.len() == 1);

    let bytes = safely_pop_arg!(args, Vec<u8>);

    let cost = gas_params.base + gas_params.per_byte * NumBytes::new(bytes.len() as u64);
    context.charge(cost)?;

    // SipHash of the serialized bytes
    let mut hasher = siphasher::sip::SipHasher::new();
    hasher.write(&bytes);
    let hash = hasher.finish();

    Ok(smallvec![Value::u64(hash)])
}

#[derive(Debug, Clone)]
pub struct Keccak256HashGasParameters {
    pub base: InternalGas,
    pub per_byte: InternalGasPerByte,
}

// See: https://github.com/aptos-labs/aptos-core/security/advisories/GHSA-x43p-vm4h-r828
fn native_keccak256(
    gas_params: &Keccak256HashGasParameters,
    context: &mut SafeNativeContext,
    mut _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert!(_ty_args.is_empty());
    debug_assert!(args.len() == 1);

    let bytes = safely_pop_arg!(args, Vec<u8>);

    let cost = gas_params.base + gas_params.per_byte * NumBytes::new(bytes.len() as u64);
    context.charge(cost)?;

    let mut hasher = Keccak::v256();
    hasher.update(&bytes);
    let mut output = [0u8; 32];
    hasher.finalize(&mut output);

    Ok(smallvec![Value::vector_u8(output)])
}

#[derive(Debug, Clone)]
pub struct Sha2_512HashGasParameters {
    pub base: InternalGas,
    pub per_byte: InternalGasPerByte,
}

// See: https://github.com/aptos-labs/aptos-core/security/advisories/GHSA-x43p-vm4h-r828
fn native_sha2_512(
    gas_params: &Sha2_512HashGasParameters,
    context: &mut SafeNativeContext,
    mut _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert!(_ty_args.is_empty());
    debug_assert!(args.len() == 1);

    let bytes = safely_pop_arg!(args, Vec<u8>);

    let cost = gas_params.base + gas_params.per_byte * NumBytes::new(bytes.len() as u64);
    context.charge(cost)?;

    let mut hasher = sha2::Sha512::new();
    hasher.update(&bytes);
    let output = hasher.finalize().to_vec();

    Ok(smallvec![Value::vector_u8(output)])
}

#[derive(Debug, Clone)]
pub struct Sha3_512HashGasParameters {
    pub base: InternalGas,
    pub per_byte: InternalGasPerByte,
}

// See: https://github.com/aptos-labs/aptos-core/security/advisories/GHSA-x43p-vm4h-r828
fn native_sha3_512(
    gas_params: &Sha3_512HashGasParameters,
    context: &mut SafeNativeContext,
    mut _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert!(_ty_args.is_empty());
    debug_assert!(args.len() == 1);

    let bytes = safely_pop_arg!(args, Vec<u8>);

    let cost = gas_params.base + gas_params.per_byte * NumBytes::new(bytes.len() as u64);
    context.charge(cost)?;

    let mut hasher = sha3::Sha3_512::new();
    hasher.update(&bytes);
    let output = hasher.finalize().to_vec();

    Ok(smallvec![Value::vector_u8(output)])
}

#[derive(Debug, Clone)]
pub struct Ripemd160HashGasParameters {
    pub base: InternalGas,
    pub per_byte: InternalGasPerByte,
}

// See: https://github.com/aptos-labs/aptos-core/security/advisories/GHSA-x43p-vm4h-r828
fn native_ripemd160(
    gas_params: &Ripemd160HashGasParameters,
    context: &mut SafeNativeContext,
    mut _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert!(_ty_args.is_empty());
    debug_assert!(args.len() == 1);

    let bytes = safely_pop_arg!(args, Vec<u8>);

    let cost = gas_params.base + gas_params.per_byte * NumBytes::new(bytes.len() as u64);
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
#[derive(Debug, Clone)]
pub struct GasParameters {
    pub sip_hash: SipHashGasParameters,
    pub keccak256: Keccak256HashGasParameters,
    pub sha2_512: Sha2_512HashGasParameters,
    pub sha3_512: Sha3_512HashGasParameters,
    pub ripemd160: Ripemd160HashGasParameters,
}

pub fn make_all(
    gas_params: GasParameters,
    timed_features: TimedFeatures,
) -> impl Iterator<Item = (String, NativeFunction)> {
    // See: https://github.com/aptos-labs/aptos-core/security/advisories/GHSA-x43p-vm4h-r828
    let natives = [
        (
            "sip_hash",
            make_safe_native(gas_params.sip_hash, timed_features.clone(), native_sip_hash),
        ),
        (
            "keccak256",
            make_safe_native(
                gas_params.keccak256,
                timed_features.clone(),
                native_keccak256,
            ),
        ),
        (
            "sha2_512_internal",
            make_safe_native(gas_params.sha2_512, timed_features.clone(), native_sha2_512),
        ),
        (
            "sha3_512_internal",
            make_safe_native(gas_params.sha3_512, timed_features.clone(), native_sha3_512),
        ),
        (
            "ripemd160_internal",
            make_safe_native(gas_params.ripemd160, timed_features, native_ripemd160),
        ),
    ];

    crate::natives::helpers::make_module_natives(natives)
}
