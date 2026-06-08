/// Authorization layer for native-trading stores: the
/// `TradingNativeCapability` token that gates writes and the
/// `ExchangeRegistry` (at `@aptos_trading`) that decides who can mint one.
/// Governance `register`s / `deny`s exchanges; an exchange mints a cap
/// per tx via `get_capability`.
module aptos_trading::trading_native_capability {
    use std::error;
    use std::signer;

    use aptos_framework::big_ordered_map::{Self, BigOrderedMap};
    use aptos_framework::features;
    use aptos_framework::system_addresses;

    /// Feature `TRADING_NATIVE` is not enabled on this chain.
    const EFEATURE_DISABLED: u64 = 1;
    /// Exchange has not been registered yet.
    const EEXCHANGE_NOT_REGISTERED: u64 = 2;
    /// This exchange address has been disabled by governance.
    const EEXCHANGE_DENIED: u64 = 3;
    /// `init_module` was invoked with a signer other than `@aptos_trading`.
    const ENOT_DEPLOYER: u64 = 4;

    /// Zero-sized value type for the `BigOrderedMap` sets (satisfies the
    /// map's constant-serialized-size requirement).
    struct Empty has copy, drop, store {}

    /// `store` so the exchange can hold it across transactions; not
    /// `copy`, so it can't be duplicated. Validity (registered, not
    /// denied, flag enabled) is re-checked on every write via
    /// `assert_valid`, so a stored cap stops working as soon as
    /// governance denies the exchange.
    struct TradingNativeCapability has store, drop {
        exchange: address,
    }

    /// Registered exchanges and the governance deny-list, at
    /// `@aptos_trading`. `BigOrderedMap` keeps each entry in its own slot
    /// so writes to distinct addresses don't contend under block-STM.
    enum ExchangeRegistry has key {
        V1 {
            registered: BigOrderedMap<address, Empty>,
            denied: BigOrderedMap<address, Empty>,
        },
    }

    /// Initialize the `ExchangeRegistry` at `@aptos_trading`. Invoked by
    /// vm-genesis (and on republish by the VM).
    fun init_module(deployer: &signer) {
        assert!(
            signer::address_of(deployer) == @aptos_trading,
            error::permission_denied(ENOT_DEPLOYER),
        );
        if (!exists<ExchangeRegistry>(@aptos_trading)) {
            move_to(deployer, ExchangeRegistry::V1 {
                registered: big_ordered_map::new(),
                denied: big_ordered_map::new(),
            });
        };
    }

    #[test_only]
    public fun init_for_test(deployer: &signer) {
        init_module(deployer);
    }

    public fun is_denied(exchange: address): bool acquires ExchangeRegistry {
        let registry = borrow_global<ExchangeRegistry>(@aptos_trading);
        registry.denied.contains(&exchange)
    }

    /// Governance-only: enroll an `exchange`. Idempotent.
    public fun register(framework: &signer, exchange: address) acquires ExchangeRegistry {
        system_addresses::assert_aptos_framework(framework);
        assert!(
            features::is_trading_native_enabled(),
            error::permission_denied(EFEATURE_DISABLED),
        );
        let registry = borrow_global_mut<ExchangeRegistry>(@aptos_trading);
        if (!registry.registered.contains(&exchange)) {
            registry.registered.add(exchange, Empty {});
        };
    }

    /// Abort unless `addr` is currently allowed to write: `TRADING_NATIVE`
    /// enabled, registered, and not denied.
    fun assert_active(addr: address) acquires ExchangeRegistry {
        assert!(
            features::is_trading_native_enabled(),
            error::permission_denied(EFEATURE_DISABLED),
        );
        let registry = borrow_global<ExchangeRegistry>(@aptos_trading);
        assert!(
            registry.registered.contains(&addr),
            error::permission_denied(EEXCHANGE_NOT_REGISTERED),
        );
        assert!(
            !registry.denied.contains(&addr),
            error::permission_denied(EEXCHANGE_DENIED),
        );
    }

    /// Mint a cap for a registered, non-denied exchange. The caller may
    /// store it; every write re-checks validity via `assert_valid`.
    public fun get_capability(exchange: &signer): TradingNativeCapability
    acquires ExchangeRegistry {
        let addr = signer::address_of(exchange);
        assert_active(addr);
        TradingNativeCapability { exchange: addr }
    }

    /// Abort unless `cap`'s exchange is still allowed to write. Called on
    /// every native-position write so a stored cap is invalidated the
    /// moment governance denies the exchange (or the flag is turned off).
    public fun assert_valid(cap: &TradingNativeCapability) acquires ExchangeRegistry {
        assert_active(cap.exchange);
    }

    /// Governance-only: lock an `exchange` out (persisted state
    /// untouched). Deliberately does not check `TRADING_NATIVE` —
    /// governance must be able to lock out even with the flag off.
    public fun deny(framework: &signer, exchange: address) acquires ExchangeRegistry {
        system_addresses::assert_aptos_framework(framework);
        let registry = borrow_global_mut<ExchangeRegistry>(@aptos_trading);
        if (!registry.denied.contains(&exchange)) {
            registry.denied.add(exchange, Empty {});
        };
    }

