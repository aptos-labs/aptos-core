/// Module for testing monotonically increasing counter native function throughput
module 0xABCD::transaction_context_example {
    use velor_framework::transaction_context;

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
}
