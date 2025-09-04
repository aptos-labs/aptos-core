// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use velor_crypto::{bls12381, traits};
#[cfg(feature = "testing")]
use velor_crypto::{
    bls12381::{PrivateKey, ProofOfPossession, PublicKey},
    test_utils::KeyPair,
    SigningKey, Uniform,
};
use velor_gas_schedule::gas_params::natives::velor_framework::*;
use velor_native_interface::{
    safely_pop_arg, safely_pop_vec_arg, RawSafeNative, SafeNativeBuilder, SafeNativeContext,
    SafeNativeResult,
};
use move_binary_format::errors::PartialVMError;
use move_core_types::{
    gas_algebra::{InternalGas, InternalGasPerArg, InternalGasPerByte, NumArgs, NumBytes},
    vm_status::StatusCode,
};
use move_vm_runtime::native_functions::NativeFunction;
use move_vm_types::{
    loaded_data::runtime_types::Type,
    values::{Struct, Value},
};
#[cfg(feature = "testing")]
use rand_core::OsRng;
use smallvec::{smallvec, SmallVec};
use std::{collections::VecDeque, convert::TryFrom};

/// Pops a `Vec<T>` off the argument stack and converts it to a `Vec<Vec<u8>>` by reading the first
/// field of `T`, which is a `Vec<u8>` field named `bytes`.
fn pop_as_vec_of_vec_u8(arguments: &mut VecDeque<Value>) -> SafeNativeResult<Vec<Vec<u8>>> {
    let structs = safely_pop_vec_arg!(arguments, Struct);
    let mut v = Vec::with_capacity(structs.len());

    for s in structs {
        let field = s
            .unpack()?
            .next()
            .ok_or_else(|| PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR))?;

        v.push(field.value_as::<Vec<u8>>()?);
    }

    Ok(v)
}

#[derive(Debug, Clone)]
pub struct GasParameters {
    pub base: InternalGas,

    pub per_pubkey_deserialize: InternalGasPerArg,
    pub per_pubkey_aggregate: InternalGasPerArg,
    pub per_pubkey_subgroup_check: InternalGasPerArg,

    pub per_sig_deserialize: InternalGasPerArg,
    pub per_sig_aggregate: InternalGasPerArg,
    pub per_sig_subgroup_check: InternalGasPerArg,

    pub per_sig_verify: InternalGasPerArg,
    pub per_pop_verify: InternalGasPerArg,

    pub per_pairing: InternalGasPerArg, // a size-n BLS aggregate signature requires n+1 pairings

    pub per_msg_hashing: InternalGasPerArg,
    pub per_byte_hashing: InternalGasPerByte, // signature verification involves signing |msg| bytes
}

/// Deserializes a vector of PK bytes into bls12381::PublicKey structs.
fn bls12381_deserialize_pks(
    pks_serialized: Vec<Vec<u8>>,
    context: &mut SafeNativeContext,
) -> SafeNativeResult<Vec<bls12381::PublicKey>> {
    let mut pks = vec![];

    for pk_bytes in pks_serialized {
        let pk = match bls12381_deserialize_pk(pk_bytes, context)? {
            Some(key) => key,
            // If PK does not deserialize correctly, break early
            None => break,
        };

        pks.push(pk);
    }

    Ok(pks)
}

/// Deserializes a sequence of bytes into bls12381::PublicKey struct.
fn bls12381_deserialize_pk(
    pk_bytes: Vec<u8>,
    context: &mut SafeNativeContext,
) -> SafeNativeResult<Option<bls12381::PublicKey>> {
    context.charge(BLS12381_PER_PUBKEY_DESERIALIZE * NumArgs::one())?;

    match bls12381::PublicKey::try_from(&pk_bytes[..]) {
        Ok(key) => Ok(Some(key)),
        // If PK does not deserialize correctly, return None
        Err(_) => Ok(None),
    }
}

/// Deserializes a vector of signature bytes into bls12381::Signature structs.
fn bls12381_deserialize_sigs(
    sigs_serialized: Vec<Vec<u8>>,
    context: &mut SafeNativeContext,
) -> SafeNativeResult<Vec<bls12381::Signature>> {
    let mut sigs = vec![];

    for sig_bytes in sigs_serialized {
        let sig = match bls12381_deserialize_sig(sig_bytes, context)? {
            Some(sig) => sig,
            // If sig does not deserialize correctly, break early
            None => break,
        };

        sigs.push(sig);
    }

    Ok(sigs)
}

