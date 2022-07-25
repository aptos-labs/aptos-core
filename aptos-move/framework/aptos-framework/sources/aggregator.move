module aptos_framework::aggregator {

    /// When aggregator's value (actual or accumulated) overflows (raised by
    /// native code).
    const EAGGREGATOR_OVERFLOW: u64 = 1600;

    /// When aggregator's actual value goes below zero (raised by native code).
    const EAGGREGATOR_UNDERFLOW: u64 = 1601;

    /// Aggregator struct that can be uniquely identified by a key, internally
    /// stores an opaque value, initialized to 0, and overflowing on exceeding
    /// `limit`. `table_handle` identifies the parent `AggregatorTable`.
    struct Aggregator has store {
        table_handle: u128,
        key: u128,
        limit: u128,
    }

    /// Adds `value` to aggregator. Aborts on overflow.
    public native fun add(aggregator: &mut Aggregator, value: u128);

    /// Subtracts `value` from aggregator. Aborts on going below zero.
    public native fun sub(aggregator: &mut Aggregator, value: u128);

    /// Returns a value stored in this aggregator.
    public native fun read(aggregator: &Aggregator): u128;

    /// Destroys aggregator and removes it from its `AggregatorTable`.
    public fun destroy(aggregator: Aggregator) {
        let Aggregator { table_handle, key, limit, } = aggregator;
        remove_aggregator(table_handle, key, limit);
    }

    native fun remove_aggregator(table_handle: u128, key: u128, limit: u128);
}
