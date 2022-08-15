// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{natives::util::make_native_from_func, pop_vec_arg};
use aptos_crypto::{bls12381, traits};
use move_deps::move_vm_types::values::Struct;
use move_deps::{
    move_binary_format::errors::PartialVMResult,
    move_vm_runtime::native_functions::{NativeContext, NativeFunction},
    move_vm_types::{
        loaded_data::runtime_types::Type, natives::function::NativeResult, pop_arg, values::Value,
    },
};
use smallvec::smallvec;
use std::{collections::VecDeque, convert::TryFrom};

/// TODO: remove these
/// Returns the equivalent of a Move std::option::none() natively in Rust.
fn none_option() -> Value {
    Value::struct_(Struct::pack(std::iter::once(
        Value::vector_for_testing_only(std::iter::empty()),
    )))
}

/// TODO: remove these
/// Returns the equivalent of a Move std::option<vector<u8>>::some(v) natively in Rust.
fn some_option(v: Vec<u8>) -> Value {
    let vv = Value::vector_u8(v.into_iter());
    Value::struct_(Struct::pack(std::iter::once(
        Value::vector_for_testing_only(std::iter::once(vv)),
    )))
}

/// Deserializes a vector of PK bytes into bls12381::PublicKey structs.
fn bls12381_deserialize_pks_with_gas(
    pks_serialized: Vec<Vec<u8>>,
    cost: &mut u64,
    per_pubkey_cost: u64,
) -> Vec<bls12381::PublicKey> {
    let mut pks = vec![];

    for pk_bytes in pks_serialized {
        *cost += per_pubkey_cost;

        let pk = match bls12381::PublicKey::try_from(&pk_bytes[..]) {
            Ok(key) => key,
            // If PK does not deserialize correctly, break early
            Err(_) => break,
        };

        pks.push(pk);
    }

    pks
}

/// Deserializes a sequence of bytes into bls12381::PublicKey struct.
fn bls12381_deserialize_pk_with_gas(
    pk_bytes: Vec<u8>,
    cost: &mut u64,
    per_pubkey_cost: u64,
) -> Option<bls12381::PublicKey> {
    *cost += per_pubkey_cost;

    match bls12381::PublicKey::try_from(&pk_bytes[..]) {
        Ok(key) => Some(key),
        // If PK does not deserialize correctly, break early
        Err(_) => None,
    }
}

/// Deserializes a vector of signature bytes into bls12381::Signature structs.
fn bls12381_deserialize_sigs_with_gas(
    sigs_serialized: Vec<Vec<u8>>,
    cost: &mut u64,
    per_sig_cost: u64,
) -> Vec<bls12381::Signature> {
    let mut sigs = vec![];

    for sig_bytes in sigs_serialized {
        *cost += per_sig_cost;

        let pk = match bls12381::Signature::try_from(&sig_bytes[..]) {
            Ok(sig) => sig,
            // If sig does not deserialize correctly, break early
            Err(_) => break,
        };

        sigs.push(pk);
    }

    sigs
}

/// Deserializes a sequence of bytes into bls12381::Signature struct.
fn bls12381_deserialize_sig_with_gas(
    sig_bytes: Vec<u8>,
    cost: &mut u64,
    per_sig_cost: u64,
) -> Option<bls12381::Signature> {
    *cost += per_sig_cost;

    match bls12381::Signature::try_from(&sig_bytes[..]) {
        Ok(sig) => Some(sig),
        // If PK does not deserialize correctly, break early
        Err(_) => None,
    }
}

/// Deserializes a sequence of bytes into bls12381::Signature struct.
fn bls12381_deserialize_pop_with_gas(
    pop_bytes: Vec<u8>,
    cost: &mut u64,
    per_pop_cost: u64,
) -> Option<bls12381::ProofOfPossession> {
    *cost += per_pop_cost;

    match bls12381::ProofOfPossession::try_from(&pop_bytes[..]) {
        Ok(pop) => Some(pop),
        // If PK does not deserialize correctly, break early
        Err(_) => None,
    }
}

