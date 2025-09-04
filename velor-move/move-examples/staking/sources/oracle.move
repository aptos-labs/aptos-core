/// Module used to read the price of APT from the oracle.
///
/// Note that this is the bare minimum implementation of the oracle module. More controls beyond price staleness check
/// can be added and developers using this module should consider various security and economic implications when using
/// this oracle in their protocols.
module staking::oracle {
    use velor_std::math128;
    use pyth::i64;
    use pyth::price;
    use pyth::price_identifier;
    use pyth::pyth;

    const PRECISION: u128 = 100000000; // 1e8
    const INITIAL_MAX_AGE_SECS: u64 = 120; // 2 minutes
    const PYTH_APT_ID: vector<u8> = x"03ae4db29ed4ae33d323568895aa00337e658e348b37509f5372ae51f0af00d5";
    const PYTH :vector<u8> = b"pyth";

    /// Price read from oracle is stale
    const ESTALE_PRICE: u64 = 1;

    struct OracleConfig has key {
        /// Maximum age of the price in seconds. If the price is older than this, reading the price will fail.
        max_age_secs: u64,
    }

    fun init_module(staking_signer: &signer) {
        move_to(staking_signer, OracleConfig {
            max_age_secs: INITIAL_MAX_AGE_SECS,
        });
    }

    #[view]
    public fun get_apt_price(): u128 acquires OracleConfig, TestPrice {
        if (exists<TestPrice>(@staking)) {
            return TestPrice[@staking].price;
        };

        let config = &OracleConfig[@staking];
        let price = pyth::get_price_no_older_than(price_identifier::from_byte_vec(PYTH_APT_ID), config.max_age_secs);
        let raw_price = i64::get_magnitude_if_positive(&price::get_price(&price));
        let expo = price::get_expo(&price);
        // Standardize precision or otherwise we'll get different magnitudes for different decimals
        math128::mul_div(
            (raw_price as u128),
            PRECISION,
            math128::pow(10, (i64::get_magnitude_if_negative(&expo) as u128)),
        )
    }

    public inline fun precision(): u128 {
        PRECISION
    }

    #[test_only]
    use velor_framework::account;

    // This struct is used to test the commission contract only and will not be used in production.
    struct TestPrice has key {
        price: u128,
    }

    #[test_only]
    public fun set_test_price(price: u128) acquires TestPrice {
        if (exists<TestPrice>(@staking)) {
            TestPrice[@staking].price = price;
        } else {
            move_to(&account::create_signer_for_test(@staking), TestPrice { price });
        }
    }
}
