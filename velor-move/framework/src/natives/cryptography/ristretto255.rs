// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::natives::cryptography::{ristretto255_point, ristretto255_scalar};
use velor_gas_algebra::GasExpression;
use velor_gas_schedule::{gas_params::natives::velor_framework::*, NativeGasParameters};
use velor_native_interface::{
    safely_assert_eq, safely_pop_arg, safely_pop_vec_arg, RawSafeNative, SafeNativeBuilder,
    SafeNativeError, SafeNativeResult,
};
use velor_types::vm_status::StatusCode;
use curve25519_dalek::scalar::Scalar;
use move_binary_format::errors::PartialVMError;
use move_core_types::gas_algebra::{InternalGasUnit, NumArgs};
use move_vm_runtime::native_functions::NativeFunction;
use move_vm_types::values::{Reference, StructRef, Value};
use std::collections::VecDeque;

/// The size of a serialized scalar, in bytes.
pub(crate) const SCALAR_NUM_BYTES: usize = 32;

/// The size of a serialized compressed Ristretto point, in bytes.
pub(crate) const COMPRESSED_POINT_NUM_BYTES: usize = 32;

/// Returns gas costs for a variable-time multiscalar multiplication (MSM) of size-n. The MSM
/// employed in curve25519 is:
///  1. Strauss, when n <= 190, see <https://www.jstor.org/stable/2310929>
///  2. Pippinger, when n > 190, which roughly requires O(n / log_2 n) scalar multiplications
/// For simplicity, we estimate the complexity as O(n / log_2 n)
pub fn multi_scalar_mul_gas(
    size: usize,
) -> impl GasExpression<NativeGasParameters, Unit = InternalGasUnit> {
    RISTRETTO255_POINT_MUL * NumArgs::new((size as f64 / f64::log2(size as f64)).ceil() as u64)
}

pub fn make_all(
    builder: &SafeNativeBuilder,
) -> impl Iterator<Item = (String, NativeFunction)> + '_ {
    let mut natives = vec![];

    #[cfg(feature = "testing")]
    natives.extend([(
        "random_scalar_internal",
        ristretto255_scalar::native_scalar_random as RawSafeNative,
    )]);

    natives.extend([
        (
            "point_is_canonical_internal",
            ristretto255_point::native_point_is_canonical as RawSafeNative,
        ),
        (
            "point_identity_internal",
            ristretto255_point::native_point_identity,
        ),
        (
            "point_decompress_internal",
            ristretto255_point::native_point_decompress,
        ),
        (
            "point_clone_internal",
            ristretto255_point::native_point_clone,
        ),
        (
            "point_compress_internal",
            ristretto255_point::native_point_compress,
        ),
        ("point_mul_internal", ristretto255_point::native_point_mul),
        (
            "point_double_mul_internal",
            ristretto255_point::native_double_scalar_mul,
        ),
        ("point_equals", ristretto255_point::native_point_equals),
        ("point_neg_internal", ristretto255_point::native_point_neg),
        ("point_add_internal", ristretto255_point::native_point_add),
        ("point_sub_internal", ristretto255_point::native_point_sub),
        (
            "basepoint_mul_internal",
            ristretto255_point::native_basepoint_mul,
        ),
        (
            "basepoint_double_mul_internal",
            ristretto255_point::native_basepoint_double_mul,
        ),
        (
            // NOTE: This was supposed to be more clearly named with *_sha2_512_*.
            "new_point_from_sha512_internal",
            ristretto255_point::native_new_point_from_sha512,
        ),
        (
            "new_point_from_64_uniform_bytes_internal",
            ristretto255_point::native_new_point_from_64_uniform_bytes,
        ),
        (
            "double_scalar_mul_internal",
            ristretto255_point::native_double_scalar_mul,
        ),
        (
            "multi_scalar_mul_internal",
            ristretto255_point::safe_native_multi_scalar_mul_no_floating_point,
        ),
        (
            "scalar_is_canonical_internal",
            ristretto255_scalar::native_scalar_is_canonical,
        ),
        (
            "scalar_invert_internal",
            ristretto255_scalar::native_scalar_invert,
        ),
        // NOTE: This was supposed to be more clearly named with *_sha2_512_*.
        (
            "scalar_from_sha512_internal",
            ristretto255_scalar::native_scalar_from_sha512,
        ),
        (
            "scalar_mul_internal",
            ristretto255_scalar::native_scalar_mul,
        ),
        (
            "scalar_add_internal",
            ristretto255_scalar::native_scalar_add,
        ),
        (
            "scalar_sub_internal",
            ristretto255_scalar::native_scalar_sub,
        ),
        (
            "scalar_neg_internal",
            ristretto255_scalar::native_scalar_neg,
        ),
        (
            "scalar_from_u64_internal",
            ristretto255_scalar::native_scalar_from_u64,
        ),
        (
            "scalar_from_u128_internal",
            ristretto255_scalar::native_scalar_from_u128,
        ),
        (
            "scalar_reduced_from_32_bytes_internal",
            ristretto255_scalar::native_scalar_reduced_from_32_bytes,
        ),
        (
            "scalar_uniform_from_64_bytes_internal",
            ristretto255_scalar::native_scalar_uniform_from_64_bytes,
        ),
    ]);

    builder.make_named_natives(natives)
}

/// Pops a 32 byte slice off the argument stack.
pub fn pop_32_byte_slice(arguments: &mut VecDeque<Value>) -> SafeNativeResult<[u8; 32]> {
    let bytes = safely_pop_arg!(arguments, Vec<u8>);

    <[u8; 32]>::try_from(bytes).map_err(|_| {
        SafeNativeError::InvariantViolation(PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR))
    })
}

/// Pops a 64 byte slice off the argument stack.
pub fn pop_64_byte_slice(arguments: &mut VecDeque<Value>) -> SafeNativeResult<[u8; 64]> {
    let bytes = safely_pop_arg!(arguments, Vec<u8>);

    <[u8; 64]>::try_from(bytes).map_err(|_| {
        SafeNativeError::InvariantViolation(PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR))
    })
}

/// Pops a Scalar off the argument stack when the argument was a `vector<u8>`.
pub fn pop_scalar_from_bytes(arguments: &mut VecDeque<Value>) -> SafeNativeResult<Scalar> {
    let bytes = safely_pop_arg!(arguments, Vec<u8>);

    scalar_from_valid_bytes(bytes)
}

/// Pops a Scalars off the argument stack when the argument was a `vector<vector<u8>>`.
pub fn pop_scalars_from_bytes(arguments: &mut VecDeque<Value>) -> SafeNativeResult<Vec<Scalar>> {
    let bytes = safely_pop_vec_arg!(arguments, Vec<u8>);

    bytes
        .into_iter()
        .map(scalar_from_valid_bytes)
        .collect::<SafeNativeResult<Vec<_>>>()
}

/// The 'data' field inside a Move Scalar struct is at index 0.
const DATA_FIELD_INDEX: usize = 0;

/// Get a curve25519-dalek Scalar struct from a Move Scalar struct.
pub fn scalar_from_struct(move_scalar: Value) -> SafeNativeResult<Scalar> {
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
pub fn scalar_from_valid_bytes(bytes: Vec<u8>) -> SafeNativeResult<Scalar> {
    // A Move Scalar's length should be exactly 32 bytes
    let slice = <[u8; 32]>::try_from(bytes).map_err(|_| {
        SafeNativeError::InvariantViolation(PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR))
    })?;

    // NOTE: This will clear the high bit of 'slice'
    let s = Scalar::from_bits(slice);

    safely_assert_eq!(s.is_canonical(), true);

    Ok(s)
}
