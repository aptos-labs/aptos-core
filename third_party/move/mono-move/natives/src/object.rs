// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Natives for the `object` module, plus the extension backing them.

use crate::{monomorphic_natives, polymorphic_natives, NativeEntry};
use aptos_types::transaction::authenticator::AuthenticationKey;
use mono_move_core::{
    native::{NativeContext, NativeContextFamily, NativeExtension, NativeStatus},
    VMResult,
};
use move_core_types::account_address::AccountAddress;
use std::collections::HashMap;

/// Per-transaction memo cache for derived object addresses. Pure compute
/// optimization — the derivation is deterministic, so caching only saves work.
#[derive(Default)]
pub struct ObjectContextExtension {
    /// The keys are user-controlled addresses, so the DoS-resistant std
    /// hash table is used.
    derived: HashMap<(AccountAddress, AccountAddress), AccountAddress>,
}

impl ObjectContextExtension {
    pub fn new() -> Self {
        Self::default()
    }
}

impl NativeExtension for ObjectContextExtension {
    unsafe fn relocate_roots(&mut self, _relocate: &mut dyn FnMut(*mut u8) -> Option<*mut u8>) {}

    fn on_checkpoint(&mut self) {}

    fn on_rollback(&mut self, _n: usize) -> VMResult<()> {
        Ok(())
    }
}

/// Derives an object address from another object:
/// `sha3_256(source || derive_from || DeriveObjectAddressFromObject)`.
fn object_address_from_object(
    source: &AccountAddress,
    derive_from: &AccountAddress,
) -> AccountAddress {
    AuthenticationKey::object_address_from_object(source, derive_from).account_address()
}

/// `0x1::object::create_user_derived_object_address_impl(source: address, derive_from: address): address`
//
// TODO(metering): charge gas (constant cost) once the gas API lands.
pub fn native_create_user_derived_object_address_impl<C: NativeContext>(
    ctx: &C,
) -> VMResult<NativeStatus> {
    // SAFETY: args 0 and 1 are both `address` per the Move declaration.
    let source: AccountAddress = unsafe { ctx.arg(0)? };
    let derive_from: AccountAddress = unsafe { ctx.arg(1)? };

    let mut ext = ctx.get_extension::<ObjectContextExtension>()?;
    let address = *ext
        .derived
        .entry((source, derive_from))
        .or_insert_with(|| object_address_from_object(&source, &derive_from));

    // SAFETY: return 0 is `address`.
    unsafe { ctx.set_return(0, address)? };
    Ok(NativeStatus::Success)
}

/// `0x1::object::exists_at<T>(object: address): bool`
///
/// Whether a resource of type `T` exists at `object`.
//
// TODO(metering): charge gas (base + per-byte-loaded) once the gas API lands.
// TODO(perf): consider lowering this in the specializer to the `Exists` micro-op
// directly, dropping the native path.
pub fn native_exists_at<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    // SAFETY: arg 0 is `address`.
    let address: AccountAddress = unsafe { ctx.arg(0)? };
    let exists = ctx.resource_exists(address, ctx.ty_arg(0)?)?;
    // SAFETY: return 0 is `bool`.
    unsafe { ctx.set_return(0, exists)? };
    Ok(NativeStatus::Success)
}

/// Natives for the `object` module.
pub fn make_all_object_natives<F: NativeContextFamily>() -> Vec<NativeEntry<F>> {
    let mut natives = monomorphic_natives![(
        "0x1::object::create_user_derived_object_address_impl",
        native_create_user_derived_object_address_impl
    ),];
    natives.extend(polymorphic_natives![(
        "0x1::object::exists_at",
        native_exists_at
    )]);
    natives
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn object_from_object_known_answer() {
        let source = AccountAddress::from_hex_literal("0xa").unwrap();
        let derive_from = AccountAddress::from_hex_literal("0xb").unwrap();
        let addr = object_address_from_object(&source, &derive_from);
        assert_eq!(
            addr.to_hex_literal(),
            "0xc168433b37d568f2c5cb143f04e177e102d9e40247cefdcb41b8dcc56caa44b0"
        );
    }
}
