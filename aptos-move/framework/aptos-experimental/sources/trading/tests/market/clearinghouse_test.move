#[test_only]
module aptos_experimental::clearinghouse_test {
    #[test_only]
    friend aptos_experimental::market_single_order_tests;
    #[test_only]
    friend aptos_experimental::pre_cancellation_tests;
    #[test_only]
    friend aptos_experimental::market_tests_common;
    #[test_only]
    friend aptos_experimental::market_test_utils;
    #[test_only]
    friend aptos_experimental::market_bulk_order_tests;
    #[test_only]
    friend aptos_experimental::market_mixed_order_tests;
    #[test_only]
    friend aptos_experimental::dead_mans_switch_operations_test;

    use std::error;
    use std::option;
    use std::signer;
    use std::vector;
    use aptos_std::table;
    use aptos_std::table::Table;
    use aptos_trading::order_book_types::OrderId;
    use aptos_experimental::market_types::{
        SettleTradeResult,
        new_settle_trade_result,
        MarketClearinghouseCallbacks,
        new_market_clearinghouse_callbacks,
        new_callback_result_continue_matching,
        new_callback_result_stop_matching,
        ValidationResult,
        new_validation_result,
        PlaceMakerOrderResult,
        new_place_maker_order_result
    };

    const EINVALID_ADDRESS: u64 = 1;
    const E_DUPLICATE_ORDER: u64 = 2;
    const E_ORDER_NOT_FOUND: u64 = 3;
    const E_ORDER_NOT_CLEANED_UP: u64 = 4;

    struct TestOrderMetadata has store, copy, drop {
        id: u64
    }

    public fun new_test_order_metadata(id: u64): TestOrderMetadata {
        TestOrderMetadata { id }
    }

    public fun get_order_metadata_bytes(
        _order_metadata: &TestOrderMetadata
    ): vector<u8> {
        vector::empty<u8>()
    }

    struct Position has store, drop {
        size: u64,
        is_long: bool
    }

    struct PlaceBulkOrderCallbackData has store, drop {
        order_id: OrderId,
        bid_prices: vector<u64>,
        bid_sizes: vector<u64>,
        ask_prices: vector<u64>,
        ask_sizes: vector<u64>,
        cancelled_bid_prices: vector<u64>,
        cancelled_bid_sizes: vector<u64>,
        cancelled_ask_prices: vector<u64>,
        cancelled_ask_sizes: vector<u64>
    }

    struct GlobalState has key {
        user_positions: Table<address, Position>,
        open_orders: Table<OrderId, bool>,
        bulk_open_bids: Table<address, bool>,
        bulk_open_asks: Table<address, bool>,
        maker_order_calls: Table<OrderId, bool>,
        place_bulk_order_calls: Table<address, PlaceBulkOrderCallbackData>
    }

    public(friend) fun initialize(admin: &signer) {
        assert!(
            signer::address_of(admin) == @0x1,
            error::invalid_argument(EINVALID_ADDRESS)
        );
        move_to(
            admin,
            GlobalState {
                user_positions: table::new(),
                open_orders: table::new(),
                bulk_open_bids: table::new(),
                bulk_open_asks: table::new(),
                maker_order_calls: table::new(),
                place_bulk_order_calls: table::new()
            }
        );
    }

    public(friend) fun validate_order_placement(
        order_id: OrderId
    ): ValidationResult acquires GlobalState {
        let open_orders = &mut borrow_global_mut<GlobalState>(@0x1).open_orders;
        assert!(
            !open_orders.contains(order_id),
            error::invalid_argument(E_DUPLICATE_ORDER)
        );
        open_orders.add(order_id, true);
        return new_validation_result(option::none())
    }

    public(friend) fun validate_bulk_order_placement(
        account: address
    ): ValidationResult acquires GlobalState {
        let bulk_open_bids = &mut borrow_global_mut<GlobalState>(@0x1).bulk_open_bids;
        if (!bulk_open_bids.contains(account)) {
            bulk_open_bids.add(account, true);
        };
        let bulk_open_asks = &mut borrow_global_mut<GlobalState>(@0x1).bulk_open_asks;
        if (!bulk_open_asks.contains(account)) {
            bulk_open_asks.add(account, true);
        };
        return new_validation_result(option::none())
    }

    public(friend) fun get_position_size(user: address): u64 acquires GlobalState {
        let user_positions = &borrow_global<GlobalState>(@0x1).user_positions;
        if (!user_positions.contains(user)) {
            return 0;
        };
        user_positions.borrow(user).size
    }

