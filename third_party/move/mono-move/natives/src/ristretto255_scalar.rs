// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Scalar natives for the `ristretto255` module (`aptos_std::ristretto255`).

use crate::{monomorphic_natives, NativeEntry};
use curve25519_dalek::scalar::Scalar;
use mono_move_core::{
    native::{
        native_invariant_violation, NativeContext, NativeContextFamily, NativeStatus, Vector,
    },
    VMResult,
};
#[cfg(feature = "testing")]
use rand_core::{OsRng, RngCore};
use sha2::Sha512;

/// A Move `Scalar`'s encoding is exactly 32 bytes.
const SCALAR_NUM_BYTES: usize = 32;

/// Builds a scalar from its 32-byte encoding. `Scalar::from_bits` clears the
/// high bit, matching V1's `scalar_from_valid_bytes`. The Move `Scalar` type
/// guarantees the length, so a mismatch is an invariant violation, not a user
/// abort.
pub(crate) fn scalar_from_bytes(bytes: &[u8]) -> VMResult<Scalar> {
    let slice = <[u8; SCALAR_NUM_BYTES]>::try_from(bytes)
        .map_err(|_| native_invariant_violation("ristretto255 scalar must be 32 bytes".into()))?;
    Ok(Scalar::from_bits(slice))
}

/// `0x1::ristretto255::scalar_is_canonical_internal(s: vector<u8>): bool`
///
/// TODO(metering): charge gas.
pub fn native_scalar_is_canonical<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    // SAFETY: arg 0 is `vector<u8>`.
    let s: Vector<u8> = unsafe { ctx.arg(0)? };

    // SAFETY: byte slice consumed before any allocation.
    let is_canonical = match <[u8; SCALAR_NUM_BYTES]>::try_from(unsafe { s.as_bytes() }) {
        Ok(bytes) => Scalar::from_canonical_bytes(bytes).is_some(),
        // A wrong length is not canonical; return `false` rather than aborting.
        Err(_) => false,
    };

    // SAFETY: return slot 0 is `bool`.
    unsafe { ctx.set_return(0, is_canonical)? };
    Ok(NativeStatus::Success)
}

/// `0x1::ristretto255::scalar_invert_internal(bytes: vector<u8>): vector<u8>`
///
/// TODO(metering): charge gas.
pub fn native_scalar_invert<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    // SAFETY: arg 0 is `vector<u8>`.
    let s: Vector<u8> = unsafe { ctx.arg(0)? };

    // SAFETY: byte slice consumed into an owned scalar before any allocation.
    let s = scalar_from_bytes(unsafe { s.as_bytes() })?;
    let out = ctx.new_byte_vector(&s.invert().to_bytes())?;

    // SAFETY: return slot 0 is `vector<u8>`.
    unsafe { ctx.set_return(0, out)? };
    Ok(NativeStatus::Success)
}

/// `0x1::ristretto255::scalar_from_sha512_internal(sha2_512_input: vector<u8>): vector<u8>`
///
/// TODO(metering): charge gas.
pub fn native_scalar_from_sha512<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    // SAFETY: arg 0 is `vector<u8>`.
    let input: Vector<u8> = unsafe { ctx.arg(0)? };

    // SAFETY: byte slice consumed into an owned scalar before any allocation.
    let s = Scalar::hash_from_bytes::<Sha512>(unsafe { input.as_bytes() });
    let out = ctx.new_byte_vector(&s.to_bytes())?;

    // SAFETY: return slot 0 is `vector<u8>`.
    unsafe { ctx.set_return(0, out)? };
    Ok(NativeStatus::Success)
}

/// `0x1::ristretto255::scalar_mul_internal(a_bytes: vector<u8>, b_bytes: vector<u8>): vector<u8>`
///
/// TODO(metering): charge gas.
pub fn native_scalar_mul<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    // SAFETY: args 0 and 1 are `vector<u8>`.
    let a: Vector<u8> = unsafe { ctx.arg(0)? };
    let b: Vector<u8> = unsafe { ctx.arg(1)? };

    // SAFETY: byte slices consumed into owned scalars before any allocation.
    let a = scalar_from_bytes(unsafe { a.as_bytes() })?;
    let b = scalar_from_bytes(unsafe { b.as_bytes() })?;
    let out = ctx.new_byte_vector(&(a * b).to_bytes())?;

    // SAFETY: return slot 0 is `vector<u8>`.
    unsafe { ctx.set_return(0, out)? };
    Ok(NativeStatus::Success)
}

