/// This module provides an interface for aggregators (version 2).
module aptos_framework::aggregator_v2 {
    use std::error;

    /// The value of aggregator overflows. Raised by uncoditional add() call
    const EAGGREGATOR_OVERFLOW: u64 = 1;

    /// The value of aggregator underflows (goes below zero). Raised by uncoditional sub call
    const EAGGREGATOR_UNDERFLOW: u64 = 2;

    /// Tried casting into a narrower type (i.e. u64), but aggregator range of valid values
    /// cannot fit (i.e. limit exceeds type::MAX).
    /// Raised by native code (i.e. inside snapshot_with_u64_limit())
    const EAGGREGATOR_LIMIT_ABOVE_CAST_MAX: u64 = 2;
    
    /// Represents an integer which supports parallel additions and subtractions
    /// across multiple transactions. See the module description for more details.
    struct Aggregator has store {
        value: u128,
        limit: u128,
    }

    struct AggregatorSnapshot<Element> has store {
        value: Element,
    }

    /// Returns `limit` exceeding which aggregator overflows.
    public fun limit(aggregator: &Aggregator): u128 {
        aggregator.limit
    }

    public native fun create_aggregator(limit: u128): Aggregator;

    /// Adds `value` to aggregator.
    /// If addition would exceed the limit, `false` is returned, and aggregator value is left unchanged.
    public native fun try_add(aggregator: &mut Aggregator, value: u128): bool;

    // Adds `value` to aggregator, uncoditionally.
    // If addition would exceed the limit, EAGGREGATOR_OVERFLOW exception will be thrown
    public fun add(aggregator: &mut Aggregator, value: u128) {
        assert!(try_add(aggregator, value), error::out_of_range(EAGGREGATOR_OVERFLOW));
    }

    /// Subtracts `value` from aggregator.
    /// If subtraction would result in a negative value, `false` is returned, and aggregator value is left unchanged.
    public native fun try_sub(aggregator: &mut Aggregator, value: u128): bool;

    // Adds `value` to aggregator, uncoditionally.
    // If addition would exceed the limit, EAGGREGATOR_UNDERFLOW exception will be thrown
    public fun sub(aggregator: &mut Aggregator, value: u128) {
        assert!(try_sub(aggregator, value), error::out_of_range(EAGGREGATOR_UNDERFLOW));
    }

    /// Returns a value stored in this aggregator.
    public native fun read(aggregator: &Aggregator): u128;

    public native fun snapshot(aggregator: &Aggregator): AggregatorSnapshot<u128>;

    // Do automatic conversion to u64, if all possible values of aggregator fit it (i.e. limit is <= u64::MAX)
    // If limit of the aggregator exceeds u64::MAX, this will throw an exception.
    // This doesn't check if actual value can be converted, only if it can always be converted, based on the limit.
    public native fun snapshot_with_u64_limit(aggregator: &Aggregator): AggregatorSnapshot<u64>;

    public native fun read_snapshot<Element>(snapshot: &AggregatorSnapshot<Element>): Element;
}
