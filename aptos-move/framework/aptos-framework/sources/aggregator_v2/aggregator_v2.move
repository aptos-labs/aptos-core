/// This module provides an interface for aggregators. Aggregators are similar to
/// unsigned integers and support addition and subtraction (aborting on underflow
/// or on overflowing a custom upper limit). The difference from integers is that
/// aggregators allow to perform both additions and subtractions in parallel across
/// multiple transactions, enabling parallel execution. For example, if the first
/// transaction is doing `add(X, 1)` for aggregator resource `X`, and the second
/// is doing `sub(X,3)`, they can be executed in parallel avoiding a read-modify-write
/// dependency.
/// However, reading the aggregator value (i.e. calling `read(X)`) is an expensive
/// operation and should be avoided as much as possible because it reduces the
/// parallelism. Moreover, **aggregators can only be created by Aptos Framework (0x1)
/// at the moment.**
module aptos_framework::aggregator_v2 {

    /// Aggregator feature is not supported. Raised by native code.
    const ENOT_SUPPORTED: u64 = 3;

    /// Represents an integer which supports parallel additions and subtractions
    /// across multiple transactions. See the module description for more details.
    struct Aggregator has store {
        value: u128,
        limit: u128,
        last_overflow: u64,
        last_underflow: u64,
    }

    struct AggregatorSnapshot<Element> has store {
        value: Element,
    }

    /// Returns `limit` exceeding which aggregator overflows.
    public fun limit(aggregator: &Aggregator): u128 {
        aggregator.limit
    }

    /// Adds `value` to aggregator.
    /// Returns `true` if the addition succeeded and `false` if it exceeded the limit.
    /// Adds branch prediction, to avoid re-execution due to speculative execution.
    public fun try_add(aggregator: &mut Aggregator, value: u128): bool {
        if (value == 0) {
            return true;
        };
        if (aggregator.last_overflow > 0 && aggregator.last_overflow <= value) {
            return false;
        };
        let result = aggregator_v2::try_add_impl(aggregator.aggregator, value);
        if (result) {
            aggregator.last_underflow = 0;
        } else {
            aggregator.last_overflow = value;
        };
        result
    }

    /// Subtracts `value` from aggregator.
    /// Returns `true` if the subtraction succeeded and `false` if it tried going below 0.
    /// Adds branch prediction, to avoid re-execution due to speculative execution.
    public fun try_sub(aggregator: &mut Aggregator, value: u128): bool {
        if (value == 0) {
            return true;
        };
        if (aggregator.last_underflow > 0 && aggregator.last_underflow <= value) {
            return false;
        };
        let result = aggregator_v2::try_sub_impl(aggregator.aggregator, value);
        if (result) {
            aggregator.last_overflow = 0;
        } else {
            aggregator.last_underflow = value;
        };
        result
    }

    public native fun create_aggregator(limit: u128): Aggregator;

    /// Adds `value` to aggregator.
    /// Returns `true` if the addition succeeded and `false` if it exceeded the limit.
    native fun try_add_impl(aggregator: &mut Aggregator, value: u128): bool;

    /// Subtracts `value` from aggregator.
    /// Returns `true` if the subtraction succeeded and `false` if it tried going below 0.
    native fun try_sub_impl(aggregator: &mut Aggregator, value: u128): bool;

    /// Returns a value stored in this aggregator.
    public native fun read(aggregator: &Aggregator): u128;

    public native fun snapshot(aggregator: &Aggregator): AggregatorSnapshot<u128>;

    public native fun try_snapshot_u64(aggregator: &Aggregator): Option<AggregatorSnapshot<u64>>;

    public native fun read_snapshot<Element>(aggregator: &AggregatorSnapshot<Element>): Element;
}
