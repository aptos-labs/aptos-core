/// Native collateral storage, keyed by `(exchange, account, scope)`. Write API
/// is `set_collateral` / `delete_collateral`, gated by a
/// `TradingNativeCapability` (re-checked via `assert_valid` on every call).
/// Native state is read only on the validator side (Rust), never from Move.
module aptos_experimental::native_collateral {
    use aptos_experimental::native_collateral_types::{Collateral, CollateralScope};
    use aptos_experimental::trading_native_capability::{Self, TradingNativeCapability};

    /// Set the collateral at `(exchange, account, scope)`. `exchange` is derived
    /// from the cap; every call re-checks the cap via `assert_valid`.
    public fun set_collateral(
        cap: &TradingNativeCapability,
        account: address,
        scope: CollateralScope,
        collateral: Collateral,
    ) {
        trading_native_capability::assert_valid(cap);
        native_set_collateral(trading_native_capability::exchange(cap), account, scope, collateral);
    }

    /// Delete the collateral at `(exchange, account, scope)`.
    public fun delete_collateral(
        cap: &TradingNativeCapability,
        account: address,
        scope: CollateralScope,
    ) {
        trading_native_capability::assert_valid(cap);
        native_delete_collateral(trading_native_capability::exchange(cap), account, scope);
    }

    native fun native_set_collateral(
        exchange: address,
        account: address,
        scope: CollateralScope,
        collateral: Collateral,
    );
    native fun native_delete_collateral(
        exchange: address,
        account: address,
        scope: CollateralScope,
    );

    // =====================================================================
    // Tests
    // =====================================================================

    #[test_only]
    use std::signer;
    #[test_only]
    use aptos_framework::features;
    #[test_only]
    use aptos_experimental::native_collateral_types::{
        cross,
        isolated,
        new_collateral_v1_from_balance,
        new_secondary_balance
    };

    #[test_only]
    fun setup(
        framework: &signer, trading: &signer, exchange: &signer
    ): TradingNativeCapability {
        features::change_feature_flags_for_testing(
            framework,
            vector[features::get_trading_native_feature()],
            vector[]
        );
        trading_native_capability::init_for_test(trading);
        trading_native_capability::register(framework, signer::address_of(exchange));
        trading_native_capability::get_capability(exchange)
    }

    // Unit tests run with the default native feature set, where
    // NATIVE_COLLATERAL is off, so each native aborts at its flag check.

    #[test(framework = @aptos_framework, trading = @aptos_experimental, exchange = @0x1234)]
    #[expected_failure(abort_code = 1, location = Self)]
    fun test_set_collateral_requires_native_collateral(
        framework: &signer, trading: &signer, exchange: &signer
    ) {
        let cap = setup(framework, trading, exchange);
        let collateral = new_collateral_v1_from_balance(
            -5, vector[new_secondary_balance(@0x42, 7)], 3
        );
        set_collateral(&cap, @0x9, isolated(@0x77), collateral);
    }

    #[test(framework = @aptos_framework, trading = @aptos_experimental, exchange = @0x1234)]
    #[expected_failure(abort_code = 1, location = Self)]
    fun test_delete_collateral_requires_native_collateral(
        framework: &signer, trading: &signer, exchange: &signer
    ) {
        let cap = setup(framework, trading, exchange);
        delete_collateral(&cap, @0x9, cross());
    }

    #[test(framework = @aptos_framework, trading = @aptos_experimental, exchange = @0x1234)]
    #[expected_failure(
        abort_code = 0x50003, location = aptos_experimental::trading_native_capability
    )]
    fun test_denied_exchange_cannot_write_collateral(
        framework: &signer, trading: &signer, exchange: &signer
    ) {
        let cap = setup(framework, trading, exchange);
        trading_native_capability::deny(framework, signer::address_of(exchange));
        delete_collateral(&cap, @0x9, cross());
    }
}
