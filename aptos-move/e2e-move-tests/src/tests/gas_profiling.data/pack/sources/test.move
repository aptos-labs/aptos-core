/// Test module for gas profiling that exercises:
/// - Non-system dependencies (this module itself)
/// - Events
/// - Storage writes
/// - Storage deletions (refunds)
module 0xCAFE::gas_profiling_test {
    use std::signer;
    use aptos_std::table::{Self, Table};
    use aptos_framework::event;

    /// A simple resource to store data
    struct StoredData has key {
        value: u64,
        items: Table<u64, u64>,
    }

    #[event]
    struct TestEvent has drop, store {
        old_value: u64,
        new_value: u64,
        items_added: u64,
        items_removed: u64,
    }

    /// Initialize storage - creates a resource with a table and some items
    public entry fun init_storage(account: &signer, num_items: u64) {
        let items = table::new<u64, u64>();
        let i = 0;
        while (i < num_items) {
            table::add(&mut items, i, i * 100);
            i = i + 1;
        };
        move_to(account, StoredData { value: num_items, items });
    }

    /// All-in-one function: writes new items, removes old items (refund), emits event
    public entry fun write_and_refund(account: &signer, add_count: u64, remove_count: u64) acquires StoredData {
        let addr = signer::address_of(account);
        let data = borrow_global_mut<StoredData>(addr);
        let old_value = data.value;

        // Add new items (storage writes)
        let i = 0;
        while (i < add_count) {
            table::add(&mut data.items, data.value + i, (data.value + i) * 100);
            i = i + 1;
        };

        // Remove old items (storage refunds)
        let j = 0;
        while (j < remove_count) {
            table::remove(&mut data.items, j);
            j = j + 1;
        };

        data.value = data.value + add_count;

        // Emit event
        event::emit(TestEvent {
            old_value,
            new_value: data.value,
            items_added: add_count,
            items_removed: remove_count,
        });
    }
}
