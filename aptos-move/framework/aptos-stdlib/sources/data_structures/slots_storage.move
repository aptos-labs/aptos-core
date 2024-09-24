

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

    struct SlotsStorage<T: store> has store {
        slots: Table<u64, Link<T>>,
        new_slot_index: u64,
        never_deallocate: bool,
        reuse_head_index: u64,
    }

    struct TransientSlot {
        slot_index: u64,
    }

    public fun new_storage_slots<T: store>(): SlotsStorage<T> {
        SlotsStorage {
            slots: table::new(),
            new_slot_index: 1,
            never_deallocate: false,
            reuse_head_index: NULL_INDEX,
        }
    }

    public fun add<T: store>(self: &mut SlotsStorage<T>, val: T): u64 {
        let slot_index = self.new_slot_index;
        self.new_slot_index = self.new_slot_index + 1;
        self.slots.add(slot_index, Link::Some { value: val });
        slot_index
    }

    public fun remove<T: store>(self: &mut SlotsStorage<T>, slot_index: u64): T {
        let Link::Some { value } = self.slots.remove(slot_index);
        value
    }

    public fun destroy_empty<T: store>(self: SlotsStorage<T>) {
        let SlotsStorage {
            slots,
            new_slot_index: _,
            never_deallocate: _,
            reuse_head_index: _,
        } = self;
        slots.destroy_empty();
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
        let slot_index = self.new_slot_index;
        self.new_slot_index = self.new_slot_index + 1;
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

    public fun destroy_transient_slot(self: TransientSlot) {
        let TransientSlot { slot_index: _ } = self;
    }
}
