spec aptos_std::storage_slots_allocator {

    spec free_reserved_slot<T: store>(
        self: &mut StorageSlotsAllocator<T>,
        reserved_slot: ReservedSlot,
        stored_slot: StoredSlot
    ) {
        use 0x1::option;
        use 0x1::table_with_length;
        pragma opaque = true;
        aborts_if reserved_slot.slot_index != stored_slot.slot_index;
        // Releasing a reservation delegates to maybe_push_to_reuse_queue.
        aborts_if self.should_reuse && self.slots.is_none();
        aborts_if self.should_reuse
            && self.slots.is_some()
            && table_with_length::spec_contains(
                option::borrow(self.slots), reserved_slot.slot_index
            );
        aborts_if self.should_reuse && self.reuse_spare_count == MAX_U32;
        pragma aborts_if_is_partial = false;
        ensures !old(self).should_reuse ==> self == old(self);
        ensures old(self).should_reuse ==>
            ensures_of<maybe_push_to_reuse_queue<T>> (
                old(self), reserved_slot.slot_index, self
            );
    }

    // -------------------------------------------
    // Complex functions: loop / enum pattern match
    // -------------------------------------------

    spec allocate_spare_slots<T: store>(
        self: &mut StorageSlotsAllocator<T>, num_to_allocate: u64
    ) {
        aborts_if !self.should_reuse;
        pragma aborts_if_is_partial;
        pragma opaque = true;
        ensures [inferred = sathard] old(self).should_reuse ==>
            (
                forall x: u64:
                    S1.. |~ x < num_to_allocate ==> {
                        let a = ..S1 |~ result_of<next_slot_index<T>> (self);
                        ensures_of<maybe_push_to_reuse_queue<T>> (self, a)
                    }
            );
        ensures [inferred = sathard] old(self).should_reuse ==>
            (forall x: StorageSlotsAllocator<T> : self == x);
        ensures [inferred = sathard]!old(self).should_reuse ==> self == old(self);
        aborts_if [inferred = sathard]!self.should_reuse;
        aborts_if [inferred = sathard] self.should_reuse
            && num_to_allocate == 18446744073709551616;
    }

    spec add<T: store>(self: &mut StorageSlotsAllocator<T>, val: T): StoredSlot {
        use 0x1::option;
        use 0x1::table_with_length;
        pragma opaque;
        // Overflows when reuse queue empty and new_slot_index wraps
        aborts_if self.reuse_head_index == 0 && self.new_slot_index + 1 > MAX_U64;
        pragma aborts_if_is_partial;
        ensures self.slots.is_some();
        ensures table_with_length::spec_contains(
            option::borrow(self.slots), result.slot_index
        );
        ensures table_with_length::spec_get(option::borrow(self.slots), result.slot_index) is Link::Occupied;
        ensures table_with_length::spec_get(
            option::borrow(self.slots), result.slot_index
        ).value == val;
        ensures [inferred]({
            let (a_0, a_1) = ..S1 |~ result_of<reserve_slot<T>> (self);
            option::is_some<table_with_length::TableWithLength<u64, Link<T>>>(self.slots)
                && (
                    table_with_length::spec_contains<u64, Link<T>>(
                        option::borrow<table_with_length::TableWithLength<u64, Link<T>>>(
                            self.slots
                        ),
                        a_0.slot_index
                    )
                        && (
                            (
                                table_with_length::spec_get<u64,
                                Link<T>> (
                                    option::borrow<table_with_length::TableWithLength<u64,
                                    Link<T>> >(self.slots),
                                    a_0.slot_index
                                ) is Occupied
                            )
                                && table_with_length::spec_get<u64, Link<T>>(
                                    option::borrow<table_with_length::TableWithLength<u64, Link<T>>>(
                                        self.slots
                                    ),
                                    a_0.slot_index
                                ).value == val
                        )
                ) ==> result == a_0
        });
        ensures [inferred]({
            let (a_0, a_1) = ..S1 |~ result_of<reserve_slot<T>> (self);
            S1.. |~ ensures_of<fill_reserved_slot<T>> (self, a_1, val, self)
        });
    }

    spec remove<T: store>(
        self: &mut StorageSlotsAllocator<T>, slot: StoredSlot
    ): T {
        pragma opaque;
        aborts_if self.slots.is_none();
        aborts_if !table_with_length::spec_contains(
            option::borrow(self.slots), slot.slot_index
        );
        aborts_if !(
            table_with_length::spec_get(option::borrow(self.slots), slot.slot_index) is Link::Occupied
        );
        aborts_if self.should_reuse && self.reuse_spare_count == MAX_U32;
        pragma aborts_if_is_partial = false;
        ensures result
            == old(
                table_with_length::spec_get(option::borrow(self.slots), slot.slot_index)
            ).value;
        // slots is still initialised after remove (slot becomes Vacant, not deleted)
        ensures self.slots.is_some();
    }

    spec borrow<T: store>(
        self: &StorageSlotsAllocator<T>, slot_index: u64
    ): &T {
        pragma opaque;
        aborts_if self.slots.is_none();
        aborts_if !table_with_length::spec_contains(
            option::borrow(self.slots), slot_index
        );
        aborts_if !(
            table_with_length::spec_get(option::borrow(self.slots), slot_index) is Link::Occupied
        );
        ensures result
            == table_with_length::spec_get(option::borrow(self.slots), slot_index).value;
    }

    spec borrow_mut<T: store>(
        self: &mut StorageSlotsAllocator<T>, slot_index: u64
    ): &mut T {
        pragma opaque;
        aborts_if self.slots.is_none();
        aborts_if !table_with_length::spec_contains(
            option::borrow(self.slots), slot_index
        );
        aborts_if !(
            table_with_length::spec_get(option::borrow(self.slots), slot_index) is Link::Occupied
        );
        ensures result
            == table_with_length::spec_get(option::borrow(self.slots), slot_index).value;
    }

    spec reserve_slot<T: store>(self: &mut StorageSlotsAllocator<T>)
        : (StoredSlot, ReservedSlot) {
        pragma opaque = true;
        // Popping a reusable slot requires an initialized table entry holding
        // a vacant link and a nonzero spare count.
        aborts_if self.reuse_head_index != 0 && self.slots.is_none();
        aborts_if self.reuse_head_index != 0
            && self.slots.is_some()
            && !table_with_length::spec_contains(
                option::borrow(self.slots), self.reuse_head_index
            );
        aborts_if self.reuse_head_index != 0
            && self.slots.is_some()
            && table_with_length::spec_contains(
                option::borrow(self.slots), self.reuse_head_index
            )
            && !(
                table_with_length::spec_get(
                    option::borrow(self.slots), self.reuse_head_index
                ) is Link::Vacant
            );
        aborts_if self.reuse_head_index != 0 && self.reuse_spare_count == 0;
        // A fresh allocation increments new_slot_index.
        aborts_if self.reuse_head_index == 0 && self.new_slot_index == MAX_U64;
        pragma aborts_if_is_partial = false;
        ensures result_1.slot_index == result_2.slot_index;
    }

    spec fill_reserved_slot<T: store>(
        self: &mut StorageSlotsAllocator<T>, slot: ReservedSlot, val: T
    ) {
        pragma opaque;
        aborts_if self.slots.is_none();
        aborts_if table_with_length::spec_contains(
            option::borrow(self.slots), slot.slot_index
        );
        ensures table_with_length::spec_contains(
            option::borrow(self.slots), slot.slot_index
        );
        ensures table_with_length::spec_get(option::borrow(self.slots), slot.slot_index) is Link::Occupied;
        ensures table_with_length::spec_get(option::borrow(self.slots), slot.slot_index).value
            == val;
        ensures self.slots.is_some();
    }

    spec remove_and_reserve<T: store>(
        self: &mut StorageSlotsAllocator<T>, slot_index: u64
    ): (ReservedSlot, T) {
        use 0x1::option;
        use 0x1::table_with_length;
        pragma opaque;
        pragma aborts_if_is_partial = false;
        aborts_if self.slots.is_none();
        aborts_if !table_with_length::spec_contains(
            option::borrow(self.slots), slot_index
        );
        aborts_if !(
            table_with_length::spec_get(option::borrow(self.slots), slot_index) is Link::Occupied
        );
        ensures result_1.slot_index == slot_index;
        ensures result_2
            == old(table_with_length::spec_get(option::borrow(self.slots), slot_index)).value;
        ensures self.new_slot_index == old(self).new_slot_index;
        ensures self.should_reuse == old(self).should_reuse;
        ensures self.reuse_head_index == old(self).reuse_head_index;
        ensures self.reuse_spare_count == old(self).reuse_spare_count;
        ensures self.slots.is_some();
        ensures !table_with_length::spec_contains(
            option::borrow(self.slots), slot_index
        );
        ensures forall i: u64 where i != slot_index:
            table_with_length::spec_contains(option::borrow(self.slots), i)
                == table_with_length::spec_contains(option::borrow(old(self).slots), i);
        ensures forall i: u64 where i != slot_index
            && table_with_length::spec_contains(option::borrow(old(self).slots), i):
            table_with_length::spec_get(option::borrow(self.slots), i)
                == table_with_length::spec_get(option::borrow(old(self).slots), i);
    }

    spec new<T: store>(should_reuse: bool)
        : 0x1::storage_slots_allocator::StorageSlotsAllocator<T> {
        use 0x1::option;
        use 0x1::table_with_length;
        pragma opaque = true;
        ensures [inferred] result
            == StorageSlotsAllocator::V1<T> {
                slots: option::none<table_with_length::TableWithLength<u64, Link<T>>>(),
                new_slot_index: 10,
                should_reuse: should_reuse,
                reuse_head_index: 0,
                reuse_spare_count: 0
            };
        aborts_if [inferred] false;
    }

    spec destroy_empty<T: store>(
        self: 0x1::storage_slots_allocator::StorageSlotsAllocator<T>
    ) {
        use 0x1::option;
        use 0x1::table_with_length;
        pragma opaque = true, aborts_if_is_partial = false;
        ensures [inferred = vacuous] forall x: StorageSlotsAllocator<T> :
            (..S1 |~ result_of<maybe_pop_from_reuse_queue<T>> (x) == 0)
                && (
                    (x is V1)
                        && (
                            x.reuse_head_index == 0
                                && option::is_some<table_with_length::TableWithLength<u64, Link<T>>>(
                                    x.slots
                                )
                        )
                ) ==>
                (
                    S1.. |~ ensures_of<table_with_length::destroy_empty<u64,
                    Link<T>> >(
                        option::destroy_some<table_with_length::TableWithLength<u64,
                        Link<T>> >(x.slots)
                    )
                );
        ensures [inferred = vacuous] forall x: StorageSlotsAllocator<T> :
            (..S1 |~ result_of<maybe_pop_from_reuse_queue<T>> (x) == 0)
                && (
                    (x is V1)
                        && (
                            x.reuse_head_index == 0
                                && !option::is_some<table_with_length::TableWithLength<u64, Link<T>>>(
                                    x.slots
                                )
                        )
                ) ==>
                (
                    S1.. |~ ensures_of<option::destroy_none<table_with_length::TableWithLength<u64,
                    Link<T>> >> (x.slots)
                );
        aborts_if [inferred = vacuous]..S1 |~(
            exists x: StorageSlotsAllocator<T> : result_of<maybe_pop_from_reuse_queue<T>> (x)
            == 0 && ((x is V1) && x.reuse_head_index != 0)
        );
        aborts_if [inferred = vacuous] exists x: StorageSlotsAllocator<T> :
            (..S1 |~ result_of<maybe_pop_from_reuse_queue<T>> (x) == 0)
                && (
                    (x is V1)
                        && (
                            x.reuse_head_index == 0
                                && (
                                    option::is_some<table_with_length::TableWithLength<u64, Link<T>>>(
                                        x.slots
                                    )
                                        && aborts_of<option::destroy_some<table_with_length::TableWithLength<u64,
                                        Link<T>> >> (x.slots)
                                )
                        )
                );
        aborts_if [inferred = vacuous] exists x: StorageSlotsAllocator<T> :
            (..S1 |~ result_of<maybe_pop_from_reuse_queue<T>> (x) == 0)
                && (
                    (x is V1)
                        && (
                            x.reuse_head_index == 0
                                && (
                                    !option::is_some<table_with_length::TableWithLength<u64, Link<T>>>(
                                        x.slots
                                    )
                                        && (
                                            S1 |~ aborts_of<option::destroy_none<table_with_length::TableWithLength<u64,
                                            Link<T>> >> (x.slots)
                                        )
                                )
                        )
                );
    }

    spec add_link<T: store>(
        self: &mut 0x1::storage_slots_allocator::StorageSlotsAllocator<T>,
        slot_index: u64,
        link: 0x1::storage_slots_allocator::Link<T>
    ) {
        use 0x1::option;
        use 0x1::table_with_length;
        pragma opaque = true, aborts_if_is_partial = false;
        ensures self
            == update_field(
                old(self),
                slots,
                update_field(
                    old(self).slots,
                    e,
                    table_with_length::spec_set(
                        option::borrow(old(self).slots), slot_index, link
                    )
                )
            );
        /* The following raw WP clauses are superseded by the exact
           source-level transformer above.
        ensures [inferred] self
            == update_field(
                old(self),
                slots,
                update_field(self.slots, e, self.slots.e)
            );
        ensures [inferred]({
            let a =
                ..S1 |~ result_of<option::borrow_mut<table_with_length::TableWithLength<u64, Link<T
                    >> >> (
                    update_field(
                        old(self), slots, update_field(self.slots, e, self.slots.e)
                    ).slots
                );
            S1.. |~ ensures_of<table_with_length::add<u64, Link<T>> >(
                a, slot_index, link, self.slots.e
            )
        });
        */
        aborts_if [inferred] aborts_of<option::borrow_mut<table_with_length::TableWithLength<u64, Link<T
            >> >> (self.slots);
        aborts_if option::is_some(self.slots)
            && table_with_length::spec_contains(option::borrow(self.slots), slot_index);
    }

    spec get_num_spare_slot_count<T: store>(
        self: &0x1::storage_slots_allocator::StorageSlotsAllocator<T>
    ): u32 {
        pragma opaque = true;
        ensures [inferred] self.should_reuse ==>
            result == self.reuse_spare_count;
        aborts_if [inferred]!self.should_reuse;
    }

    spec is_null_index(slot_index: u64): bool {
        pragma opaque = true;
        ensures [inferred] result == (slot_index == 0);
        aborts_if [inferred] false;
    }

    spec is_special_unused_index(slot_index: u64): bool {
        pragma opaque = true;
        ensures [inferred] slot_index != 0 ==> result == (slot_index < 10);
        ensures [inferred] slot_index == 0 ==> result == false;
        aborts_if [inferred] false;
    }

    spec maybe_pop_from_reuse_queue<T: store>(
        self: &mut 0x1::storage_slots_allocator::StorageSlotsAllocator<T>
    ): u64 {
        use 0x1::option;
        use 0x1::table_with_length;
        pragma opaque = true, aborts_if_is_partial = false;
        ensures [inferred] result == old(self).reuse_head_index;
        ensures [inferred] old(self).reuse_head_index != 0 ==>
            self
                == update_field(
                    update_field(
                        self,
                        reuse_head_index,
                        result_of<remove_link<T>> (self, old(self).reuse_head_index).next
                    ),
                    reuse_spare_count,
                    update_field(
                        self,
                        reuse_head_index,
                        result_of<remove_link<T>> (self, old(self).reuse_head_index).next
                    ).reuse_spare_count - 1
                );
        ensures [inferred] old(self).reuse_head_index == 0 ==>
            self == old(self);
        aborts_if [inferred] self.reuse_head_index != 0
            && update_field(
                self,
                reuse_head_index,
                result_of<remove_link<T>> (self, self.reuse_head_index).next
            ).reuse_spare_count == 0;
        aborts_if [inferred] self.reuse_head_index != 0
            && (result_of<remove_link<T>> (self, self.reuse_head_index) is Occupied);
        aborts_if self.reuse_head_index != 0 && self.slots.is_none();
        aborts_if self.reuse_head_index != 0
            && self.slots.is_some()
            && !table_with_length::spec_contains(
                option::borrow(self.slots), self.reuse_head_index
            );
    }

    spec maybe_push_to_reuse_queue<T: store>(
        self: &mut 0x1::storage_slots_allocator::StorageSlotsAllocator<T>, slot_index: u64
    ) {
        use 0x1::option;
        use 0x1::table_with_length;
        pragma opaque = true, aborts_if_is_partial = false;
        ensures old(self).should_reuse ==>
            ({
                let pre = old(self);
                let with_link =
                    update_field(
                        pre,
                        slots,
                        update_field(
                            pre.slots,
                            e,
                            table_with_length::spec_set(
                                option::borrow(pre.slots),
                                slot_index,
                                Link::Vacant<T> { next: pre.reuse_head_index }
                            )
                        )
                    );
                self
                    == update_field(
                        update_field(with_link, reuse_head_index, slot_index),
                        reuse_spare_count,
                        pre.reuse_spare_count + 1
                    )
            });
        ensures [inferred]!old(self).should_reuse ==> self == old(self);
        /* The raw WP clause below is superseded by the exact source-level
           transformer above. It is retained temporarily as provenance while
           the remaining allocator contracts are simplified.
        ensures [inferred] old(self).should_reuse ==>
            ensures_of<add_link<T>> (
                update_field(
                    update_field(
                        update_field(
                            update_field(
                                update_field(
                                    update_field(self, reuse_head_index, slot_index),
                                    reuse_spare_count,
                                    update_field(self, reuse_head_index, slot_index).reuse_spare_count
                                    + 1
                                ),
                                reuse_head_index,
                                slot_index
                            ),
                            reuse_spare_count,
                            update_field(
                                update_field(
                                    update_field(self, reuse_head_index, slot_index),
                                    reuse_spare_count,
                                    update_field(self, reuse_head_index, slot_index).reuse_spare_count
                                    + 1
                                ),
                                reuse_head_index,
                                slot_index
                            ).reuse_spare_count + 1
                        ),
                        reuse_head_index,
                        slot_index
                    ),
                    reuse_spare_count,
                    update_field(
                        update_field(
                            update_field(
                                update_field(
                                    update_field(self, reuse_head_index, slot_index),
                                    reuse_spare_count,
                                    update_field(self, reuse_head_index, slot_index).reuse_spare_count
                                    + 1
                                ),
                                reuse_head_index,
                                slot_index
                            ),
                            reuse_spare_count,
                            update_field(
                                update_field(
                                    update_field(self, reuse_head_index, slot_index),
                                    reuse_spare_count,
                                    update_field(self, reuse_head_index, slot_index).reuse_spare_count
                                    + 1
                                ),
                                reuse_head_index,
                                slot_index
                            ).reuse_spare_count + 1
                        ),
                        reuse_head_index,
                        slot_index
                    ).reuse_spare_count + 1
                ),
                slot_index,
                Link::Vacant<T> {
                    next: update_field(
                        update_field(
                            update_field(
                                update_field(
                                    update_field(
                                        update_field(self, reuse_head_index, slot_index),
                                        reuse_spare_count,
                                        update_field(self, reuse_head_index, slot_index).reuse_spare_count
                                        + 1
                                    ),
                                    reuse_head_index,
                                    slot_index
                                ),
                                reuse_spare_count,
                                update_field(
                                    update_field(
                                        update_field(self, reuse_head_index, slot_index),
                                        reuse_spare_count,
                                        update_field(self, reuse_head_index, slot_index).reuse_spare_count
                                        + 1
                                    ),
                                    reuse_head_index,
                                    slot_index
                                ).reuse_spare_count + 1
                            ),
                            reuse_head_index,
                            slot_index
                        ),
                        reuse_spare_count,
                        update_field(
                            update_field(
                                update_field(
                                    update_field(
                                        update_field(self, reuse_head_index, slot_index),
                                        reuse_spare_count,
                                        update_field(self, reuse_head_index, slot_index).reuse_spare_count
                                        + 1
                                    ),
                                    reuse_head_index,
                                    slot_index
                                ),
                                reuse_spare_count,
                                update_field(
                                    update_field(
                                        update_field(self, reuse_head_index, slot_index),
                                        reuse_spare_count,
                                        update_field(self, reuse_head_index, slot_index).reuse_spare_count
                                        + 1
                                    ),
                                    reuse_head_index,
                                    slot_index
                                ).reuse_spare_count + 1
                            ),
                            reuse_head_index,
                            slot_index
                        ).reuse_spare_count + 1
                    ).reuse_head_index
                },
                update_field(
                    update_field(
                        update_field(
                            update_field(
                                update_field(
                                    update_field(self, reuse_head_index, slot_index),
                                    reuse_spare_count,
                                    update_field(self, reuse_head_index, slot_index).reuse_spare_count
                                    + 1
                                ),
                                reuse_head_index,
                                slot_index
                            ),
                            reuse_spare_count,
                            update_field(
                                update_field(
                                    update_field(self, reuse_head_index, slot_index),
                                    reuse_spare_count,
                                    update_field(self, reuse_head_index, slot_index).reuse_spare_count
                                    + 1
                                ),
                                reuse_head_index,
                                slot_index
                            ).reuse_spare_count + 1
                        ),
                        reuse_head_index,
                        slot_index
                    ),
                    reuse_spare_count,
                    update_field(
                        update_field(
                            update_field(
                                update_field(
                                    update_field(self, reuse_head_index, slot_index),
                                    reuse_spare_count,
                                    update_field(self, reuse_head_index, slot_index).reuse_spare_count
                                    + 1
                                ),
                                reuse_head_index,
                                slot_index
                            ),
                            reuse_spare_count,
                            update_field(
                                update_field(
                                    update_field(self, reuse_head_index, slot_index),
                                    reuse_spare_count,
                                    update_field(self, reuse_head_index, slot_index).reuse_spare_count
                                    + 1
                                ),
                                reuse_head_index,
                                slot_index
                            ).reuse_spare_count + 1
                        ),
                        reuse_head_index,
                        slot_index
                    ).reuse_spare_count + 1
                )
            );
        */
        aborts_if [inferred] self.should_reuse
            && update_field(self, reuse_head_index, slot_index).reuse_spare_count
                == MAX_U32;
        aborts_if self.should_reuse && self.slots.is_none();
        aborts_if self.should_reuse
            && self.slots.is_some()
            && table_with_length::spec_contains(option::borrow(self.slots), slot_index);
    }

    spec next_slot_index<T: store>(
        self: &mut 0x1::storage_slots_allocator::StorageSlotsAllocator<T>
    ): u64 {
        use 0x1::option;
        use 0x1::table_with_length;
        pragma opaque = true;
        ensures [inferred] result == old(self).new_slot_index;
        ensures [inferred] self.new_slot_index == old(self).new_slot_index + 1;
        ensures [inferred] self.should_reuse == old(self).should_reuse;
        ensures [inferred] self.reuse_head_index == old(self).reuse_head_index;
        ensures [inferred] self.reuse_spare_count == old(self).reuse_spare_count;
        // A pre-existing table is not touched. If the allocator has none,
        // this call creates the empty intrinsic table used for its slots.
        ensures [inferred] option::is_some(old(self).slots) ==>
            self.slots == old(self).slots;
        ensures [inferred] option::is_none(old(self).slots) ==>
            option::is_some(self.slots);
        ensures [inferred] option::is_none(old(self).slots) ==>
            table_with_length::spec_len(option::borrow(self.slots)) == 0;
        ensures [inferred] option::is_none(old(self).slots) ==>
            (
                forall i: u64:
                    !table_with_length::spec_contains(option::borrow(self.slots), i)
            );
        aborts_if [inferred] self.new_slot_index == MAX_U64;
    }

    spec remove_link<T: store>(
        self: &mut 0x1::storage_slots_allocator::StorageSlotsAllocator<T>, slot_index: u64
    ): 0x1::storage_slots_allocator::Link<T> {
        use 0x1::option;
        use 0x1::table_with_length;
        pragma opaque = true;
        pragma aborts_if_is_partial = false;
        aborts_if [inferred] option::is_none(self.slots);
        aborts_if [inferred] option::is_some(self.slots)
            && !table_with_length::spec_contains(option::borrow(self.slots), slot_index);
        ensures [inferred] result
            == table_with_length::spec_get(option::borrow(old(self).slots), slot_index);
        /* Intrinsic tables carry hidden identity, so removal is specified by
        its complete observable map effect rather than whole-value equality. */
        ensures [inferred] self.new_slot_index == old(self).new_slot_index;
        ensures [inferred] self.should_reuse == old(self).should_reuse;
        ensures [inferred] self.reuse_head_index == old(self).reuse_head_index;
        ensures [inferred] self.reuse_spare_count == old(self).reuse_spare_count;
        ensures [inferred] self.slots.is_some();
        ensures [inferred]!table_with_length::spec_contains(
            option::borrow(self.slots), slot_index
        );
        ensures [inferred] table_with_length::spec_len(option::borrow(self.slots))
            == table_with_length::spec_len(option::borrow(old(self).slots)) - 1;
        ensures forall i: u64 where i != slot_index:
            table_with_length::spec_contains(option::borrow(self.slots), i)
                == table_with_length::spec_contains(option::borrow(old(self).slots), i);
        ensures forall i: u64 where i != slot_index
            && table_with_length::spec_contains(option::borrow(old(self).slots), i):
            table_with_length::spec_get(option::borrow(self.slots), i)
                == table_with_length::spec_get(option::borrow(old(self).slots), i);
    }

    spec reserved_to_index(
        self: &0x1::storage_slots_allocator::ReservedSlot
    ): u64 {
        pragma opaque = true;
        ensures [inferred] result == self.slot_index;
        aborts_if [inferred] false;
    }

    spec stored_to_index(
        self: &0x1::storage_slots_allocator::StoredSlot
    ): u64 {
        pragma opaque = true;
        ensures [inferred] result == self.slot_index;
        aborts_if [inferred] false;
    }
}