/// Checks prime-order subgroup membership on a bls12381::PublicKey struct.
fn bls12381_pk_subgroub_check_with_gas(
    pk: &bls12381::PublicKey,
    cost: &mut u64,
    per_pk_cost: u64,
) -> bool {
    // NOTE(Gas): constant-time; around 39 microseconds on Apple M1
    *cost += per_pk_cost;
    pk.subgroup_check().is_ok()
}

/// Checks prime-order subgroup membership on a bls12381::Signature struct.
fn bls12381_sig_subgroub_check_with_gas(
    sig: &bls12381::Signature,
    cost: &mut u64,
    per_sig_cost: u64,
) -> bool {
    *cost += per_sig_cost;
    sig.subgroup_check().is_ok()
}

/// Verifies a signature on an arbitrary message.
fn signature_verify_with_gas<S: traits::Signature>(
    sig: &S,
    pk: &S::VerifyingKeyMaterial,
    msg: Vec<u8>,
    cost: &mut u64,
    sig_verify_cost: u64,
    msg_hashing_base_cost: u64,
    msg_hashing_per_byte_cost: u64,
) -> bool {
    *cost += sig_verify_cost + msg_hashing_base_cost + msg_hashing_per_byte_cost * msg.len() as u64;

    sig.verify_arbitrary_msg(&msg[..], pk).is_ok()
}

/// This trait defines gas costs for verifying:
///  * normal (non-aggregated) signatures,
///  * signature shares (in the multisignature scheme & the aggregate signature scheme)
///  * multisignatures
pub trait Bls12381VerifySignatureGasParametersTrait {
    fn base_cost(&self) -> u64;
    fn per_pubkey_deserialize_cost(&self) -> u64;
    fn per_pubkey_subgroup_check_cost(&self) -> u64;
    fn per_sig_deserialize_cost(&self) -> u64;
    fn per_sig_verify_cost(&self) -> u64;
    fn per_msg_hashing_base_cost(&self) -> u64;
    fn per_msg_byte_hashing_cost(&self) -> u64; // signature verification involves signing |msg| bytes
}

/// This is a helper function called by our `bls12381_verify_*` functions for:
///  * normal (non-aggregated) signatures,
///  * signature shares (in the multisignature scheme & the aggregate signature scheme)
///  * multisignatures
///
/// Gas cost: base_cost + per_pubkey_deserialize_cost
///                     +? ( per_pubkey_subgroup_check_cost * check_pk_subgroup
///                          +? ( per_sig_deserialize_cost
///                              +? ( per_sig_verify_cost + per_msg_hashing_base_cost
///                                   + per_msg_byte_hashing_cost * |msg| ) ) )
///
/// where +? indicates that the expression stops evaluating there if the previous gas-charging step
/// failed.
pub fn bls12381_verify_signature_helper<T: Bls12381VerifySignatureGasParametersTrait>(
    gas_params: &T,
    _context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
    check_pk_subgroup: bool,
) -> PartialVMResult<NativeResult> {
    debug_assert!(_ty_args.is_empty());
    debug_assert!(arguments.len() == 3);

    let mut cost = gas_params.base_cost();
    let msg_bytes = pop_arg!(arguments, Vec<u8>);
    let aggpk_bytes = pop_arg!(arguments, Vec<u8>);
    let multisig_bytes = pop_arg!(arguments, Vec<u8>);

    let pk = match bls12381_deserialize_pk_with_gas(
        aggpk_bytes,
        &mut cost,
        gas_params.per_pubkey_deserialize_cost(),
    ) {
        Some(pk) => pk,
        None => {
            return Ok(NativeResult::ok(cost, smallvec![Value::bool(false)]));
        }
    };

    if check_pk_subgroup
        && !bls12381_pk_subgroub_check_with_gas(
            &pk,
            &mut cost,
            gas_params.per_pubkey_subgroup_check_cost(),
        )
    {
        return Ok(NativeResult::ok(cost, smallvec![Value::bool(false)]));
    }

    let sig = match bls12381_deserialize_sig_with_gas(
        multisig_bytes,
        &mut cost,
        gas_params.per_sig_deserialize_cost(),
    ) {
        Some(sig) => sig,
        None => {
            return Ok(NativeResult::ok(cost, smallvec![Value::bool(false)]));
        }
    };

    // NOTE(Gas): 2 bilinear pairings and a hash-to-curve
    let verify_result = signature_verify_with_gas(
        &sig,
        &pk,
        msg_bytes,
        &mut cost,
        gas_params.per_sig_verify_cost(),
        gas_params.per_msg_hashing_base_cost(),
        gas_params.per_msg_byte_hashing_cost(),
    );

    Ok(NativeResult::ok(
        cost,
        smallvec![Value::bool(verify_result)],
    ))
}

