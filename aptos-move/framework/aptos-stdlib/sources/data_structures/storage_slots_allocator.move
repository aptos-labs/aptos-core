/// Abstraction to having "addressable" storage slots (i.e. items) in global storage.
/// Addresses are local u64 values (unique within a single StorageSlotsAllocator instance,
/// but can and do overlap across instances).
///
/// Allows optionally to initialize slots (and pay for them upfront), and then reuse them,
/// providing predictable storage costs.
///
/// If we need to mutate multiple slots at the same time, we can workaround borrow_mut preventing us from that,
/// via provided pair of `transiently_remove` and `add_transient_slot` methods, to do so in non-conflicting manner.
///
/// Similarly allows getting an address upfront via `create_transient_slot`, for a slot created
/// later (i.e. if we need address to initialize the value itself).
///
/// In the future, more sophisticated strategies can be added, without breaking/modifying callers,
/// for example:
/// * having one slot embeded into the struct itself
/// * having a fee-payer for any storage creation operations
module aptos_std::storage_slots_allocator {
    friend aptos_std::big_ordered_map;

    use aptos_std::table::{Self, Table};

    const NULL_INDEX: u64 = 0;
    const FIRST_INDEX: u64 = 3;

    enum Link<T: store> has store {
        Occupied {
            value: T,
        },
        Vacant {
            next: u64,
        }
    }

    enum StorageSlotsAllocator<T: store> has store {
        Simple {
            slots: Table<u64, Link<T>>,
            new_slot_index: u64,
        },
        Reuse {
            slots: Table<u64, Link<T>>,
            new_slot_index: u64,
            reuse_head_index: u64,
        },
        // TODO implement variant that inlines first node?
    }

    struct ReservedSlot {
        slot_index: u64,
    }

    public fun new_storage_slots<T: store>(): StorageSlotsAllocator<T> {
        StorageSlotsAllocator::Simple {
            slots: table::new(),
            new_slot_index: FIRST_INDEX,
        }
    }

    public fun new_reuse_storage_slots<T: store>(num_to_preallocate: u64): StorageSlotsAllocator<T> {
        let self = StorageSlotsAllocator::Reuse {
            slots: table::new(),
            new_slot_index: FIRST_INDEX,
            reuse_head_index: NULL_INDEX,
        };

        for (i in 0..num_to_preallocate) {
            let slot_index = self.next_slot_index();
            self.push_to_reuse_queue_if_enabled(slot_index);
        };

        self
    }

    public fun add<T: store>(self: &mut StorageSlotsAllocator<T>, val: T): u64 {
        if (self is StorageSlotsAllocator::Reuse<T>) {
            let slot_index = self.reuse_head_index;
            if (slot_index != NULL_INDEX) {
                let Link::Vacant { next } = self.slots.remove(slot_index);
                self.reuse_head_index = next;
                self.slots.add(slot_index, Link::Occupied { value: val });
                return slot_index
            };
        };

        let slot_index = self.next_slot_index();
        self.slots.add(slot_index, Link::Occupied { value: val });
        slot_index
    }

    public fun remove<T: store>(self: &mut StorageSlotsAllocator<T>, slot_index: u64): T {
        let Link::Occupied { value } = self.slots.remove(slot_index);

        self.push_to_reuse_queue_if_enabled(slot_index);

        value
    }

    public(friend) fun destroy<T: store>(self: StorageSlotsAllocator<T>) {
        match (self) {
            Simple {
                slots,
                new_slot_index: _,
            } => slots.destroy(),
            Reuse {
                slots,
                new_slot_index: _,
                reuse_head_index,
            } => {
                while (reuse_head_index != NULL_INDEX) {
                    let Link::Vacant { next } = slots.remove(reuse_head_index);
                    reuse_head_index = next;
                };
                slots.destroy();
            },
        };
    }

    public fun borrow<T: store>(self: &StorageSlotsAllocator<T>, slot_index: u64): &T {
        &self.slots.borrow(slot_index).value
    }

    public fun borrow_mut<T: store>(self: &mut StorageSlotsAllocator<T>, slot_index: u64): &mut T {
        &mut self.slots.borrow_mut(slot_index).value
    }

    public fun get_index(self: &ReservedSlot): u64 {
        self.slot_index
    }

    // We also provide here operations where `add()` is split into `reserve_slot`,
    // and then doing fill_reserved_slot later.

    // Similarly we have `remove_and_reserve`, and then `fill_reserved_slot` later.

    public fun reserve_slot<T: store>(self: &mut StorageSlotsAllocator<T>): ReservedSlot {
        if (self is StorageSlotsAllocator::Reuse<T>) {
            let slot_index = self.reuse_head_index;
            if (slot_index != NULL_INDEX) {
                let Link::Vacant { next } = self.slots.remove(slot_index);
                self.reuse_head_index = next;
                return ReservedSlot {
                    slot_index,
                };
            };
        };

        let slot_index = self.next_slot_index();
        ReservedSlot {
            slot_index,
        }
    }

    public fun fill_reserved_slot<T: store>(self: &mut StorageSlotsAllocator<T>, slot: ReservedSlot, val: T) {
        let ReservedSlot { slot_index } = slot;
        self.slots.add(slot_index, Link::Occupied { value: val });
    }

    /// Remove storage slot, but reserve it for later.
    public fun remove_and_reserve<T: store>(self: &mut StorageSlotsAllocator<T>, slot_index: u64): (ReservedSlot, T) {
        let Link::Occupied { value } = self.slots.remove(slot_index);
        (ReservedSlot { slot_index }, value)
    }

    public fun free_reserved_slot<T: store>(self: &mut StorageSlotsAllocator<T>, slot: ReservedSlot) {
        let ReservedSlot { slot_index } = slot;
        self.push_to_reuse_queue_if_enabled(slot_index);
    }

    fun push_to_reuse_queue_if_enabled<T: store>(self: &mut StorageSlotsAllocator<T>, slot_index: u64) {
        if (self is StorageSlotsAllocator::Reuse<T>) {
            self.slots.add(slot_index, Link::Vacant { next: self.reuse_head_index });
            self.reuse_head_index = slot_index;
        };
    }

    fun next_slot_index<T: store>(self: &mut StorageSlotsAllocator<T>): u64 {
        let slot_index = self.new_slot_index;
        self.new_slot_index = self.new_slot_index + 1;
        slot_index
    }
}
