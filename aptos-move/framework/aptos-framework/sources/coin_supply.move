/// This module contains logic for upgradable and parallelizable coin supply
/// tracking in Move. It is a seprate module in order to hide aggregator from
/// the coin.
///
///  +--------+     +---------------+     +-----------------------+
///  |  coin  | --> |  coin_supply  | --> |  optional_aggregator  |
///  +--------+     +---------------+     +-----------------------+
///                                         coin doesn't know
///                                         this exists!
///
/// This is not u128, and is intnded to be used from `coin` module only. Thus,
/// only `coin` is allowed to create or upgrade the supply struct.
///
/// `Supply` is stored within `CoinInfo` and is updated when coins are minted
/// or burnt. In addition, one can get the coin supply in existence by calling
/// `coin::supply<CoinType>()` that uses API from this module under the hood.
module aptos_framework::coin_supply {
    use aptos_framework::optional_aggregator::{Self, OptionalAggregator};
    use aptos_framework::system_addresses;
    use aptos_std::type_info;

    friend aptos_framework::coin;

    #[test_only]
    friend aptos_framework::coin_supply_tests;

    /// Maximum possible coin supply.
    const MAX_U128: u128 = 340282366920938463463374607431768211455;

    /// Tracks supply of coins of type `CoinType` in the system.
    struct Supply<phantom CoinType> has store {
        inner: OptionalAggregator,
    }

    /// Creates a new supply tracker for `CoinType`.
    public(friend) fun new<CoinType>(): Supply<CoinType> {
        let type_info = type_info::type_of<CoinType>();
        let addr = type_info::account_address(&type_info);
        new_from_address(addr)
    }

    fun new_from_address<CoinType>(addr: address): Supply<CoinType> {
        if (system_addresses::is_aptos_framework_address(addr)) {
            Supply {
                // TODO: set to true once execution is working.
                inner: optional_aggregator::new(MAX_U128, /*parallelizable=*/false),
            }
        } else {
            Supply {
                inner: optional_aggregator::new(MAX_U128, /*parallelizable=*/false),
            }
        }
    }

    /// Upgardes non-parallelizable supply to parallelizable. The owner of supply
    /// (i.e. the coin) is responsible for calling this function.
    public(friend) fun upgrade<CoinType>(supply: &mut Supply<CoinType>) {
        if (!optional_aggregator::is_parallelizable(&supply.inner)) {
            optional_aggregator::switch(&mut supply.inner);
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
