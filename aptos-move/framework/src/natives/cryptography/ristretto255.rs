// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::natives::cryptography::{ristretto255_point, ristretto255_scalar};
use crate::natives::util::make_native_from_func;
use aptos_types::vm_status::StatusCode;
use curve25519_dalek::scalar::Scalar;
use move_deps::move_binary_format::errors::{PartialVMError, PartialVMResult};
use move_deps::move_core_types::gas_algebra::{InternalGasPerArg, InternalGasPerByte};
use move_deps::move_vm_types::values::{Reference, StructRef, Value};
use move_deps::{move_vm_runtime::native_functions::NativeFunction, move_vm_types::pop_arg};
use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct GasParameters {
    pub basepoint_mul: InternalGasPerArg,
    pub basepoint_double_mul: InternalGasPerArg,

    pub point_add: InternalGasPerArg,
    pub point_compress: InternalGasPerArg,
    pub point_decompress: InternalGasPerArg,
    pub point_equals: InternalGasPerArg,
    pub point_from_64_uniform_bytes: InternalGasPerArg,
    pub point_identity: InternalGasPerArg,
    pub point_mul: InternalGasPerArg,
    pub point_neg: InternalGasPerArg,
    pub point_sub: InternalGasPerArg,
    pub point_parse_arg: InternalGasPerArg,

    pub sha512_per_byte: InternalGasPerByte,
    pub sha512_per_hash: InternalGasPerArg,

    pub scalar_add: InternalGasPerArg,
    pub scalar_reduced_from_32_bytes: InternalGasPerArg,
    pub scalar_uniform_from_64_bytes: InternalGasPerArg,
    pub scalar_from_u128: InternalGasPerArg,
    pub scalar_from_u64: InternalGasPerArg,
    pub scalar_invert: InternalGasPerArg,
    pub scalar_is_canonical: InternalGasPerArg,
    pub scalar_mul: InternalGasPerArg,
    pub scalar_neg: InternalGasPerArg,
    pub scalar_sub: InternalGasPerArg,
    pub scalar_parse_arg: InternalGasPerArg,
}

pub fn make_all(gas_params: GasParameters) -> impl Iterator<Item = (String, NativeFunction)> {
    let natives = [
        (
            "point_is_canonical_internal",
            make_native_from_func(
                gas_params.clone(),
                ristretto255_point::native_point_is_canonical,
            ),
        ),
        (
            "point_identity_internal",
            make_native_from_func(
                gas_params.clone(),
                ristretto255_point::native_point_identity,
            ),
        ),
        (
            "point_decompress_internal",
            make_native_from_func(
                gas_params.clone(),
                ristretto255_point::native_point_decompress,
            ),
        ),
        (
            "point_compress_internal",
            make_native_from_func(
                gas_params.clone(),
                ristretto255_point::native_point_compress,
            ),
        ),
        (
            "point_mul_internal",
            make_native_from_func(gas_params.clone(), ristretto255_point::native_point_mul),
        ),
        (
            "point_equals",
            make_native_from_func(gas_params.clone(), ristretto255_point::native_point_equals),
        ),
        (
            "point_neg_internal",
            make_native_from_func(gas_params.clone(), ristretto255_point::native_point_neg),
        ),
        (
            "point_add_internal",
            make_native_from_func(gas_params.clone(), ristretto255_point::native_point_add),
        ),
        (
            "point_sub_internal",
            make_native_from_func(gas_params.clone(), ristretto255_point::native_point_sub),
        ),
        (
            "basepoint_mul_internal",
            make_native_from_func(gas_params.clone(), ristretto255_point::native_basepoint_mul),
        ),
        (
            "basepoint_double_mul_internal",
            make_native_from_func(
                gas_params.clone(),
                ristretto255_point::native_basepoint_double_mul,
            ),
        ),
        (
            "new_point_from_sha512_internal",
            make_native_from_func(
                gas_params.clone(),
                ristretto255_point::native_new_point_from_sha512,
            ),
        ),
        (
            "new_point_from_64_uniform_bytes_internal",
            make_native_from_func(
                gas_params.clone(),
                ristretto255_point::native_new_point_from_64_uniform_bytes,
            ),
        ),
        (
            "multi_scalar_mul_internal",
            make_native_from_func(
                gas_params.clone(),
                ristretto255_point::native_multi_scalar_mul,
            ),
        ),
        (
            "scalar_is_canonical_internal",
            make_native_from_func(
                gas_params.clone(),
                ristretto255_scalar::native_scalar_is_canonical,
            ),
        ),
        (
            "scalar_invert_internal",
            make_native_from_func(
                gas_params.clone(),
                ristretto255_scalar::native_scalar_invert,
            ),
        ),
        (
            "scalar_from_sha512_internal",
            make_native_from_func(
                gas_params.clone(),
                ristretto255_scalar::native_scalar_from_sha512,
            ),
        ),
        (
            "scalar_mul_internal",
            make_native_from_func(gas_params.clone(), ristretto255_scalar::native_scalar_mul),
        ),
        (
            "scalar_add_internal",
            make_native_from_func(gas_params.clone(), ristretto255_scalar::native_scalar_add),
        ),
        (
            "scalar_sub_internal",
            make_native_from_func(gas_params.clone(), ristretto255_scalar::native_scalar_sub),
        ),
        (
            "scalar_neg_internal",
            make_native_from_func(gas_params.clone(), ristretto255_scalar::native_scalar_neg),
        ),
        (
            "scalar_from_u64_internal",
            make_native_from_func(
                gas_params.clone(),
                ristretto255_scalar::native_scalar_from_u64,
            ),
        ),
        (
            "scalar_from_u128_internal",
            make_native_from_func(
                gas_params.clone(),
                ristretto255_scalar::native_scalar_from_u128,
            ),
        ),
        (
            "scalar_reduced_from_32_bytes_internal",
            make_native_from_func(
                gas_params.clone(),
                ristretto255_scalar::native_scalar_reduced_from_32_bytes,
            ),
        ),
        (
            "scalar_uniform_from_64_bytes_internal",
            make_native_from_func(
                gas_params,
                ristretto255_scalar::native_scalar_uniform_from_64_bytes,
            ),
        ),
    ];

    crate::natives::helpers::make_module_natives(natives)
}

