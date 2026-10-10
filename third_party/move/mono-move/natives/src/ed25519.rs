// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Natives for the `ed25519` module (`aptos_std::ed25519`).

use crate::{monomorphic_natives, NativeEntry};
#[cfg(feature = "testing")]
use aptos_crypto::{ed25519::Ed25519PrivateKey, test_utils::KeyPair, SigningKey, Uniform};
use aptos_crypto::{
    ed25519::{Ed25519PublicKey, Ed25519Signature, ED25519_PUBLIC_KEY_LENGTH},
    traits::Signature,
};
use curve25519_dalek::edwards::CompressedEdwardsY;
use mono_move_core::{
    native::{NativeContext, NativeContextFamily, NativeStatus, Vector},
    VMResult,
};
#[cfg(feature = "testing")]
use rand_core::OsRng;

/// `0x1::ed25519::public_key_validate_internal(bytes: vector<u8>): bool`
///
/// TODO(metering): charge gas.
pub fn native_public_key_validate<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    // SAFETY: arg 0 is `vector<u8>`.
    let key: Vector<u8> = unsafe { ctx.arg(0)? };

    // A wrong-length key returns `false` rather than aborting, matching V1 under
    // ED25519_PUBKEY_VALIDATE_RETURN_FALSE_WRONG_LENGTH.
    // TODO(completeness): VM gates this by a flag, check if it is safe not to gate it.
    // SAFETY: byte slice consumed before any allocation.
    let Ok(key) = <[u8; ED25519_PUBLIC_KEY_LENGTH]>::try_from(unsafe { key.as_bytes() }) else {
        // SAFETY: return slot 0 is `bool`.
        unsafe { ctx.set_return(0, false)? };
        return Ok(NativeStatus::Success);
    };

    // Reject points off the curve or in the small subgroup.
    let valid = CompressedEdwardsY(key)
        .decompress()
        .is_some_and(|point| !point.is_small_order());
    // SAFETY: return slot 0 is `bool`.
    unsafe { ctx.set_return(0, valid)? };
    Ok(NativeStatus::Success)
}

/// `0x1::ed25519::signature_verify_strict_internal(signature: vector<u8>, public_key: vector<u8>, message: vector<u8>): bool`
///
/// TODO(metering): charge gas.
pub fn native_signature_verify_strict<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    // SAFETY: args 0, 1, and 2 are `vector<u8>`.
    let sig_vec: Vector<u8> = unsafe { ctx.arg(0)? };
    let pk: Vector<u8> = unsafe { ctx.arg(1)? };
    let msg: Vector<u8> = unsafe { ctx.arg(2)? };

    // SAFETY: byte slice consumed before any allocation.
    let Ok(pk) = Ed25519PublicKey::try_from(unsafe { pk.as_bytes() }) else {
        // SAFETY: return slot 0 is `bool`.
        unsafe { ctx.set_return(0, false)? };
        return Ok(NativeStatus::Success);
    };

    // SAFETY: byte slice consumed before any allocation.
    let Ok(sig) = Ed25519Signature::try_from(unsafe { sig_vec.as_bytes() }) else {
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

/// Production natives for the `ed25519` module.
pub fn make_all_ed25519_natives<F: NativeContextFamily>() -> Vec<NativeEntry<F>> {
    monomorphic_natives![
        (
            "0x1::ed25519::public_key_validate_internal",
            native_public_key_validate
        ),
        (
            "0x1::ed25519::signature_verify_strict_internal",
            native_signature_verify_strict
        ),
    ]
}

/// `0x1::ed25519::generate_keys_internal(): (vector<u8>, vector<u8>)`
///
/// Test-only native. No need to charge gas.
#[cfg(feature = "testing")]
pub fn native_generate_keys<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    let key_pair = KeyPair::<Ed25519PrivateKey, Ed25519PublicKey>::generate(&mut OsRng);
    let sk = ctx.new_byte_vector(&key_pair.private_key.to_bytes())?;
    let pk = ctx.new_byte_vector(&key_pair.public_key.to_bytes())?;

    // SAFETY: returns 0 and 1 are `vector<u8>`.
    unsafe { ctx.set_return(0, sk)? };
    unsafe { ctx.set_return(1, pk)? };
    Ok(NativeStatus::Success)
}

/// `0x1::ed25519::sign_internal(sk: vector<u8>, msg: vector<u8>): vector<u8>`
///
/// Test-only native. No need to charge gas.
#[cfg(feature = "testing")]
pub fn native_sign<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    // SAFETY: args 0 and 1 are `vector<u8>`.
    let sk: Vector<u8> = unsafe { ctx.arg(0)? };
    let msg: Vector<u8> = unsafe { ctx.arg(1)? };

    // SAFETY: byte slice consumed before any allocation.
    let Ok(sk) = Ed25519PrivateKey::try_from(unsafe { sk.as_bytes() }) else {
        return Ok(NativeStatus::Abort {
            code: 0x0A_0001,
            message: Some("Ed25519 sign computation failed".to_string()),
        });
    };

    // SAFETY: byte slice consumed before any allocation.
    let sig = sk.sign_arbitrary_message(unsafe { msg.as_bytes() });
    let out = ctx.new_byte_vector(&sig.to_bytes())?;

    // SAFETY: return 0 is `vector<u8>`.
    unsafe { ctx.set_return(0, out)? };
    Ok(NativeStatus::Success)
}

/// Test-only natives for the `ed25519` module.
#[cfg(feature = "testing")]
pub fn make_all_ed25519_test_natives<F: NativeContextFamily>() -> Vec<NativeEntry<F>> {
    monomorphic_natives![
        ("0x1::ed25519::generate_keys_internal", native_generate_keys),
        ("0x1::ed25519::sign_internal", native_sign),
    ]
}
