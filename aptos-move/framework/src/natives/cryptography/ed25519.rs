// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::natives::util::make_native_from_func;
use aptos_crypto::ed25519::ED25519_PUBLIC_KEY_LENGTH;
use aptos_crypto::{ed25519, traits::*};
use curve25519_dalek::edwards::CompressedEdwardsY;
use move_deps::move_core_types::gas_algebra::{InternalGasPerArg, NumArgs};
use move_deps::{
    move_binary_format::errors::PartialVMResult,
    move_core_types::gas_algebra::{InternalGas, InternalGasPerByte, NumBytes},
    move_vm_runtime::native_functions::{NativeContext, NativeFunction},
    move_vm_types::{
        loaded_data::runtime_types::Type, natives::function::NativeResult, pop_arg, values::Value,
    },
};
use smallvec::smallvec;
use std::{collections::VecDeque, convert::TryFrom};

pub mod abort_codes {
    pub const E_WRONG_PUBKEY_SIZE: u64 = 1;
    pub const E_WRONG_SIGNATURE_SIZE: u64 = 2;
}

/***************************************************************************************************
 * native fun pubkey_validate_internal
 *
 *   gas cost: base_cost + per_pubkey_deserialize_cost +? per_pubkey_small_order_check
 *
 * where +? indicates that the expression stops evaluating there if the previous gas-charging step
 * failed
 **************************************************************************************************/
fn native_public_key_validate(
    gas_params: &GasParameters,
    _context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    debug_assert!(_ty_args.is_empty());
    debug_assert!(arguments.len() == 1);

    let key_bytes = pop_arg!(arguments, Vec<u8>);

    let mut cost = gas_params.base + gas_params.per_pubkey_deserialize * NumArgs::one();

    let key_bytes_slice = match <[u8; ED25519_PUBLIC_KEY_LENGTH]>::try_from(key_bytes) {
        Ok(slice) => slice,
        Err(_) => {
            return Ok(NativeResult::err(cost, abort_codes::E_WRONG_PUBKEY_SIZE));
        }
    };

    // This deserialization only performs point-on-curve checks, so we check for small subgroup below
    // NOTE(Gas): O(1) cost: some arithmetic for converting to (X, Y, Z, T) coordinates
    let point = match CompressedEdwardsY(key_bytes_slice).decompress() {
        Some(point) => point,
        None => {
            return Ok(NativeResult::ok(cost, smallvec![Value::bool(false)]));
        }
    };

    // Check if the point lies on a small subgroup. This is required when using curves with a
    // small cofactor (e.g., in Ed25519, cofactor = 8).
    // NOTE(Gas): O(1) cost: multiplies the point by the cofactor
    cost += gas_params.per_pubkey_small_order_check * NumArgs::one();
    let valid = !point.is_small_order();

    Ok(NativeResult::ok(cost, smallvec![Value::bool(valid)]))
}

/***************************************************************************************************
 * native fun signature_verify_strict_internal
 *
 *   gas cost: base_cost + per_pubkey_deserialize_cost
 *                       +? ( per_sig_deserialize_cost
 *                            +? ( per_sig_strict_verify_cost + per_msg_hashing_base_cost
 *                                 + per_msg_byte_hashing_cost * |msg| ) )
 *
 * where +? indicates that the expression stops evaluating there if the previous gas-charging step
 * failed
 **************************************************************************************************/
fn native_signature_verify_strict(
    gas_params: &GasParameters,
    _context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    debug_assert!(_ty_args.is_empty());
    debug_assert!(arguments.len() == 3);

    let msg = pop_arg!(arguments, Vec<u8>);
    let pubkey = pop_arg!(arguments, Vec<u8>);
    let signature = pop_arg!(arguments, Vec<u8>);

    let mut cost = gas_params.base;

    cost += gas_params.per_pubkey_deserialize * NumArgs::one();
    let pk = match ed25519::Ed25519PublicKey::try_from(pubkey.as_slice()) {
        Ok(pk) => pk,
        Err(_) => {
            return Ok(NativeResult::ok(cost, smallvec![Value::bool(false)]));
        }
    };

    cost += gas_params.per_sig_deserialize * NumArgs::one();
    let sig = match ed25519::Ed25519Signature::try_from(signature.as_slice()) {
        Ok(sig) => sig,
        Err(_) => {
            return Ok(NativeResult::ok(cost, smallvec![Value::bool(false)]));
        }
    };

    // NOTE(Gas): hashing the message to the group and a size-2 multi-scalar multiplication
    cost += gas_params.per_sig_strict_verify * NumArgs::one()
        + gas_params.per_msg_hashing_base * NumArgs::one()
        + gas_params.per_msg_byte_hashing * NumBytes::new(msg.len() as u64);

    let verify_result = sig.verify_arbitrary_msg(msg.as_slice(), &pk).is_ok();
    Ok(NativeResult::ok(
        cost,
        smallvec![Value::bool(verify_result)],
    ))
}

/***************************************************************************************************
 * module
 *
 **************************************************************************************************/
#[derive(Debug, Clone)]
pub struct GasParameters {
    pub base: InternalGas,
    pub per_pubkey_deserialize: InternalGasPerArg,
    pub per_pubkey_small_order_check: InternalGasPerArg,
    pub per_sig_deserialize: InternalGasPerArg,
    pub per_sig_strict_verify: InternalGasPerArg,
    pub per_msg_hashing_base: InternalGasPerArg,
    pub per_msg_byte_hashing: InternalGasPerByte, // signature verification involves signing |msg| bytes
}

pub fn make_all(gas_params: GasParameters) -> impl Iterator<Item = (String, NativeFunction)> {
    let natives = [
        // Ed25519
        (
            "public_key_validate_internal",
            make_native_from_func(gas_params.clone(), native_public_key_validate),
        ),
        (
            "signature_verify_strict_internal",
            make_native_from_func(gas_params, native_signature_verify_strict),
        ),
    ];

    crate::natives::helpers::make_module_natives(natives)
}
