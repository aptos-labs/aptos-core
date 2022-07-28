// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_crypto::{bls12381, ed25519, traits::*};
use curve25519_dalek::edwards::CompressedEdwardsY;
use move_deps::move_vm_types::values::Struct;
use move_deps::{
    move_binary_format::errors::PartialVMResult,
    move_vm_runtime::native_functions::{NativeContext, NativeFunction},
    move_vm_types::{
        loaded_data::runtime_types::Type, natives::function::NativeResult, pop_arg, values::Value,
    },
};
use smallvec::smallvec;
use std::{collections::VecDeque, convert::TryFrom, sync::Arc};

/// Returns the equivalent of a Move std::option::none() natively in Rust.
/// TODO: vector_for_testing_only is not an API we conceptually support and misusing it could cause the VM to crash.
fn none_option() -> Value {
    Value::struct_(Struct::pack(std::iter::once(
        Value::vector_for_testing_only(std::iter::empty()),
    )))
}

/// Returns the equivalent of a Move std::option<vector<u8>>::some(v) natively in Rust.
/// TODO: vector_for_testing_only is not an API we conceptually support and misusing it could cause the VM to crash.
fn some_option(v: Vec<u8>) -> Value {
    let vv = Value::vector_u8(v.into_iter());
    Value::struct_(Struct::pack(std::iter::once(
        Value::vector_for_testing_only(std::iter::once(vv)),
    )))
}

/// Used to pop a Vec<Vec<u8>> argument off the stack.
macro_rules! pop_vec_arg {
    ($arguments:ident, $t:ty) => {{
        // Replicating the code from pop_arg! here
        use move_deps::move_vm_types::natives::function::{PartialVMError, StatusCode};
        let value_vec = match $arguments.pop_back().map(|v| v.value_as::<Vec<Value>>()) {
            None => {
                return Err(PartialVMError::new(
                    StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
                ))
            }
            Some(Err(e)) => return Err(e),
            Some(Ok(v)) => v,
        };

        // Pop each Value from the popped Vec<Value>, cast it as a Vec<u8>, and push it to a Vec<Vec<u8>>
        let mut vec_vec = vec![];
        for value in value_vec {
            let vec = match value.value_as::<$t>() {
                Err(e) => return Err(e),
                Ok(v) => v,
            };
            vec_vec.push(vec);
        }

        vec_vec
    }};
}

/***************************************************************************************************
 * native fun bls12381_aggregate_pop_verified_pubkeys
 *
 *   gas cost: base_cost
 *
 **************************************************************************************************/
#[derive(Debug, Clone)]
pub struct Bls12381AggregatePopVerifiedPubkeysGasParameters {
    pub base_cost: u64,
    pub per_pubkey_cost: u64,
}

fn native_bls12381_aggregate_pop_verified_pubkeys(
    gas_params: &Bls12381AggregatePopVerifiedPubkeysGasParameters,
    _context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    debug_assert!(_ty_args.is_empty());
    debug_assert!(arguments.len() == 1);

    // Parses a Vec<Vec<u8>> of all serialized public keys
    let pks_serialized = pop_vec_arg!(arguments, Vec<u8>);
    let mut pks = vec![];

    let cost = gas_params.base_cost + gas_params.per_pubkey_cost * pks_serialized.len() as u64;

    for pk_bytes in pks_serialized {
        // NOTE(Gas): O(1) deserialization cost
        let pk = match bls12381::PublicKey::try_from(&pk_bytes[..]) {
            Ok(key) => key,
            // If PK does not deserialize correctly, return None.
            Err(_) => return Ok(NativeResult::ok(cost, smallvec![none_option()])),
        };

        pks.push(pk);
    }

    // If zero PKs were given as input, return None.
    if pks.is_empty() {
        return Ok(NativeResult::ok(cost, smallvec![none_option()]));
    }

    // Aggregate the public keys (this will NOT group-check the individual PKs)
    let aggpk =
        // NOTE(Gas): O(|pks|) cost: |pks| elliptic curve additions
        match bls12381::PublicKey::aggregate(pks.iter().collect::<Vec<&bls12381::PublicKey>>()) {
            Ok(aggpk) => aggpk,
            Err(_) => return Ok(NativeResult::ok(cost, smallvec![none_option()])),
        };

    Ok(NativeResult::ok(
        cost,
        smallvec![some_option(aggpk.to_bytes().to_vec())],
    ))
}

