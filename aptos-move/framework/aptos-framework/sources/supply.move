/// This module contains logic for upgradable and parallelizable coin supply
/// tracking in Move.
module aptos_framework::supply {
    use aptos_framework::optional_aggregator::{Self, OptionalAggregator};
    use aptos_framework::system_addresses;
    use aptos_std::type_info;

    #[test_only]
    friend aptos_framework::supply_tests;

    const MAX_U128: u128 = 340282366920938463463374607431768211455;

    /// Tracks supply of coins of type `CoinType` in the system.
    struct Supply<phantom CoinType> has store {
        inner: OptionalAggregator,
    }

    /// Creates a new supply tracker for `CoinType`.
    public fun new<CoinType>(): Supply<CoinType> {
        let type_info = type_info::type_of<CoinType>();
        let addr = type_info::account_address(&type_info);
        new_from_address(addr)
    }

    fun new_from_address<CoinType>(addr: address): Supply<CoinType> {
        // TODO: for now, only coins on Aptos framework accounts are parallizable.
        // When the feature matures, we can enable it for everyone.
        if (system_addresses::is_aptos_framework_address(addr)) {
            Supply {
                inner: optional_aggregator::new(MAX_U128, /*parallelizable=*/true),
            }
        } else {
            Supply {
                inner: optional_aggregator::new(MAX_U128, /*parallelizable=*/false),
            }
        }
    }

    /// Upgardes non-parallelizable supply to parallelizable. The owner of supply
    /// is responsible for calling this function.
    public(friend) fun upgrade<CoinType>(supply: Supply<CoinType>): Supply<CoinType> {
        if (!optional_aggregator::is_parallelizable(&supply.inner)) {
            // This supply uses a simple integer - upgrade.
            let Supply { inner } = supply;
            let inner =  optional_aggregator::switch(inner);
            Supply { inner }
        } else {
            // Otherwise, upgarde is not-required.
            supply
        }
    }

    /// Adds `amount` to total supply of `CoinType`. Called when minting coins.
    public fun add<CoinType>(supply: &mut Supply<CoinType>, amount: u128) {
        optional_aggregator::add(&mut supply.inner, amount);
    }

    /// Subtracts `amount` from total supply of `CoinType`. Called when burning coins.
    public fun sub<CoinType>(supply: &mut Supply<CoinType>, amount: u128) {
        optional_aggregator::sub(&mut supply.inner, amount);
    }

    /// Returns the total supply of `CoinType` in existence.
    public fun read<CoinType>(supply: &Supply<CoinType>): u128 {
        optional_aggregator::read(&supply.inner)
    }

    #[test_only]
    public fun drop_unchecked<CoinType>(supply: Supply<CoinType>) {
        let Supply { inner } = supply;
        optional_aggregator::destroy(inner);
    }

    #[test_only]
    public fun is_parallelizable<CoinType>(supply: &Supply<CoinType>): bool {
        optional_aggregator::is_parallelizable(&supply.inner)
    }
}
