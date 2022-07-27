module aptos_framework::aggregator_table {
    use std::signer;

    use aptos_framework::aggregator::{Self, Aggregator};
    use aptos_framework::table::{Self, Table};

    /// When aggregator table has already been published.
    const EAGGREGATOR_TABLE_EXISTS: u64 = 1500;

    /// When aggregator table does not exist.
    const EAGGREGATOR_TABLE_DOES_NOT_EXIST: u64 = 1501;

    /// A global table of all registered aggregators stored as pairs:
    ///     (aggregator_key, agregator_value)
    /// Access to values is restricted and only `Aggregator` associated with
    /// a key can read or update the value.
    struct AggregatorTable has key {
        table: Table<u128, u128>,
    }

    /// Creates a new table for aggregators.
    public fun register_aggregator_table(account: &signer) {
        assert!(
            !exists<AggregatorTable>(signer::address_of(account)),
            EAGGREGATOR_TABLE_EXISTS
        );

        let aggregator_table = AggregatorTable {
            // Note that calling `table::new()` only generates a new table
            // handle.
            table: table::new()
        };
        move_to(account, aggregator_table);
    }

    /// Creates a new aggregator instance associated with `aggregator_table`
    /// and which overflows on exceeding `limit`.
    native fun new_aggregator(aggregator_table: &mut AggregatorTable, limit: u128): Aggregator;

    #[test(account = @0xFF)]
    fun test_can_add_and_sub_and_read(account: signer) acquires AggregatorTable {
        register_aggregator_table(&account);

        let addr = signer::address_of(&account);
        let aggregator_table = borrow_global_mut<AggregatorTable>(addr);

        let aggregator = new_aggregator(aggregator_table, /*limit=*/1000);

        aggregator::add(&mut aggregator, 12);
        assert!(aggregator::read(&aggregator) == 12, 0);

        aggregator::add(&mut aggregator, 3);
        assert!(aggregator::read(&aggregator) == 15, 0);

        aggregator::add(&mut aggregator, 3);
        aggregator::add(&mut aggregator, 2);
        aggregator::sub(&mut aggregator, 20);
        assert!(aggregator::read(&aggregator) == 0, 0);

        aggregator::add(&mut aggregator, 1000);
        aggregator::sub(&mut aggregator, 1000);

        aggregator::destroy(aggregator);
    }

    #[test(account = @0xFF)]
    #[expected_failure(abort_code = 1600)]
    fun test_overflow(account: signer) acquires AggregatorTable {
        register_aggregator_table(&account);

        let addr = signer::address_of(&account);
        let aggregator_table = borrow_global_mut<AggregatorTable>(addr);

        let aggregator = new_aggregator(aggregator_table, /*limit=*/10);

        // Overflow!
        aggregator::add(&mut aggregator, 12);

        aggregator::destroy(aggregator);
    }

    #[test(account = @0xFF)]
    #[expected_failure(abort_code = 1601)]
    fun test_underflow(account: signer) acquires AggregatorTable {
        register_aggregator_table(&account);

        let addr = signer::address_of(&account);
        let aggregator_table = borrow_global_mut<AggregatorTable>(addr);

        let aggregator = new_aggregator(aggregator_table, /*limit=*/10);

        // Underflow!
        aggregator::sub(&mut aggregator, 100);
        aggregator::add(&mut aggregator, 100);

        aggregator::destroy(aggregator);
    }
}
