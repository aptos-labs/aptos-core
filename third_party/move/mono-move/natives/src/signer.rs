// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Natives for the `signer` type.
//!
//! MonoMove represents a `signer` as a bare 32-byte account address — the same layout
//! as `address`. The permissioned signer feature has been removed; its native remains
//! registered only so historical versions of `0x1::permissioned_signer`, which declare
//! it, can still be loaded, and it aborts exactly like the legacy VM's flag-disabled
//! implementation.

use crate::{monomorphic_natives, NativeEntry};
use mono_move_core::{
    native::{NativeContext, NativeContextFamily, NativeStatus},
    VMResult,
};

/// Abort code of the removed permissioned signer natives (`EPERMISSION_SIGNER_DISABLED`).
const EPERMISSION_SIGNER_DISABLED: u64 = 9;

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

/// Builds a list of all signer-related natives.
pub fn make_all_signer_natives<F: NativeContextFamily>() -> Vec<NativeEntry<F>> {
    monomorphic_natives![
        ("0x1::signer::borrow_address", native_borrow_address),
        ("0x1::create_signer::create_signer", native_create_signer),
    ]
}
