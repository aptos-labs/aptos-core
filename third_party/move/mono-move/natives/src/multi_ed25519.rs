// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Natives for the `multi_ed25519` module (`aptos_std::multi_ed25519`).

use crate::{monomorphic_natives, NativeEntry};
use aptos_crypto::{
    ed25519::ED25519_PUBLIC_KEY_LENGTH,
    multi_ed25519::{self, MultiEd25519PublicKey, MultiEd25519Signature},
    traits::Signature,
};
#[cfg(feature = "testing")]
use aptos_crypto::{
    ed25519::{Ed25519PrivateKey, Ed25519PublicKey},
    multi_ed25519::MultiEd25519PrivateKey,
    test_utils::KeyPair,
    SigningKey, Uniform,
};
use curve25519_dalek::edwards::CompressedEdwardsY;
use mono_move_core::{
    native::{NativeContext, NativeContextFamily, NativeStatus, Vector},
    VMResult,
};
#[cfg(feature = "testing")]
use rand_core::OsRng;

/// Counts the leading sub-public-keys of `pks_bytes` that are valid points and
/// not in the small subgroup, stopping at the first invalid one (matching V1).
fn num_valid_subpks(pks_bytes: &[u8]) -> usize {
    let mut num_valid = 0;
    for chunk in pks_bytes.chunks_exact(ED25519_PUBLIC_KEY_LENGTH) {
        match <[u8; ED25519_PUBLIC_KEY_LENGTH]>::try_from(chunk) {
            Ok(slice) => {
                if CompressedEdwardsY(slice)
                    .decompress()
                    .is_some_and(|point| !point.is_small_order())
                {
                    num_valid += 1;
                } else {
                    break;
                }
            },
            Err(_) => break,
        }
    }
    num_valid
}

/// `0x1::multi_ed25519::public_key_validate_internal(bytes: vector<u8>): bool`
///
/// TODO(metering): charge gas.
pub fn native_public_key_validate<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    // SAFETY: arg 0 is `vector<u8>`.
    let pks_vec: Vector<u8> = unsafe { ctx.arg(0)? };
    // SAFETY: byte slice consumed before any allocation.
    let pks_bytes = unsafe { pks_vec.as_bytes() };

    let num_sub_pks = pks_bytes.len() / ED25519_PUBLIC_KEY_LENGTH;
    if num_sub_pks > multi_ed25519::MAX_NUM_OF_KEYS {
        // SAFETY: return slot 0 is `bool`.
        unsafe { ctx.set_return(0, false)? };
        return Ok(NativeStatus::Success);
    }
    let valid = num_valid_subpks(pks_bytes) == num_sub_pks;
    // SAFETY: return slot 0 is `bool`.
    unsafe { ctx.set_return(0, valid)? };
    Ok(NativeStatus::Success)
}

/// `0x1::multi_ed25519::public_key_validate_v2_internal(bytes: vector<u8>): bool`
///
/// TODO(metering): charge gas.
pub fn native_public_key_validate_v2<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    // SAFETY: arg 0 is `vector<u8>`.
    let pks_vec: Vector<u8> = unsafe { ctx.arg(0)? };
    // SAFETY: byte slice consumed before any allocation.
    let pks_bytes = unsafe { pks_vec.as_bytes() };

    // Unlike v1, first check the t-out-of-n encoding (a trailing threshold byte).
    let Ok((_, num_sub_pks)) =
        multi_ed25519::check_and_get_threshold(pks_bytes, ED25519_PUBLIC_KEY_LENGTH)
    else {
        // SAFETY: return slot 0 is `bool`.
        unsafe { ctx.set_return(0, false)? };
        return Ok(NativeStatus::Success);
    };
    let valid = num_valid_subpks(pks_bytes) == num_sub_pks as usize;
    // SAFETY: return slot 0 is `bool`.
    unsafe { ctx.set_return(0, valid)? };
    Ok(NativeStatus::Success)
}

