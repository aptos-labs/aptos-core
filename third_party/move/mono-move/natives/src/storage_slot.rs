// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Natives for the `storage_slot` module.
//!
//! These borrow a `StorageSlotResource<T>` from global storage at the address
//! held in the `StorageSlot<T>` argument. The resource flows through the same
//! read-write set as any other resource, so no storage-slot-specific extension
//! is needed.

use crate::{polymorphic_natives, NativeEntry};
use mono_move_core::native::{
    NativeContext, NativeContextFamily, NativeStatus, Ref, VMInternalError,
};
use move_core_types::account_address::AccountAddress;

/// `StorageSlotResource<T>` not found at the slot's address.
const ESTORAGE_SLOT_NOT_FOUND: u64 = 2;

/// Borrows `StorageSlotResource<T>` from global storage at the address held in
/// the `StorageSlot<T>` argument, writing the reference into return slot 0, or
/// aborts if the resource is missing.
//
// TODO(metering): charge gas.
fn borrow_storage_slot_resource<C: NativeContext>(
    ctx: &C,
    mutable: bool,
) -> Result<NativeStatus, VMInternalError> {
    // SAFETY: arg 0 is `&[mut] StorageSlot<T>`, whose single `addr: address`
    // field gives it the same representation as `&address`.
    let slot: Ref<AccountAddress> = unsafe { ctx.arg(0)? };
    let addr = *slot.get();
    // ty_arg 1 is `StorageSlotResource<T>` -- the resource to borrow.
    let resource_ty = ctx.ty_arg(1)?;
    match ctx.borrow_resource(addr, mutable, resource_ty)? {
        // SAFETY: return 0 is the `&[mut] StorageSlotResource<T>` reference.
        Some(r) => unsafe { ctx.set_return(0, r) }.map(|()| NativeStatus::Success),
        None => Ok(NativeStatus::Abort {
            code: ESTORAGE_SLOT_NOT_FOUND,
            message: Some(format!("StorageSlotResource at address {} not found", addr)),
        }),
    }
}

/// `0x1::storage_slot::borrow_storage_slot_resource<T, BR>(self: &StorageSlot<T>): &BR`
pub fn native_borrow_storage_slot_resource<C: NativeContext>(
    ctx: &C,
) -> Result<NativeStatus, VMInternalError> {
    borrow_storage_slot_resource(ctx, false)
}

/// `0x1::storage_slot::borrow_storage_slot_resource_mut<T, BR>(self: &mut StorageSlot<T>): &mut BR`
pub fn native_borrow_storage_slot_resource_mut<C: NativeContext>(
    ctx: &C,
) -> Result<NativeStatus, VMInternalError> {
    borrow_storage_slot_resource(ctx, true)
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
