// Storage-slot borrows of a resource committed by an earlier step: reading it, modifying it
// (committed and read back), and borrowing it after the same transaction moved it out, which
// aborts. A collection is also forced inside the mutable borrow, which copies the resource
// into the transaction: `bump_fill` shows the heap is sized so the copy collects after its
// first allocation, and the last `bump_churned` step runs two more collections before
// reading, so a stale pointer would read overwritten memory.

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

    public fun publish_at(owner: signer) {
        move_to(&owner, StorageSlotResource { val: vector[1u64, 2, 3] });
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

    public fun take<T: store>(addr: address): T acquires StorageSlotResource {
        let StorageSlotResource { val } = move_from<StorageSlotResource<T>>(addr);
        val
    }
}
module 0x42::m {
    use 0x1::storage_slot;

    enum Unit has drop { Only }

    fun churn(rounds: u64) {
        let counter = 0;
        while (counter < rounds) {
            let junk = vector[counter, counter, counter, counter];
            counter = counter + junk.length();
        };
    }

    public fun read_first(addr: address): (u64, u64) {
        let slot = storage_slot::unchecked_slot<vector<u64>>(addr);
        let vals = storage_slot::borrow(&slot);
        let len = vals.length();
        let first = vals[0];
        storage_slot::destroy_slot(slot);
        (len, first)
    }

    public fun bump_first(addr: address, rounds: u64): (u64, u64, u64) {
        churn(rounds);
        let slot = storage_slot::unchecked_slot<vector<u64>>(addr);
        let vals = storage_slot::borrow_mut(&mut slot);
        vals[0] += 10;
        let len = vals.length();
        let first = vals[0];
        let last = vals[2];
        storage_slot::destroy_slot(slot);
        (len, first, last)
    }

    // Bumps the first element, optionally churns with the reference live, then
    // reads every element.
    public fun bump_churned(addr: address, rounds: u64, more: u64): (u64, u64, u64, u64) {
        churn(rounds);
        let slot = storage_slot::unchecked_slot<vector<u64>>(addr);
        let vals = storage_slot::borrow_mut(&mut slot);
        vals[0] += 10;
        churn(more);
        let len = vals.length();
        let first = vals[0];
        let second = vals[1];
        let third = vals[2];
        storage_slot::destroy_slot(slot);
        (len, first, second, third)
    }

    // One allocation the size of the resource, the copy's first object.
    public fun bump_fill(addr: address, rounds: u64): address {
        churn(rounds);
        let slot = storage_slot::unchecked_slot<vector<u64>>(addr);
        let _first = Unit::Only;
        storage_slot::destroy_slot(slot);
        addr
    }

    public fun take_then_borrow(addr: address): u64 {
        let taken = storage_slot::take<vector<u64>>(addr);
        let slot = storage_slot::unchecked_slot<vector<u64>>(addr);
        let vals = storage_slot::borrow(&slot);
        let out = vals.length() + taken.length();
        storage_slot::destroy_slot(slot);
        out
    }
}

// RUN: execute 0x1::storage_slot::publish_at --args 0x7 --commit
// CHECK: writes: create 0x1::storage_slot::StorageSlotResource<vector<u64>> at 0x7 0x03010000000000000002000000000000000300000000000000

// RUN: execute 0x42::m::read_first --args 0x7
// CHECK: results: 3, 1

// RUN: execute 0x42::m::bump_first --args 0x7, 0 --commit
// CHECK: results: 3, 11, 3 | writes: modify 0x1::storage_slot::StorageSlotResource<vector<u64>> at 0x7 0x030b0000000000000002000000000000000300000000000000

// RUN: execute 0x42::m::read_first --args 0x7
// CHECK: results: 3, 11

// RUN: execute 0x42::m::bump_fill --args 0x7, 2728 --heap-size 32768
// CHECK: results: 0x7
// CHECK-GC-COUNT: 0

// RUN: execute 0x42::m::bump_churned --args 0x7, 2728, 0 --heap-size 32768
// CHECK: results: 3, 21, 2, 3
// CHECK-GC-COUNT: 1

// RUN: execute 0x42::m::bump_churned --args 0x7, 2728, 6800 --heap-size 32768
// CHECK: results: 3, 21, 2, 3
// CHECK-GC-COUNT: 3

// RUN: execute 0x42::m::take_then_borrow --args 0x7
// CHECK: aborted: code 2 (StorageSlotResource at address 0x7 not found) in 0x1::storage_slot