/***************************************************************************************************
 * native fun bls12381_aggregate_pop_verified_pubkeys
 *
 *   gas cost: base_cost + per_pubkey_deserialize_cost * min(num_validatable_pubkeys + 1, num_pubkeys)
 *                       +? per_pubkey_aggregate_cost * num_pubkeys
 *
 * where +? indicates that the expression stops evaluating there if the previous gas-charging step
 * failed, num_pubkeys is the # of public keys given as input, and num_validatable_pubkeys is the
 * # of public keys that deserialize successfully.
 *
 * NOTE(ValidatablePK): We refer to the public keys that deserialize correctly as "validatable"
 * above, since successful deserialization is not a sufficient criteria for the "validatability"
 * of a PK: e.g., the PK could still be susceptible to small-subgroup attacks or rogue-key attacks.
 *
 * NOTE: If all PKs deserialize, then per_pubkey_deserialize_cost is charged num_pubkeys times.
 * Otherwise, if only num_validatable_pubkeys deserialize correctly, an extra per_pubkey_deserialize_cost
 * must be charged for the failed deserialization.
 **************************************************************************************************/
#[derive(Debug, Clone)]
pub struct Bls12381AggregatePopVerifiedPubkeysGasParameters {
    pub base_cost: u64,
    pub per_pubkey_deserialize_cost: u64,
    pub per_pubkey_aggregate_cost: u64,
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
    let num_pks = pks_serialized.len();
    let mut cost = gas_params.base_cost;

    // If zero PKs were given as input, return None.
    if pks_serialized.is_empty() {
        return Ok(NativeResult::ok(cost, smallvec![none_option()]));
    }

    let pks = bls12381_deserialize_pks_with_gas(
        pks_serialized,
        &mut cost,
        gas_params.per_pubkey_deserialize_cost,
    );
    debug_assert!(pks.len() <= num_pks);

    // If not all PKs were successfully deserialized, return None and only charge for the actual work done
    if pks.len() != num_pks {
        return Ok(NativeResult::ok(cost, smallvec![none_option()]));
    }

    // Aggregate the public keys (this will NOT subgroup-check the individual PKs)
    // NOTE(Gas): |pks| elliptic curve additions
    cost += gas_params.per_pubkey_aggregate_cost * num_pks as u64;
    let aggpk =
        match bls12381::PublicKey::aggregate(pks.iter().collect::<Vec<&bls12381::PublicKey>>()) {
            Ok(aggpk) => aggpk,
            Err(_) => return Ok(NativeResult::ok(cost, smallvec![none_option()])),
        };

    Ok(NativeResult::ok(
        cost,
        smallvec![some_option(aggpk.to_bytes().to_vec())],
    ))
}

/***************************************************************************************************
 * native fun bls12381_aggregate_signatures
 *
 *   gas cost: base_cost + num_viable_sigs * per_sig_deserialize_cost
 *                       +? num_sigs * per_sig_aggregate_cost
 *
 * where +? indicates that the expression stops evaluating there if the previous gas-charging step
 * failed
 *
 * NOTE(ViableSig): We refer to the signatures that deserialize correctly as "viable" above, since
 * successful deserialization is not a sufficient criteria for the "viability" of a signature: e.g.,
 * the signature might not verify under the desired (message, public key) pair, or the signature
 * could lie in a small-order subgroup.
 **************************************************************************************************/
#[derive(Debug, Clone)]
pub struct Bls12381AggregateSignaturesGasParameters {
    pub base_cost: u64,
    pub per_sig_deserialize_cost: u64,
    pub per_sig_aggregate_cost: u64,
}

