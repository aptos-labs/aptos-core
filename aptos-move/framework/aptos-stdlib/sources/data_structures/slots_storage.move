/// Abstraction to having "addressable" storage slots (i.e. items) in global storage.
/// Addresses are local u64 values (unique within a single SlotsStorage instance,
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
module aptos_std::slots_storage {
    use aptos_std::table::{Self, Table};

    const NULL_INDEX: u64 = 0;

    enum Link<T: store> has store {
        Some {
            value: T,
        },
        Empty {
            next: u64,
        }
    }

    enum SlotsStorage<T: store> has store {
        Simple {
            slots: Table<u64, Link<T>>,
            new_slot_index: u64,
        },
        Reuse {
            slots: Table<u64, Link<T>>,
            new_slot_index: u64,
            reuse_head_index: u64,
        },
    }

    struct TransientSlot {
        slot_index: u64,
    }

    public fun new_storage_slots<T: store>(): SlotsStorage<T> {
        SlotsStorage::Simple {
            slots: table::new(),
            new_slot_index: 1,
        }
    }

    public fun new_reuse_storage_slots<T: store>(num_to_preallocate: u64): SlotsStorage<T> {
        let self = SlotsStorage::Reuse {
            slots: table::new(),
            new_slot_index: 1,
            reuse_head_index: NULL_INDEX,
        };

        for (i in 0..num_to_preallocate) {
            let slot_index = self.next_slot_index();
            self.push_to_reuse_queue(slot_index);
        };

        self
    }

    public fun add<T: store>(self: &mut SlotsStorage<T>, val: T): u64 {
        if (self is SlotsStorage::Reuse<T>) {
            let slot_index = self.reuse_head_index;
            if (slot_index != NULL_INDEX) {
                let Link::Empty { next } = self.slots.remove(slot_index);
                self.reuse_head_index = next;
                self.slots.add(slot_index, Link::Some { value: val });
                return slot_index
            };
        };

        let slot_index = self.next_slot_index();
        self.slots.add(slot_index, Link::Some { value: val });
        slot_index
    }

    public fun remove<T: store>(self: &mut SlotsStorage<T>, slot_index: u64): T {
        let Link::Some { value } = self.slots.remove(slot_index);

        self.push_to_reuse_queue(slot_index);

        value
    }

    public fun destroy_empty<T: store>(self: SlotsStorage<T>) {
        match (self) {
            Simple {
                slots,
                new_slot_index: _,
            } => slots.destroy_empty(),
            Reuse {
                slots,
                new_slot_index: _,
                reuse_head_index,
            } => {
                while (reuse_head_index != NULL_INDEX) {
                    let Link::Empty { next } = slots.remove(reuse_head_index);
                    reuse_head_index = next;
                };
                slots.destroy_empty();
            }
        };
    }

    public fun borrow<T: store>(self: &SlotsStorage<T>, slot_index: u64): &T {
        &self.slots.borrow(slot_index).value
    }

    public fun borrow_mut<T: store>(self: &mut SlotsStorage<T>, slot_index: u64): &mut T {
        &mut self.slots.borrow_mut(slot_index).value
    }

    public fun get_index(self: &TransientSlot): u64 {
        self.slot_index
    }

    // splitting add into getting TransientSlot, and then inserting it later
    public fun create_transient_slot<T: store>(self: &mut SlotsStorage<T>): TransientSlot {
        if (self is SlotsStorage::Reuse<T>) {
            let slot_index = self.reuse_head_index;
            if (slot_index != NULL_INDEX) {
                let Link::Empty { next } = self.slots.remove(slot_index);
                self.reuse_head_index = next;
                return TransientSlot {
                    slot_index,
                };
            };
        };

        let slot_index = self.next_slot_index();
        TransientSlot {
            slot_index,
        }
    }

    public fun add_transient_slot<T: store>(self: &mut SlotsStorage<T>, slot: TransientSlot, val: T) {
        let TransientSlot { slot_index } = slot;
        self.slots.add(slot_index, Link::Some { value: val });
    }

    public fun transiently_remove<T: store>(self: &mut SlotsStorage<T>, slot_index: u64): (TransientSlot, T) {
        let Link::Some { value } = self.slots.remove(slot_index);
        (TransientSlot { slot_index }, value)
    }

    public fun destroy_transient_slot<T: store>(self: &mut SlotsStorage<T>, slot: TransientSlot) {
        let TransientSlot { slot_index } = slot;

        self.push_to_reuse_queue(slot_index);
    }

    fun push_to_reuse_queue<T: store>(self: &mut SlotsStorage<T>, slot_index: u64) {
        if (self is SlotsStorage::Reuse<T>) {
            self.slots.add(slot_index, Link::Empty { next: self.reuse_head_index });
            self.reuse_head_index = slot_index;
        }
    }

    fun next_slot_index<T: store>(self: &mut SlotsStorage<T>): u64 {
        let slot_index = self.new_slot_index;
        self.new_slot_index = self.new_slot_index + 1;
        slot_index
    }
}
