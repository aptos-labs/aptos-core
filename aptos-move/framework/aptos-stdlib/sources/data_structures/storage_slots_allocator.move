/// Abstraction to having "addressable" storage slots (i.e. items) in global storage.
/// Addresses are local u64 values (unique within a single StorageSlotsAllocator instance,
/// but can and do overlap across instances).
///
/// Allows optionally to initialize slots (and pay for them upfront), and then reuse them,
/// providing predictable storage costs.
///
/// If we need to mutate multiple slots at the same time, we can workaround borrow_mut preventing us from that,
/// via provided pair of `remove_and_reserve` and `fill_reserved_slot` methods, to do so in non-conflicting manner.
///
/// Similarly allows getting an address upfront via `reserve_slot`, for a slot created
/// later (i.e. if we need address to initialize the value itself).
///
/// In the future, more sophisticated strategies can be added, without breaking/modifying callers,
/// for example:
/// * inlining some nodes
/// * having a fee-payer for any storage creation operations
module aptos_std::storage_slots_allocator {
    use std::error;
    use aptos_std::table_with_length::{Self, TableWithLength};
    use std::option::{Self, Option};

    const EINVALID_ARGUMENT: u64 = 1;
    const ECANNOT_HAVE_SPARES_WITHOUT_REUSE: u64 = 2;
    const EINTERNAL_INVARIANT_BROKEN: u64 = 7;

    const NULL_INDEX: u64 = 0;
    const FIRST_INDEX: u64 = 10; // keeping space for usecase-specific values

    /// Data stored in an individual slot
    enum Link<T: store> has store {
        /// Variant that stores actual data
        Occupied {
            value: T,
        },
        /// Empty variant (that keeps storage item from being deleted)
        /// and represents a node in a linked list of empty slots.
        Vacant {
            next: u64,
        }
    }

    enum StorageSlotsAllocator<T: store> has store {
        // V1 is sequential - any two operations on the StorageSlotsAllocator will conflict.
        // In general, StorageSlotsAllocator is invoked on less frequent operations, so
        // that shouldn't be a big issue.
        V1 {
            slots: Option<TableWithLength<u64, Link<T>>>, // Lazily create slots table only when needed
            new_slot_index: u64,
            should_reuse: bool,
            reuse_head_index: u64,
            reuse_spare_count: u32,
        },
    }

    /// Handle to a reserved slot within a transaction.
    /// Not copy/drop/store-able, to guarantee reservation
    /// is used or released within the transaction.
    struct ReservedSlot {
        slot_index: u64,
    }

    /// Ownership handle to a slot.
    /// Not copy/drop-able to make sure slots are released when not needed,
    /// and there is unique owner for each slot.
    struct StoredSlot has store {
        slot_index: u64,
    }

    public fun new<T: store>(should_reuse: bool): StorageSlotsAllocator<T> {
        StorageSlotsAllocator::V1 {
            slots: option::none(),
            new_slot_index: FIRST_INDEX,
            should_reuse,
            reuse_head_index: NULL_INDEX,
            reuse_spare_count: 0,
        }
    }

    public fun allocate_spare_slots<T: store>(self: &mut StorageSlotsAllocator<T>, num_to_allocate: u64) {
        assert!(self.should_reuse, error::invalid_argument(ECANNOT_HAVE_SPARES_WITHOUT_REUSE));
        for (i in 0..num_to_allocate) {
            let slot_index = self.next_slot_index();
            self.maybe_push_to_reuse_queue(slot_index);
        };
    }

    public fun get_num_spare_slot_count<T: store>(self: &StorageSlotsAllocator<T>): u32 {
        assert!(self.should_reuse, error::invalid_argument(ECANNOT_HAVE_SPARES_WITHOUT_REUSE));
        self.reuse_spare_count
    }

    public fun add<T: store>(self: &mut StorageSlotsAllocator<T>, val: T): StoredSlot {
        let (stored_slot, reserved_slot) = self.reserve_slot();
        self.fill_reserved_slot(reserved_slot, val);
        stored_slot
    }

    public fun remove<T: store>(self: &mut StorageSlotsAllocator<T>, slot: StoredSlot): T {
        let (reserved_slot, value) = self.remove_and_reserve(slot.stored_to_index());
        self.free_reserved_slot(reserved_slot, slot);
        value
    }