/// `0x1::ristretto255::scalar_add_internal(a_bytes: vector<u8>, b_bytes: vector<u8>): vector<u8>`
///
/// TODO(metering): charge gas.
pub fn native_scalar_add<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    // SAFETY: args 0 and 1 are `vector<u8>`.
    let a: Vector<u8> = unsafe { ctx.arg(0)? };
    let b: Vector<u8> = unsafe { ctx.arg(1)? };

    // SAFETY: byte slices consumed into owned scalars before any allocation.
    let a = scalar_from_bytes(unsafe { a.as_bytes() })?;
    let b = scalar_from_bytes(unsafe { b.as_bytes() })?;
    let out = ctx.new_byte_vector(&(a + b).to_bytes())?;

    // SAFETY: return slot 0 is `vector<u8>`.
    unsafe { ctx.set_return(0, out)? };
    Ok(NativeStatus::Success)
}

/// `0x1::ristretto255::scalar_sub_internal(a_bytes: vector<u8>, b_bytes: vector<u8>): vector<u8>`
///
/// TODO(metering): charge gas.
pub fn native_scalar_sub<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    // SAFETY: args 0 and 1 are `vector<u8>`.
    let a: Vector<u8> = unsafe { ctx.arg(0)? };
    let b: Vector<u8> = unsafe { ctx.arg(1)? };

    // SAFETY: byte slices consumed into owned scalars before any allocation.
    let a = scalar_from_bytes(unsafe { a.as_bytes() })?;
    let b = scalar_from_bytes(unsafe { b.as_bytes() })?;
    let out = ctx.new_byte_vector(&(a - b).to_bytes())?;

    // SAFETY: return slot 0 is `vector<u8>`.
    unsafe { ctx.set_return(0, out)? };
    Ok(NativeStatus::Success)
}

/// `0x1::ristretto255::scalar_neg_internal(a_bytes: vector<u8>): vector<u8>`
///
/// TODO(metering): charge gas.
pub fn native_scalar_neg<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    // SAFETY: arg 0 is `vector<u8>`.
    let a: Vector<u8> = unsafe { ctx.arg(0)? };

    // SAFETY: byte slice consumed into an owned scalar before any allocation.
    let a = scalar_from_bytes(unsafe { a.as_bytes() })?;
    let out = ctx.new_byte_vector(&(-a).to_bytes())?;

    // SAFETY: return slot 0 is `vector<u8>`.
    unsafe { ctx.set_return(0, out)? };
    Ok(NativeStatus::Success)
}

/// `0x1::ristretto255::scalar_from_u64_internal(num: u64): vector<u8>`
///
/// TODO(metering): charge gas.
pub fn native_scalar_from_u64<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    // SAFETY: arg 0 is `u64`.
    let num: u64 = unsafe { ctx.arg(0)? };

    let out = ctx.new_byte_vector(&Scalar::from(num).to_bytes())?;

    // SAFETY: return slot 0 is `vector<u8>`.
    unsafe { ctx.set_return(0, out)? };
    Ok(NativeStatus::Success)
}

/// `0x1::ristretto255::scalar_from_u128_internal(num: u128): vector<u8>`
///
/// TODO(metering): charge gas.
pub fn native_scalar_from_u128<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    // SAFETY: arg 0 is `u128`.
    let num: u128 = unsafe { ctx.arg(0)? };

    let out = ctx.new_byte_vector(&Scalar::from(num).to_bytes())?;

    // SAFETY: return slot 0 is `vector<u8>`.
    unsafe { ctx.set_return(0, out)? };
    Ok(NativeStatus::Success)
}

