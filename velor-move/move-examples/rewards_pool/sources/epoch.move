module rewards_pool::epoch {
    use velor_framework::timestamp;

    /// The epoch duration is fixed at 1 day (in seconds).
    const EPOCH_DURATION: u64 = 86400;

    #[view]
    public fun now(): u64 {
        to_epoch(timestamp::now_seconds())
    }

    public inline fun duration(): u64 {
        // Equal to EPOCH_DURATION. Inline functions cannot use constants defined in their module.
        86400
    }

    public inline fun to_epoch(timestamp_secs: u64): u64 {
        // Equal to EPOCH_DURATION. Inline functions cannot use constants defined in their module.
        timestamp_secs / 86400
    }

    public inline fun to_seconds(epoch: u64): u64 {
        // Equal to EPOCH_DURATION. Inline functions cannot use constants defined in their module.
        epoch * 86400
    }

    #[test_only]
    public fun fast_forward(epochs: u64) {
        velor_framework::timestamp::fast_forward_seconds(epochs * EPOCH_DURATION);
    }
}
