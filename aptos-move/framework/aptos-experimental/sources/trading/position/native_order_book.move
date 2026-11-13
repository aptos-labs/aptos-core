/// Native order storage, keyed by `(exchange, market, order_id)` for single
/// orders and `(exchange, market, account)` for bulk orders. Write API is
/// `set_*` / `delete_*`, gated by a `TradingNativeCapability` (re-checked via
/// `assert_valid` on every call). Native state is read only on the validator
/// side (Rust), never from Move.
module aptos_experimental::native_order_book {
    use aptos_trading::order_book_types::OrderId;
    use aptos_experimental::native_order_types::{NativeSingleOrder, NativeBulkOrder};
    use aptos_experimental::trading_native_capability::{Self, TradingNativeCapability};

    /// Store a single order at `(exchange, market, order.order_id)`. `exchange`
    /// is derived from the cap; every call re-checks the cap via `assert_valid`.
    public fun set_single_order<M: store + copy + drop>(
        cap: &TradingNativeCapability,
        market: address,
        order: NativeSingleOrder<M>,
    ) {
        trading_native_capability::assert_valid(cap);
        native_set_single_order(trading_native_capability::exchange(cap), market, order);
    }

    /// Delete the single order at `(exchange, market, order_id)`.
    public fun delete_single_order(
        cap: &TradingNativeCapability,
        market: address,
        order_id: OrderId,
    ) {
        trading_native_capability::assert_valid(cap);
        native_delete_single_order(trading_native_capability::exchange(cap), market, order_id);
    }

    /// Store a bulk order at `(exchange, market, order.account)`.
    public fun set_bulk_order<M: store + copy + drop>(
        cap: &TradingNativeCapability,
        market: address,
        order: NativeBulkOrder<M>,
    ) {
        trading_native_capability::assert_valid(cap);
        native_set_bulk_order(trading_native_capability::exchange(cap), market, order);
    }

    /// Delete the bulk order at `(exchange, market, account)`.
    public fun delete_bulk_order(
        cap: &TradingNativeCapability,
        market: address,
        account: address,
    ) {
        trading_native_capability::assert_valid(cap);
        native_delete_bulk_order(trading_native_capability::exchange(cap), market, account);
    }

    native fun native_set_single_order<M: store + copy + drop>(
        exchange: address,
        market: address,
        order: NativeSingleOrder<M>,
    );
    native fun native_delete_single_order(
        exchange: address,
        market: address,
        order_id: OrderId,
    );
    native fun native_set_bulk_order<M: store + copy + drop>(
        exchange: address,
        market: address,
        order: NativeBulkOrder<M>,
    );
    native fun native_delete_bulk_order(
        exchange: address,
        market: address,
        account: address,
    );

    // =====================================================================
    // Tests
    // =====================================================================

    #[test_only]
    use std::option;
    #[test_only]
    use std::signer;
    #[test_only]
    use std::string;
    #[test_only]
    use aptos_framework::features;
    #[test_only]
    use aptos_trading::order_book_types::{
        good_till_cancelled,
        new_increasing_idx_type,
        new_order_id_type,
        price_move_up_condition
    };
    #[test_only]
    use aptos_experimental::native_order_types::{new_bulk_order_v1, new_single_order_v1};

    #[test_only]
    struct TestMetadata has store, copy, drop {
        tag: u64
    }

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

    #[test_only]
    fun single_order(): NativeSingleOrder<TestMetadata> {
        new_single_order_v1(
            @0x9,
            new_order_id_type(7),
            option::some(string::utf8(b"coid")),
            100,
            10,
            10,
            true,
            option::some(price_move_up_condition(90)),
            good_till_cancelled(),
            0,
            new_increasing_idx_type(1),
            TestMetadata { tag: 1 }
        )
    }

    #[test_only]
    fun bulk_order(): NativeBulkOrder<TestMetadata> {
        new_bulk_order_v1(
            @0x9,
            new_order_id_type(8),
            1,
            vector[100, 99],
            vector[1, 2],
            vector[101],
            vector[3],
            new_increasing_idx_type(2),
            0,
            TestMetadata { tag: 2 }
        )
    }

    // Unit tests run with the default native feature set, where
    // NATIVE_ORDERBOOK is off, so each native aborts at its flag check.

    #[test(framework = @aptos_framework, trading = @aptos_experimental, exchange = @0x1234)]
    #[expected_failure(abort_code = 1, location = Self)]
    fun test_set_single_order_requires_native_orderbook(
        framework: &signer, trading: &signer, exchange: &signer
    ) {
        let cap = setup(framework, trading, exchange);
        set_single_order(&cap, @0x77, single_order());
    }

    #[test(framework = @aptos_framework, trading = @aptos_experimental, exchange = @0x1234)]
    #[expected_failure(abort_code = 1, location = Self)]
    fun test_delete_single_order_requires_native_orderbook(
        framework: &signer, trading: &signer, exchange: &signer
    ) {
        let cap = setup(framework, trading, exchange);
        delete_single_order(&cap, @0x77, new_order_id_type(7));
    }

    #[test(framework = @aptos_framework, trading = @aptos_experimental, exchange = @0x1234)]
    #[expected_failure(abort_code = 1, location = Self)]
    fun test_set_bulk_order_requires_native_orderbook(
        framework: &signer, trading: &signer, exchange: &signer
    ) {
        let cap = setup(framework, trading, exchange);
        set_bulk_order(&cap, @0x77, bulk_order());
    }

    #[test(framework = @aptos_framework, trading = @aptos_experimental, exchange = @0x1234)]
    #[expected_failure(abort_code = 1, location = Self)]
    fun test_delete_bulk_order_requires_native_orderbook(
        framework: &signer, trading: &signer, exchange: &signer
    ) {
        let cap = setup(framework, trading, exchange);
        delete_bulk_order(&cap, @0x77, @0x9);
    }

    #[test(framework = @aptos_framework, trading = @aptos_experimental, exchange = @0x1234)]
    #[expected_failure(
        abort_code = 0x50003, location = aptos_experimental::trading_native_capability
    )]
    fun test_denied_exchange_cannot_write_orders(
        framework: &signer, trading: &signer, exchange: &signer
    ) {
        let cap = setup(framework, trading, exchange);
        trading_native_capability::deny(framework, signer::address_of(exchange));
        set_single_order(&cap, @0x77, single_order());
    }
}
