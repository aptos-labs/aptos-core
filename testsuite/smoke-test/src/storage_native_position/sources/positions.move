module exchange::positions {
    use std::signer;
    use aptos_framework::aptos_governance;
    use aptos_experimental::native_position;
    use aptos_experimental::native_position_types;
    use aptos_experimental::trading_native_capability::{Self, TradingNativeCapability};

    struct CapStore has key {
        cap: TradingNativeCapability,
    }

    public entry fun register_exchange(core_resources: &signer, exchange: address) {
        let framework = aptos_governance::get_signer_testnet_only(core_resources, @aptos_framework);
        trading_native_capability::register(&framework, exchange);
    }

    public entry fun init_cap(exchange: &signer) {
        let cap = trading_native_capability::get_capability(exchange);
        move_to(exchange, CapStore { cap });
    }

    /// Sets `account`'s position on each market in `markets`, sized `size`.
    public entry fun set_positions(
        exchange: &signer,
        account: address,
        markets: vector<address>,
        size: u64,
    ) acquires CapStore {
        let cap = &borrow_global<CapStore>(signer::address_of(exchange)).cap;
        markets.for_each(|market| {
            let position = native_position_types::new_perp_v1(
                size,
                true,
                (size as u128) * 1000,
                1000,
                10,
                false,
                native_position_types::new_accumulative_index(0),
                0,
                0,
            );
            native_position::set_position(cap, market, account, position);
        });
    }

    public entry fun delete_position(
        exchange: &signer,
        account: address,
        market: address,
    ) acquires CapStore {
        let cap = &borrow_global<CapStore>(signer::address_of(exchange)).cap;
        native_position::delete_position(cap, market, account);
    }
}
