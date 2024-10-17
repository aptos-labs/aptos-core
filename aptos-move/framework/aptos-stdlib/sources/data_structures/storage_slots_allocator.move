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
/// * having a fee-payer for any storage creation operations
module aptos_std::storage_slots_allocator {
    friend aptos_std::big_ordered_map;

    use aptos_std::table::{Self, Table};
    use std::enum_option::{Self as option, Option};

    const EINVALID_ARGUMENT: u64 = 1;
    const EINTERNAL_INVARIANT_BROKEN: u64 = 7;

    const NULL_INDEX: u64 = 0;
    const INLINE_SLOT_INDEX: u64 = 1;
    const FIRST_INDEX: u64 = 10; // keeping space for new special values

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

    enum StorageSlotsAllocatorConfig has copy, drop {
        V1 {
            should_inline: bool,
            should_reuse: bool,
            num_to_preallocate: u32,
        }
    }

    enum StorageSlotsAllocator<T: store> has store {
        V1 {
            slots: Option<Table<u64, Link<T>>>, // Lazily create slots table only when needed
            new_slot_index: u64,
            should_reuse: bool,
            reuse_head_index: u64,
            reuse_spare_count: u32,
            inline_slot: Option<Link<T>>, // Optionally put one slot directly inline
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

    /// (Weak) Reference to a slot.
    /// We can have variety of `RefToSlot`, but only a single `StoredSlot`.
    /// It is on the caller to make sure references are not used after slot is freed.
    struct RefToSlot has copy, drop, store {
        slot_index: u64,
    }

    public fun new<T: store>(config: StorageSlotsAllocatorConfig): StorageSlotsAllocator<T> {
        let result = StorageSlotsAllocator::V1 {
            slots: option::none(),
            new_slot_index: FIRST_INDEX,
            should_reuse: config.should_reuse,
            reuse_head_index: NULL_INDEX,
            reuse_spare_count: 0,
            inline_slot: option::none(),
        };

        for (i in 0..config.num_to_preallocate) {
            let slot_index = result.next_slot_index();
            result.maybe_push_to_reuse_queue(slot_index);
        };

        if (config.should_inline) {
            result.maybe_push_to_reuse_queue(INLINE_SLOT_INDEX);
        };

        result
    }

    public fun new_default_config(): StorageSlotsAllocatorConfig {
        StorageSlotsAllocatorConfig::V1 {
            should_inline: true,
            should_reuse: false,
            num_to_preallocate: 0,
        }
    }

    public fun new_config(should_inline: bool, should_reuse: bool, num_to_preallocate: u32): StorageSlotsAllocatorConfig {
        StorageSlotsAllocatorConfig::V1 {
            should_inline,
            should_reuse,
            num_to_preallocate,
        }
    }

    public fun add<T: store>(self: &mut StorageSlotsAllocator<T>, val: T): StoredSlot {
        let (stored_slot, reserved_slot) = self.reserve_slot();
        self.fill_reserved_slot(reserved_slot, val);
        stored_slot
    }

    public fun remove<T: store>(self: &mut StorageSlotsAllocator<T>, slot: StoredSlot): T {
        let (reserved_slot, value) = self.remove_and_reserve(slot.stored_as_ref());
        self.free_reserved_slot(reserved_slot, slot);
        value
    }

    public(friend) fun destroy<T: store>(self: StorageSlotsAllocator<T>) {
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
                inline_slot,
            } => {
                inline_slot.destroy_none();
                assert!(reuse_head_index == NULL_INDEX, EINTERNAL_INVARIANT_BROKEN);
                slots.destroy_some().destroy();
            },
        };
    }

    public fun borrow<T: store>(self: &StorageSlotsAllocator<T>, slot: RefToSlot): &T {
        let slot_index = slot.slot_index;

        if (slot_index == INLINE_SLOT_INDEX) {
            return &self.inline_slot.borrow().value;
        };

        &self.slots.borrow().borrow(slot_index).value
    }

    public fun borrow_mut<T: store>(self: &mut StorageSlotsAllocator<T>, slot: RefToSlot): &mut T {
        let slot_index = slot.slot_index;
        if (slot_index == INLINE_SLOT_INDEX) {
            return &mut self.inline_slot.borrow_mut().value;
        };

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
    public fun remove_and_reserve<T: store>(self: &mut StorageSlotsAllocator<T>, slot: RefToSlot): (ReservedSlot, T) {
        let slot_index = slot.slot_index;
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

    public fun reserved_as_ref(self: &ReservedSlot): RefToSlot {
        RefToSlot { slot_index: self.slot_index }
    }

    public fun stored_as_ref(self: &StoredSlot): RefToSlot {
        RefToSlot { slot_index: self.slot_index }
    }

    public fun null_ref(): RefToSlot {
        RefToSlot { slot_index: NULL_INDEX }
    }

    public fun ref_is_null(self: &RefToSlot): bool {
        self.slot_index == NULL_INDEX
    }

    // ========== Section for private internal utility methods ========

    fun maybe_pop_from_reuse_queue<T: store>(self: &mut StorageSlotsAllocator<T>): u64 {
        let slot_index = self.reuse_head_index;
        if (slot_index != NULL_INDEX) {
            let Link::Vacant { next } = self.remove_link(slot_index);
            self.reuse_head_index = next;
            self.reuse_spare_count = self.reuse_spare_count - 1;
        };
        slot_index
    }

    fun maybe_push_to_reuse_queue<T: store>(self: &mut StorageSlotsAllocator<T>, slot_index: u64) {
        if (self.should_reuse || slot_index == INLINE_SLOT_INDEX) {
            self.add_link(slot_index, Link::Vacant { next: self.reuse_head_index });
            self.reuse_head_index = slot_index;
            self.reuse_spare_count = self.reuse_spare_count + 1;
        };
    }

    fun next_slot_index<T: store>(self: &mut StorageSlotsAllocator<T>): u64 {
        let slot_index = self.new_slot_index;
        self.new_slot_index = self.new_slot_index + 1;
        if (self.slots.is_none()) {
            self.slots.fill(table::new<u64, Link<T>>());
        };
        slot_index
    }

    fun add_link<T: store>(self: &mut StorageSlotsAllocator<T>, slot_index: u64, link: Link<T>) {
        if (slot_index == INLINE_SLOT_INDEX) {
            self.inline_slot.fill(link)
        } else {
            self.slots.borrow_mut().add(slot_index, link);
        }
    }

    fun remove_link<T: store>(self: &mut StorageSlotsAllocator<T>, slot_index: u64): Link<T> {
        if (slot_index == INLINE_SLOT_INDEX) {
            self.inline_slot.extract()
        } else {
            self.slots.borrow_mut().remove(slot_index)
        }
    }
}
