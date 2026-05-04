module aptos_framework::storage_slot_or_inline {
    use std::mem;
    use aptos_framework::storage_slot::{Self, StorageSlot};

    /// StorageSlotOrInline found in inconsistent (transient) state, should never happen.
    const ESTORAGE_SLOT_INCORRECTLY_IN_TRANSIENT_STATE: u64 = 1;

    enum StorageSlotOrInline<T> has store {
        Inline{ value: T },
        StorageSlot { slot: StorageSlot<T> },
        Transient,
    }

    public fun new_inline<T: store>(value: T): StorageSlotOrInline<T> {
        StorageSlotOrInline::Inline { value }
    }

    public fun new_storage_slot<T: store>(value: T): StorageSlotOrInline<T> {
        StorageSlotOrInline::StorageSlot { slot: storage_slot::new(value) }
    }

    public fun borrow<T: store>(self: &StorageSlotOrInline<T>): &T {
        match (self) {
            StorageSlotOrInline::Inline { value } => value,
            StorageSlotOrInline::StorageSlot { slot } => slot.borrow(),
            StorageSlotOrInline::Transient => abort ESTORAGE_SLOT_INCORRECTLY_IN_TRANSIENT_STATE,
        }
    }

    public fun borrow_mut<T: store>(self: &mut StorageSlotOrInline<T>): &mut T {
        match (self) {
            StorageSlotOrInline::Inline { value } => value,
            StorageSlotOrInline::StorageSlot { slot } => slot.borrow_mut(),
            StorageSlotOrInline::Transient => abort ESTORAGE_SLOT_INCORRECTLY_IN_TRANSIENT_STATE,
        }
    }

    public fun destroy<T: store>(self: StorageSlotOrInline<T>): T {
        match (self) {
            StorageSlotOrInline::Inline { value } => value,
            StorageSlotOrInline::StorageSlot { slot } => slot.destroy(),
            StorageSlotOrInline::Transient => abort ESTORAGE_SLOT_INCORRECTLY_IN_TRANSIENT_STATE,
        }
    }

    public fun move_to_inline<T: store>(self: &mut StorageSlotOrInline<T>) {
        match (self) {
            StorageSlotOrInline::Inline { value: _ } => {},
            StorageSlotOrInline::StorageSlot { slot: _ } => {
                let StorageSlotOrInline::StorageSlot { slot } = mem::replace(self, StorageSlotOrInline::Transient);
                let StorageSlotOrInline::Transient = mem::replace(self, new_inline(slot.destroy()));
            },
            StorageSlotOrInline::Transient => abort ESTORAGE_SLOT_INCORRECTLY_IN_TRANSIENT_STATE,
        }
    }

    public fun move_to_storage_slot<T: store>(self: &mut StorageSlotOrInline<T>) {
        match (self) {
            StorageSlotOrInline::Inline { value: _ } => {
                let StorageSlotOrInline::Inline { value } = mem::replace(self, StorageSlotOrInline::Transient);
                let StorageSlotOrInline::Transient = mem::replace(self, new_storage_slot(value));
            },
            StorageSlotOrInline::StorageSlot { slot: _ } => {},
            StorageSlotOrInline::Transient => abort ESTORAGE_SLOT_INCORRECTLY_IN_TRANSIENT_STATE,
        }
    }

    #[test]
    fun test_storage_slot_or_inline() {
        let value = new_storage_slot(Dummy {});
        value.move_to_inline();
        value.move_to_storage_slot();
        value.move_to_inline();
        let Dummy {} = value.destroy();
    }

    struct Dummy has store {}
}