    fun update_position(
        position: &mut Position, size: u64, is_bid: bool
    ) {
        if (position.is_long != is_bid) {
            if (size > position.size) {
                position.size = size - position.size;
                position.is_long = is_bid;
            } else {
                position.size -= size;
            }
        } else {
            position.size += size;
        }
    }

    public(friend) fun settle_trade(
        taker: address,
        maker: address,
        size: u64,
        is_taker_long: bool
    ): SettleTradeResult<u64> acquires GlobalState {
        let user_positions = &mut borrow_global_mut<GlobalState>(@0x1).user_positions;
        let taker_position =
            user_positions.borrow_mut_with_default(
                taker, Position { size: 0, is_long: true }
            );
        update_position(taker_position, size, is_taker_long);
        let maker_position =
            user_positions.borrow_mut_with_default(
                maker, Position { size: 0, is_long: true }
            );
        update_position(maker_position, size, !is_taker_long);
        new_settle_trade_result(
            size,
            option::none(),
            option::none(),
            new_callback_result_continue_matching(size)
        )
    }

    public(friend) fun place_maker_order(
        order_id: OrderId
    ): PlaceMakerOrderResult<u64> acquires GlobalState {
        let maker_order_calls =
            &mut borrow_global_mut<GlobalState>(@0x1).maker_order_calls;
        assert!(
            !maker_order_calls.contains(order_id),
            error::invalid_argument(E_DUPLICATE_ORDER)
        );
        maker_order_calls.add(order_id, true);
        new_place_maker_order_result(option::none(), option::none())
    }

    public(friend) fun is_maker_order_called(order_id: OrderId): bool acquires GlobalState {
        let maker_order_calls = &borrow_global<GlobalState>(@0x1).maker_order_calls;
        maker_order_calls.contains(order_id)
    }

    public(friend) fun cleanup_order(order_id: OrderId) acquires GlobalState {
        let open_orders = &mut borrow_global_mut<GlobalState>(@0x1).open_orders;
        assert!(
            open_orders.contains(order_id),
            error::invalid_argument(E_ORDER_NOT_FOUND)
        );
        open_orders.remove(order_id);
    }

    public(friend) fun cleanup_bulk_order(account: address) acquires GlobalState {
        let global_state = borrow_global_mut<GlobalState>(@0x1);
        let bulk_open_bids = &mut global_state.bulk_open_bids;
        let bulk_open_asks = &mut global_state.bulk_open_asks;
        if (!bulk_open_bids.contains(account) && !bulk_open_asks.contains(account)) {
            return
        };
        bulk_open_asks.remove(account);
        bulk_open_bids.remove(account);
    }

    public(friend) fun order_exists(order_id: OrderId): bool acquires GlobalState {
        let open_orders = &borrow_global<GlobalState>(@0x1).open_orders;
        open_orders.contains(order_id)
    }

    public(friend) fun bulk_order_exists(account: address): bool acquires GlobalState {
        let open_orders = &borrow_global<GlobalState>(@0x1).bulk_open_bids;
        open_orders.contains(account)
    }

    public(friend) fun record_place_bulk_order(
        account: address,
        order_id: OrderId,
        bid_prices: &vector<u64>,
        bid_sizes: &vector<u64>,
        ask_prices: &vector<u64>,
        ask_sizes: &vector<u64>,
        cancelled_bid_prices: &vector<u64>,
        cancelled_bid_sizes: &vector<u64>,
        cancelled_ask_prices: &vector<u64>,
        cancelled_ask_sizes: &vector<u64>
    ) acquires GlobalState {
        let place_bulk_order_calls =
            &mut borrow_global_mut<GlobalState>(@0x1).place_bulk_order_calls;
        let callback_data = PlaceBulkOrderCallbackData {
            order_id,
            bid_prices: *bid_prices,
            bid_sizes: *bid_sizes,
            ask_prices: *ask_prices,
            ask_sizes: *ask_sizes,
            cancelled_bid_prices: *cancelled_bid_prices,
            cancelled_bid_sizes: *cancelled_bid_sizes,
            cancelled_ask_prices: *cancelled_ask_prices,
            cancelled_ask_sizes: *cancelled_ask_sizes
        };
        if (place_bulk_order_calls.contains(account)) {
            place_bulk_order_calls.remove(account);
        };
        place_bulk_order_calls.add(account, callback_data);
    }

