spec aptos_experimental::trading_native_capability {
    use std::features::TRADING_NATIVE;

    spec fun spec_registry(): ExchangeRegistry {
        global<ExchangeRegistry>(@aptos_experimental)
    }

    /// The feature flag, registration and denial checks every capability use
    /// shares.
    spec fun spec_active_aborts(addr: address): bool {
        !features::spec_is_enabled(TRADING_NATIVE)
            || !exists<ExchangeRegistry>(@aptos_experimental)
            || !big_ordered_map::spec_contains_key(spec_registry().registered, addr)
            || big_ordered_map::spec_contains_key(spec_registry().denied, addr)
    }

    spec init_module(deployer: &signer) {
        pragma opaque;
        aborts_if signer::address_of(deployer) != @aptos_experimental;
        modifies global<ExchangeRegistry>(@aptos_experimental);
        ensures exists<ExchangeRegistry>(@aptos_experimental);
        ensures old(exists<ExchangeRegistry>(@aptos_experimental))
            ==> spec_registry() == old(spec_registry());
        ensures !old(exists<ExchangeRegistry>(@aptos_experimental))
            ==> big_ordered_map::spec_len(spec_registry().registered) == 0
                && big_ordered_map::spec_len(spec_registry().denied) == 0;
    }

    spec is_denied(exchange: address): bool {
        pragma opaque;
        aborts_if !exists<ExchangeRegistry>(@aptos_experimental);
        ensures result == big_ordered_map::spec_contains_key(spec_registry().denied, exchange);
    }

    spec register(framework: &signer, exchange: address) {
        pragma opaque;
        aborts_if signer::address_of(framework) != @aptos_framework;
        aborts_if !features::spec_is_enabled(TRADING_NATIVE);
        aborts_if !exists<ExchangeRegistry>(@aptos_experimental);
        modifies global<ExchangeRegistry>(@aptos_experimental);
        ensures big_ordered_map::spec_contains_key(spec_registry().registered, exchange);
        ensures spec_registry().denied == old(spec_registry().denied);
        ensures spec_registry().registered
            == (if (big_ordered_map::spec_contains_key(old(spec_registry().registered), exchange)) {
                old(spec_registry().registered)
            } else {
                big_ordered_map::spec_set(old(spec_registry().registered), exchange, Empty {})
            });
    }

    spec assert_active(addr: address) {
        pragma opaque;
        aborts_if spec_active_aborts(addr);
    }

    spec get_capability(exchange: &signer): TradingNativeCapability {
        pragma opaque;
        aborts_if spec_active_aborts(signer::address_of(exchange));
        ensures result == TradingNativeCapability { exchange: signer::address_of(exchange) };
    }

    spec assert_valid(cap: &TradingNativeCapability) {
        pragma opaque;
        aborts_if spec_active_aborts(cap.exchange);
    }

    spec deny(framework: &signer, exchange: address) {
        pragma opaque;
        aborts_if signer::address_of(framework) != @aptos_framework;
        aborts_if !exists<ExchangeRegistry>(@aptos_experimental);
        modifies global<ExchangeRegistry>(@aptos_experimental);
        ensures big_ordered_map::spec_contains_key(spec_registry().denied, exchange);
        ensures spec_registry().registered == old(spec_registry().registered);
        ensures spec_registry().denied
            == (if (big_ordered_map::spec_contains_key(old(spec_registry().denied), exchange)) {
                old(spec_registry().denied)
            } else {
                big_ordered_map::spec_set(old(spec_registry().denied), exchange, Empty {})
            });
    }

    spec reenable(framework: &signer, exchange: address) {
        pragma opaque;
        aborts_if signer::address_of(framework) != @aptos_framework;
        aborts_if !exists<ExchangeRegistry>(@aptos_experimental);
        modifies global<ExchangeRegistry>(@aptos_experimental);
        ensures !big_ordered_map::spec_contains_key(spec_registry().denied, exchange);
        ensures spec_registry().registered == old(spec_registry().registered);
        ensures spec_registry().denied
            == (if (big_ordered_map::spec_contains_key(old(spec_registry().denied), exchange)) {
                big_ordered_map::spec_remove(old(spec_registry().denied), exchange)
            } else {
                old(spec_registry().denied)
            });
    }

    spec exchange(cap: &TradingNativeCapability): address {
        pragma opaque;
        aborts_if false;
        ensures result == cap.exchange;
    }
}
