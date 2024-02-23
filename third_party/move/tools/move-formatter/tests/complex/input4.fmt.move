module oracle::oracle {
    use std::error;
    use std::signer::{ address_of };
    use aptos_std::table::{Self, Table};
    use aptos_std::type_info::{TypeInfo, type_of};
    use aptos_framework::account::{ new_event_handle };
    use aptos_framework::event::{emit_event, EventHandle};

    use switchboard::aggregator::{Self as switchboard_aggregator};
    use switchboard::math::{
        Self as switchboard_math,
        SwitchboardDecimal
    };

    struct Oracle has store {
        feed: address,
        adapter: u8,
    }

    struct OracleStore has key {
        oracles: Table<TypeInfo, Oracle>,
        update_oracle_events: EventHandle<UpdateOracleEvent>,
    }

    // Errors
    const E_BAD_SIGNER: u64 = 1;
    const E_BAD_FEED: u64 = 2;
    const E_BAD_PRICE: u64 = 3;

    // Events

    struct UpdateOracleEvent has drop, store {
        feed: address,
        adapter: u8,
        coin_type: TypeInfo,
    }

    // Adapters
    const ADAPTER_SWITCHBOARD: u8 = 1;
    // const ADAPTER_PYTH: u8 = 2;

    public fun switchboard_adapter(): u8 {
         ADAPTER_SWITCHBOARD
    }

    // Functions

    fun init_module(oracle: &signer) {
        move_to(
            oracle,
            OracleStore {
                oracles: table::new(),
                update_oracle_events: new_event_handle(oracle),
            }
        );
    }

    fun assert_feed_adapter(feed: address, adapter: u8) {
        if (adapter == ADAPTER_SWITCHBOARD) {
            assert!(
                switchboard_aggregator::exist(feed),
                error::not_found(E_BAD_FEED)
            );
            // } else if (adapter == ADAPTER_PYTH) {
        } else {
            abort error::not_found(E_BAD_FEED)
        };
    }

    public entry fun update_oracle<CoinType>(
        oracle: &signer,
        feed: address,
        adapter: u8
    )
        acquires OracleStore {
        assert!(
            address_of(oracle) == @oracle,
            error::unauthenticated(E_BAD_SIGNER)
        );
        assert_feed_adapter(feed, adapter);

        let oracle_store = borrow_global_mut<OracleStore>(@oracle);
        let coin_type = type_of<CoinType>();

        if (table::contains(&oracle_store.oracles, coin_type)) {
            let oracle = table::borrow_mut(
                &mut oracle_store.oracles,
                coin_type
            );
            oracle.feed = feed;
            oracle.adapter = adapter;
        } else {
            table::add(
                &mut oracle_store.oracles,
                coin_type,
                Oracle {feed, adapter}
            );
        };

        emit_event(
            &mut oracle_store.update_oracle_events,
            UpdateOracleEvent {feed, adapter, coin_type,}
        );
    }

    public fun lookup(coin_type: TypeInfo): (address, u8)
        acquires OracleStore {
        let oracle_store = borrow_global<OracleStore>(@oracle);
        let oracle = table::borrow(&oracle_store.oracles, coin_type);
        (oracle.feed, oracle.adapter)
    }

    public fun get_price<CoinType>(): u64
        acquires OracleStore {
        let coin_type = type_of<CoinType>();
        get_price_by_type(coin_type)
    }

    public fun get_price_by_type(coin_type: TypeInfo): u64
        acquires OracleStore {
        let (feed, adapter) = lookup(coin_type);

        if (adapter == ADAPTER_SWITCHBOARD) switchboard_get_price(feed)
        // TODO: support pyth
        // else if (adapter == ADAPTER_PYTH) pyth_get_price(feed)
        else abort error::not_found(E_BAD_FEED)
    }

    // TODO: validate timestamp
    fun switchboard_get_price(feed: address): u64 {
        let latest_value = switchboard_aggregator::latest_value(feed);
        decimal_to_u64(latest_value)
    }

    fun decimal_to_u64(decimal: SwitchboardDecimal): u64 {
        let (value, dec, neg) = switchboard_math::unpack(decimal);
        assert!(
            !neg,
            error::out_of_range(E_BAD_PRICE)
        );
        assert!(
            dec <= 9,
            error::out_of_range(E_BAD_PRICE)
        );
        assert!(
            value > 0,
            error::out_of_range(E_BAD_PRICE)
        );
        (value * base(9 - dec) as u64)
    }

    public fun base(n: u8): u128 {
        if (n == 9) 1000000000 else
            if (n == 8) 100000000 else
                if (n == 7) 10000000 else
                    if (n == 6) 1000000 else
                        if (n == 5) 100000 else
                            if (n == 4) 10000 else
                                if (n == 3) 1000 else
                                    if (n == 2) 100 else
                                        if (n == 1) 10 else
                                            if (n == 0) 1 else abort 0
    }

    // Tests

    #[test_only]
    public fun init_test(oracle: &signer) {
        init_module(oracle);
    }
}