    public(friend) fun place_bulk_order_callback_called(account: address): bool acquires GlobalState {
        let place_bulk_order_calls =
            &borrow_global<GlobalState>(@0x1).place_bulk_order_calls;
        place_bulk_order_calls.contains(account)
    }

    public(friend) fun get_place_bulk_order_callback_data(
        account: address
    ): (
        OrderId,
        vector<u64>,
        vector<u64>,
        vector<u64>,
        vector<u64>,
        vector<u64>,
        vector<u64>,
        vector<u64>,
        vector<u64>
    ) acquires GlobalState {
        let place_bulk_order_calls =
            &borrow_global<GlobalState>(@0x1).place_bulk_order_calls;
        let callback_data = place_bulk_order_calls.borrow(account);
        (
            callback_data.order_id,
            callback_data.bid_prices,
            callback_data.bid_sizes,
            callback_data.ask_prices,
            callback_data.ask_sizes,
            callback_data.cancelled_bid_prices,
            callback_data.cancelled_bid_sizes,
            callback_data.cancelled_ask_prices,
            callback_data.cancelled_ask_sizes
        )
    }

    public(friend) fun settle_trade_with_taker_cancelled(
        _taker: address,
        _maker: address,
        size: u64,
        _is_taker_long: bool
    ): SettleTradeResult<u64> {
        new_settle_trade_result(
            size / 2,
            option::none(),
            option::some(std::string::utf8(b"Max open interest violation")),
            new_callback_result_stop_matching(size)
        )
    }

    /// Similar to settle_trade_with_taker_cancelled but does NOT provide a taker cancellation reason.
    /// This tests the scenario where the clearinghouse signals to stop matching without cancelling
    /// the taker order explicitly. The trade is fully settled but then matching is stopped.
    public(friend) fun settle_trade_with_stop_matching(
        _taker: address,
        _maker: address,
        size: u64,
        _is_taker_long: bool
    ): SettleTradeResult<u64> {
        new_settle_trade_result(
            size, // Fully settle this trade
            option::none(),
            option::none(), // No taker cancellation reason
            new_callback_result_stop_matching(size)
        )
    }

    public(friend) fun test_market_callbacks()
        : MarketClearinghouseCallbacks<TestOrderMetadata, u64> acquires GlobalState {
        new_market_clearinghouse_callbacks(
            |
                _market,
                taker_order_info,
                maker_order_info,
                _fill_id,
                _price,
                size
            | {
                settle_trade(
                    taker_order_info.get_account(),
                    maker_order_info.get_account(),
                    size,
                    taker_order_info.is_bid()
                )
            },
            |order_info, _size| {
                validate_order_placement(order_info.get_order_id())
            },
            |
                account,
                _bid_prices,
                _bid_sizes,
                _ask_prices,
                _ask_sizes,
                _order_metadata
            | { validate_bulk_order_placement(account) },
            |order_info, _size| {
                place_maker_order(order_info.get_order_id())
            },
            |order_info, _remaining_size, _| {
                cleanup_order(order_info.get_order_id());
            },
            |account, _order_id, _is_bid, _price, _size| {
                cleanup_bulk_order(account);
            },
            |
                account,
                order_id,
                bid_prices,
                bid_sizes,
                ask_prices,
                ask_sizes,
                cancelled_bid_prices,
                cancelled_bid_sizes,
                cancelled_ask_prices,
                cancelled_ask_sizes,
                _metadata
            | {
                record_place_bulk_order(
                    account,
                    order_id,
                    bid_prices,
                    bid_sizes,
                    ask_prices,
                    ask_sizes,
                    cancelled_bid_prices,
                    cancelled_bid_sizes,
                    cancelled_ask_prices,
                    cancelled_ask_sizes
                );
            },
            |_order_info, _size| {
                // decrease order size is not used in this test
            },
            |order_metadata| { get_order_metadata_bytes(order_metadata) }
        )
    }

