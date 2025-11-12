module aptos_framework::storage_slot {
    use aptos_framework::object;

    /// Storage slot natives are not enabled.
    const ESTORAGE_SLOT_NATIVES_NOT_ENABLED: u64 = 1;
    /// Resource under storage slot not found, this should never happen.
    /// Emitted by native functions.
    const ESTORAGE_SLOT_NOT_FOUND: u64 = 2;

    struct StorageSlotResource<T> has key {
        val: T
    }

    struct StorageSlot<phantom T> has store {
        addr: address
    }

    public fun new<T: store>(value: T): StorageSlot<T> {
        let unique_signer = object::create_unique_onchain_signer().generate_signer_for_extending();
        move_to(&unique_signer, StorageSlotResource { val: value });
        StorageSlot { addr: unique_signer.address_of() }
    }

    // Internal natives that take StorageSlotResource<T> as a type parameter (like table's borrow_box)
    native fun borrow_storage_slot_resource<T: store, BR>(self: &StorageSlot<T>): &BR;
    native fun borrow_storage_slot_resource_mut<T: store, BR>(self: &mut StorageSlot<T>): &mut BR;

    public fun borrow<T: store>(self: &StorageSlot<T>): &T {
        assert!(std::features::is_storage_slot_natives_enabled(), ESTORAGE_SLOT_NATIVES_NOT_ENABLED);
        &self.borrow_storage_slot_resource<T, StorageSlotResource<T>>().val
    }

    public fun borrow_mut<T: store>(self: &mut StorageSlot<T>): &mut T {
        assert!(std::features::is_storage_slot_natives_enabled(), ESTORAGE_SLOT_NATIVES_NOT_ENABLED);
        &mut self.borrow_storage_slot_resource_mut<T, StorageSlotResource<T>>().val
    }

    public fun copy_storage_slot<T: store + copy>(self: &StorageSlot<T>): StorageSlot<T> {
        new(*self.borrow())
    }

    public fun destroy<T: store>(self: StorageSlot<T>): T {
        let StorageSlot { addr } = self;
        let StorageSlotResource { val } = move_from<StorageSlotResource<T>>(addr);
        val
    }

    #[test]
    public fun test_storage_slot() {
        let slot = new(1u64);
        assert!(slot.borrow() == &1);
        *slot.borrow_mut() += 1;
        assert!(slot.borrow() == &2);
        assert!(slot.destroy() == 2);
    }
}