pub fn native_bls12381_aggregate_signatures(
    gas_params: &Bls12381AggregateSignaturesGasParameters,
    _context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    debug_assert!(_ty_args.is_empty());
    debug_assert!(arguments.len() == 1);

    // Parses a Vec<Vec<u8>> of all serialized signatures
    let sigs_serialized = pop_vec_arg!(arguments, Vec<u8>);
    let num_sigs = sigs_serialized.len();

    let mut cost = gas_params.base_cost;

    // If zero signatures were given as input, return None.
    if sigs_serialized.is_empty() {
        return Ok(NativeResult::ok(cost, smallvec![none_option()]));
    }

    let sigs = bls12381_deserialize_sigs_with_gas(
        sigs_serialized,
        &mut cost,
        gas_params.per_sig_deserialize_cost,
    );

    if sigs.len() <= num_sigs {
        return Ok(NativeResult::ok(cost, smallvec![none_option()]));
    }

    // Aggregate the signatures (this will NOT group-check the individual signatures)
    // NOTE(Gas): |sigs| elliptic curve additions
    cost += gas_params.per_sig_aggregate_cost * sigs.len() as u64;
    let aggsig = match bls12381::Signature::aggregate(sigs) {
        Ok(aggsig) => aggsig,
        Err(_) => return Ok(NativeResult::ok(cost, smallvec![none_option()])),
    };

    Ok(NativeResult::ok(
        cost,
        smallvec![some_option(aggsig.to_bytes().to_vec())],
    ))
}

/***************************************************************************************************
 * native fun bls12381_signature_subgroup_check
 *
 *   gas cost: base_cost + per_sig_deserialize_cost +? per_sig_subgroup_check_cost
 *
 * where +? indicates that the expression stops evaluating there if the previous gas-charging step
 * failed
 **************************************************************************************************/
#[derive(Debug, Clone)]
pub struct Bls12381SignatureSubgroupCheckGasParameters {
    pub base_cost: u64,
    pub per_sig_deserialize_cost: u64,
    pub per_sig_subgroup_check_cost: u64,
}

pub fn native_bls12381_signature_subgroup_check(
    gas_params: &Bls12381SignatureSubgroupCheckGasParameters,
    _context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    debug_assert!(_ty_args.is_empty());
    debug_assert!(arguments.len() == 1);

    let mut cost = gas_params.base_cost;

    let sig_bytes = pop_arg!(arguments, Vec<u8>);

    let sig = match bls12381_deserialize_sig_with_gas(
        sig_bytes,
        &mut cost,
        gas_params.per_sig_deserialize_cost,
    ) {
        Some(key) => key,
        None => return Ok(NativeResult::ok(cost, smallvec![Value::bool(false)])),
    };

    let valid = bls12381_sig_subgroub_check_with_gas(
        &sig,
        &mut cost,
        gas_params.per_sig_subgroup_check_cost,
    );

    Ok(NativeResult::ok(cost, smallvec![Value::bool(valid)]))
}

/***************************************************************************************************
 * native fun bls12381_validate_pubkey
 *
 *   gas cost: base_cost + per_pubkey_deserialize_cost +? per_pubkey_subgroup_check_cost
 *
 * where +? indicates that the expression stops evaluating there if the previous gas-charging step
 * failed
 **************************************************************************************************/
#[derive(Debug, Clone)]
pub struct Bls12381ValidatePubkeyGasParameters {
    pub base_cost: u64,
    pub per_pubkey_deserialize_cost: u64,
    pub per_pubkey_subgroup_check_cost: u64,
}

fn native_bls12381_validate_pubkey(
    gas_params: &Bls12381ValidatePubkeyGasParameters,
    _context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    debug_assert!(_ty_args.is_empty());
    debug_assert!(arguments.len() == 1);

    let mut cost = gas_params.base_cost;
    let pk_bytes = pop_arg!(arguments, Vec<u8>);

    let pk = match bls12381_deserialize_pk_with_gas(
        pk_bytes,
        &mut cost,
        gas_params.per_pubkey_deserialize_cost,
    ) {
        Some(key) => key,
        None => return Ok(NativeResult::ok(cost, smallvec![Value::bool(false)])),
    };

    let valid = bls12381_pk_subgroub_check_with_gas(
        &pk,
        &mut cost,
        gas_params.per_pubkey_subgroup_check_cost,
    );

    Ok(NativeResult::ok(cost, smallvec![Value::bool(valid)]))
}