/// Deserializes a sequence of bytes into bls12381::Signature struct.
fn bls12381_deserialize_sig(
    sig_bytes: Vec<u8>,
    context: &mut SafeNativeContext,
) -> SafeNativeResult<Option<bls12381::Signature>> {
    context.charge(BLS12381_PER_SIG_DESERIALIZE * NumArgs::one())?;

    match bls12381::Signature::try_from(&sig_bytes[..]) {
        Ok(sig) => Ok(Some(sig)),
        // If PK does not deserialize correctly, return None
        Err(_) => Ok(None),
    }
}

/// Deserializes a sequence of bytes into bls12381::Signature struct.
fn bls12381_deserialize_pop(
    pop_bytes: Vec<u8>,
    context: &mut SafeNativeContext,
) -> SafeNativeResult<Option<bls12381::ProofOfPossession>> {
    context.charge(BLS12381_PER_SIG_DESERIALIZE * NumArgs::one())?;

    match bls12381::ProofOfPossession::try_from(&pop_bytes[..]) {
        Ok(pop) => Ok(Some(pop)),
        // If PK does not deserialize correctly, break early
        Err(_) => Ok(None),
    }
}

/// Checks prime-order subgroup membership on a bls12381::PublicKey struct.
fn bls12381_pk_subgroub_check(
    pk: &bls12381::PublicKey,
    context: &mut SafeNativeContext,
) -> SafeNativeResult<bool> {
    // NOTE(Gas): constant-time; around 39 microseconds on Apple M1
    context.charge(BLS12381_PER_PUBKEY_DESERIALIZE * NumArgs::one())?;

    Ok(pk.subgroup_check().is_ok())
}

/// Checks prime-order subgroup membership on a bls12381::Signature struct.
fn bls12381_sig_subgroub_check(
    sig: &bls12381::Signature,
    context: &mut SafeNativeContext,
) -> SafeNativeResult<bool> {
    context.charge(BLS12381_PER_SIG_SUBGROUP_CHECK * NumArgs::one())?;

    Ok(sig.subgroup_check().is_ok())
}

/// Verifies a signature on an arbitrary message.
fn signature_verify<S: traits::Signature>(
    sig: &S,
    pk: &S::VerifyingKeyMaterial,
    msg: Vec<u8>,
    context: &mut SafeNativeContext,
) -> SafeNativeResult<bool> {
    // NOTE(Gas): 2 bilinear pairings and a hash-to-curve
    context.charge(
        BLS12381_PER_SIG_VERIFY * NumArgs::one()
            + BLS12381_PER_MSG_HASHING * NumArgs::one()
            + BLS12381_PER_BYTE_HASHING * NumBytes::new(msg.len() as u64),
    )?;

    Ok(sig.verify_arbitrary_msg(&msg[..], pk).is_ok())
}

