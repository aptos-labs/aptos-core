module AptosFramework::Aggregator {

    /// When aggregator's value (actual or accumulated) overflows (raised by
    /// native code).
    const EAGGREGATOR_OVERFLOW: u64 = 1600;

    /// When resolving aggregator's value fails (raised by native code).
    const EAGGREGATOR_RESOLVE_FAILURE: u64 = 1601;

    /// When `Aggregator` cannot find its value resolving from storage (raised
    /// by native code).
    const EAGGREGATOR_VALUE_NOT_FOUND: u64 = 1602;

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

    /// Returns a value stored in this aggregator.
    public native fun read(aggregator: &Aggregator): u128;

    /// Destroys aggregator and removes it from its `AggregatorTable`.
    public fun destroy(aggregator: Aggregator) {
        let Aggregator { table_handle, key, limit, } = aggregator;
        remove_aggregator(table_handle, key, limit);
    }

    native fun remove_aggregator(table_handle: u128, key: u128, limit: u128);
}