/// `0x1::multi_ed25519::signature_verify_strict_internal(multisignature: vector<u8>, public_key: vector<u8>, message: vector<u8>): bool`
///
/// TODO(metering): charge gas.
pub fn native_signature_verify_strict<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    // SAFETY: args 0, 1, and 2 are `vector<u8>`.
    let sig_vec: Vector<u8> = unsafe { ctx.arg(0)? };
    let pk: Vector<u8> = unsafe { ctx.arg(1)? };
    let msg: Vector<u8> = unsafe { ctx.arg(2)? };

    // SAFETY: byte slice consumed before any allocation.
    let Ok(pk) = MultiEd25519PublicKey::try_from(unsafe { pk.as_bytes() }) else {
        // SAFETY: return slot 0 is `bool`.
        unsafe { ctx.set_return(0, false)? };
        return Ok(NativeStatus::Success);
    };

    // SAFETY: byte slice consumed before any allocation.
    let Ok(sig) = MultiEd25519Signature::try_from(unsafe { sig_vec.as_bytes() }) else {
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

/// Production natives for the `multi_ed25519` module.
pub fn make_all_multi_ed25519_natives<F: NativeContextFamily>() -> Vec<NativeEntry<F>> {
    monomorphic_natives![
        (
            "0x1::multi_ed25519::public_key_validate_internal",
            native_public_key_validate
        ),
        (
            "0x1::multi_ed25519::public_key_validate_v2_internal",
            native_public_key_validate_v2
        ),
        (
            "0x1::multi_ed25519::signature_verify_strict_internal",
            native_signature_verify_strict
        ),
    ]
}

/// `0x1::multi_ed25519::generate_keys_internal(threshold: u8, n: u8): (vector<u8>, vector<u8>)`
///
/// Test-only native. No need to charge gas.
#[cfg(feature = "testing")]
pub fn native_generate_keys<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    // SAFETY: args 0 and 1 are `u8`.
    let threshold: u8 = unsafe { ctx.arg(0)? };
    let n: u8 = unsafe { ctx.arg(1)? };

    let key_pairs = (0..n)
        .map(|_| KeyPair::<Ed25519PrivateKey, Ed25519PublicKey>::generate(&mut OsRng))
        .collect::<Vec<_>>();
    let private_keys = key_pairs
        .iter()
        .map(|pair| pair.private_key.clone())
        .collect();
    let public_keys = key_pairs
        .iter()
        .map(|pair| pair.public_key.clone())
        .collect();

    let Ok(group_sk) = MultiEd25519PrivateKey::new(private_keys, threshold) else {
        return Ok(NativeStatus::Abort {
            code: 0x0A_0001,
            message: Some("MultiEd25519 secret key creation failed".to_string()),
        });
    };
    let Ok(group_pk) = MultiEd25519PublicKey::new(public_keys, threshold) else {
        return Ok(NativeStatus::Abort {
            code: 0x0A_0002,
            message: Some("MultiEd25519 public key creation failed".to_string()),
        });
    };

    let sk = ctx.new_byte_vector(&group_sk.to_bytes())?;
    let pk = ctx.new_byte_vector(&group_pk.to_bytes())?;

    // SAFETY: returns 0 and 1 are `vector<u8>`.
    unsafe { ctx.set_return(0, sk)? };
    unsafe { ctx.set_return(1, pk)? };
    Ok(NativeStatus::Success)
}

/// `0x1::multi_ed25519::sign_internal(sk: vector<u8>, message: vector<u8>): vector<u8>`
///
/// Test-only native. No need to charge gas.
#[cfg(feature = "testing")]
pub fn native_sign<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    // SAFETY: args 0 and 1 are `vector<u8>`.
    let sk: Vector<u8> = unsafe { ctx.arg(0)? };
    let msg: Vector<u8> = unsafe { ctx.arg(1)? };

    // SAFETY: byte slice consumed before any allocation.
    let Ok(group_sk) = MultiEd25519PrivateKey::try_from(unsafe { sk.as_bytes() }) else {
        return Ok(NativeStatus::Abort {
            code: 0x0A_0003,
            message: Some("MultiEd25519 secret key deserialization failed".to_string()),
        });
    };

    // SAFETY: byte slice consumed before any allocation.
    let sig = group_sk.sign_arbitrary_message(unsafe { msg.as_bytes() });
    let out = ctx.new_byte_vector(&sig.to_bytes())?;

    // SAFETY: return 0 is `vector<u8>`.
    unsafe { ctx.set_return(0, out)? };
    Ok(NativeStatus::Success)
}

/// Test-only natives for the `multi_ed25519` module.
#[cfg(feature = "testing")]
pub fn make_all_multi_ed25519_test_natives<F: NativeContextFamily>() -> Vec<NativeEntry<F>> {
    monomorphic_natives![
        (
            "0x1::multi_ed25519::generate_keys_internal",
            native_generate_keys
        ),
        ("0x1::multi_ed25519::sign_internal", native_sign),
    ]
}
