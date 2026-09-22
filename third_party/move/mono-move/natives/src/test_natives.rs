// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Synthetic natives used by the differential harness. Expected to go
//! away once real natives are wired up.

use crate::{monomorphic_natives, NativeEntry};
use mono_move_core::{
    native::{NativeContext, NativeContextFamily, NativeStatus, Vector},
    VMResult,
};

pub fn native_u64_add<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    // SAFETY: u64 matches the Move-level `u64` type of args 0/1 and return 0.
    let a: u64 = unsafe { ctx.arg(0) }?;
    let b: u64 = unsafe { ctx.arg(1) }?;
    let sum = match a.checked_add(b) {
        Some(s) => s,
        None => {
            return Ok(NativeStatus::Abort {
                code: 1,
                message: None,
            })
        },
    };
    unsafe { ctx.set_return(0, sum) }?;
    Ok(NativeStatus::Success)
}

pub fn native_u64_identity<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    // SAFETY: u64 matches the Move-level `u64` type of arg 0 and return 0.
    let x: u64 = unsafe { ctx.arg(0) }?;
    unsafe { ctx.set_return(0, x) }?;
    Ok(NativeStatus::Success)
}

/// `0x1::test_natives::split_bytes(bytes: vector<u8>, chunk: u64): vector<vector<u8>>`
///
/// The only caller of [`NativeContext::new_byte_vector_vector`] that can reach
/// its corner cases: an empty result (`bytes` empty or `chunk` zero) and a
/// ragged final chunk.
pub fn native_split_bytes<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    // SAFETY: arg 0 is `vector<u8>` and arg 1 is `u64`.
    let (bytes, chunk) = unsafe {
        let bytes = ctx.arg::<Vector<u8>>(0)?;
        let chunk = ctx.arg::<u64>(1)? as usize;
        // The slice is copied out before the allocation below.
        (bytes.as_bytes().to_vec(), chunk)
    };

    // `chunks(0)` panics.
    let chunks = if chunk == 0 {
        vec![]
    } else {
        bytes.chunks(chunk).collect::<Vec<_>>()
    };
    let result = ctx.new_byte_vector_vector(&chunks)?;

    // SAFETY: return slot 0 is `vector<vector<u8>>`.
    unsafe { ctx.set_return(0, result) }?;
    Ok(NativeStatus::Success)
}

pub fn make_all_test_natives<F: NativeContextFamily>() -> Vec<NativeEntry<F>> {
    monomorphic_natives![
        ("0x1::test_natives::u64_add", native_u64_add),
        ("0x1::test_natives::u64_identity", native_u64_identity),
        ("0x1::test_natives::split_bytes", native_split_bytes),
    ]
}