pub fn make_native_bls12381_aggregate_pop_verified_pubkeys(
    gas_params: Bls12381AggregatePopVerifiedPubkeysGasParameters,
) -> NativeFunction {
    Arc::new(move |context, ty_args, args| {
        native_bls12381_aggregate_pop_verified_pubkeys(&gas_params, context, ty_args, args)
    })
}

/***************************************************************************************************
 * native fun bls12381_verify_proof_of_possession
 *
 *   gas cost: base_cost
 *
 **************************************************************************************************/
#[derive(Debug, Clone)]
pub struct Bls12381VerifyProofOfPosessionGasParameters {
    pub base_cost: u64,
}

fn native_bls12381_verify_proof_of_possession(
    gas_params: &Bls12381VerifyProofOfPosessionGasParameters,
    _context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    debug_assert!(_ty_args.is_empty());
    debug_assert!(arguments.len() == 2);

    let pop_bytes = pop_arg!(arguments, Vec<u8>);
    let key_bytes = pop_arg!(arguments, Vec<u8>);

    // NOTE(Gas): O(1) deserialization cost
    let pop = match bls12381::ProofOfPossession::try_from(&pop_bytes[..]) {
        Ok(pop) => pop,
        Err(_) => {
            return Ok(NativeResult::ok(
                gas_params.base_cost,
                smallvec![Value::bool(false)],
            ))
        }
    };

    // NOTE(Gas): O(1) deserialization cost
    let public_key = match bls12381::PublicKey::try_from(&key_bytes[..]) {
        Ok(key) => key,
        Err(_) => {
            return Ok(NativeResult::ok(
                gas_params.base_cost,
                smallvec![Value::bool(false)],
            ))
        }
    };

    // NOTE(Gas): O(1) cost: 2 bilinear pairings and a hash-to-curve
    let valid = pop.verify(&public_key).is_ok();

    Ok(NativeResult::ok(
        gas_params.base_cost,
        smallvec![Value::bool(valid)],
    ))
}

pub fn make_native_bls12381_verify_proof_of_possession(
    gas_params: Bls12381VerifyProofOfPosessionGasParameters,
) -> NativeFunction {
    Arc::new(move |context, ty_args, args| {
        native_bls12381_verify_proof_of_possession(&gas_params, context, ty_args, args)
    })
}

/***************************************************************************************************
 * native fun bls12381_validate_pubkey
 *
 *   gas cost: base_cost
 *
 **************************************************************************************************/
#[derive(Debug, Clone)]
pub struct Bls12381ValidatePubkeyGasParameters {
    pub base_cost: u64,
}

fn native_bls12381_validate_pubkey(
    gas_params: &Bls12381ValidatePubkeyGasParameters,
    _context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    debug_assert!(_ty_args.is_empty());
    debug_assert!(arguments.len() == 1);

    let pk_bytes = pop_arg!(arguments, Vec<u8>);

    // NOTE(Gas): O(1) deserialization cost
    let public_key = match bls12381::PublicKey::try_from(&pk_bytes[..]) {
        Ok(key) => key,
        Err(_) => {
            return Ok(NativeResult::ok(
                gas_params.base_cost,
                smallvec![Value::bool(false)],
            ))
        }
    };

    // NOTE(Gas): O(1) cost: uses endomorphisms for performing faster subgroup checks
    let valid = public_key.group_check().is_ok();

    Ok(NativeResult::ok(
        gas_params.base_cost,
        smallvec![Value::bool(valid)],
    ))
}

pub fn make_native_bls12381_validate_pubkey(
    gas_params: Bls12381ValidatePubkeyGasParameters,
) -> NativeFunction {
    Arc::new(move |context, ty_args, args| {
        native_bls12381_validate_pubkey(&gas_params, context, ty_args, args)
    })
}

/***************************************************************************************************
 * native fun ed25519_validate_pubkey
 *
 *   gas cost: base_cost
 *
 **************************************************************************************************/
#[derive(Debug, Clone)]
pub struct Ed25519ValidatePubkeyGasParameters {
    pub base_cost: u64,
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

    // NOTE(Gas): O(1) deserialization cost
    let key_bytes_slice = match <[u8; 32]>::try_from(key_bytes) {
        Ok(slice) => slice,
        Err(_) => {
            return Ok(NativeResult::ok(
                gas_params.base_cost,
                smallvec![Value::bool(false)],
            ));
        }
    };

