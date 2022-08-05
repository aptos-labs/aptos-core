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

/// Deserializes a vector of PK bytes into bls12381::PublicKey structs.
fn bls12381_deserialize_pks_helper(pks_serialized: Vec<Vec<u8>>) -> Vec<bls12381::PublicKey> {
    let mut pks = vec![];

    for pk_bytes in pks_serialized {
        // NOTE(Gas): O(1) deserialization cost
        let pk = match bls12381::PublicKey::try_from(&pk_bytes[..]) {
            Ok(key) => key,
            // If PK does not deserialize correctly, break early
            Err(_) => break,
        };

        pks.push(pk);
    }

    pks
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

    cost += gas_params.per_pubkey_deserialize_cost();
    let pk = match bls12381::PublicKey::try_from(&aggpk_bytes[..]) {
        Ok(pk) => pk,
        Err(_) => {
            return Ok(NativeResult::ok(cost, smallvec![Value::bool(false)]));
        }
    };

    if check_pk_subgroup {
        // NOTE(Gas): constant-time; around 39 microseconds on Apple M1
        cost += gas_params.per_pubkey_subgroup_check_cost();
        if pk.subgroup_check().is_err() {
            return Ok(NativeResult::ok(cost, smallvec![Value::bool(false)]));
        }
    }

    cost += gas_params.per_sig_deserialize_cost();
    let sig = match bls12381::Signature::try_from(&multisig_bytes[..]) {
        Ok(sig) => sig,
        Err(_) => {
            return Ok(NativeResult::ok(cost, smallvec![Value::bool(false)]));
        }
    };

    // NOTE(Gas): 2 bilinear pairings and a hash-to-curve
    cost = cost
        + gas_params.per_sig_verify_cost()
        + gas_params.per_msg_hashing_base_cost()
        + gas_params.per_msg_byte_hashing_cost() * msg_bytes.len() as u64;
    let verify_result = sig.verify_arbitrary_msg(&msg_bytes[..], &pk).is_ok();

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
    let pks = bls12381_deserialize_pks_helper(pks_serialized);
    assert!(pks.len() <= num_pks);

    // If not all PKs were successfully deserialized, return None and only charge for the actual work done
    let mut cost = gas_params.base_cost + gas_params.per_pubkey_deserialize_cost * pks.len() as u64;
    if pks.len() != num_pks {
        // The first `pks.len()` public keys deserialized correctly and then the next public key
        // failed deserialization. In this case, we must charge gas for the failed deserialization too.
        cost += gas_params.per_pubkey_deserialize_cost;

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

pub fn make_native<T: std::marker::Send + std::marker::Sync + 'static>(
    gas_params: T,
    func: fn(&T, &mut NativeContext, Vec<Type>, VecDeque<Value>) -> PartialVMResult<NativeResult>,
) -> NativeFunction {
    Arc::new(move |context, ty_args, args| func(&gas_params, context, ty_args, args))
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
    let mut sigs = vec![];

    let mut cost = gas_params.base_cost;

    for sig_bytes in sigs_serialized {
        // NOTE(Gas): O(1) deserialization cost per signature
        cost += gas_params.per_sig_deserialize_cost;

        let sig = match bls12381::Signature::try_from(&sig_bytes[..]) {
            Ok(sig) => sig,
            // If signature does not deserialize correctly, return None.
            Err(_) => return Ok(NativeResult::ok(cost, smallvec![none_option()])),
        };

        sigs.push(sig);
    }

    // If zero signatures were given as input, return None.
    if sigs.is_empty() {
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

    cost += gas_params.per_sig_deserialize_cost;
    let sig = match bls12381::Signature::try_from(&sig_bytes[..]) {
        Ok(key) => key,
        Err(_) => return Ok(NativeResult::ok(cost, smallvec![Value::bool(false)])),
    };

    // NOTE(Gas): O(1) cost; uses endomorphisms for performing faster subgroup checks
    cost += gas_params.per_sig_subgroup_check_cost;
    let valid = sig.subgroup_check().is_ok();

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

    cost += gas_params.per_pubkey_deserialize_cost;
    let public_key = match bls12381::PublicKey::try_from(&pk_bytes[..]) {
        Ok(key) => key,
        Err(_) => return Ok(NativeResult::ok(cost, smallvec![Value::bool(false)])),
    };

    // NOTE(Gas): constant cost: uses endomorphisms for performing faster subgroup checks
    cost += gas_params.per_pubkey_subgroup_check_cost;
    let valid = public_key.subgroup_check().is_ok();

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

    let pks = bls12381_deserialize_pks_helper(pks_serialized);

    // If less PKs than expected were deserialized, return None.
    cost += gas_params.per_pubkey_deserialize_cost * pks.len() as u64;
    if pks.len() != num_pks {
        // The first `pks.len()` public keys deserialized correctly and then the next public key
        // failed deserialization. In this case, we must charge gas for the failed deserialization too.
        cost += gas_params.per_pubkey_deserialize_cost;

        return Ok(NativeResult::ok(cost, smallvec![Value::bool(false)]));
    }

    cost += gas_params.per_sig_deserialize_cost;
    let aggsig = match bls12381::Signature::try_from(&aggsig_bytes[..]) {
        Ok(key) => key,
        Err(_) => return Ok(NativeResult::ok(cost, smallvec![Value::bool(false)])),
    };

    let msgs_refs = messages
        .iter()
        .map(|m| m.as_slice())
        .collect::<Vec<&[u8]>>();
    let pks_refs = pks.iter().collect::<Vec<&bls12381::PublicKey>>();

    // The cost of verifying a size-n aggregate signatures involves n+1 parings and hashing all
    // the messages to elliptic curve points (proportional to sum of all message lengths).
    cost = cost
        + gas_params.per_pairing_cost * (messages.len() + 1) as u64
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

    cost += gas_params.per_pubkey_deserialize_cost;
    let public_key = match bls12381::PublicKey::try_from(&key_bytes[..]) {
        Ok(key) => key,
        Err(_) => return Ok(NativeResult::ok(cost, smallvec![Value::bool(false)])),
    };

    cost += gas_params.per_sig_deserialize_cost;
    let pop = match bls12381::ProofOfPossession::try_from(&pop_bytes[..]) {
        Ok(pop) => pop,
        Err(_) => return Ok(NativeResult::ok(cost, smallvec![Value::bool(false)])),
    };

    // NOTE(Gas): 2 bilinear pairings and a hash-to-curve
    cost += gas_params.per_pop_verify_cost;
    let valid = pop.verify(&public_key).is_ok();

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

    // Ed25519 signatures
    pub ed25519_validate_pubkey: Ed25519ValidatePubkeyGasParameters,
    pub ed25519_verify: Ed25519VerifyGasParameters,

    // ECDSA signatures based on secp256k1 elliptic curves
    pub secp256k1_ecdsa_recover: Secp256k1ECDSARecoverGasParameters,
}

pub fn make_all(gas_params: GasParameters) -> impl Iterator<Item = (String, NativeFunction)> {
    let natives = [
        // BLS over BLS12-381
        (
            "bls12381_aggregate_pop_verified_pubkeys",
            make_native(
                gas_params.bls12381_aggregate_pop_verified_pubkeys,
                native_bls12381_aggregate_pop_verified_pubkeys,
            ),
        ),
        (
            "bls12381_aggregate_signatures",
            make_native(
                gas_params.bls12381_aggregate_signatures,
                native_bls12381_aggregate_signatures,
            ),
        ),
        (
            "bls12381_signature_subgroup_check",
            make_native(
                gas_params.bls12381_signature_subgroup_check,
                native_bls12381_signature_subgroup_check,
            ),
        ),
        (
            "bls12381_validate_pubkey",
            make_native(
                gas_params.bls12381_validate_pubkey,
                native_bls12381_validate_pubkey,
            ),
        ),
        (
            "bls12381_verify_aggregate_signature",
            make_native(
                gas_params.bls12381_verify_aggregate_signature,
                native_bls12381_verify_aggregate_signature,
            ),
        ),
        (
            "bls12381_verify_multisignature",
            make_native(
                gas_params.bls12381_verify_multisignature,
                native_bls12381_verify_multisignature,
            ),
        ),
        (
            "bls12381_verify_normal_signature",
            make_native(
                gas_params.bls12381_verify_normal_signature,
                native_bls12381_verify_normal_signature,
            ),
        ),
        (
            "bls12381_verify_proof_of_possession",
            make_native(
                gas_params.bls12381_verify_proof_of_possession,
                native_bls12381_verify_proof_of_possession,
            ),
        ),
        (
            "bls12381_verify_signature_share",
            make_native(
                gas_params.bls12381_verify_signature_share,
                native_bls12381_verify_signature_share,
            ),
        ),
        // Ed25519
        (
            "ed25519_validate_pubkey",
            make_native(
                gas_params.ed25519_validate_pubkey,
                native_ed25519_validate_pubkey,
            ),
        ),
        (
            "ed25519_verify",
            make_native(gas_params.ed25519_verify, native_ed25519_verify),
        ),
        // ECDSA over secp256k1
        (
            "secp256k1_ecdsa_recover",
            make_native(
                gas_params.secp256k1_ecdsa_recover,
                native_secp256k1_ecdsa_recover,
            ),
        ),
    ];

    crate::natives::helpers::make_module_natives(natives)
}
