/// FIXME(aldenhu): doc
module aptos_framework::state_storage_config {
    friend aptos_framework::block;
    friend aptos_framework::genesis;
    friend aptos_framework::reconfiguration;

    use std::error;
    use aptos_framework::timestamp;
    use aptos_framework::system_addresses;
    use std::signer::address_of;

    /// error codes
    const EINTRINSICS_ALREADY_EXISTS: u64 = 100;
    const ECONFIG_ALREADY_EXISTS: u64 = 101;
    const ENOT_INITIALIZED: u64 = 102;

    /// FIXME(aldenhu): doc
    struct StateStorageIntrinsics has copy, drop, key, store {
        items: u64,
        bytes: u64,
    }

    /// P(x) = min_price + (base ^ (utilization / target_utilization) - 1) / (base - 1) * (max_price - min_price)
    struct PriceCurve has copy, drop, store {
        min_price: u64,
        max_price: u64,
        target_utilization: u64,
        exponential_base: u64,
    }

    /// FIXME(aldenhu): doc
    struct StateStorageGasConfig has copy, drop, key {
        items_curve: PriceCurve,
        bytes_curve: PriceCurve,
    }

    public fun price_by_utilization(price_curve: &PriceCurve, utilization: u64): u64 {
        // TODO(aldenhu): return native_price_by_utilization(price_curve, utilization)
        return price_curve.min_price
    }

    public(friend) fun initialize(account: &signer) {
        timestamp::assert_genesis();
        system_addresses::assert_aptos_framework(account);
        assert!(
            !exists<StateStorageIntrinsics>(@aptos_framework),
            error::already_exists(EINTRINSICS_ALREADY_EXISTS)
        );
        assert!(
            !exists<StateStorageGasConfig>(@aptos_framework),
            error::already_exists(ECONFIG_ALREADY_EXISTS)
        );

        move_to(account, StateStorageIntrinsics {
            items: 0,
            bytes: 0,
        });

        // initialize the base to be 32, which means:
        //   When DB is at 50% target utilization,the price increases roughly 15% of (max_price - min_price) on top of min_price.
        //   More data points:
        //     10% -> 1%
        //     20% -> 3%
        //     30% -> 6%
        //     40% -> 10%
        //     50% -> 15%
        //     60% -> 23%
        //     70% -> 33%
        //     80% -> 48%
        //     90% -> 70%
        //     95% -> 84%
        //     99% -> 96%
        move_to(account, StateStorageGasConfig {
            items_curve: PriceCurve {
                min_price: 100,
                max_price: 100000,
                target_utilization: 1000000000,
                exponential_base: 32,
            },
            bytes_curve: PriceCurve {
                min_price: 1,
                max_price: 1000,
                target_utilization: 1000000000,
                exponential_base: 32,
            },
        });
    }

    public(friend) fun on_epoch_beginning() {
        /// TODO(aldenhu): refresh DB intrinsics -- load db size infomation at the end of the last epoch
    }

    public(friend) fun on_epoch_ending() {
        /// TODO(aldenhu): recalculate storage prices, update only when difference is big enough
    }

    public fun update_config(account: &signer, config: StateStorageGasConfig) acquires StateStorageGasConfig {
        system_addresses::assert_aptos_framework(account);
        assert!(
            exists<StateStorageGasConfig>(@aptos_framework),
            error::not_found(ENOT_INITIALIZED)
        );
        *borrow_global_mut<StateStorageGasConfig>(address_of(account)) = config;
    }
}