    // This deserialization only performs point-on-curve checks, so we check for small subgroup below
    // NOTE(Gas): O(1) cost: some arithmetic for converting to (X, Y, Z, T) coordinates
    let point = match CompressedEdwardsY(key_bytes_slice).decompress() {
        Some(point) => point,
        None => {
            return Ok(NativeResult::ok(
                gas_params.base_cost,
                smallvec![Value::bool(false)],
            ));
        }
    };

    // Check if the point lies on a small subgroup. This is required when using curves with a
    // small cofactor (e.g., in Ed25519, cofactor = 8).
    // NOTE(Gas): O(1) cost: multiplies the point by the cofactor
    let valid = !point.is_small_order();

    Ok(NativeResult::ok(
        gas_params.base_cost,
        smallvec![Value::bool(valid)],
    ))
}

pub fn make_native_ed25519_validate_pubkey(
    gas_params: Ed25519ValidatePubkeyGasParameters,
) -> NativeFunction {
    Arc::new(move |context, ty_args, args| {
        native_ed25519_validate_pubkey(&gas_params, context, ty_args, args)
    })
}

/***************************************************************************************************
 * native fun ed25519_verify
 *
 *   gas cost: base_cost + unit_cost * message_size
 *
 **************************************************************************************************/
#[derive(Debug, Clone)]
pub struct Ed25519VerifyGasParameters {
    pub base_cost: u64,
    pub unit_cost: u64,
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

    // NOTE(Gas): O(1) deserialization cost
    let sig = match ed25519::Ed25519Signature::try_from(signature.as_slice()) {
        Ok(sig) => sig,
        Err(_) => {
            return Ok(NativeResult::ok(cost, smallvec![Value::bool(false)]));
        }
    };

    // NOTE(Gas): O(1) deserialization cost
    let pk = match ed25519::Ed25519PublicKey::try_from(pubkey.as_slice()) {
        Ok(pk) => pk,
        Err(_) => {
            return Ok(NativeResult::ok(cost, smallvec![Value::bool(false)]));
        }
    };

    // NOTE(Gas): O(1) cost: a size-2 multi-scalar multiplication
    cost += gas_params.unit_cost * msg.len() as u64;

    let verify_result = sig.verify_arbitrary_msg(msg.as_slice(), &pk).is_ok();
    Ok(NativeResult::ok(
        cost,
        smallvec![Value::bool(verify_result)],
    ))
}

pub fn make_native_ed25519_verify(gas_params: Ed25519VerifyGasParameters) -> NativeFunction {
    Arc::new(move |context, ty_args, args| {
        native_ed25519_verify(&gas_params, context, ty_args, args)
    })
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

    // NOTE(Gas): O(1) cost
    // (In reality, O(|msg|) deserialization cost, with |msg| < libsecp256k1_core::util::MESSAGE_SIZE
    // which seems to be 32 bytes, so O(1) cost for all intents and purposes.)
    let msg = match libsecp256k1::Message::parse_slice(&msg) {
        Ok(msg) => msg,
        Err(_) => {
            return Ok(NativeResult::ok(
                gas_params.base_cost,
                smallvec![Value::vector_u8([0u8; 0]), Value::bool(false)],
            ));
        }
    };

    // NOTE(Gas): O(1) cost
    let rid = match libsecp256k1::RecoveryId::parse(recovery_id) {
        Ok(rid) => rid,
        Err(_) => {
            return Ok(NativeResult::ok(
                gas_params.base_cost,
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
                gas_params.base_cost,
                smallvec![Value::vector_u8([0u8; 0]), Value::bool(false)],
            ));
        }
    };

    // NOTE(Gas): O(1) cost: a size-2 multi-scalar multiplication
    let pk = match libsecp256k1::recover(&msg, &sig, &rid) {
        Ok(pk) => pk,
        Err(_) => {
            return Ok(NativeResult::ok(
                gas_params.base_cost,
                smallvec![Value::vector_u8([0u8; 0]), Value::bool(false)],
            ));
        }
    };

    Ok(NativeResult::ok(
        gas_params.base_cost,
        smallvec![
            Value::vector_u8(pk.serialize()[1..].to_vec()),
            Value::bool(true)
        ],
    ))
}

