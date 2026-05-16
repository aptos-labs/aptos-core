// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Native implementation for storage_slot::borrow_storage_slot_resource and storage_slot::borrow_storage_slot_resource_mut
//!
//! These natives borrow StorageSlotResource<T> from Move global storage and return references.
//! They work by using the VM's newly exposed borrow_resource APIs.

use aptos_gas_schedule::gas_params::natives::aptos_framework::*;
use aptos_native_interface::{
    safely_assert_eq, safely_pop_arg, RawSafeNative, SafeNativeBuilder, SafeNativeContext,
    SafeNativeError, SafeNativeResult,
};
use move_core_types::{account_address::AccountAddress, vm_status::StatusCode};
use move_vm_runtime::native_functions::NativeFunction;
use move_vm_types::{
    loaded_data::runtime_types::Type,
    values::{Reference, StructRef, Value},
};
use smallvec::{smallvec, SmallVec};
use std::collections::VecDeque;

// Error codes
const ESTORAGE_SLOT_NOT_FOUND: u64 = 0x2;

/***************************************************************************************************
 * native fun borrow_storage_slot_resource<T: store, BR>(self: &StorageSlot<T>): &BR
 *
 * Borrows StorageSlotResource<T> from global storage.
 * Called from Move as: borrow_storage_slot_resource<T, StorageSlotResource<T>>(storage_slot)
 *
 * Type args: [T, StorageSlotResource<T>]
 *   - ty_args[0] = T (the value type)
 *   - ty_args[1] = StorageSlotResource<T> (the resource type to borrow from global storage)
 **************************************************************************************************/
fn native_borrow_storage_slot_resource(
    context: &mut SafeNativeContext,
    ty_args: &[Type],
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    safely_assert_eq!(ty_args.len(), 2);
    safely_assert_eq!(args.len(), 1);

    context.charge(STORAGE_SLOT_BORROW_BASE)?;

    // Get the address from StorageSlot.addr field
    let storage_slot_ref = safely_pop_arg!(args, StructRef);
    let addr = storage_slot_ref
        .borrow_field(0)?
        .value_as::<Reference>()?
        .read_ref()?
        .value_as::<AccountAddress>()?;

    // ty_args[1] is StorageSlotResource<T> - the type we want to borrow from global storage
    let storage_slot_resource_ty = &ty_args[1];

    // Borrow the resource from global storage
    let (ref_val, num_bytes) = context
        .borrow_resource(addr, storage_slot_resource_ty)
        .map_err(|err| {
            // Check if resource doesn't exist
            if err.major_status() == StatusCode::MISSING_DATA {
                SafeNativeError::abort_with_message(
                    ESTORAGE_SLOT_NOT_FOUND,
                    format!("StorageSlotResource at address {} not found", addr),
                )
            } else {
                err.into()
            }
        })?;

    // Charge for loaded bytes
    if let Some(num_bytes) = num_bytes {
        context.charge(STORAGE_SLOT_BORROW_PER_BYTE_LOADED * num_bytes)?;
    }

    Ok(smallvec![ref_val])
}

/***************************************************************************************************
 * native fun borrow_storage_slot_resource_mut<T: store, BR>(self: &mut StorageSlot<T>): &mut BR
 *
 * Borrows StorageSlotResource<T> mutably from global storage.
 * Called from Move as: borrow_storage_slot_resource_mut<T, StorageSlotResource<T>>(storage_slot)
 *
 * Type args: [T, StorageSlotResource<T>]
 *   - ty_args[0] = T (the value type)
 *   - ty_args[1] = StorageSlotResource<T> (the resource type to borrow from global storage)
 **************************************************************************************************/
fn native_borrow_storage_slot_resource_mut(
    context: &mut SafeNativeContext,
    ty_args: &[Type],
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    safely_assert_eq!(ty_args.len(), 2);
    safely_assert_eq!(args.len(), 1);

    context.charge(STORAGE_SLOT_BORROW_MUT_BASE)?;

    // Get the address from StorageSlot.addr field
    let storage_slot_ref = safely_pop_arg!(args, StructRef);
    let addr = storage_slot_ref
        .borrow_field(0)?
        .value_as::<Reference>()?
        .read_ref()?
        .value_as::<AccountAddress>()?;

    // ty_args[1] is StorageSlotResource<T> - the type we want to borrow from global storage
    let storage_slot_resource_ty = &ty_args[1];

    // Borrow the resource mutably from global storage
    let (ref_val, num_bytes) = context
        .borrow_resource_mut(addr, storage_slot_resource_ty)
        .map_err(|err| {
            // Check if resource doesn't exist
            if err.major_status() == StatusCode::MISSING_DATA {
                SafeNativeError::abort_with_message(
                    ESTORAGE_SLOT_NOT_FOUND,
                    format!("StorageSlotResource at address {} not found", addr),
                )
            } else {
                err.into()
            }
        })?;

    // Charge for loaded bytes
    if let Some(num_bytes) = num_bytes {
        context.charge(STORAGE_SLOT_BORROW_MUT_PER_BYTE_LOADED * num_bytes)?;
    }

    Ok(smallvec![ref_val])
}

/***************************************************************************************************
 * module
 *
 **************************************************************************************************/
pub fn make_all(
    builder: &SafeNativeBuilder,
) -> impl Iterator<Item = (String, NativeFunction)> + '_ {
    let natives = [
        (
            "borrow_storage_slot_resource",
            native_borrow_storage_slot_resource as RawSafeNative,
        ),
        (
            "borrow_storage_slot_resource_mut",
            native_borrow_storage_slot_resource_mut,
        ),
    ];

    builder.make_named_natives(natives)
}
