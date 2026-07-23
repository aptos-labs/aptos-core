// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Natives for the `hash` module (`std::hash`).

use crate::{monomorphic_natives, NativeEntry};
use mono_move_core::{
    native::{NativeContext, NativeContextFamily, NativeStatus, Vector},
    VMResult,
};
use sha2::{Digest, Sha256};
use sha3::Sha3_256;

/// `0x1::hash::sha2_256(data: vector<u8>): vector<u8>`
//
// TODO(metering): charge gas.
pub fn native_sha2_256<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    // SAFETY: arg 0 is `vector<u8>`.
    let data: Vector<u8> = unsafe { ctx.arg(0)? };
    let digest = {
        // SAFETY: the bytes are consumed before any allocation, so GC cannot
        // relocate them while the slice is held.
        let bytes = unsafe { data.as_bytes() };
        Sha256::digest(bytes).to_vec()
    };
    let out = ctx.new_byte_vector(&digest)?;
    // SAFETY: return 0 is `vector<u8>`.
    unsafe { ctx.set_return(0, out)? };
    Ok(NativeStatus::Success)
}

/// `0x1::hash::sha3_256(data: vector<u8>): vector<u8>`
//
// TODO(metering): charge gas.
pub fn native_sha3_256<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    // SAFETY: arg 0 is `vector<u8>`.
    let data: Vector<u8> = unsafe { ctx.arg(0)? };
    let digest = {
        // SAFETY: the bytes are consumed before any allocation, so GC cannot
        // relocate them while the slice is held.
        let bytes = unsafe { data.as_bytes() };
        Sha3_256::digest(bytes).to_vec()
    };
    let out = ctx.new_byte_vector(&digest)?;
    // SAFETY: return 0 is `vector<u8>`.
    unsafe { ctx.set_return(0, out)? };
    Ok(NativeStatus::Success)
}

/// Natives for the `hash` module.
pub fn make_all_hash_natives<F: NativeContextFamily>() -> Vec<NativeEntry<F>> {
    monomorphic_natives![
        ("0x1::hash::sha2_256", native_sha2_256),
        ("0x1::hash::sha3_256", native_sha3_256),
    ]
}
