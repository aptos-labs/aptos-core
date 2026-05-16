// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/// Spec for storage_slot module.
/// The native functions `borrow_storage_slot_resource` and `borrow_storage_slot_resource_mut`
/// do not have Boogie models, so we mark `borrow` and `borrow_mut` as opaque and unverified.
/// The other functions are skipped from verification due to difficulty in reasoning about
/// the dynamically allocated address created by `transaction_context::generate_unique_signer`.
spec aptos_framework::storage_slot {
    spec module {
        pragma verify = false;
    }

    spec borrow<T: store>(self: &StorageSlot<T>): &T {
        pragma opaque;
        pragma verify = false;
        aborts_if [abstract] false;
    }

    spec borrow_mut<T: store>(self: &mut StorageSlot<T>): &mut T {
        pragma opaque;
        pragma verify = false;
        aborts_if [abstract] false;
    }

    spec new<T: store>(value: T): StorageSlot<T> {
        pragma verify = false;
    }

    spec destroy<T: store>(self: StorageSlot<T>): T {
        pragma verify = false;
    }

    spec copy_storage_slot<T: store + copy>(self: &StorageSlot<T>): StorageSlot<T> {
        pragma verify = false;
    }
}