    public(friend) fun test_market_callbacks_with_taker_cancelled()
        : MarketClearinghouseCallbacks<TestOrderMetadata, u64> acquires GlobalState {
        new_market_clearinghouse_callbacks(
            |
                _market,
                taker_order_info,
                maker_order_info,
                _fill_id,
                _price,
                size
            | {
                settle_trade_with_taker_cancelled(
                    taker_order_info.get_account(),
                    maker_order_info.get_account(),
                    size,
                    taker_order_info.is_bid()
                )
            },
            |order_info, _size| {
                validate_order_placement(order_info.get_order_id())
            },
            |
                account,
                _bid_prices,
                _bid_sizes,
                _ask_prices,
                _ask_sizes,
                _order_metadata
            | { validate_bulk_order_placement(account) },
            |_order_info, _size| {
                new_place_maker_order_result(option::none(), option::none())
                // place_maker_order is not used in this test
            },
            |order_info, _remaining_size, _| {
                cleanup_order(order_info.get_order_id());
            },
            |account, _order_id, _is_bid, _price, _size| {
                cleanup_bulk_order(account);
            },
            |
                _account,
                _order_id,
                _bid_prices,
                _bid_sizes,
                _ask_prices,
                _ask_sizes,
                _cancelled_bid_prices,
                _cancelled_bid_sizes,
                _cancelled_ask_prices,
                _cancelled_ask_sizes,
                _metadata
            | {
                // place_bulk_order callback - no-op for this test
            },
            |_order_info, _size| {
                // decrease order size is not used in this test
            },
            |order_metadata| { get_order_metadata_bytes(order_metadata) }
        )
    }

    /// Test callbacks that stop matching without providing a taker cancellation reason.
    /// This is used to test the ClearinghouseStoppedMatching cancellation reason.
    public(friend) fun test_market_callbacks_with_stop_matching()
        : MarketClearinghouseCallbacks<TestOrderMetadata, u64> acquires GlobalState {
        new_market_clearinghouse_callbacks(
            |
                _market,
                taker_order_info,
                maker_order_info,
                _fill_id,
                _price,
                size
            | {
                settle_trade_with_stop_matching(
                    taker_order_info.get_account(),
                    maker_order_info.get_account(),
                    size,
                    taker_order_info.is_bid()
                )
            },
            |order_info, _size| {
                validate_order_placement(order_info.get_order_id())
            },
            |
                account,
                _bid_prices,
                _bid_sizes,
                _ask_prices,
                _ask_sizes,
                _order_metadata
            | { validate_bulk_order_placement(account) },
            |_order_info, _size| {
                new_place_maker_order_result(option::none(), option::none())
            },
            |order_info, _remaining_size, _| {
                cleanup_order(order_info.get_order_id());
            },
            |account, _order_id, _is_bid, _price, _size| {
                cleanup_bulk_order(account);
            },
            |
                _account,
                _order_id,
                _bid_prices,
                _bid_sizes,
                _ask_prices,
                _ask_sizes,
                _cancelled_bid_prices,
                _cancelled_bid_sizes,
                _cancelled_ask_prices,
                _cancelled_ask_sizes,
                _metadata
            | {
                // place_bulk_order callback - no-op for this test
            },
            |_order_info, _size| {
                // decrease order size is not used in this test
            },
            |order_metadata| { get_order_metadata_bytes(order_metadata) }
        )
    }

    public(friend) fun test_market_callbacks_with_maker_cancellled()
        : MarketClearinghouseCallbacks<TestOrderMetadata, u64> acquires GlobalState {
        new_market_clearinghouse_callbacks(
            |
                _market,
                taker_order_info,
                maker_order_info,
                _fill_id,
                _price,
                size
            | {
                settle_trade(
                    taker_order_info.get_account(),
                    maker_order_info.get_account(),
                    size,
                    taker_order_info.is_bid()
                )
            },
            |order_info, _size| {
                validate_order_placement(order_info.get_order_id())
            },
            |
                account,
                _bid_sizes,
                _bid_prices,
                _ask_sizes,
                _ask_prices,
                _order_metadata
            | { validate_bulk_order_placement(account) },
            |_order_info, _size| {
                new_place_maker_order_result(
                    option::some(std::string::utf8(b"cancelled")),
                    option::none()
                )
            },
            |order_info, _remaining_size, _| {
                cleanup_order(order_info.get_order_id());
            },
            |account, _order_id, _is_bid, _price, _size| {
                cleanup_bulk_order(account);
            },
            |
                _account,
                _order_id,
                _bid_prices,
                _bid_sizes,
                _ask_prices,
                _ask_sizes,
                _cancelled_bid_prices,
                _cancelled_bid_sizes,
                _cancelled_ask_prices,
                _cancelled_ask_sizes,
                _metadata
            | {
                // place_bulk_order callback - no-op for this test
            },
            |_order_info, _size| {
                // decrease order size is not used in this test
            },
            |order_metadata| { get_order_metadata_bytes(order_metadata) }
        )
    }
}
