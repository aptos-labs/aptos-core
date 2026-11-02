/// Benchmark driver for `aptos_experimental::native_position`. Every user
/// writes its own `(exchange, market, user)` position, so the transactions
/// are conflict-free under Block-STM and measure the native write path
/// rather than contention.
module 0xABCD::native_position_bench {
    use std::signer;
    use aptos_framework::aptos_governance;
    use aptos_experimental::native_position;
    use aptos_experimental::native_position_types;
    use aptos_experimental::trading_native_capability::{Self, TradingNativeCapability};

    /// `init_cap` has not run for this exchange.
    const ENOT_INITIALIZED: u64 = 1;

    /// The publisher's capability, minted once and reused by every writer.
    struct CapStore has key {
        cap: TradingNativeCapability,
    }

    /// Root-signed, once per package: enroll the publisher as an exchange.
    /// On test genesis the root account holds the mint capability, which
    /// is what lets it borrow the framework signer.
    public entry fun register_exchange(core_resources: &signer, exchange: address) {
        let framework = aptos_governance::get_signer_testnet_only(core_resources, @aptos_framework);
        trading_native_capability::register(&framework, exchange);
    }

    /// Publisher-signed, once per package, after `register_exchange`.
    public entry fun init_cap(exchange: &signer) {
        if (!exists<CapStore>(signer::address_of(exchange))) {
            let cap = trading_native_capability::get_capability(exchange);
            move_to(exchange, CapStore { cap });
        }
    }

    /// Write the caller's position on `market`.
    public entry fun set_position(
        user: &signer,
        exchange: address,
        market: address,
        size: u64,
    ) acquires CapStore {
        assert!(exists<CapStore>(exchange), ENOT_INITIALIZED);
        let cap = &borrow_global<CapStore>(exchange).cap;
        let position = native_position_types::new_perp_v1(
            size,
            /* is_long */ true,
            (size as u128) * 1000,
            /* avg_acquire_entry_px */ 1000,
            /* user_leverage */ 10,
            /* is_isolated */ false,
            native_position_types::new_accumulative_index(0),
            /* unrealized_funding_amount_before_last_update */ 0,
            /* timestamp */ 0,
        );
        native_position::set_position(cap, market, signer::address_of(user), position);
    }

    /// Delete the caller's position on `market`.
    public entry fun delete_position(
        user: &signer,
        exchange: address,
        market: address,
    ) acquires CapStore {
        assert!(exists<CapStore>(exchange), ENOT_INITIALIZED);
        let cap = &borrow_global<CapStore>(exchange).cap;
        native_position::delete_position(cap, market, signer::address_of(user));
    }
}
