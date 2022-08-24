/// This module provides foundations to create aggregators in the system.
///
/// Design rationale (V1)
/// =====================
/// First, we encourage the reader to see rationale of `Aggregator` in
/// `aggregator.move`.
///
/// Recall that the value of any aggregator can be identified in storage by
/// (handle, key) pair. How this pair can be generated? Short answer: with
/// `AggregatorFactory`!
///
/// `AggregatorFactory` is a struct that can be stored as a resource on some
/// account and which contains a `phantom_table` field. When the factory is
/// initialized, we initialize this table. Importantly, table initialization
/// only generates a uniue table `handle` - something we can reuse.
///
/// When the user wants to create a new aggregator, he/she calls a constructor
/// provided by the factory (`create_aggregator(..)`). This constructor generates
/// a unique key, which with the handle is used to initialize `Aggregator` struct.
///
/// Use cases
/// =========
/// We limit the usage of `AggregatorFactory` by only storing it on the core
/// account.
///
/// When something whants to use an aggregator, the factory is queried and an
/// aggregator instance is created. Once aggregator is no longer in use, it
/// should be destroyed by the user.
module aptos_framework::aggregator_factory {
    use std::error;

    use aptos_framework::system_addresses;
    use aptos_std::aggregator::Aggregator;
    use aptos_std::table::{Self, Table};

    friend aptos_framework::genesis;
    friend aptos_framework::optional_aggregator;

    /// When aggregator factory is not published yet.
    const EAGGREGATOR_FACTORY_NOT_FOUND: u64 = 1;

    /// Struct that creates aggregators.
    struct AggregatorFactory has key {
        phantom_table: Table<address, u128>,
    }

    /// Can only be called during genesis.
    /// Creates a new factory for aggregators.
    public(friend) fun initialize_aggregator_factory(aptos_framework: &signer) {
        system_addresses::assert_aptos_framework(aptos_framework);
        let aggregator_factory = AggregatorFactory {
            phantom_table: table::new()
        };
        move_to(aptos_framework, aggregator_factory);
    }

    /// Creates a new aggregator instance which overflows on exceeding a `limit`.
    public(friend) fun create_aggregator_internal(limit: u128): Aggregator acquires AggregatorFactory {
        assert!(
            exists<AggregatorFactory>(@aptos_framework),
            error::not_found(EAGGREGATOR_FACTORY_NOT_FOUND)
        );

        let aggregator_factory = borrow_global_mut<AggregatorFactory>(@aptos_framework);
        new_aggregator(aggregator_factory, limit)
    }

    /// This is currently a function closed for public. This can be updated in the future by on-chain governance
    /// to allow any signer to call.
    public fun create_aggregator(account: &signer, limit: u128): Aggregator acquires AggregatorFactory {
        // Only Aptos Framework (0x1) account can call this for now.
        system_addresses::assert_aptos_framework(account);
        create_aggregator_internal(limit)
    }

    native fun new_aggregator(aggregator_factory: &mut AggregatorFactory, limit: u128): Aggregator;

    #[test_only]
    public fun initialize_aggregator_factory_for_test(aptos_framework: &signer) {
        initialize_aggregator_factory(aptos_framework);
    }
}
