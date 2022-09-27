// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::natives::util::make_native_from_func;
use move_deps::{
    move_binary_format::errors::PartialVMResult,
    move_core_types::gas_algebra::{InternalGas, InternalGasPerByte, NumBytes},
    move_vm_runtime::native_functions::{NativeContext, NativeFunction},
    move_vm_types::{
        loaded_data::runtime_types::Type, natives::function::NativeResult, pop_arg, values::Value,
    },
};
use smallvec::smallvec;
use std::{collections::VecDeque};
use base64;

/***************************************************************************************************
 * native fun sip_hash
 *
 *   gas cost: base_cost + unit_cost * data_length
 *
 **************************************************************************************************/
#[derive(Debug, Clone)]
pub struct Base64EncodeGasParameters {
    pub base: InternalGas,
    pub per_byte: InternalGasPerByte,
}

/// Feed these bytes into SipHasher. This is not cryptographically secure.
fn native_base64_encode(
    gas_params: &Base64EncodeGasParameters,
    _context: &mut NativeContext,
    mut _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    debug_assert!(_ty_args.is_empty());
    debug_assert!(args.len() == 1);

    let bytes = pop_arg!(args, Vec<u8>);

    let cost = gas_params.base + gas_params.per_byte * NumBytes::new(bytes.len() as u64);

    // SipHash of the serialized bytes
    // let mut hasher = siphasher::sip::SipHasher::new();
    // hasher.write(&bytes);
    // let hash = hasher.finish();
    let base64_encoded = base64::encode(&bytes);

    Ok(NativeResult::ok(cost, smallvec![Value::vector_u8(base64_encoded.as_bytes().to_vec())]))
}

#[derive(Debug, Clone)]
pub struct Base64DecodeGasParameters {
    pub base: InternalGas,
    pub per_byte: InternalGasPerByte,
}

/// Feed these bytes into SipHasher. This is not cryptographically secure.
fn native_base64_decode(
    gas_params: &Base64DecodeGasParameters,
    _context: &mut NativeContext,
    mut _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    debug_assert!(_ty_args.is_empty());
    debug_assert!(args.len() == 1);

    let bytes = pop_arg!(args, Vec<u8>);

    let cost = gas_params.base + gas_params.per_byte * NumBytes::new(bytes.len() as u64);

    // SipHash of the serialized bytes
    // let mut hasher = siphasher::sip::SipHasher::new();
    // hasher.write(&bytes);
    // let hash = hasher.finish();
    let base64_decoded = base64::decode(&bytes);
    if let Ok(base64_decoded) = base64_decoded{
        Ok(NativeResult::ok(cost, smallvec![Value::vector_u8(base64_decoded)]))
    } else {
        Ok(NativeResult::err(
            cost,
            super::status::NFE_EXPECTED_STRUCT_TYPE_TAG,
        ))
    }
}

/***************************************************************************************************
 * module
 *
 **************************************************************************************************/
#[derive(Debug, Clone)]
pub struct GasParameters {
    pub base64_encode: Base64EncodeGasParameters,
    pub base64_decode: Base64DecodeGasParameters,
}

pub fn make_all(gas_params: GasParameters) -> impl Iterator<Item = (String, NativeFunction)> {
    let natives = [
        (
            "base64_encode",
            make_native_from_func(gas_params.base64_encode, native_base64_encode),
        ),
        (
            "base64_decode",
            make_native_from_func(gas_params.base64_decode, native_base64_decode),
        ),
    ];

    crate::natives::helpers::make_module_natives(natives)
}
