/// This module provides an interface for aggregators.
module aptos_framework::aggregator {

    /// When the value of aggregator overflows. Raised by native code.
    const EAGGREGATOR_OVERFLOW: u64 = 1;

    /// When the value of aggregator underflows (goes below zero). Raised by native code.
    const EAGGREGATOR_UNDERFLOW: u64 = 2;

    /// When aggregator feature is not supported. Raised by native code.
    const ENOT_SUPPORTED: u64 = 3;

    /// Represents an aggregatable integer.
    struct Aggregator has store {
        handle: address,
        key: address,
        limit: u128,
    }

    /// Returns `limit` exceeding which aggregator overflows.
    public fun limit(aggregator: &Aggregator): u128 {
        aggregator.limit
    }

    /// Adds `value` to aggregator. Aborts on overflowing the limit.
    public native fun add(aggregator: &mut Aggregator, value: u128);

    /// Subtracts `value` from aggregator. Aborts on going below zero.
    public native fun sub(aggregator: &mut Aggregator, value: u128);

    /// Returns a value stored in this aggregator.
    public native fun read(aggregator: &Aggregator): u128;

    /// Destroys an aggregator and removes it from its `AggregatorFactory`.
    public native fun destroy(aggregator: Aggregator);
}