    public fun destroy_empty<T: store>(self: StorageSlotsAllocator<T>) {
        loop {
            let reuse_index = self.maybe_pop_from_reuse_queue();
            if (reuse_index == NULL_INDEX) {
                break;
            };
        };
        match (self) {
            V1 {
                slots,
                new_slot_index: _,
                should_reuse: _,
                reuse_head_index,
                reuse_spare_count: _,
            } => {
                assert!(reuse_head_index == NULL_INDEX, EINTERNAL_INVARIANT_BROKEN);
                if (slots.is_some()) {
                    slots.destroy_some().destroy_empty();
                } else {
                    slots.destroy_none();
                }
            },
        };
    }

    public fun borrow<T: store>(self: &StorageSlotsAllocator<T>, slot_index: u64): &T {
        &self.slots.borrow().borrow(slot_index).value
    }

    public fun borrow_mut<T: store>(self: &mut StorageSlotsAllocator<T>, slot_index: u64): &mut T {
        &mut self.slots.borrow_mut().borrow_mut(slot_index).value
    }

    // We also provide here operations where `add()` is split into `reserve_slot`,
    // and then doing fill_reserved_slot later.

    // Similarly we have `remove_and_reserve`, and then `fill_reserved_slot` later.

    public fun reserve_slot<T: store>(self: &mut StorageSlotsAllocator<T>): (StoredSlot, ReservedSlot) {
        let slot_index = self.maybe_pop_from_reuse_queue();
        if (slot_index == NULL_INDEX) {
            slot_index = self.next_slot_index();
        };

        (
            StoredSlot { slot_index },
            ReservedSlot { slot_index },
        )
    }

    public fun fill_reserved_slot<T: store>(self: &mut StorageSlotsAllocator<T>, slot: ReservedSlot, val: T) {
        let ReservedSlot { slot_index } = slot;
        self.add_link(slot_index, Link::Occupied { value: val });
    }

    /// Remove storage slot, but reserve it for later.
    public fun remove_and_reserve<T: store>(self: &mut StorageSlotsAllocator<T>, slot_index: u64): (ReservedSlot, T) {
        let Link::Occupied { value } = self.remove_link(slot_index);
        (ReservedSlot { slot_index }, value)
    }

    public fun free_reserved_slot<T: store>(self: &mut StorageSlotsAllocator<T>, reserved_slot: ReservedSlot, stored_slot: StoredSlot) {
        let ReservedSlot { slot_index } = reserved_slot;
        assert!(slot_index == stored_slot.slot_index, EINVALID_ARGUMENT);
        let StoredSlot { slot_index: _ } = stored_slot;
        self.maybe_push_to_reuse_queue(slot_index);
    }

    // ========== Section for methods handling references ========

    public fun reserved_to_index(self: &ReservedSlot): u64 {
        self.slot_index
    }

    public fun stored_to_index(self: &StoredSlot): u64 {
        self.slot_index
    }

    public fun is_null_index(slot_index: u64): bool {
        slot_index == NULL_INDEX
    }

    public fun is_special_unused_index(slot_index: u64): bool {
        slot_index != NULL_INDEX && slot_index < FIRST_INDEX
    }

    // ========== Section for private internal utility methods ========

    fun maybe_pop_from_reuse_queue<T: store>(self: &mut StorageSlotsAllocator<T>): u64 {
        let slot_index = self.reuse_head_index;
        if (slot_index != NULL_INDEX) {
            let Link::Vacant { next } = self.remove_link(slot_index);
            self.reuse_head_index = next;
            self.reuse_spare_count -= 1;
        };
        slot_index
    }

    fun maybe_push_to_reuse_queue<T: store>(self: &mut StorageSlotsAllocator<T>, slot_index: u64) {
        if (self.should_reuse) {
            let link = Link::Vacant { next: self.reuse_head_index };
            self.add_link(slot_index, link);
            self.reuse_head_index = slot_index;
            self.reuse_spare_count += 1;
        };
    }

    fun next_slot_index<T: store>(self: &mut StorageSlotsAllocator<T>): u64 {
        let slot_index = self.new_slot_index;
        self.new_slot_index += 1;
        if (self.slots.is_none()) {
            self.slots.fill(table_with_length::new<u64, Link<T>>());
        };
        slot_index
    }

    fun add_link<T: store>(self: &mut StorageSlotsAllocator<T>, slot_index: u64, link: Link<T>) {
        self.slots.borrow_mut().add(slot_index, link);
    }

    fun remove_link<T: store>(self: &mut StorageSlotsAllocator<T>, slot_index: u64): Link<T> {
        self.slots.borrow_mut().remove(slot_index)
    }

    spec module {
        pragma verify = false;
    }
}