/// This is a helper function called by our `bls12381_verify_*` functions for:
///  * normal (non-aggregated) signatures,
///  * signature shares (in the multisignature scheme & the aggregate signature scheme)
///  * multisignatures
///
/// Gas cost: base_cost + per_pubkey_deserialize_cost
///                     +? ( per_pubkey_subgroup_check_cost * check_pk_subgroup
///                          +? ( per_sig_deserialize_cost
///                              +? ( per_sig_verify_cost + per_msg_hashing_cost
///                                   + per_byte_hashing_cost * |msg| ) ) )
///
/// where +? indicates that the expression stops evaluating there if the previous gas-charging step
/// failed.
pub fn bls12381_verify_signature_helper(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
    check_pk_subgroup: bool,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert!(_ty_args.is_empty());
    debug_assert!(arguments.len() == 3);

    context.charge(BLS12381_BASE)?;

    let msg_bytes = safely_pop_arg!(arguments, Vec<u8>);
    let aggpk_bytes = safely_pop_arg!(arguments, Vec<u8>);
    let multisig_bytes = safely_pop_arg!(arguments, Vec<u8>);

    let pk = match bls12381_deserialize_pk(aggpk_bytes, context)? {
        Some(pk) => pk,
        None => {
            return Ok(smallvec![Value::bool(false)]);
        },
    };

    if check_pk_subgroup && !bls12381_pk_subgroub_check(&pk, context)? {
        return Ok(smallvec![Value::bool(false)]);
    }

    let sig = match bls12381_deserialize_sig(multisig_bytes, context)? {
        Some(sig) => sig,
        None => {
            return Ok(smallvec![Value::bool(false)]);
        },
    };

    let verify_result = signature_verify(&sig, &pk, msg_bytes, context)?;

    Ok(smallvec![Value::bool(verify_result)])
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
fn native_bls12381_aggregate_pubkeys(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert!(_ty_args.is_empty());
    debug_assert!(arguments.len() == 1);

    // Parses a Vec<Vec<u8>> of all serialized public keys
    let pks_bytes = pop_as_vec_of_vec_u8(&mut arguments)?;
    let num_pks = pks_bytes.len();

    context.charge(BLS12381_BASE)?;

    // If zero PKs were given as input, return None.
    if pks_bytes.is_empty() {
        return Ok(smallvec![Value::vector_u8(vec![]), Value::bool(false)]);
    }

    let pks = bls12381_deserialize_pks(pks_bytes, context)?;
    debug_assert!(pks.len() <= num_pks);

    // If not all PKs were successfully deserialized, return None and only charge for the actual work done
    if pks.len() != num_pks {
        return Ok(smallvec![Value::vector_u8(vec![]), Value::bool(false)]);
    }

    // Aggregate the public keys (this will NOT subgroup-check the individual PKs)
    // NOTE(Gas): |pks| elliptic curve additions
    context.charge(BLS12381_PER_PUBKEY_AGGREGATE * NumArgs::new(num_pks as u64))?;
    let aggpk =
        match bls12381::PublicKey::aggregate(pks.iter().collect::<Vec<&bls12381::PublicKey>>()) {
            Ok(aggpk) => aggpk,
            Err(_) => return Ok(smallvec![Value::vector_u8(vec![]), Value::bool(false)]),
        };

    Ok(smallvec![
        Value::vector_u8(aggpk.to_bytes().to_vec()),
        Value::bool(true)
    ])
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
pub fn native_bls12381_aggregate_signatures(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert!(_ty_args.is_empty());
    debug_assert!(arguments.len() == 1);

    // Parses a Vec<Vec<u8>> of all serialized signatures
    let sigs_serialized = pop_as_vec_of_vec_u8(&mut arguments)?;
    let num_sigs = sigs_serialized.len();

    context.charge(BLS12381_BASE)?;

    // If zero signatures were given as input, return None.
    if sigs_serialized.is_empty() {
        return Ok(smallvec![Value::vector_u8(vec![]), Value::bool(false)]);
    }

    let sigs = bls12381_deserialize_sigs(sigs_serialized, context)?;

    if sigs.len() != num_sigs {
        return Ok(smallvec![Value::vector_u8(vec![]), Value::bool(false)]);
    }

    // Aggregate the signatures (this will NOT group-check the individual signatures)
    // NOTE(Gas): |sigs| elliptic curve additions
    context.charge(BLS12381_PER_SIG_AGGREGATE * NumArgs::new(sigs.len() as u64))?;
    let aggsig = match bls12381::Signature::aggregate(sigs) {
        Ok(aggsig) => aggsig,
        Err(_) => return Ok(smallvec![Value::vector_u8(vec![]), Value::bool(false)]),
    };

    Ok(smallvec![
        Value::vector_u8(aggsig.to_bytes().to_vec()),
        Value::bool(true)
    ])
}

/***************************************************************************************************
 * native fun bls12381_signature_subgroup_check
 *
 *   gas cost: base_cost + per_sig_deserialize_cost +? per_sig_subgroup_check_cost
 *
 * where +? indicates that the expression stops evaluating there if the previous gas-charging step
 * failed
 **************************************************************************************************/
pub fn native_bls12381_signature_subgroup_check(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert!(_ty_args.is_empty());
    debug_assert!(arguments.len() == 1);

    context.charge(BLS12381_BASE)?;

    let sig_bytes = safely_pop_arg!(arguments, Vec<u8>);

    let sig = match bls12381_deserialize_sig(sig_bytes, context)? {
        Some(key) => key,
        None => return Ok(smallvec![Value::bool(false)]),
    };

    let valid = bls12381_sig_subgroub_check(&sig, context)?;

    Ok(smallvec![Value::bool(valid)])
}

/***************************************************************************************************
 * native fun bls12381_validate_pubkey
 *
 *   gas cost: base_cost + per_pubkey_deserialize_cost +? per_pubkey_subgroup_check_cost
 *
 * where +? indicates that the expression stops evaluating there if the previous gas-charging step
 * failed
 **************************************************************************************************/
fn native_bls12381_validate_pubkey(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert!(_ty_args.is_empty());
    debug_assert!(arguments.len() == 1);

    context.charge(BLS12381_BASE)?;

    let pk_bytes = safely_pop_arg!(arguments, Vec<u8>);

    let pk = match bls12381_deserialize_pk(pk_bytes, context)? {
        Some(key) => key,
        None => return Ok(smallvec![Value::bool(false)]),
    };

    let valid = bls12381_pk_subgroub_check(&pk, context)?;

    Ok(smallvec![Value::bool(valid)])
}

/***************************************************************************************************
* native fun native_bls12381_verify_aggregate_signature
*
*   gas cost: base_cost + per_pubkey_deserialize_cost * min(num_validatable_pubkeys + 1, num_pubkeys)
*                       +? ( per_sig_deserialize_cost * min(num_viable_sigs + 1, num_sigs)
*                            +? ( per_pairing_cost + per_msg_hashing_cost ) * num_msgs
*                                 + per_byte_hashing_cost * total_msg_bytes )
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
pub fn native_bls12381_verify_aggregate_signature(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert!(_ty_args.is_empty());
    debug_assert!(arguments.len() == 3);

    context.charge(BLS12381_BASE)?;

    // Parses a Vec<Vec<u8>> of all messages
    let messages = safely_pop_vec_arg!(arguments, Vec<u8>);
    // Parses a Vec<Vec<u8>> of all serialized public keys
    let pks_serialized = pop_as_vec_of_vec_u8(&mut arguments)?;
    let num_pks = pks_serialized.len();

    // Parses the signature as a Vec<u8>
    let aggsig_bytes = safely_pop_arg!(arguments, Vec<u8>);

    // Number of messages must match number of public keys
    if pks_serialized.len() != messages.len() {
        return Ok(smallvec![Value::bool(false)]);
    }

    let pks = bls12381_deserialize_pks(pks_serialized, context)?;
    debug_assert!(pks.len() <= num_pks);

    // If less PKs than expected were deserialized, return None.
    if pks.len() != num_pks {
        return Ok(smallvec![Value::bool(false)]);
    }

    let aggsig = match bls12381_deserialize_sig(aggsig_bytes, context)? {
        Some(aggsig) => aggsig,
        None => return Ok(smallvec![Value::bool(false)]),
    };

    let msgs_refs = messages
        .iter()
        .map(|m| m.as_slice())
        .collect::<Vec<&[u8]>>();
    let pks_refs = pks.iter().collect::<Vec<&bls12381::PublicKey>>();

    // The cost of verifying a size-n aggregate signatures involves n+1 parings and hashing all
    // the messages to elliptic curve points (proportional to sum of all message lengths).
    context.charge(
        BLS12381_PER_PAIRING * NumArgs::new((messages.len() + 1) as u64)
            + BLS12381_PER_MSG_HASHING * NumArgs::new(messages.len() as u64)
            + BLS12381_PER_BYTE_HASHING
                * messages.iter().fold(NumBytes::new(0), |sum, msg| {
                    sum + NumBytes::new(msg.len() as u64)
                }),
    )?;

    let verify_result = aggsig
        .verify_aggregate_arbitrary_msg(&msgs_refs, &pks_refs)
        .is_ok();

    Ok(smallvec![Value::bool(verify_result)])
}

/***************************************************************************************************
 * native fun bls12381_verify_multisignature
 *
 *   gas cost: base_cost + per_pubkey_deserialize_cost
 *                       +? ( per_sig_deserialize_cost
 *                            +? ( per_sig_verify_cost + per_msg_hashing_cost
 *                                 + per_byte_hashing_cost * |msg| ) )
 *
 * where +? indicates that the expression stops evaluating there if the previous gas-charging step
 * failed
 **************************************************************************************************/
pub fn native_bls12381_verify_multisignature(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    arguments: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    let check_pk_subgroup = false;
    bls12381_verify_signature_helper(context, _ty_args, arguments, check_pk_subgroup)
}

/***************************************************************************************************
 * native fun bls12381_verify_normal_signature
 *
 *   gas cost: base_cost + per_pubkey_deserialize_cost
 *                       +? ( per_sig_deserialize_cost
 *                            +? ( per_pubkey_subgroup_check_cost
 *                                 +? ( per_sig_verify_cost + per_msg_hashing_cost
 *                                     + per_byte_hashing_cost * |msg| ) ) )
 *
 * where +? indicates that the expression stops evaluating there if the previous gas-charging step
 * failed
 **************************************************************************************************/
pub fn native_bls12381_verify_normal_signature(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    arguments: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    // For normal (non-aggregated) signatures, PK's typically don't come with PoPs and the caller
    // might forget to check prime-order subgroup membership of the PK. Therefore, we always enforce
    // it here.
    let check_pk_subgroup = true;
    bls12381_verify_signature_helper(context, _ty_args, arguments, check_pk_subgroup)
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
fn native_bls12381_verify_proof_of_possession(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert!(_ty_args.is_empty());
    debug_assert!(arguments.len() == 2);

    context.charge(BLS12381_BASE)?;

    let pop_bytes = safely_pop_arg!(arguments, Vec<u8>);
    let key_bytes = safely_pop_arg!(arguments, Vec<u8>);

    let pk = match bls12381_deserialize_pk(key_bytes, context)? {
        Some(pk) => pk,
        None => return Ok(smallvec![Value::bool(false)]),
    };

    let pop = match bls12381_deserialize_pop(pop_bytes, context)? {
        Some(pop) => pop,
        None => return Ok(smallvec![Value::bool(false)]),
    };

    // NOTE(Gas): 2 bilinear pairings and a hash-to-curve
    context.charge(BLS12381_PER_POP_VERIFY * NumArgs::one())?;
    let valid = pop.verify(&pk).is_ok();

    Ok(smallvec![Value::bool(valid)])
}

/***************************************************************************************************
 * native fun bls12381_verify_signature_share
 *
 *   gas cost: base_cost + per_pubkey_deserialize_cost
 *                       +? ( per_sig_deserialize_cost
 *                            +? ( per_sig_verify_cost + per_msg_hashing_cost
 *                                 + per_byte_hashing_cost * |msg| ) )
 *
 * where +? indicates that the expression stops evaluating there if the previous gas-charging step
 * failed
 **************************************************************************************************/
pub fn native_bls12381_verify_signature_share(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    arguments: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    // For signature shares, the caller is REQUIRED to check the PK's PoP, and thus the PK is in the
    // prime-order subgroup.
    let check_pk_subgroup = false;
    bls12381_verify_signature_helper(context, _ty_args, arguments, check_pk_subgroup)
}

#[cfg(feature = "testing")]
pub fn native_generate_keys(
    _context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut _arguments: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    let key_pair = KeyPair::<PrivateKey, PublicKey>::generate(&mut OsRng);
    Ok(smallvec![
        Value::vector_u8(key_pair.private_key.to_bytes()),
        Value::vector_u8(key_pair.public_key.to_bytes()),
    ])
}

#[cfg(feature = "testing")]
pub fn native_sign(
    _context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    let msg = safely_pop_arg!(arguments, Vec<u8>);
    let sk_bytes = safely_pop_arg!(arguments, Vec<u8>);
    let sk = PrivateKey::try_from(sk_bytes.as_slice()).unwrap();
    let sig = sk.sign_arbitrary_message(msg.as_slice());
    Ok(smallvec![Value::vector_u8(sig.to_bytes()),])
}

#[cfg(feature = "testing")]
pub fn native_generate_proof_of_possession(
    _context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    let sk_bytes = safely_pop_arg!(arguments, Vec<u8>);
    let sk = PrivateKey::try_from(sk_bytes.as_slice()).unwrap();
    let pop = ProofOfPossession::create(&sk);
    Ok(smallvec![Value::vector_u8(pop.to_bytes()),])
}

/***************************************************************************************************
 * module
 *
 **************************************************************************************************/
pub fn make_all(
    builder: &SafeNativeBuilder,
) -> impl Iterator<Item = (String, NativeFunction)> + '_ {
    let mut natives = vec![];

    natives.extend([
        // BLS over BLS12-381
        (
            "aggregate_pubkeys_internal",
            native_bls12381_aggregate_pubkeys as RawSafeNative,
        ),
        (
            "aggregate_signatures_internal",
            native_bls12381_aggregate_signatures,
        ),
        (
            "signature_subgroup_check_internal",
            native_bls12381_signature_subgroup_check,
        ),
        ("validate_pubkey_internal", native_bls12381_validate_pubkey),
        (
            "verify_aggregate_signature_internal",
            native_bls12381_verify_aggregate_signature,
        ),
        (
            "verify_multisignature_internal",
            native_bls12381_verify_multisignature,
        ),
        (
            "verify_normal_signature_internal",
            native_bls12381_verify_normal_signature,
        ),
        (
            "verify_proof_of_possession_internal",
            native_bls12381_verify_proof_of_possession,
        ),
        (
            "verify_signature_share_internal",
            native_bls12381_verify_signature_share,
        ),
    ]);

    #[cfg(feature = "testing")]
    natives.append(&mut vec![
        (
            "generate_keys_internal",
            native_generate_keys as RawSafeNative,
        ),
        ("sign_internal", native_sign),
        (
            "generate_proof_of_possession_internal",
            native_generate_proof_of_possession,
        ),
    ]);

    builder.make_named_natives(natives)
}
