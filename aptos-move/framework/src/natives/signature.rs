// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::natives::util::make_native_from_func;
use aptos_crypto::{ed25519, traits::*};
use curve25519_dalek::edwards::CompressedEdwardsY;
use move_deps::{
    move_binary_format::errors::PartialVMResult,
    move_vm_runtime::native_functions::{NativeContext, NativeFunction},
    move_vm_types::{
        loaded_data::runtime_types::Type, natives::function::NativeResult, pop_arg, values::Value,
    },
};
use smallvec::smallvec;
use std::{collections::VecDeque, convert::TryFrom};

/***************************************************************************************************
 * native fun ed25519_validate_pubkey
 *
 *   gas cost: base_cost + per_pubkey_deserialize_cost +? per_pubkey_small_order_check
 *
 * where +? indicates that the expression stops evaluating there if the previous gas-charging step
 * failed
 **************************************************************************************************/
#[derive(Debug, Clone)]
pub struct Ed25519ValidatePubkeyGasParameters {
    pub base_cost: u64,
    pub per_pubkey_deserialize_cost: u64,
    pub per_pubkey_small_order_check_cost: u64,
}

fn native_ed25519_validate_pubkey(
    gas_params: &Ed25519ValidatePubkeyGasParameters,
    _context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    debug_assert!(_ty_args.is_empty());
    debug_assert!(arguments.len() == 1);

    let key_bytes = pop_arg!(arguments, Vec<u8>);

    let mut cost = gas_params.base_cost + gas_params.per_pubkey_deserialize_cost;

    let key_bytes_slice = match <[u8; 32]>::try_from(key_bytes) {
        Ok(slice) => slice,
        Err(_) => {
            return Ok(NativeResult::ok(cost, smallvec![Value::bool(false)]));
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
    cost += gas_params.per_pubkey_small_order_check_cost;
    let valid = !point.is_small_order();

    Ok(NativeResult::ok(cost, smallvec![Value::bool(valid)]))
}

/***************************************************************************************************
 * native fun ed25519_verify
 *
 *   gas cost: base_cost + per_pubkey_deserialize_cost
 *                       +? ( per_sig_deserialize_cost
 *                            +? ( per_sig_strict_verify_cost + per_msg_hashing_base_cost
 *                                 + per_msg_byte_hashing_cost * |msg| ) )
 *
 * where +? indicates that the expression stops evaluating there if the previous gas-charging step
 * failed
 **************************************************************************************************/
#[derive(Debug, Clone)]
pub struct Ed25519VerifyGasParameters {
    pub base_cost: u64,
    pub per_pubkey_deserialize_cost: u64,
    pub per_sig_deserialize_cost: u64,
    pub per_sig_strict_verify_cost: u64,
    pub per_msg_hashing_base_cost: u64,
    pub per_msg_byte_hashing_cost: u64, // signature verification involves signing |msg| bytes
}
fn native_ed25519_verify(
    gas_params: &Ed25519VerifyGasParameters,
    _context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    debug_assert!(_ty_args.is_empty());
    debug_assert!(arguments.len() == 3);

    let msg = pop_arg!(arguments, Vec<u8>);
    let pubkey = pop_arg!(arguments, Vec<u8>);
    let signature = pop_arg!(arguments, Vec<u8>);

    let mut cost = gas_params.base_cost;

    cost += gas_params.per_pubkey_deserialize_cost;
    let pk = match ed25519::Ed25519PublicKey::try_from(pubkey.as_slice()) {
        Ok(pk) => pk,
        Err(_) => {
            return Ok(NativeResult::ok(cost, smallvec![Value::bool(false)]));
        }
    };

    cost += gas_params.per_sig_deserialize_cost;
    let sig = match ed25519::Ed25519Signature::try_from(signature.as_slice()) {
        Ok(sig) => sig,
        Err(_) => {
            return Ok(NativeResult::ok(cost, smallvec![Value::bool(false)]));
        }
    };

    // NOTE(Gas): hashing the message to the group and a size-2 multi-scalar multiplication
    cost += gas_params.per_sig_strict_verify_cost
        + gas_params.per_msg_hashing_base_cost
        + gas_params.per_msg_byte_hashing_cost * msg.len() as u64;

    let verify_result = sig.verify_arbitrary_msg(msg.as_slice(), &pk).is_ok();
    Ok(NativeResult::ok(
        cost,
        smallvec![Value::bool(verify_result)],
    ))
}

/***************************************************************************************************
 * native fun secp256k1_recover
 *
 *   gas cost: base_cost
 *
 **************************************************************************************************/
#[derive(Debug, Clone)]
pub struct Secp256k1ECDSARecoverGasParameters {
    pub base_cost: u64,
}

fn native_secp256k1_ecdsa_recover(
    gas_params: &Secp256k1ECDSARecoverGasParameters,
    _context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    debug_assert!(_ty_args.is_empty());
    debug_assert!(arguments.len() == 3);

    let signature = pop_arg!(arguments, Vec<u8>);
    let recovery_id = pop_arg!(arguments, u8);
    let msg = pop_arg!(arguments, Vec<u8>);

    let cost = gas_params.base_cost;

    // NOTE(Gas): O(1) cost
    // (In reality, O(|msg|) deserialization cost, with |msg| < libsecp256k1_core::util::MESSAGE_SIZE
    // which seems to be 32 bytes, so O(1) cost for all intents and purposes.)
    let msg = match libsecp256k1::Message::parse_slice(&msg) {
        Ok(msg) => msg,
        Err(_) => {
            return Ok(NativeResult::ok(
                cost,
                smallvec![Value::vector_u8([0u8; 0]), Value::bool(false)],
            ));
        }
    };

    // NOTE(Gas): O(1) cost
    let rid = match libsecp256k1::RecoveryId::parse(recovery_id) {
        Ok(rid) => rid,
        Err(_) => {
            return Ok(NativeResult::ok(
                cost,
                smallvec![Value::vector_u8([0u8; 0]), Value::bool(false)],
            ));
        }
    };

    // NOTE(Gas): O(1) deserialization cost
    // which seems to be 64 bytes, so O(1) cost for all intents and purposes.
    let sig = match libsecp256k1::Signature::parse_standard_slice(&signature) {
        Ok(sig) => sig,
        Err(_) => {
            return Ok(NativeResult::ok(
                cost,
                smallvec![Value::vector_u8([0u8; 0]), Value::bool(false)],
            ));
        }
    };

    // NOTE(Gas): O(1) cost: a size-2 multi-scalar multiplication
    let pk = match libsecp256k1::recover(&msg, &sig, &rid) {
        Ok(pk) => pk,
        Err(_) => {
            return Ok(NativeResult::ok(
                cost,
                smallvec![Value::vector_u8([0u8; 0]), Value::bool(false)],
            ));
        }
    };

    Ok(NativeResult::ok(
        cost,
        smallvec![
            Value::vector_u8(pk.serialize()[1..].to_vec()),
            Value::bool(true)
        ],
    ))
}

/***************************************************************************************************
 * module
 *
 **************************************************************************************************/
#[derive(Debug, Clone)]
pub struct GasParameters {
    // Ed25519 signatures
    pub ed25519_validate_pubkey: Ed25519ValidatePubkeyGasParameters,
    pub ed25519_verify: Ed25519VerifyGasParameters,

    // ECDSA signatures based on secp256k1 elliptic curves
    pub secp256k1_ecdsa_recover: Secp256k1ECDSARecoverGasParameters,
}

pub fn make_all(gas_params: GasParameters) -> impl Iterator<Item = (String, NativeFunction)> {
    let natives = [
        // Ed25519
        (
            "ed25519_validate_pubkey",
            make_native_from_func(
                gas_params.ed25519_validate_pubkey,
                native_ed25519_validate_pubkey,
            ),
        ),
        (
            "ed25519_verify",
            make_native_from_func(gas_params.ed25519_verify, native_ed25519_verify),
        ),
        // ECDSA over secp256k1
        (
            "secp256k1_ecdsa_recover",
            make_native_from_func(
                gas_params.secp256k1_ecdsa_recover,
                native_secp256k1_ecdsa_recover,
            ),
        ),
    ];

    crate::natives::helpers::make_module_natives(natives)
}