/***************************************************************************************************
* native fun native_bls12381_verify_aggregate_signature
*
*   gas cost: base_cost + per_pubkey_deserialize_cost * min(num_validatable_pubkeys + 1, num_pubkeys)
*                       +? ( per_sig_deserialize_cost * min(num_viable_sigs + 1, num_sigs)
*                            +? ( per_pairing_cost + per_msg_hashing_base_cost ) * num_msgs
*                                 + per_msg_byte_hashing_cost * total_msg_bytes )
*
* where:
*    +? indicates the expression stops evaluating there if the previous gas-charging step failed,
*    num_pubkeys is the # of public keys given as input,
*    num_validatable_pubkeys is the # of public keys that deserialize successfully (i.e., "validatable"),
*    num_sigs is the # of signatures given as input,
*    num_viable_sigs is the # of signatures that deserialize successfully (i.e., "viable"),
*    total_msg_bytes is the cumulative size in bytes of all messages.
*
* NOTE(ValidatablePK): We refer to the public keys that deserialize correctly as "validatable"
* above, since successful deserialization is not a sufficient criteria for the "validatability"
* of a PK: e.g., the PK could still be susceptible to small-subgroup attacks or rogue-key attacks.
*
* NOTE(ViableSig): We refer to the signatures that deserialize correctly as "viable" above, since
* successful deserialization is not a sufficient criteria for the "viability" of a signature: e.g.,
* the signature might not verify under the desired (message, public key) pair, or the signature
* could lie in a small-order subgroup.

* NOTE: If all PKs deserialize, then per_pubkey_deserialize_cost is charged num_pubkeys times.
* Otherwise, if only num_validatable_pubkeys deserialize correctly, an extra per_pubkey_deserialize_cost
* must be charged for the failed deserialization. We proceed similarly for per_sig_deserialize_cost.
**************************************************************************************************/
#[derive(Debug, Clone)]
pub struct Bls12381VerifyAggregateSignatureGasParameters {
    pub base_cost: u64,
    pub per_pubkey_deserialize_cost: u64,
    pub per_sig_deserialize_cost: u64,
    pub per_pairing_cost: u64, // a size-n BLS aggregate signature requires n+1 pairings
    pub per_msg_hashing_base_cost: u64,
    pub per_msg_byte_hashing_cost: u64,
}
pub fn native_bls12381_verify_aggregate_signature(
    gas_params: &Bls12381VerifyAggregateSignatureGasParameters,
    _context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    debug_assert!(_ty_args.is_empty());
    debug_assert!(arguments.len() == 3);

    let mut cost = gas_params.base_cost;

    // Parses a Vec<Vec<u8>> of all messages
    let messages = pop_vec_arg!(arguments, Vec<u8>);
    // Parses a Vec<Vec<u8>> of all serialized public keys
    let pks_serialized = pop_vec_arg!(arguments, Vec<u8>);
    let num_pks = pks_serialized.len();

    // Parses the signature as a Vec<u8>
    let aggsig_bytes = pop_arg!(arguments, Vec<u8>);

    // Number of messages must match number of public keys
    if pks_serialized.len() != messages.len() {
        return Ok(NativeResult::ok(cost, smallvec![Value::bool(false)]));
    }

    let pks = bls12381_deserialize_pks_with_gas(
        pks_serialized,
        &mut cost,
        gas_params.per_pubkey_deserialize_cost,
    );
    debug_assert!(pks.len() <= num_pks);

    // If less PKs than expected were deserialized, return None.
    if pks.len() != num_pks {
        return Ok(NativeResult::ok(cost, smallvec![Value::bool(false)]));
    }

    let aggsig = match bls12381_deserialize_sig_with_gas(
        aggsig_bytes,
        &mut cost,
        gas_params.per_sig_deserialize_cost,
    ) {
        Some(aggsig) => aggsig,
        None => return Ok(NativeResult::ok(cost, smallvec![Value::bool(false)])),
    };

    let msgs_refs = messages
        .iter()
        .map(|m| m.as_slice())
        .collect::<Vec<&[u8]>>();
    let pks_refs = pks.iter().collect::<Vec<&bls12381::PublicKey>>();

    // The cost of verifying a size-n aggregate signatures involves n+1 parings and hashing all
    // the messages to elliptic curve points (proportional to sum of all message lengths).
    cost += gas_params.per_pairing_cost * (messages.len() + 1) as u64
        + gas_params.per_msg_hashing_base_cost * messages.len() as u64
        + messages.iter().fold(0, |sum, msg| sum + msg.len() as u64);

    let verify_result = aggsig
        .verify_aggregate_arbitrary_msg(&msgs_refs, &pks_refs)
        .is_ok();

    Ok(NativeResult::ok(
        cost,
        smallvec![Value::bool(verify_result)],
    ))
}