/// `0x1::ristretto255::scalar_reduced_from_32_bytes_internal(bytes: vector<u8>): vector<u8>`
///
/// TODO(metering): charge gas.
pub fn native_scalar_reduced_from_32_bytes<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    // SAFETY: arg 0 is `vector<u8>`.
    let bytes: Vector<u8> = unsafe { ctx.arg(0)? };

    // SAFETY: byte slice copied into an owned array before any allocation.
    let slice = <[u8; 32]>::try_from(unsafe { bytes.as_bytes() })
        .map_err(|_| native_invariant_violation("expected 32 bytes".into()))?;
    let out = ctx.new_byte_vector(&Scalar::from_bytes_mod_order(slice).to_bytes())?;

    // SAFETY: return slot 0 is `vector<u8>`.
    unsafe { ctx.set_return(0, out)? };
    Ok(NativeStatus::Success)
}

/// `0x1::ristretto255::scalar_uniform_from_64_bytes_internal(bytes: vector<u8>): vector<u8>`
///
/// TODO(metering): charge gas.
pub fn native_scalar_uniform_from_64_bytes<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    // SAFETY: arg 0 is `vector<u8>`.
    let bytes: Vector<u8> = unsafe { ctx.arg(0)? };

    // SAFETY: byte slice copied into an owned array before any allocation.
    let slice = <[u8; 64]>::try_from(unsafe { bytes.as_bytes() })
        .map_err(|_| native_invariant_violation("expected 64 bytes".into()))?;
    let out = ctx.new_byte_vector(&Scalar::from_bytes_mod_order_wide(&slice).to_bytes())?;

    // SAFETY: return slot 0 is `vector<u8>`.
    unsafe { ctx.set_return(0, out)? };
    Ok(NativeStatus::Success)
}

/// Production scalar natives for the `ristretto255` module.
pub fn make_all_ristretto255_scalar_natives<F: NativeContextFamily>() -> Vec<NativeEntry<F>> {
    monomorphic_natives![
        (
            "0x1::ristretto255::scalar_is_canonical_internal",
            native_scalar_is_canonical
        ),
        (
            "0x1::ristretto255::scalar_invert_internal",
            native_scalar_invert
        ),
        (
            "0x1::ristretto255::scalar_from_sha512_internal",
            native_scalar_from_sha512
        ),
        ("0x1::ristretto255::scalar_mul_internal", native_scalar_mul),
        ("0x1::ristretto255::scalar_add_internal", native_scalar_add),
        ("0x1::ristretto255::scalar_sub_internal", native_scalar_sub),
        ("0x1::ristretto255::scalar_neg_internal", native_scalar_neg),
        (
            "0x1::ristretto255::scalar_from_u64_internal",
            native_scalar_from_u64
        ),
        (
            "0x1::ristretto255::scalar_from_u128_internal",
            native_scalar_from_u128
        ),
        (
            "0x1::ristretto255::scalar_reduced_from_32_bytes_internal",
            native_scalar_reduced_from_32_bytes
        ),
        (
            "0x1::ristretto255::scalar_uniform_from_64_bytes_internal",
            native_scalar_uniform_from_64_bytes
        ),
    ]
}

/// `0x1::ristretto255::random_scalar_internal(): vector<u8>`
///
/// Test-only native. No need to charge gas.
#[cfg(feature = "testing")]
pub fn native_scalar_random<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    let mut scalar_bytes = [0u8; 64];
    let mut rng = OsRng;
    rng.fill_bytes(&mut scalar_bytes);
    let s = Scalar::from_bytes_mod_order_wide(&scalar_bytes);
    let out = ctx.new_byte_vector(&s.to_bytes())?;

    // SAFETY: return slot 0 is `vector<u8>`.
    unsafe { ctx.set_return(0, out)? };
    Ok(NativeStatus::Success)
}

/// Test-only scalar natives for the `ristretto255` module.
#[cfg(feature = "testing")]
pub fn make_all_ristretto255_scalar_test_natives<F: NativeContextFamily>() -> Vec<NativeEntry<F>> {
    monomorphic_natives![(
        "0x1::ristretto255::random_scalar_internal",
        native_scalar_random
    ),]
}
