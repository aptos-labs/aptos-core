// RUN: publish
module 0x1::storage_slot {
    struct StorageSlotResource<T> has key {
        val: T
    }

    struct StorageSlot<phantom T> has store {
        addr: address
    }

    native fun borrow_storage_slot_resource<T: store, BR>(slot: &StorageSlot<T>): &BR;
    native fun borrow_storage_slot_resource_mut<T: store, BR>(slot: &mut StorageSlot<T>): &mut BR;

    // The framework's `new` mints an address through `object`, which the
    // prelude does not publish, so the slot is anchored at a signer instead.
    public fun new_at<T: store>(owner: &signer, value: T): StorageSlot<T> {
        move_to(owner, StorageSlotResource { val: value });
        StorageSlot { addr: std::signer::address_of(owner) }
    }

    public fun unchecked_slot<T: store>(addr: address): StorageSlot<T> {
        StorageSlot { addr }
    }

    public fun destroy_slot<T: store>(slot: StorageSlot<T>) {
        let StorageSlot { addr: _ } = slot;
    }

    public fun borrow<T: store>(slot: &StorageSlot<T>): &T {
        &borrow_storage_slot_resource<T, StorageSlotResource<T>>(slot).val
    }

    public fun borrow_mut<T: store>(slot: &mut StorageSlot<T>): &mut T {
        &mut borrow_storage_slot_resource_mut<T, StorageSlotResource<T>>(slot).val
    }

    // `BR` is unconstrained, so Move cannot rule out a non-struct
    // instantiation. Both VMs reject it before the storage lookup.
    public fun borrow_non_struct<T: store>(slot: &StorageSlot<T>): u64 {
        *borrow_storage_slot_resource<T, u64>(slot)
    }
}
module 0x42::m {
    public fun reads_back(owner: signer, x: u64): u64 {
        let slot = 0x1::storage_slot::new_at<u64>(&owner, x);
        let v = *0x1::storage_slot::borrow<u64>(&slot);
        0x1::storage_slot::destroy_slot<u64>(slot);
        v
    }

    public fun writes_through(owner: signer, x: u64): u64 {
        let slot = 0x1::storage_slot::new_at<u64>(&owner, x);
        *0x1::storage_slot::borrow_mut<u64>(&mut slot) = x + 1;
        let v = *0x1::storage_slot::borrow<u64>(&slot);
        0x1::storage_slot::destroy_slot<u64>(slot);
        v
    }

    public fun missing_aborts(a: address): u64 {
        let slot = 0x1::storage_slot::unchecked_slot<u64>(a);
        let v = *0x1::storage_slot::borrow<u64>(&slot);
        0x1::storage_slot::destroy_slot<u64>(slot);
        v
    }

    // The slot is populated first, so the expected abort does not depend on
    // the order of the native's two guards.
    public fun non_struct_aborts(owner: signer, x: u64): u64 {
        let slot = 0x1::storage_slot::new_at<u64>(&owner, x);
        let v = 0x1::storage_slot::borrow_non_struct<u64>(&slot);
        0x1::storage_slot::destroy_slot<u64>(slot);
        v
    }
}

// RUN: execute 0x42::m::reads_back --args 0x7, 41
// CHECK: results: 41

// RUN: execute 0x42::m::writes_through --args 0x7, 41
// CHECK: results: 42

// RUN: execute 0x42::m::missing_aborts --args 0x7
// CHECK: aborted: code 2 (StorageSlotResource at address 0x7 not found) in 0x1::storage_slot

// RUN: execute 0x42::m::non_struct_aborts --args 0x7, 1
// CHECK: aborted: code 3 (Storage slot resource type argument must be a struct type) in 0x1::storage_slot