/***************************************************************************************************
 * native fun bls12381_verify_multisignature
 *
 *   gas cost: base_cost + per_pubkey_deserialize_cost
 *                       +? ( per_sig_deserialize_cost
 *                            +? ( per_sig_verify_cost + per_msg_hashing_base_cost
 *                                 + per_msg_byte_hashing_cost * |msg| ) )
 *
 * where +? indicates that the expression stops evaluating there if the previous gas-charging step
 * failed
 **************************************************************************************************/
#[derive(Debug, Clone)]
pub struct Bls12381VerifyMultisignatureGasParameters {
    pub base_cost: u64,
    pub per_pubkey_deserialize_cost: u64,
    pub per_pubkey_subgroup_check_cost: u64,
    pub per_sig_deserialize_cost: u64,
    pub per_sig_verify_cost: u64,
    pub per_msg_hashing_base_cost: u64,
    pub per_msg_byte_hashing_cost: u64, // signature verification involves signing |msg| bytes
}

impl Bls12381VerifySignatureGasParametersTrait for Bls12381VerifyMultisignatureGasParameters {
    fn base_cost(&self) -> u64 {
        self.base_cost
    }

    fn per_pubkey_deserialize_cost(&self) -> u64 {
        self.per_pubkey_deserialize_cost
    }

    fn per_pubkey_subgroup_check_cost(&self) -> u64 {
        self.per_pubkey_subgroup_check_cost
    }

    fn per_sig_deserialize_cost(&self) -> u64 {
        self.per_sig_deserialize_cost
    }

    fn per_sig_verify_cost(&self) -> u64 {
        self.per_sig_verify_cost
    }

    fn per_msg_hashing_base_cost(&self) -> u64 {
        self.per_msg_hashing_base_cost
    }

    fn per_msg_byte_hashing_cost(&self) -> u64 {
        self.per_msg_byte_hashing_cost
    }
}

pub fn native_bls12381_verify_multisignature(
    gas_params: &Bls12381VerifyMultisignatureGasParameters,
    _context: &mut NativeContext,
    _ty_args: Vec<Type>,
    arguments: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    let check_pk_subgroup = false;
    bls12381_verify_signature_helper(gas_params, _context, _ty_args, arguments, check_pk_subgroup)
}

/***************************************************************************************************
 * native fun bls12381_verify_normal_signature
 *
 *   gas cost: base_cost + per_pubkey_deserialize_cost
 *                       +? ( per_sig_deserialize_cost
 *                            +? ( per_pubkey_subgroup_check_cost
 *                                 +? ( per_sig_verify_cost + per_msg_hashing_base_cost
 *                                     + per_msg_byte_hashing_cost * |msg| ) ) )
 *
 * where +? indicates that the expression stops evaluating there if the previous gas-charging step
 * failed
 **************************************************************************************************/
#[derive(Debug, Clone)]
pub struct Bls12381VerifyNormalSignatureGasParameters {
    pub base_cost: u64,
    pub per_pubkey_deserialize_cost: u64,
    pub per_pubkey_subgroup_check_cost: u64,
    pub per_sig_deserialize_cost: u64,
    pub per_sig_verify_cost: u64,
    pub per_msg_hashing_base_cost: u64,
    pub per_msg_byte_hashing_cost: u64, // signature verification involves signing |msg| bytes
}

impl Bls12381VerifySignatureGasParametersTrait for Bls12381VerifyNormalSignatureGasParameters {
    fn base_cost(&self) -> u64 {
        self.base_cost
    }

    fn per_pubkey_deserialize_cost(&self) -> u64 {
        self.per_pubkey_deserialize_cost
    }

    fn per_pubkey_subgroup_check_cost(&self) -> u64 {
        self.per_pubkey_subgroup_check_cost
    }

