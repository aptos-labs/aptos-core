/// This module provides an API for aggregatable integers that allow addition,
/// subtraction, and reading.
module aptos_framework::aggregator {

    /// When aggregator's value (actual or accumulated) overflows (raised by
    /// native code).
    const EAGGREGATOR_OVERFLOW: u64 = 1;

    /// When aggregator's value (actual or accumulated) underflows (raised by
    /// native code).
    const EAGGREGATOR_UNDERFLOW: u64 = 2;

    struct Aggregator has store {
        handle: u128,
        key: u128,
        limit: u128,
    }

    /// Adds `value` to aggregator. Aborts on overflowing the limit.
    public native fun add(aggregator: &mut Aggregator, value: u128);

    /// Subtracts `value` from aggregator. Aborts on going below zero.
    public native fun sub(aggregator: &mut Aggregator, value: u128);

    /// Returns a value stored in this aggregator.
    public native fun read(aggregator: &Aggregator): u128;

    /// Destroys aggregator and removes it from its `AggregatorFactory`.
    public fun destroy(aggregator: Aggregator) {
        let Aggregator { handle, key, limit: _, } = aggregator;
        remove_aggregator(handle, key);
    }

    native fun remove_aggregator(handle: u128, key: u128);
}
