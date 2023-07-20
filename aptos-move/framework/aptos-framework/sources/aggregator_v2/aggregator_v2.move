/// This module provides an interface for aggregators (version 2).
module aptos_framework::aggregator_v2 {
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
    /// Returns `true` if the addition succeeded and `false` if it exceeded the limit.
    public native fun try_add(aggregator: &mut Aggregator, value: u128): bool;

    /// Subtracts `value` from aggregator.
    /// Returns `true` if the subtraction succeeded and `false` if it tried going below 0.
    public native fun try_sub(aggregator: &mut Aggregator, value: u128): bool;

    /// Returns a value stored in this aggregator.
    public native fun read(aggregator: &Aggregator): u128;

    public native fun snapshot(aggregator: &Aggregator): AggregatorSnapshot<u128>;

    public native fun try_snapshot_u64(aggregator: &Aggregator): Option<AggregatorSnapshot<u64>>;

    public native fun read_snapshot<Element>(snapshot: &AggregatorSnapshot<Element>): Element;
}
