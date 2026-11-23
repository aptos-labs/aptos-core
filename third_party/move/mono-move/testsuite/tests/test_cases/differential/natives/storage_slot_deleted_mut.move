// A mutable storage-slot borrow of a committed resource that the same transaction moved
// out first aborts with code 2 (not found). Without the move_from the borrow succeeds, and
// it also succeeds after the resource is moved out and published again.

// RUN: publish
module 0x1::storage_slot {
    struct StorageSlotResource<T> has key {
        val: T
    }

    struct StorageSlot<phantom T> has store {
        addr: address
    }

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

    public fun borrow_mut<T: store>(slot: &mut StorageSlot<T>): &mut T {
        &mut borrow_storage_slot_resource_mut<T, StorageSlotResource<T>>(slot).val
    }

    public fun take<T: store>(addr: address): T acquires StorageSlotResource {
        let StorageSlotResource { val } = move_from<StorageSlotResource<T>>(addr);
        val
    }

    public fun put_back<T: store>(owner: &signer, val: T) {
        move_to(owner, StorageSlotResource { val });
    }
}
module 0x42::m {
    use 0x1::storage_slot;

    fun bump(owner_addr: address): (u64, u64) {
        let slot = storage_slot::unchecked_slot<vector<u64>>(owner_addr);
        let values = storage_slot::borrow_mut(&mut slot);
        values[0] += 10;
        let len = values.length();
        let first = values[0];
        storage_slot::destroy_slot(slot);
        (len, first)
    }

    public fun bump_present(owner_addr: address): (u64, u64) {
        bump(owner_addr)
    }

    public fun take_then_bump(owner_addr: address): (u64, u64, u64) {
        let taken = storage_slot::take<vector<u64>>(owner_addr);
        let (len, first) = bump(owner_addr);
        (len, first, taken.length())
    }

    public fun take_restore_then_bump(owner: signer): (u64, u64) {
        let owner_addr = std::signer::address_of(&owner);
        let taken = storage_slot::take<vector<u64>>(owner_addr);
        taken.push_back(4);
        storage_slot::put_back(&owner, taken);
        bump(owner_addr)
    }
}

// RUN: execute 0x1::storage_slot::publish_at --args 0x7 --commit
// CHECK: writes: create 0x1::storage_slot::StorageSlotResource<vector<u64>> at 0x7 0x03010000000000000002000000000000000300000000000000

// RUN: execute 0x42::m::bump_present --args 0x7
// CHECK: results: 3, 11

// RUN: execute 0x42::m::take_then_bump --args 0x7
// CHECK: aborted: code 2 (StorageSlotResource at address 0x7 not found) in 0x1::storage_slot

// RUN: execute 0x42::m::take_restore_then_bump --args 0x7
// CHECK: results: 4, 11
