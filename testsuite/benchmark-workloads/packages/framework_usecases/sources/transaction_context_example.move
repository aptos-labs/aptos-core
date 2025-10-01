/// Module for testing monotonically increasing counter native function throughput
module 0xABCD::transaction_context_example {
    use std::signer;
    use aptos_framework::big_ordered_map;
    use aptos_framework::transaction_context;

    struct All has key {
        all: big_ordered_map::BigOrderedMap<u128, bool>
    }

    fun init_module(publisher: &signer) {
        assert!(
            signer::address_of(publisher) == @publisher_address,
        );
        move_to(publisher, All { all: big_ordered_map::new()})
    }

    /// Test single call to monotonic counter per transaction
    public entry fun test_monotonic_counter_single() {
        // Call the monotonic counter once
        let _counter = transaction_context::monotonically_increasing_counter();
    }

    /// Test multiple calls to monotonic counter per transaction
    public entry fun test_monotonic_counter_multiple(count: u64) {
        let i = 0;
        while (i < count) {
            let _counter = transaction_context::monotonically_increasing_counter();
            i = i + 1;
        }
    }

    /// Test single call to monotonic counter per transaction, with inserting into a map of all previous values
    public entry fun test_monotonic_counter_insert() {
        // Call the monotonic counter once
        All[@publisher_address].all.add(transaction_context::monotonically_increasing_counter(), true);
    }

}