/// Pops a 32 byte slice off the argument stack.
pub fn pop_32_byte_slice(arguments: &mut VecDeque<Value>) -> PartialVMResult<[u8; 32]> {
    let bytes = pop_arg!(arguments, Vec<u8>);

    <[u8; 32]>::try_from(bytes).map_err(|_| PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR))
}

/// Pops a 64 byte slice off the argument stack.
pub fn pop_64_byte_slice(arguments: &mut VecDeque<Value>) -> PartialVMResult<[u8; 64]> {
    let bytes = pop_arg!(arguments, Vec<u8>);

    <[u8; 64]>::try_from(bytes).map_err(|_| PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR))
}

/// Pops a Scalar off the argument stack when the argument was a vector<u8>.
pub fn pop_scalar_from_bytes(arguments: &mut VecDeque<Value>) -> PartialVMResult<Scalar> {
    let bytes = pop_arg!(arguments, Vec<u8>);

    scalar_from_valid_bytes(bytes)
}

/// The 'data' field inside a Move Scalar struct is at index 0.
const DATA_FIELD_INDEX: usize = 0;

/// Get a curve25519-dalek Scalar struct from a Move Scalar struct.
pub fn scalar_from_struct(move_scalar: Value) -> PartialVMResult<Scalar> {
    let move_struct = move_scalar.value_as::<StructRef>()?;

    let bytes_field_ref = move_struct
        .borrow_field(DATA_FIELD_INDEX)?
        .value_as::<Reference>()?;

    let scalar_bytes = bytes_field_ref.read_ref()?.value_as::<Vec<u8>>()?;

    scalar_from_valid_bytes(scalar_bytes)
}

/// Constructs a curve25519-dalek Scalar from a sequence of bytes which are assumed to
/// canonically-encode it. Callers who are not sure of the canonicity of the encoding MUST call
/// Scalar::is_canonical() after on the returned Scalar.
pub fn scalar_from_valid_bytes(bytes: Vec<u8>) -> PartialVMResult<Scalar> {
    // A Move Scalar's length should be exactly 32 bytes
    let slice = <[u8; 32]>::try_from(bytes)
        .map_err(|_| PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR))?;

    // NOTE: This will clear the high bit of 'slice'
    let s = Scalar::from_bits(slice);

    debug_assert!(s.is_canonical());

    Ok(s)
}
