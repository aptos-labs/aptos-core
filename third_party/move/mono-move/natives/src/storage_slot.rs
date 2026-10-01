// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Natives for the `storage_slot` module.
//!
//! A storage slot names an address holding a `StorageSlotResource<T>`, so the
//! two borrows below are resource borrows that flow through the read-write set
//! like the global-storage micro-ops.

use crate::{polymorphic_natives, NativeEntry};
use mono_move_core::{
    native::{NativeContext, NativeContextFamily, NativeStatus, Ref},
    types::is_resource_type,
    VMResult,
};
use move_core_types::account_address::AccountAddress;

/// No resource at the slot's address.
const NOT_FOUND: u64 = 0x2;
/// The borrowed type argument is not a struct or enum.
const NOT_A_RESOURCE_TYPE: u64 = 0x3;

/// Borrows the slot's resource, writing the reference into return slot 0.
fn borrow<C: NativeContext>(ctx: &C, mutable: bool) -> VMResult<NativeStatus> {
    // SAFETY: arg 0 is `&[mut] StorageSlot<T>`, which has the same
    // representation as `&address` — its single `addr` field.
    let slot: Ref<AccountAddress> = unsafe { ctx.arg(0)? };
    let address = slot.get();

    let resource_ty = ctx.ty_arg(1)?;
    if !is_resource_type(resource_ty) {
        return Ok(NativeStatus::Abort {
            code: NOT_A_RESOURCE_TYPE,
            message: Some("Storage slot resource type argument must be a struct type".to_string()),
        });
    }

    match ctx.resource_borrow(address, resource_ty, mutable)? {
        // SAFETY: return 0 is the `&[mut] BR` reference.
        Some(r) => unsafe { ctx.set_return(0, r) }.map(|()| NativeStatus::Success),
        None => Ok(NativeStatus::Abort {
            code: NOT_FOUND,
            message: Some(format!(
                "StorageSlotResource at address {} not found",
                address
            )),
        }),
    }
}

/// `0x1::storage_slot::borrow_storage_slot_resource<T, BR>(self: &StorageSlot<T>): &BR`
//
// TODO(metering): charge gas.
pub fn native_borrow_storage_slot_resource<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    borrow(ctx, false)
}

/// `0x1::storage_slot::borrow_storage_slot_resource_mut<T, BR>(self: &mut StorageSlot<T>): &mut BR`
//
// TODO(metering): charge gas.
pub fn native_borrow_storage_slot_resource_mut<C: NativeContext>(
    ctx: &C,
) -> VMResult<NativeStatus> {
    borrow(ctx, true)
}

/// Natives for the `storage_slot` module.
pub fn make_all_storage_slot_natives<F: NativeContextFamily>() -> Vec<NativeEntry<F>> {
    polymorphic_natives![
        (
            "0x1::storage_slot::borrow_storage_slot_resource",
            native_borrow_storage_slot_resource
        ),
        (
            "0x1::storage_slot::borrow_storage_slot_resource_mut",
            native_borrow_storage_slot_resource_mut
        ),
    ]
}
