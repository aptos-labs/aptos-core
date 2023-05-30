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
module aptos_framework::aggregator {

    /// The value of aggregator overflows. Raised by native code.
    const EAGGREGATOR_OVERFLOW: u64 = 1;

    /// The value of aggregator underflows (goes below zero). Raised by native code.
    const EAGGREGATOR_UNDERFLOW: u64 = 2;

    /// Aggregator feature is not supported. Raised by native code.
    const ENOT_SUPPORTED: u64 = 3;

    /// Represents an integer which supports parallel additions and subtractions
    /// across multiple transactions. See the module description for more details.
    struct Aggregator has store {
        handle: address,
        key: address,
        limit: u128,
    }

    /// Returns `limit` exceeding which aggregator overflows.
    public fun limit(aggregator: &Aggregator): u128 {
        aggregator.limit
    }

    /// Adds `value` to aggregator. 
    /// Returns a bool flag indicating whether the operation is successful. 
    /// If the output is true, then there is no error in the operation.
    /// If the output is false, then there is overflow error.
    public native fun try_add(aggregator: &mut Aggregator, value: u128): bool;

    /// Adds `value` to aggregator. Aborts on overflowing the limit.
    public fun add(aggregator: &mut Aggregator, value: u128) {
        assert!(try_add(aggregator, value), EAGGREGATOR_OVERFLOW);
    }

    /// Subtracts `value` from aggregator. 
    /// Returns a bool flag indicating whether the operation is successful. 
    /// If the output is true, then there is no error in the operation.
    /// If the output is false, then there is underflow error.
    public native fun try_sub(aggregator: &mut Aggregator, value: u128): bool;

    /// Subtracts `value` from aggregator. Aborts on going below zero.
    public fun sub(aggregator: &mut Aggregator, value: u128) {
        assert!(try_sub(aggregator, value), EAGGREGATOR_UNDERFLOW);
    }

    /// Returns a value stored in this aggregator.
    public native fun read(aggregator: &Aggregator): u128;

    /// Destroys an aggregator and removes it from its `AggregatorFactory`.
    public native fun destroy(aggregator: Aggregator);
}
