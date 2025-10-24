module aptos_experimental::transaction_context_unstable {
    use std::error;
    use std::features;
    use std::timestamp;

    /// The monotonically increasing counter is not enabled.
    const EMONOTONICALLY_INCREASING_COUNTER_NOT_ENABLED: u64 = 1;

    /// The monotonically increasing counter has overflowed (too many calls in a single session).
    const EMONOTONICALLY_INCREASING_COUNTER_OVERFLOW: u64 = 2;

    /// The transaction context extension feature is not enabled.
    const ETRANSACTION_CONTEXT_EXTENSION_NOT_ENABLED: u64 = 3;

    /// Returns a monotonically increasing counter value that combines timestamp, transaction index,
    /// session counter, and local counter into a 128-bit value.
    /// Format: `<reserved_byte (8 bits)> || timestamp_us (64 bits) || transaction_index (32 bits) || session_counter (8 bits) || local_counter (16 bits)`
    /// The function aborts if the local counter overflows (after 65535 calls in a single session).
    /// When compiled for testing, this function bypasses feature checks and returns a simplified counter value.
    public fun monotonically_increasing_counter(): u128 {
        if (__COMPILE_FOR_TESTING__) {
            monotonically_increasing_counter_internal_for_test_only()
        } else {
            assert!(features::transaction_context_extension_enabled(), error::invalid_state(ETRANSACTION_CONTEXT_EXTENSION_NOT_ENABLED));
            assert!(features::is_monotonically_increasing_counter_enabled(), error::invalid_state(EMONOTONICALLY_INCREASING_COUNTER_NOT_ENABLED));
            monotonically_increasing_counter_internal(timestamp::now_microseconds())
        }
    }
    native fun monotonically_increasing_counter_internal(timestamp_us: u64): u128;

    /// Test-only version of monotonically_increasing_counter that returns increasing values
    /// without requiring a transaction context. This allows unit tests to verify
    /// the monotonically increasing behavior.
    native fun monotonically_increasing_counter_internal_for_test_only(): u128;

    #[test]
    // Run this test using
    // USE_LATEST_LANGUAGE=1 TEST_FILTER="test_monotonically_increasing_counter" cargo test -p aptos-framework move_framework_unit_tests
    fun test_monotonically_increasing_counter() {
        let counter1 = monotonically_increasing_counter();
        let counter2 = monotonically_increasing_counter();
        let counter3 = monotonically_increasing_counter();

        // Verify that the increment is exactly 1 (since only local_counter changes in test mode)
        assert!(counter2 == counter1 + 1, 2);
        assert!(counter3 == counter2 + 1, 3);
    }
}
