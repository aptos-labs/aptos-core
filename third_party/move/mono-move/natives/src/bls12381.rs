// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Natives for the `bls12381` module.

use crate::{monomorphic_natives, NativeEntry};
use aptos_crypto::{bls12381, traits::Signature};
#[cfg(feature = "testing")]
use aptos_crypto::{
    bls12381::{PrivateKey, ProofOfPossession, PublicKey},
    test_utils::KeyPair,
    SigningKey, Uniform,
};
use mono_move_core::{
    native::{NativeContext, NativeContextFamily, NativeStatus, Vector},
    VMResult,
};
#[cfg(feature = "testing")]
use rand_core::OsRng;

fn verify_signature<C: NativeContext>(ctx: &C, check_pk_subgroup: bool) -> VMResult<NativeStatus> {
    // SAFETY: args 0, 1, and 2 are `vector<u8>`.
    let sig_vec: Vector<u8> = unsafe { ctx.arg(0)? };
    let pk: Vector<u8> = unsafe { ctx.arg(1)? };
    let msg: Vector<u8> = unsafe { ctx.arg(2)? };

    // SAFETY: byte slice consumed before any allocation.
    let Ok(pk) = PublicKey::try_from(unsafe { pk.as_bytes() }) else {
        // SAFETY: return slot 0 is `bool`.
        unsafe { ctx.set_return(0, false)? };
        return Ok(NativeStatus::Success);
    };

    if check_pk_subgroup && pk.subgroup_check().is_err() {
        // SAFETY: return slot 0 is `bool`.
        unsafe { ctx.set_return(0, false)? };
        return Ok(NativeStatus::Success);
    }

    // SAFETY: byte slice consumed before any allocation.
    let Ok(sig) = bls12381::Signature::try_from(unsafe { sig_vec.as_bytes() }) else {
        // SAFETY: return slot 0 is `bool`.
        unsafe { ctx.set_return(0, false)? };
        return Ok(NativeStatus::Success);
    };

    // SAFETY: byte slice consumed before any allocation.
    let valid = sig
        .verify_arbitrary_msg(unsafe { msg.as_bytes() }, &pk)
        .is_ok();
    // SAFETY: return slot 0 is `bool`.
    unsafe { ctx.set_return(0, valid)? };
    Ok(NativeStatus::Success)
}

/// `0x1::bls12381::validate_pubkey_internal(public_key: vector<u8>): bool`
///
/// TODO(metering): charge gas.
pub fn native_validate_pubkey<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    // SAFETY: arg 0 is `vector<u8>`.
    let pk: Vector<u8> = unsafe { ctx.arg(0)? };
    // SAFETY: byte slice consumed before any allocation.
    let valid = PublicKey::try_from(unsafe { pk.as_bytes() })
        .map(|pk| pk.subgroup_check().is_ok())
        .unwrap_or(false);
    // SAFETY: return slot 0 is `bool`.
    unsafe { ctx.set_return(0, valid)? };
    Ok(NativeStatus::Success)
}

/// `0x1::bls12381::signature_subgroup_check_internal(signature: vector<u8>): bool`
///
/// TODO(metering): charge gas.
pub fn native_signature_subgroup_check<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    // SAFETY: arg 0 is `vector<u8>`.
    let sig_vec: Vector<u8> = unsafe { ctx.arg(0)? };
    // SAFETY: byte slice consumed before any allocation.
    let valid = bls12381::Signature::try_from(unsafe { sig_vec.as_bytes() })
        .map(|sig| sig.subgroup_check().is_ok())
        .unwrap_or(false);
    // SAFETY: return slot 0 is `bool`.
    unsafe { ctx.set_return(0, valid)? };
    Ok(NativeStatus::Success)
}

/// `0x1::bls12381::verify_signature_share_internal(signature_share: vector<u8>, public_key: vector<u8>, message: vector<u8>): bool`
///
/// TODO(metering): charge gas.
pub fn native_verify_signature_share<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    // For signature shares, the caller is REQUIRED to check the PK's PoP, and
    // thus the PK is in the prime-order subgroup.
    verify_signature(ctx, false)
}

/// `0x1::bls12381::verify_multisignature_internal(multisignature: vector<u8>, agg_public_key: vector<u8>, message: vector<u8>): bool`
///
/// TODO(metering): charge gas.
pub fn native_verify_multisignature<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    verify_signature(ctx, false)
}

/// `0x1::bls12381::verify_normal_signature_internal(signature: vector<u8>, public_key: vector<u8>, message: vector<u8>): bool`
///
/// TODO(metering): charge gas.
pub fn native_verify_normal_signature<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    // For normal (non-aggregated) signatures, PK's typically don't come with
    // PoPs and the caller might forget to check prime-order subgroup membership
    // of the PK. Therefore, we always enforce it here.
    verify_signature(ctx, true)
}

