// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Natives for the `signer` type.
//!
//! MonoMove currently represents a `signer` as a bare 32-byte account address — the
//! same layout as `address`. Permissioned signers are not supported, for now.

use crate::{monomorphic_natives, NativeEntry};
use mono_move_core::{
    native::{NativeContext, NativeContextFamily, NativeStatus},
    VMResult,
};

/// `0x1::create_signer::create_signer(addr: address): signer`
///
/// No-op. A `signer` has the same 32-byte layout as its `address`, so no conversion
/// needs to be done.
pub fn native_create_signer<C: NativeContext>(_ctx: &C) -> VMResult<NativeStatus> {
    Ok(NativeStatus::Success)
}

/// `0x1::signer::borrow_address(self: &signer): &address`
///
/// No-op. A `signer` has the same 32-byte layout as its `address`, so no change to
/// the reference as well.
pub fn native_borrow_address<C: NativeContext>(_ctx: &C) -> VMResult<NativeStatus> {
    Ok(NativeStatus::Success)
}

/// `0x1::permissioned_signer::is_permissioned_signer_impl(s: &signer): bool`
///
/// Always returns `false` as we do not support permissioned signers for now.
pub fn native_is_permissioned_signer<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    // SAFETY: `bool` matches the Move-level `bool` return at slot 0.
    unsafe { ctx.set_return(0, false) }?;
    Ok(NativeStatus::Success)
}

/// Builds a list of all signer-related natives.
pub fn make_all_signer_natives<F: NativeContextFamily>() -> Vec<NativeEntry<F>> {
    monomorphic_natives![
        ("0x1::signer::borrow_address", native_borrow_address),
        ("0x1::create_signer::create_signer", native_create_signer),
        (
            "0x1::permissioned_signer::is_permissioned_signer_impl",
            native_is_permissioned_signer
        ),
    ]
}
