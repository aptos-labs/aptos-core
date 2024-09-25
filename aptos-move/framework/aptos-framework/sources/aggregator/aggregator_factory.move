/// This module provides foundations to create aggregators. Currently only
/// Aptos Framework (0x1) can create them, so this module helps to wrap
/// the constructor of `Aggregator` struct so that only a system account
/// can initialize one. In the future, this might change and aggregators
/// can be enabled for the public.
module aptos_framework::aggregator_factory {
    use std::error;

    use aptos_framework::aggregator::Aggregator;
    use aptos_std::table::Table;

    friend aptos_framework::genesis;

    /// Aggregator factory is not published yet.
    const EAGGREGATOR_FACTORY_NOT_FOUND: u64 = 1;

    /// Creating aggregators V1 is no longer supported
    const ENO_LONGER_SUPPORTED: u64 = 2;

    #[deprecated]
    /// Creates new aggregators. Used to control the numbers of aggregators in the
    /// system and who can create them. At the moment, only Aptos Framework (0x1)
    /// account can.
    struct AggregatorFactory has key {
        phantom_table: Table<address, u128>,
    }

    #[deprecated]
    public fun create_aggregator(_account: &signer, _limit: u128): Aggregator {
        abort error::invalid_argument(ENO_LONGER_SUPPORTED)
    }

    /// Returns a new aggregator.
    native fun new_aggregator(aggregator_factory: &mut AggregatorFactory, limit: u128): Aggregator;

    #[test_only]
    public fun create_aggregator_for_testing(account: &signer, limit: u128): Aggregator {
        new_aggregator(account, limit)
    }
}
