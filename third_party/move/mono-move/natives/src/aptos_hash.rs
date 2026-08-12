// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Natives for the `aptos_hash` module (`aptos_std::aptos_hash`).
//!
//! The `sha2_512`, `sha3_512`, `ripemd160`, and `blake2b_256` natives are the
//! `*_internal` entry points; their public counterparts are feature-gated Move
//! wrappers in the framework.

use crate::{monomorphic_natives, NativeEntry};
use mono_move_core::{
    native::{NativeContext, NativeContextFamily, NativeStatus, Vector},
    VMResult,
};
use ripemd::Digest as OtherDigest;
use sha2::Digest;
use std::hash::Hasher;
use tiny_keccak::{Hasher as KeccakHasher, Keccak};

/// Reads arg 0 as `vector<u8>`, hashes it with `hash`, and writes the digest
/// back as a `vector<u8>` in return slot 0. Shared by every hash whose result
/// is a byte vector.
//
// TODO(metering): charge gas.
fn native_hash<C: NativeContext>(
    ctx: &C,
    hash: impl FnOnce(&[u8]) -> Vec<u8>,
) -> VMResult<NativeStatus> {
    // SAFETY: arg 0 is `vector<u8>`.
    let data: Vector<u8> = unsafe { ctx.arg(0)? };
    let digest = {
        // SAFETY: the bytes are consumed before any allocation, so GC cannot
        // relocate them while the slice is held.
        let bytes = unsafe { data.as_bytes() };
        hash(bytes)
    };
    let out = ctx.new_byte_vector(&digest)?;
    // SAFETY: return 0 is `vector<u8>`.
    unsafe { ctx.set_return(0, out)? };
    Ok(NativeStatus::Success)
}

/// `0x1::aptos_hash::keccak256(bytes: vector<u8>): vector<u8>`
pub fn native_keccak256<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    native_hash(ctx, |bytes| {
        let mut output = [0u8; 32];
        let mut hasher = Keccak::v256();
        hasher.update(bytes);
        hasher.finalize(&mut output);
        output.to_vec()
    })
}

/// `0x1::aptos_hash::sip_hash(bytes: vector<u8>): u64`
//
// TODO(metering): charge gas.
pub fn native_sip_hash<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    // SAFETY: arg 0 is `vector<u8>`.
    let data: Vector<u8> = unsafe { ctx.arg(0)? };
    let hash = {
        // SAFETY: the bytes are consumed before any allocation, so GC cannot
        // relocate them while the slice is held.
        let bytes = unsafe { data.as_bytes() };
        let mut hasher = siphasher::sip::SipHasher::new();
        hasher.write(bytes);
        hasher.finish()
    };
    // SAFETY: return 0 is `u64`.
    unsafe { ctx.set_return(0, hash)? };
    Ok(NativeStatus::Success)
}

/// `0x1::aptos_hash::sha2_512_internal(bytes: vector<u8>): vector<u8>`
pub fn native_sha2_512<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    native_hash(ctx, |bytes| {
        let mut hasher = sha2::Sha512::new();
        hasher.update(bytes);
        hasher.finalize().to_vec()
    })
}

/// `0x1::aptos_hash::sha3_512_internal(bytes: vector<u8>): vector<u8>`
pub fn native_sha3_512<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    native_hash(ctx, |bytes| {
        let mut hasher = sha3::Sha3_512::new();
        hasher.update(bytes);
        hasher.finalize().to_vec()
    })
}

/// `0x1::aptos_hash::ripemd160_internal(bytes: vector<u8>): vector<u8>`
pub fn native_ripemd160<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    native_hash(ctx, |bytes| {
        let mut hasher = ripemd::Ripemd160::new();
        hasher.update(bytes);
        hasher.finalize().to_vec()
    })
}

/// `0x1::aptos_hash::blake2b_256_internal(bytes: vector<u8>): vector<u8>`
pub fn native_blake2b_256<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    native_hash(ctx, |bytes| {
        blake2_rfc::blake2b::blake2b(32, &[], bytes)
            .as_bytes()
            .to_vec()
    })
}

/// Natives for the `aptos_hash` module.
pub fn make_all_aptos_hash_natives<F: NativeContextFamily>() -> Vec<NativeEntry<F>> {
    monomorphic_natives![
        ("0x1::aptos_hash::keccak256", native_keccak256),
        ("0x1::aptos_hash::sip_hash", native_sip_hash),
        ("0x1::aptos_hash::sha2_512_internal", native_sha2_512),
        ("0x1::aptos_hash::sha3_512_internal", native_sha3_512),
        ("0x1::aptos_hash::ripemd160_internal", native_ripemd160),
        ("0x1::aptos_hash::blake2b_256_internal", native_blake2b_256),
    ]
}