/// `0x1::bls12381::verify_proof_of_possession_internal(pk: vector<u8>, pop: vector<u8>): bool`
///
/// TODO(metering): charge gas.
pub fn native_verify_proof_of_possession<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    // SAFETY: args 0 and 1 are `vector<u8>`.
    let pk: Vector<u8> = unsafe { ctx.arg(0)? };
    let pop: Vector<u8> = unsafe { ctx.arg(1)? };

    // SAFETY: byte slices consumed before any allocation.
    let Ok(pk) = PublicKey::try_from(unsafe { pk.as_bytes() }) else {
        // SAFETY: return slot 0 is `bool`.
        unsafe { ctx.set_return(0, false)? };
        return Ok(NativeStatus::Success);
    };
    let Ok(pop) = ProofOfPossession::try_from(unsafe { pop.as_bytes() }) else {
        // SAFETY: return slot 0 is `bool`.
        unsafe { ctx.set_return(0, false)? };
        return Ok(NativeStatus::Success);
    };

    let valid = pop.verify(&pk).is_ok();
    // SAFETY: return slot 0 is `bool`.
    unsafe { ctx.set_return(0, valid)? };
    Ok(NativeStatus::Success)
}

/// Production natives for the `bls12381` module.
pub fn make_all_bls12381_natives<F: NativeContextFamily>() -> Vec<NativeEntry<F>> {
    monomorphic_natives![
        (
            "0x1::bls12381::validate_pubkey_internal",
            native_validate_pubkey
        ),
        (
            "0x1::bls12381::signature_subgroup_check_internal",
            native_signature_subgroup_check
        ),
        (
            "0x1::bls12381::verify_normal_signature_internal",
            native_verify_normal_signature
        ),
        (
            "0x1::bls12381::verify_signature_share_internal",
            native_verify_signature_share
        ),
        (
            "0x1::bls12381::verify_multisignature_internal",
            native_verify_multisignature
        ),
        (
            "0x1::bls12381::verify_proof_of_possession_internal",
            native_verify_proof_of_possession
        ),
    ]
}

/// `0x1::bls12381::generate_keys_internal(): (vector<u8>, vector<u8>)`
///
/// Test-only native. No need to charge gas.
#[cfg(feature = "testing")]
pub fn native_generate_keys<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    let key_pair = KeyPair::<PrivateKey, PublicKey>::generate(&mut OsRng);
    let sk = ctx.new_byte_vector(&key_pair.private_key.to_bytes())?;
    let pk = ctx.new_byte_vector(&key_pair.public_key.to_bytes())?;

    // SAFETY: returns 0 and 1 are `vector<u8>`.
    unsafe { ctx.set_return(0, sk)? };
    unsafe { ctx.set_return(1, pk)? };
    Ok(NativeStatus::Success)
}

/// `0x1::bls12381::sign_internal(sk: vector<u8>, msg: vector<u8>): vector<u8>`
///
/// Test-only native. No need to charge gas.
#[cfg(feature = "testing")]
pub fn native_sign<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    // SAFETY: args 0 and 1 are `vector<u8>`.
    let sk: Vector<u8> = unsafe { ctx.arg(0)? };
    let msg: Vector<u8> = unsafe { ctx.arg(1)? };

    // SAFETY: byte slice consumed before any allocation.
    let Ok(sk) = PrivateKey::try_from(unsafe { sk.as_bytes() }) else {
        return Ok(NativeStatus::Abort {
            code: 0x0A_0001,
            message: Some("BLS12381 sign computation failed".to_string()),
        });
    };

    // SAFETY: byte slice consumed before any allocation.
    let sig = sk.sign_arbitrary_message(unsafe { msg.as_bytes() });
    let out = ctx.new_byte_vector(&sig.to_bytes())?;

    // SAFETY: return 0 is `vector<u8>`.
    unsafe { ctx.set_return(0, out)? };
    Ok(NativeStatus::Success)
}

/// `0x1::bls12381::generate_proof_of_possession_internal(sk: vector<u8>): vector<u8>`
///
/// Test-only native. No need to charge gas.
#[cfg(feature = "testing")]
pub fn native_generate_proof_of_possession<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    // SAFETY: arg 0 is `vector<u8>`.
    let sk: Vector<u8> = unsafe { ctx.arg(0)? };

    // SAFETY: byte slice consumed before any allocation.
    let Ok(sk) = PrivateKey::try_from(unsafe { sk.as_bytes() }) else {
        let msg = "BLS12381 proof-of-possession secret key deserialization failed".to_string();
        return Ok(NativeStatus::Abort {
            code: 0x0A_0002,
            message: Some(msg),
        });
    };

    let pop = ProofOfPossession::create(&sk);
    let out = ctx.new_byte_vector(&pop.to_bytes())?;

    // SAFETY: return 0 is `vector<u8>`.
    unsafe { ctx.set_return(0, out)? };
    Ok(NativeStatus::Success)
}

/// Test-only natives for the `bls12381` module.
#[cfg(feature = "testing")]
pub fn make_all_bls12381_test_natives<F: NativeContextFamily>() -> Vec<NativeEntry<F>> {
    monomorphic_natives![
        (
            "0x1::bls12381::generate_keys_internal",
            native_generate_keys
        ),
        ("0x1::bls12381::sign_internal", native_sign),
        (
            "0x1::bls12381::generate_proof_of_possession_internal",
            native_generate_proof_of_possession
        ),
    ]
}
