// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Natives for the `hash` module.

use crate::{monomorphic_natives, NativeEntry};
use mono_move_core::native::{
    NativeContext, NativeContextFamily, NativeStatus, VMInternalError, Vector,
};
use sha3::{Digest, Keccak256, Sha3_256};

/// Reads the `vector<u8>` argument off the VM heap, hashes it with `H`, and
/// returns the digest as a fresh `vector<u8>`.
fn hash_arg<C: NativeContext, H: Digest>(ctx: &C) -> Result<NativeStatus, VMInternalError> {
    // SAFETY: arg 0 is `vector<u8>`, passed by value.
    let data: Vector<u8> = unsafe { ctx.arg(0)? };
    // Copy the bytes off the VM heap before hashing: `new_byte_vector` allocates
    // (and may GC), which would invalidate a heap-resident slice.
    // SAFETY: the bytes are copied immediately, before any allocation.
    let bytes = unsafe { data.as_bytes() }.to_vec();
    let digest = H::digest(&bytes);
    let out = ctx.new_byte_vector(digest.as_slice())?;
    // SAFETY: return 0 is `vector<u8>`.
    unsafe { ctx.set_return(0, out)? };
    Ok(NativeStatus::Success)
}

/// `0x1::hash::sha3_256(data: vector<u8>): vector<u8>`
//
// TODO: charge gas.
pub fn native_sha3_256<C: NativeContext>(ctx: &C) -> Result<NativeStatus, VMInternalError> {
    hash_arg::<C, Sha3_256>(ctx)
}

/// `0x1::aptos_hash::keccak256(bytes: vector<u8>): vector<u8>`
//
// TODO: charge gas.
pub fn native_keccak256<C: NativeContext>(ctx: &C) -> Result<NativeStatus, VMInternalError> {
    hash_arg::<C, Keccak256>(ctx)
}

/// Natives for the `hash` and `aptos_hash` modules.
pub fn make_all_hash_natives<F: NativeContextFamily>() -> Vec<NativeEntry<F>> {
    monomorphic_natives![
        ("0x1::hash::sha3_256", native_sha3_256),
        ("0x1::aptos_hash::keccak256", native_keccak256),
    ]
}
