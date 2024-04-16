/// This module provides foundations to create aggregators. Currently only
/// Aptos Framework (0x1) can create them, so this module helps to wrap
/// the constructor of `Aggregator` struct so that only a system account
/// can initialize one. In the future, this might change and aggregators
/// can be enabled for the public.
module supra_framework::aggregator_factory {
    use std::error;

    use supra_framework::system_addresses;
    use supra_framework::aggregator::Aggregator;
    use aptos_std::table::{Self, Table};

    friend supra_framework::genesis;
    friend supra_framework::optional_aggregator;

    /// Aggregator factory is not published yet.
    const EAGGREGATOR_FACTORY_NOT_FOUND: u64 = 1;

    /// Creates new aggregators. Used to control the numbers of aggregators in the
    /// system and who can create them. At the moment, only Aptos Framework (0x1)
    /// account can.
    struct AggregatorFactory has key {
        phantom_table: Table<address, u128>,
    }

    /// Creates a new factory for aggregators. Can only be called during genesis.
    public(friend) fun initialize_aggregator_factory(supra_framework: &signer) {
        system_addresses::assert_supra_framework(supra_framework);
        let aggregator_factory = AggregatorFactory {
            phantom_table: table::new()
        };
        move_to(supra_framework, aggregator_factory);
    }

    /// Creates a new aggregator instance which overflows on exceeding a `limit`.
    public(friend) fun create_aggregator_internal(limit: u128): Aggregator acquires AggregatorFactory {
        assert!(
            exists<AggregatorFactory>(@supra_framework),
            error::not_found(EAGGREGATOR_FACTORY_NOT_FOUND)
        );

        let aggregator_factory = borrow_global_mut<AggregatorFactory>(@supra_framework);
        new_aggregator(aggregator_factory, limit)
    }

    /// This is currently a function closed for public. This can be updated in the future by on-chain governance
    /// to allow any signer to call.
    public fun create_aggregator(account: &signer, limit: u128): Aggregator acquires AggregatorFactory {
        // Only Aptos Framework (0x1) account can call this for now.
        system_addresses::assert_supra_framework(account);
        create_aggregator_internal(limit)
    }

    /// Returns a new aggregator.
    native fun new_aggregator(aggregator_factory: &mut AggregatorFactory, limit: u128): Aggregator;

    #[test_only]
    public fun initialize_aggregator_factory_for_test(supra_framework: &signer) {
        initialize_aggregator_factory(supra_framework);
    }
}