    /// Governance-only: clear a `deny`. Like `deny`, does not check
    /// `TRADING_NATIVE`.
    public fun reenable(framework: &signer, exchange: address) acquires ExchangeRegistry {
        system_addresses::assert_aptos_framework(framework);
        let registry = borrow_global_mut<ExchangeRegistry>(@aptos_trading);
        if (registry.denied.contains(&exchange)) {
            registry.denied.remove(&exchange);
        };
    }

    public fun exchange(cap: &TradingNativeCapability): address {
        cap.exchange
    }

    // =====================================================================
    // Tests
    // =====================================================================

    #[test_only]
    fun enable_trading_native(framework: &signer) {
        features::change_feature_flags_for_testing(
            framework,
            vector[features::get_trading_native_feature()],
            vector[],
        );
    }

    #[test(framework = @aptos_framework, trading = @aptos_trading, exchange = @0x1234)]
    fun test_register_then_get_capability(
        framework: &signer, trading: &signer, exchange: &signer
    ) acquires ExchangeRegistry {
        enable_trading_native(framework);
        init_for_test(trading);
        let addr = signer::address_of(exchange);
        register(framework, addr);
        let cap = get_capability(exchange);
        assert!(exchange(&cap) == addr, 0);
    }

    #[test(framework = @aptos_framework, trading = @aptos_trading, exchange = @0x1234)]
    #[expected_failure(abort_code = 0x50002, location = Self)]
    fun test_get_capability_unregistered_aborts(
        framework: &signer, trading: &signer, exchange: &signer
    ) acquires ExchangeRegistry {
        enable_trading_native(framework);
        init_for_test(trading);
        get_capability(exchange);
    }

    #[test(framework = @aptos_framework, trading = @aptos_trading, exchange = @0x1234)]
    #[expected_failure(abort_code = 0x50003, location = Self)]
    fun test_denied_exchange_cannot_get_capability(
        framework: &signer, trading: &signer, exchange: &signer
    ) acquires ExchangeRegistry {
        enable_trading_native(framework);
        init_for_test(trading);
        let addr = signer::address_of(exchange);
        register(framework, addr);
        deny(framework, addr);
        assert!(is_denied(addr), 0);
        get_capability(exchange);
    }

    #[test(framework = @aptos_framework, trading = @aptos_trading, exchange = @0x1234)]
    fun test_reenable_restores_capability(
        framework: &signer, trading: &signer, exchange: &signer
    ) acquires ExchangeRegistry {
        enable_trading_native(framework);
        init_for_test(trading);
        let addr = signer::address_of(exchange);
        register(framework, addr);
        deny(framework, addr);
        reenable(framework, addr);
        assert!(!is_denied(addr), 0);
        let cap = get_capability(exchange);
        assert!(exchange(&cap) == addr, 1);
    }

    #[test(framework = @aptos_framework, trading = @aptos_trading, exchange = @0x1234)]
    #[expected_failure(abort_code = 0x50001, location = Self)]
    fun test_get_capability_requires_trading_native_flag(
        framework: &signer, trading: &signer, exchange: &signer
    ) acquires ExchangeRegistry {
        // Register while the flag is on, then disable it: the umbrella
        // kill-switch must block cap minting even for a registered exchange.
        enable_trading_native(framework);
        init_for_test(trading);
        let addr = signer::address_of(exchange);
        register(framework, addr);
        features::change_feature_flags_for_testing(
            framework, vector[], vector[features::get_trading_native_feature()]
        );
        get_capability(exchange);
    }

    #[test(framework = @aptos_framework, trading = @aptos_trading, exchange = @0x1234)]
    fun test_assert_valid_passes_when_active(
        framework: &signer, trading: &signer, exchange: &signer
    ) acquires ExchangeRegistry {
        enable_trading_native(framework);
        init_for_test(trading);
        register(framework, signer::address_of(exchange));
        let cap = get_capability(exchange);
        assert_valid(&cap);
    }

    #[test(framework = @aptos_framework, trading = @aptos_trading, exchange = @0x1234)]
    #[expected_failure(abort_code = 0x50003, location = Self)]
    fun test_held_cap_invalidated_by_deny(
        framework: &signer, trading: &signer, exchange: &signer
    ) acquires ExchangeRegistry {
        // Mint a cap, then deny the exchange: the stored cap must stop
        // validating immediately, not on the next tx.
        enable_trading_native(framework);
        init_for_test(trading);
        let addr = signer::address_of(exchange);
        register(framework, addr);
        let cap = get_capability(exchange);
        deny(framework, addr);
        assert_valid(&cap);
    }

    #[test(framework = @aptos_framework, trading = @aptos_trading, exchange = @0x1234)]
    #[expected_failure(abort_code = 0x50001, location = Self)]
    fun test_held_cap_invalidated_by_flag_off(
        framework: &signer, trading: &signer, exchange: &signer
    ) acquires ExchangeRegistry {
        enable_trading_native(framework);
        init_for_test(trading);
        register(framework, signer::address_of(exchange));
        let cap = get_capability(exchange);
        features::change_feature_flags_for_testing(
            framework, vector[], vector[features::get_trading_native_feature()]
        );
        assert_valid(&cap);
    }
}
