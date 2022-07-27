/// This module provides foundations to create aggregators in the system.
module aptos_framework::aggregator_table {
    use std::error;
    use std::signer;

    use aptos_framework::aggregator::{Self, Aggregator};
    use aptos_framework::system_addresses;
    use aptos_framework::table::{Self, Table};
    use aptos_framework::timestamp;

    /// When aggregator table has already been published.
    const EAGGREGATOR_TABLE_EXISTS: u64 = 1500;

    /// Struct that stores aggregators, as a (key, value) pair, where `key` is
    /// a unique key that can be used to identify an aggregator, and `value` is
    /// an actual value.
    /// Note that the **only** way to access the value is via `Aggregator` API.
    struct AggregatorTable has key {
        table: Table<u128, u128>,
    }

    /// Creates a new table for aggregators.
    public fun initialize_aggregator_table(account: &signer) {
        // Currently `AggregatorTable` can live ony on aptos framework and
        // should be created with Genesis.
        timestamp::assert_genesis();
        system_addresses::assert_aptos_framework(account);

        assert!(
            !exists<AggregatorTable>(signer::address_of(account)),
            error::already_exists(EAGGREGATOR_TABLE_EXISTS)
        );

        let aggregator_table = AggregatorTable {
            // Note: calling `table::new()` only generates a new table handle.
            table: table::new()
        };
        move_to(account, aggregator_table);
    }

    /// Creates a new aggregator instance associated with this `aggregator_table`
    /// and which overflows on exceeding `limit`.
    public(friend) native fun new_aggregator(aggregator_table: &mut AggregatorTable, limit: u128): Aggregator;

    #[test(account = @aptos_framework)]
    fun test_can_add_and_sub_and_read(account: signer) acquires AggregatorTable {
        initialize_aggregator_table(&account);

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

    #[test(account = @aptos_framework)]
    #[expected_failure(abort_code = 1600)]
    fun test_overflow(account: signer) acquires AggregatorTable {
        initialize_aggregator_table(&account);

        let addr = signer::address_of(&account);
        let aggregator_table = borrow_global_mut<AggregatorTable>(addr);

        let aggregator = new_aggregator(aggregator_table, /*limit=*/10);

        // Overflow!
        aggregator::add(&mut aggregator, 12);

        aggregator::destroy(aggregator);
    }

    #[test(account = @aptos_framework)]
    #[expected_failure(abort_code = 1601)]
    fun test_underflow(account: signer) acquires AggregatorTable {
        initialize_aggregator_table(&account);

        let addr = signer::address_of(&account);
        let aggregator_table = borrow_global_mut<AggregatorTable>(addr);

        let aggregator = new_aggregator(aggregator_table, /*limit=*/10);

        // Underflow!
        aggregator::sub(&mut aggregator, 100);
        aggregator::add(&mut aggregator, 100);

        aggregator::destroy(aggregator);
    }
}
