// RUN: publish
module 0x1::storage_slot {
    struct StorageSlotResource<T> has key {
        val: T,
    }

    struct StorageSlot<phantom T> has store {
        addr: address,
    }

    native fun borrow_storage_slot_resource<T: store, BR>(self: &StorageSlot<T>): &BR;
    native fun borrow_storage_slot_resource_mut<T: store, BR>(self: &mut StorageSlot<T>): &mut BR;

    // Publishes the backing resource under `account` and returns a slot at
    // `addr`; the caller passes an `addr` that matches `account`.
    public fun new_at<T: store>(account: &signer, addr: address, value: T): StorageSlot<T> {
        move_to(account, StorageSlotResource<T> { val: value });
        StorageSlot { addr }
    }

    // A slot pointing at `addr` with no backing resource, to exercise the
    // missing-resource abort.
    public fun unbacked<T: store>(addr: address): StorageSlot<T> {
        StorageSlot { addr }
    }

    public fun borrow<T: store>(self: &StorageSlot<T>): &T {
        &borrow_storage_slot_resource<T, StorageSlotResource<T>>(self).val
    }

    public fun borrow_mut<T: store>(self: &mut StorageSlot<T>): &mut T {
        &mut borrow_storage_slot_resource_mut<T, StorageSlotResource<T>>(self).val
    }

    public fun destroy<T: store>(self: StorageSlot<T>): T {
        let StorageSlot { addr } = self;
        let StorageSlotResource { val } = move_from<StorageSlotResource<T>>(addr);
        val
    }
}

module 0x42::main {
    use 0x1::storage_slot;

    // borrow reads back the published value.
    public fun borrow_reads(s: signer, a: address): u64 {
        let slot = storage_slot::new_at<u64>(&s, a, 42);
        let v = *storage_slot::borrow(&slot);
        storage_slot::destroy(slot);
        v
    }

    // borrow_mut writes through the reference; a later borrow sees the update.
    public fun borrow_mut_updates(s: signer, a: address): u64 {
        let slot = storage_slot::new_at<u64>(&s, a, 10);
        let before = *storage_slot::borrow(&slot);
        *storage_slot::borrow_mut(&mut slot) = before + 5;
        let after = *storage_slot::borrow(&slot);
        storage_slot::destroy(slot);
        after
    }

    // A heap-boxed value type (vector<u8>) round-trips through the borrow.
    public fun borrow_vector(s: signer, a: address): vector<u8> {
        let slot = storage_slot::new_at<vector<u8>>(&s, a, b"hello");
        let v = *storage_slot::borrow(&slot);
        storage_slot::destroy(slot);
        v
    }

    // borrow on a slot with no backing resource aborts.
    public fun borrow_missing_aborts(a: address): u64 {
        let slot = storage_slot::unbacked<u64>(a);
        let v = *storage_slot::borrow(&slot);
        storage_slot::destroy(slot);
        v
    }

    // borrow_mut on a slot with no backing resource aborts.
    public fun borrow_mut_missing_aborts(a: address): u64 {
        let slot = storage_slot::unbacked<u64>(a);
        *storage_slot::borrow_mut(&mut slot) = 0;
        storage_slot::destroy(slot);
        0
    }
}

// RUN: execute 0x42::main::borrow_reads --args 0x42, 0x42
// CHECK: results: 42

// RUN: execute 0x42::main::borrow_mut_updates --args 0x7, 0x7
// CHECK: results: 15

// RUN: execute 0x42::main::borrow_vector --args 0x9, 0x9
// CHECK: results: 0x68656c6c6f

// RUN: execute 0x42::main::borrow_missing_aborts --args 0x12
// CHECK-SUBSTR: aborted: code 2

// RUN: execute 0x42::main::borrow_mut_missing_aborts --args 0x13
// CHECK-SUBSTR: aborted: code 2
