/// This module provides an API for aggregatable integers that allow addition,
/// subtraction, and reading.
///
/// Design rationale (V1)
/// =====================
/// Aggregator can be seen as a parellizable integer that supports addition,
/// subtraction and reading. The first version (V1) of aggregator has the
/// the following specification.
///
/// add(value: u128)
///   Speculatively adds a `value` to aggregator. This is a cheap operation
///   which is parallelizable. If the result of addition overflows a `limit`
///   (one of aggregator's fields), an error is produced and the execution
///   aborts.
///
/// sub(value: u128)
///   Speculatively subtracts a `value` from aggregator. This is a cheap
///   operation which is parallelizable. If the result goes below zero, an
///   error is produced and the execution aborts.
///
/// read(): u128
///   Reads (materializes) the value of an aggregator. This is an expensive
///   operation which usually involves reading from the storage.
///
/// destroy()
///   Destroys and aggregator, also cleaning up storage if necessary.
///
/// Note that there is no constructor in `Aggregator` API. This is done on purpose.
/// For every aggregator, we need to know where its value is stored on chain.
/// Currently, Move does not allow fine grained access to struct fields. For
/// example, given a struct
///
///   struct Foo<A> has key {
///       a: A,
///       b: u128,
///   }
///
/// there is no way of getting a value of `Foo::a` without hardcoding the layout
/// of `Foo` and the field offset. To mitigate this problem, one can use a table.
/// Every item stored in the table is uniqely identified by (handle, key) pair:
/// `handle` identifies a table instance, `key` identifies an item within the table.
///
/// So how is this related to aggregator? Well, aggregator can reuse the table's
/// approach for fine-grained storage. However, since native functions only see a
/// reference to aggregator, we must ensure that both `handle` and `key` are
/// included as fields. Therefore, the struct looks like
///
///  struct Aggregator {
///      handle: u128,
///      key: u128,
///      ..
///  }
///
/// Remaining question is how to generate this (handle, key) pair. For that, we have
/// a dedicated struct called `AggregatorFactory` which is responsible for constructing
/// aggregators. See `aggregator_factory.move` for more details.
///
/// Advice to users (V1)
/// ====================
/// Users are encouraged to use "cheap" operations (e.g. additions) to exploit the
/// parallelism in execution.
module aptos_framework::aggregator {

    /// When the value of aggregator (actual or accumulated) overflows (raised by native code).
    const EAGGREGATOR_OVERFLOW: u64 = 1;

    /// When the value of aggregator (actual or accumulated) underflows, i.e goes below zero (raised by native code).
    const EAGGREGATOR_UNDERFLOW: u64 = 2;

    /// When aggregator feature is not supported (raised by native code).
    const ENOT_SUPPORTED: u64 = 3;

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
