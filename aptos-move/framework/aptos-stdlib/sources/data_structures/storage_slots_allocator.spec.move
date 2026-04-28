spec aptos_std::storage_slots_allocator {

    spec free_reserved_slot<T: store>(
        self: &mut StorageSlotsAllocator<T>,
        reserved_slot: ReservedSlot,
        stored_slot: StoredSlot,
    ) {
        aborts_if reserved_slot.slot_index != stored_slot.slot_index;
        // add_link aborts if slots is None when should_reuse is true
        aborts_if self.should_reuse && self.slots.is_none();
        pragma aborts_if_is_partial;
    }


    // -------------------------------------------
    // Complex functions: loop / enum pattern match
    // -------------------------------------------

    spec allocate_spare_slots<T: store>(self: &mut StorageSlotsAllocator<T>, num_to_allocate: u64) {
        aborts_if !self.should_reuse;
        pragma aborts_if_is_partial;
    }

    spec add<T: store>(self: &mut StorageSlotsAllocator<T>, val: T): StoredSlot {
        pragma opaque;
        // Overflows when reuse queue empty and new_slot_index wraps
        aborts_if self.reuse_head_index == 0 && self.new_slot_index + 1 > MAX_U64;
        pragma aborts_if_is_partial;
        ensures self.slots.is_some();
        ensures table_with_length::spec_contains(option::borrow(self.slots), result.slot_index);
        ensures table_with_length::spec_get(option::borrow(self.slots), result.slot_index) is Link::Occupied;
        ensures table_with_length::spec_get(option::borrow(self.slots), result.slot_index).value == val;
    }

    spec remove<T: store>(self: &mut StorageSlotsAllocator<T>, slot: StoredSlot): T {
        pragma opaque;
        aborts_if self.slots.is_none();
        aborts_if !table_with_length::spec_contains(option::borrow(self.slots), slot.slot_index);
        aborts_if !(table_with_length::spec_get(option::borrow(self.slots), slot.slot_index) is Link::Occupied);
        pragma aborts_if_is_partial;
        ensures result == old(table_with_length::spec_get(option::borrow(self.slots), slot.slot_index)).value;
        // slots is still initialised after remove (slot becomes Vacant, not deleted)
        ensures self.slots.is_some();
    }

    spec borrow<T: store>(self: &StorageSlotsAllocator<T>, slot_index: u64): &T {
        pragma opaque;
        aborts_if self.slots.is_none();
        aborts_if !table_with_length::spec_contains(option::borrow(self.slots), slot_index);
        aborts_if !(table_with_length::spec_get(option::borrow(self.slots), slot_index) is Link::Occupied);
        ensures result == table_with_length::spec_get(option::borrow(self.slots), slot_index).value;
    }

    spec borrow_mut<T: store>(self: &mut StorageSlotsAllocator<T>, slot_index: u64): &mut T {
        pragma opaque;
        aborts_if self.slots.is_none();
        aborts_if !table_with_length::spec_contains(option::borrow(self.slots), slot_index);
        aborts_if !(table_with_length::spec_get(option::borrow(self.slots), slot_index) is Link::Occupied);
        ensures result == table_with_length::spec_get(option::borrow(self.slots), slot_index).value;
    }

    spec reserve_slot<T: store>(self: &mut StorageSlotsAllocator<T>): (StoredSlot, ReservedSlot) {
        // Overflows in next_slot_index when reuse queue is empty
        aborts_if self.reuse_head_index == 0 && self.new_slot_index + 1 > MAX_U64;
        pragma aborts_if_is_partial;
        ensures result_1.slot_index == result_2.slot_index;
    }

    spec fill_reserved_slot<T: store>(self: &mut StorageSlotsAllocator<T>, slot: ReservedSlot, val: T) {
        pragma opaque;
        aborts_if self.slots.is_none();
        aborts_if table_with_length::spec_contains(option::borrow(self.slots), slot.slot_index);
        ensures table_with_length::spec_contains(option::borrow(self.slots), slot.slot_index);
        ensures table_with_length::spec_get(option::borrow(self.slots), slot.slot_index) is Link::Occupied;
        ensures table_with_length::spec_get(option::borrow(self.slots), slot.slot_index).value == val;
        ensures self.slots.is_some();
    }

    spec remove_and_reserve<T: store>(self: &mut StorageSlotsAllocator<T>, slot_index: u64): (ReservedSlot, T) {
        pragma opaque;
        aborts_if self.slots.is_none();
        aborts_if !table_with_length::spec_contains(option::borrow(self.slots), slot_index);
        aborts_if !(table_with_length::spec_get(option::borrow(self.slots), slot_index) is Link::Occupied);
        ensures result_1.slot_index == slot_index;
        ensures result_2 == old(table_with_length::spec_get(option::borrow(self.slots), slot_index)).value;
        ensures self.slots.is_some();
    }
}