pub fn make_native_secp256k1_recover(
    gas_params: Secp256k1ECDSARecoverGasParameters,
) -> NativeFunction {
    Arc::new(move |context, ty_args, args| {
        native_secp256k1_ecdsa_recover(&gas_params, context, ty_args, args)
    })
}

/***************************************************************************************************
 * native fun bls12381_verify_signature
 *
 *   gas cost: base_cost + unit_cost * message_size
 *
 **************************************************************************************************/
#[derive(Debug, Clone)]
pub struct Bls12381VerifySignatureGasParams {
    pub base_cost: u64,
    pub unit_cost: u64,
}

fn native_bls12381_verify_signature(
    gas_params: &Bls12381VerifySignatureGasParams,
    _context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    debug_assert!(_ty_args.is_empty());
    debug_assert!(arguments.len() == 3);

    let mut cost = gas_params.base_cost;

    let msg_bytes = pop_arg!(arguments, Vec<u8>);
    let pk_bytes = pop_arg!(arguments, Vec<u8>);
    let sig_bytes = pop_arg!(arguments, Vec<u8>);

    // NOTE(Gas): O(1) deserialization cost
    let pk = match bls12381::PublicKey::try_from(&pk_bytes[..]) {
        Ok(pk) => pk,
        Err(_) => {
            return Ok(NativeResult::ok(cost, smallvec![Value::bool(false)]));
        }
    };

    // NOTE(Gas): O(1) deserialization cost
    let sig = match bls12381::Signature::try_from(&sig_bytes[..]) {
        Ok(sig) => sig,
        Err(_) => {
            return Ok(NativeResult::ok(cost, smallvec![Value::bool(false)]));
        }
    };

    cost += gas_params.unit_cost * msg_bytes.len() as u64;

    // NOTE(Gas): O(1) cost: 2 bilinear pairings and a hash-to-curve
    let verify_result = sig.verify_arbitrary_msg(&msg_bytes[..], &pk).is_ok();

    Ok(NativeResult::ok(
        cost,
        smallvec![Value::bool(verify_result)],
    ))
}

pub fn make_native_bls12381_verify_signature(
    gas_params: Bls12381VerifySignatureGasParams,
) -> NativeFunction {
    Arc::new(move |context, ty_args, args| {
        native_bls12381_verify_signature(&gas_params, context, ty_args, args)
    })
}

/***************************************************************************************************
 * module
 *
 **************************************************************************************************/
#[derive(Debug, Clone)]
pub struct GasParameters {
    pub bls12381_validate_pubkey: Bls12381ValidatePubkeyGasParameters,
    pub ed25519_validate_pubkey: Ed25519ValidatePubkeyGasParameters,
    pub ed25519_verify: Ed25519VerifyGasParameters,
    pub secp256k1_ecdsa_recover: Secp256k1ECDSARecoverGasParameters,
    pub bls12381_verify_signature: Bls12381VerifySignatureGasParams,
    pub bls12381_aggregate_pop_verified_pubkeys: Bls12381AggregatePopVerifiedPubkeysGasParameters,
    pub bls12381_verify_proof_of_possession: Bls12381VerifyProofOfPosessionGasParameters,
}

pub fn make_all(gas_params: GasParameters) -> impl Iterator<Item = (String, NativeFunction)> {
    let natives = [
        (
            "bls12381_validate_pubkey",
            make_native_bls12381_validate_pubkey(gas_params.bls12381_validate_pubkey),
        ),
        (
            "ed25519_validate_pubkey",
            make_native_ed25519_validate_pubkey(gas_params.ed25519_validate_pubkey),
        ),
        (
            "ed25519_verify",
            make_native_ed25519_verify(gas_params.ed25519_verify),
        ),
        (
            "secp256k1_ecdsa_recover",
            make_native_secp256k1_recover(gas_params.secp256k1_ecdsa_recover),
        ),
        (
            "bls12381_verify_signature",
            make_native_bls12381_verify_signature(gas_params.bls12381_verify_signature),
        ),
        (
            "bls12381_aggregate_pop_verified_pubkeys",
            make_native_bls12381_aggregate_pop_verified_pubkeys(
                gas_params.bls12381_aggregate_pop_verified_pubkeys,
            ),
        ),
        (
            "bls12381_verify_proof_of_possession",
            make_native_bls12381_verify_proof_of_possession(
                gas_params.bls12381_verify_proof_of_possession,
            ),
        ),
    ];

    crate::natives::helpers::make_module_natives(natives)
}
