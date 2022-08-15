/// This module has the same API as `TableWithLength`, but instead of integer
/// uses an aggregator.
module aptos_std::table_with_aggregator_length {
    use std::error;

    use aptos_std::aggregator_factory;
    use aptos_std::aggregator::{Self, Aggregator};
    use aptos_std::table::{Self, Table};

    // Native code raises this with error::invalid_arguments()
    const EALREADY_EXISTS: u64 = 100;
    // Native code raises this with error::invalid_arguments()
    const ENOT_FOUND: u64 = 101;
    const ENOT_EMPTY: u64 = 102;

    const MAX_U64: u128 = 18446744073709551615;

    struct TableWithAggregatorLength<phantom K: copy + drop, phantom V> has store {
        inner: Table<K, V>,
        length: Aggregator,
    }

    /// Creates a new table with aggregatable length.
    public fun new<K: copy + drop, V: store>(): TableWithAggregatorLength<K, V> {
        let aggregator = aggregator_factory::create_aggregator(MAX_U64);
        TableWithAggregatorLength {
            inner: table::new<K, V>(),
            length: aggregator,
        }
    }

    /// Destroys a table. The table must be empty to succeed.
    public fun destroy_empty<K: copy + drop, V>(table: TableWithAggregatorLength <K, V>) {
        assert!(aggregator::read(&table.length) == 0, error::invalid_state(ENOT_EMPTY));
        let TableWithAggregatorLength { inner, length } = table;
        table::destroy(inner);
        aggregator::destroy(length);
    }

    /// Adds a new entry to the table. Aborts if an entry for this
    /// key already exists. The entry itself is not stored in the
    /// table, and cannot be discovered from it.
    public fun add<K: copy + drop, V>(table: &mut TableWithAggregatorLength <K, V>, key: K, val: V) {
        table::add(&mut table.inner, key, val);
        aggregator::add(&mut table.length, 1);
    }

    /// Acquires an immutable reference to the value which `key` maps to.
    /// Aborts if there is no entry for `key`.
    public fun borrow<K: copy + drop, V>(table: &TableWithAggregatorLength <K, V>, key: K): &V {
        table::borrow(&table.inner, key)
    }

    /// Acquires a mutable reference to the value which `key` maps to.
    /// Aborts if there is no entry for `key`.
    public fun borrow_mut<K: copy + drop, V>(table: &mut TableWithAggregatorLength <K, V>, key: K): &mut V {
        table::borrow_mut(&mut table.inner, key)
    }

    /// Returns the length of the table, i.e. the number of entries.
    public fun length<K: copy + drop, V>(table: &TableWithAggregatorLength <K, V>): u64 {
        (aggregator::read(&table.length) as u64)
    }

    /// Returns true if this table is empty.
    public fun empty<K: copy + drop, V>(table: &TableWithAggregatorLength <K, V>): bool {
        aggregator::read(&table.length) == 0
    }

    /// Acquires a mutable reference to the value which `key` maps to.
    /// Insert the pair (`key`, `default`) first if there is no entry for `key`.
    public fun borrow_mut_with_default<K: copy + drop, V: drop>(table: &mut TableWithAggregatorLength <K, V>, key: K, default: V): &mut V {
        table::borrow_mut_with_default(&mut table.inner, key, default)
    }

    /// Removes the value which `key` maps to and returns it.
    /// Aborts if there is no entry for `key`.
    public fun remove<K: copy + drop, V>(table: &mut TableWithAggregatorLength <K, V>, key: K): V {
        let val = table::remove(&mut table.inner, key);
        aggregator::sub(&mut table.length, 1);
        val
    }

    /// Returns true iff `table` contains an entry for `key`.
    public fun contains<K: copy + drop, V>(table: &TableWithAggregatorLength <K, V>, key: K): bool {
        table::contains(&table.inner, key)
    }

    #[test(account = @aptos_framework)]
    fun table_with_aggregatable_length_test(account: signer) {
        // Factory should always exist on the core account.
        aggregator_factory::initialize_aggregator_factory(&account);

        let table = new<u128, u128>();
        let i = 0;
        while (i < 500) {
            add(&mut table, i, i);
            i = i + 1;
        };

        // Check aggregator materializes the length correctly.
        assert!(length(&table) == 500, 0);

        while (i > 0) {
            i = i - 1;
            assert!(contains(&table, i), 0);
            assert!(remove(&mut table, i) == i, 0);
        };

        destroy_empty(table);
    }
}