    fn per_sig_deserialize_cost(&self) -> u64 {
        self.per_sig_deserialize_cost
    }

    fn per_sig_verify_cost(&self) -> u64 {
        self.per_sig_verify_cost
    }

    fn per_msg_hashing_base_cost(&self) -> u64 {
        self.per_msg_hashing_base_cost
    }

    fn per_msg_byte_hashing_cost(&self) -> u64 {
        self.per_msg_byte_hashing_cost
    }
}

pub fn native_bls12381_verify_normal_signature(
    gas_params: &Bls12381VerifyNormalSignatureGasParameters,
    _context: &mut NativeContext,
    _ty_args: Vec<Type>,
    arguments: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    // For normal (non-aggregated) signatures, PK's typically don't come with PoPs and the caller
    // might forget to check prime-order subgroup membership of the PK. Therefore, we always enforce
    // it here.
    let check_pk_subgroup = true;
    bls12381_verify_signature_helper(gas_params, _context, _ty_args, arguments, check_pk_subgroup)
}

/***************************************************************************************************
 * native fun bls12381_verify_proof_of_possession
 *
 *   gas cost: base_cost + per_pubkey_deserialize_cost
 *                       +? ( per_sig_deserialize_cost
 *                            +? per_pop_verify_cost )
 *
 * where +? indicates that the expression stops evaluating there if the previous gas-charging step
 * failed
 **************************************************************************************************/
#[derive(Debug, Clone)]
pub struct Bls12381VerifyProofOfPosessionGasParameters {
    pub base_cost: u64,
    pub per_pubkey_deserialize_cost: u64,
    pub per_sig_deserialize_cost: u64,
    pub per_pop_verify_cost: u64,
}

fn native_bls12381_verify_proof_of_possession(
    gas_params: &Bls12381VerifyProofOfPosessionGasParameters,
    _context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    debug_assert!(_ty_args.is_empty());
    debug_assert!(arguments.len() == 2);

    let mut cost = gas_params.base_cost;
    let pop_bytes = pop_arg!(arguments, Vec<u8>);
    let key_bytes = pop_arg!(arguments, Vec<u8>);

    let pk = match bls12381_deserialize_pk_with_gas(
        key_bytes,
        &mut cost,
        gas_params.per_pubkey_deserialize_cost,
    ) {
        Some(pk) => pk,
        None => return Ok(NativeResult::ok(cost, smallvec![Value::bool(false)])),
    };

    let pop = match bls12381_deserialize_pop_with_gas(
        pop_bytes,
        &mut cost,
        gas_params.per_sig_deserialize_cost,
    ) {
        Some(pop) => pop,
        None => return Ok(NativeResult::ok(cost, smallvec![Value::bool(false)])),
    };

    // NOTE(Gas): 2 bilinear pairings and a hash-to-curve
    cost += gas_params.per_pop_verify_cost;
    let valid = pop.verify(&pk).is_ok();

    Ok(NativeResult::ok(cost, smallvec![Value::bool(valid)]))
}

/***************************************************************************************************
 * native fun bls12381_verify_signature_share
 *
 *   gas cost: base_cost + per_pubkey_deserialize_cost
 *                       +? ( per_sig_deserialize_cost
 *                            +? ( per_sig_verify_cost + per_msg_hashing_base_cost
 *                                 + per_msg_byte_hashing_cost * |msg| ) )
 *
 * where +? indicates that the expression stops evaluating there if the previous gas-charging step
 * failed
 **************************************************************************************************/
#[derive(Debug, Clone)]
pub struct Bls12381VerifySignatureShareGasParameters {
    pub base_cost: u64,
    pub per_pubkey_deserialize_cost: u64,
    pub per_pubkey_subgroup_check_cost: u64,
    pub per_sig_deserialize_cost: u64,
    pub per_sig_verify_cost: u64,
    pub per_msg_hashing_base_cost: u64,
    pub per_msg_byte_hashing_cost: u64, // signature verification involves signing |msg| bytes
}

impl Bls12381VerifySignatureGasParametersTrait for Bls12381VerifySignatureShareGasParameters {
    fn base_cost(&self) -> u64 {
        self.base_cost
    }

    fn per_pubkey_deserialize_cost(&self) -> u64 {
        self.per_pubkey_deserialize_cost
    }

    fn per_pubkey_subgroup_check_cost(&self) -> u64 {
        self.per_pubkey_subgroup_check_cost
    }

    fn per_sig_deserialize_cost(&self) -> u64 {
        self.per_sig_deserialize_cost
    }

    fn per_sig_verify_cost(&self) -> u64 {
        self.per_sig_verify_cost
    }

    fn per_msg_hashing_base_cost(&self) -> u64 {
        self.per_msg_hashing_base_cost
    }

    fn per_msg_byte_hashing_cost(&self) -> u64 {
        self.per_msg_byte_hashing_cost
    }
}

pub fn native_bls12381_verify_signature_share(
    gas_params: &Bls12381VerifySignatureShareGasParameters,
    _context: &mut NativeContext,
    _ty_args: Vec<Type>,
    arguments: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    // For signature shares, the caller is REQUIRED to check the PK's PoP, and thus the PK is in the
    // prime-order subgroup.
    let check_pk_subgroup = false;
    bls12381_verify_signature_helper(gas_params, _context, _ty_args, arguments, check_pk_subgroup)
}

/***************************************************************************************************
 * module
 *
 **************************************************************************************************/
#[derive(Debug, Clone)]
pub struct GasParameters {
    // BLS signatures based on BLS12-381 elliptic curves
    pub bls12381_aggregate_pop_verified_pubkeys: Bls12381AggregatePopVerifiedPubkeysGasParameters,
    pub bls12381_aggregate_signatures: Bls12381AggregateSignaturesGasParameters,
    pub bls12381_signature_subgroup_check: Bls12381SignatureSubgroupCheckGasParameters,
    pub bls12381_validate_pubkey: Bls12381ValidatePubkeyGasParameters,
    pub bls12381_verify_aggregate_signature: Bls12381VerifyAggregateSignatureGasParameters,
    pub bls12381_verify_multisignature: Bls12381VerifyMultisignatureGasParameters,
    pub bls12381_verify_normal_signature: Bls12381VerifyNormalSignatureGasParameters,
    pub bls12381_verify_proof_of_possession: Bls12381VerifyProofOfPosessionGasParameters,
    pub bls12381_verify_signature_share: Bls12381VerifySignatureShareGasParameters,
}

pub fn make_all(gas_params: GasParameters) -> impl Iterator<Item = (String, NativeFunction)> {
    let natives = [
        // BLS over BLS12-381
        (
            "aggregate_pop_verified_pubkeys",
            make_native_from_func(
                gas_params.bls12381_aggregate_pop_verified_pubkeys,
                native_bls12381_aggregate_pop_verified_pubkeys,
            ),
        ),
        (
            "aggregate_signatures",
            make_native_from_func(
                gas_params.bls12381_aggregate_signatures,
                native_bls12381_aggregate_signatures,
            ),
        ),
        (
            "signature_subgroup_check",
            make_native_from_func(
                gas_params.bls12381_signature_subgroup_check,
                native_bls12381_signature_subgroup_check,
            ),
        ),
        (
            "validate_pubkey",
            make_native_from_func(
                gas_params.bls12381_validate_pubkey,
                native_bls12381_validate_pubkey,
            ),
        ),
        (
            "verify_aggregate_signature",
            make_native_from_func(
                gas_params.bls12381_verify_aggregate_signature,
                native_bls12381_verify_aggregate_signature,
            ),
        ),
        (
            "verify_multisignature",
            make_native_from_func(
                gas_params.bls12381_verify_multisignature,
                native_bls12381_verify_multisignature,
            ),
        ),
        (
            "verify_normal_signature",
            make_native_from_func(
                gas_params.bls12381_verify_normal_signature,
                native_bls12381_verify_normal_signature,
            ),
        ),
        (
            "verify_proof_of_possession",
            make_native_from_func(
                gas_params.bls12381_verify_proof_of_possession,
                native_bls12381_verify_proof_of_possession,
            ),
        ),
        (
            "verify_signature_share",
            make_native_from_func(
                gas_params.bls12381_verify_signature_share,
                native_bls12381_verify_signature_share,
            ),
        ),
    ];

    crate::natives::helpers::make_module_natives(natives)
}
