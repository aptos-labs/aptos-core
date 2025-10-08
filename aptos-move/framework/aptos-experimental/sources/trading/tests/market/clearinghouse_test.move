#[test_only]
module aptos_experimental::clearinghouse_test {
    use std::error;
    use std::option;
    use std::signer;
    use std::vector;
    use aptos_std::table;
    use aptos_std::table::Table;
    use aptos_experimental::order_book_types::OrderIdType;
    use aptos_experimental::market_types::{
        SettleTradeResult,
        new_settle_trade_result,
        MarketClearinghouseCallbacks,
        new_market_clearinghouse_callbacks,
        new_callback_result_continue_matching,
        new_callback_result_stop_matching, ValidationResult, new_validation_result
    };

    const EINVALID_ADDRESS: u64 = 1;
    const E_DUPLICATE_ORDER: u64 = 2;
    const E_ORDER_NOT_FOUND: u64 = 3;
    const E_ORDER_NOT_CLEANED_UP: u64 = 4;

    struct TestOrderMetadata has store, copy, drop {
        id: u64
    }

    public fun new_test_order_metadata(id: u64): TestOrderMetadata {
        TestOrderMetadata { id}
    }

    public fun get_order_metadata_bytes(
        _order_metadata: TestOrderMetadata
    ): vector<u8> {
        vector::empty<u8>()
    }

    struct Position has store, drop {
        size: u64,
        is_long: bool
    }

    struct GlobalState has key {
        user_positions: Table<address, Position>,
        open_orders: Table<OrderIdType, bool>,
        bulk_open_bids: Table<address, bool>,
        bulk_open_asks: Table<address, bool>,
        maker_order_calls: Table<OrderIdType, bool>
    }

    public(package) fun initialize(admin: &signer) {
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
                maker_order_calls: table::new()
            }
        );
    }

    public(package) fun validate_order_placement(order_id: OrderIdType): ValidationResult<u64> acquires GlobalState {
        let open_orders = &mut borrow_global_mut<GlobalState>(@0x1).open_orders;
        assert!(
            !open_orders.contains(order_id),
            error::invalid_argument(E_DUPLICATE_ORDER)
        );
        open_orders.add(order_id, true);
        return new_validation_result(option::none(), option::none())
    }

    public(package) fun validate_bulk_order_placement(account: address): bool acquires GlobalState {
        let bulk_open_bids = &mut borrow_global_mut<GlobalState>(@0x1).bulk_open_bids;
        if (!bulk_open_bids.contains(account)) {
            bulk_open_bids.add(account, true);
        };
        let bulk_open_asks = &mut borrow_global_mut<GlobalState>(@0x1).bulk_open_asks;
        if (!bulk_open_asks.contains(account)) {
            bulk_open_asks.add(account, true);
        };
        return true
    }

    public(package) fun get_position_size(user: address): u64 acquires GlobalState {
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

    public(package) fun settle_trade(
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
        new_settle_trade_result(size, option::none(), option::none(), new_callback_result_continue_matching(size))
    }

    public(package) fun place_maker_order(order_id: OrderIdType) acquires GlobalState {
        let maker_order_calls =
            &mut borrow_global_mut<GlobalState>(@0x1).maker_order_calls;
        assert!(
            !maker_order_calls.contains(order_id),
            error::invalid_argument(E_DUPLICATE_ORDER)
        );
        maker_order_calls.add(order_id, true);
    }

    public(package) fun is_maker_order_called(order_id: OrderIdType): bool acquires GlobalState {
        let maker_order_calls = &borrow_global<GlobalState>(@0x1).maker_order_calls;
        maker_order_calls.contains(order_id)
    }

    public(package) fun cleanup_order(order_id: OrderIdType) acquires GlobalState {
        let open_orders = &mut borrow_global_mut<GlobalState>(@0x1).open_orders;
        assert!(
            open_orders.contains(order_id),
            error::invalid_argument(E_ORDER_NOT_FOUND)
        );
        open_orders.remove(order_id);
    }

    public(package) fun cleanup_bulk_order(account: address) acquires GlobalState {
        let global_state = borrow_global_mut<GlobalState>(@0x1);
        let bulk_open_bids = &mut global_state.bulk_open_bids;
        let bulk_open_asks = &mut global_state.bulk_open_asks;
        if (!bulk_open_bids.contains(account)
            && !bulk_open_asks.contains(account)) {
            return
        };
        if (bulk_open_asks.contains(account)) {
            bulk_open_asks.remove(account);
        };
        if (bulk_open_bids.contains(account)) {
            bulk_open_bids.remove(account);
        }
    }

    public(package) fun order_exists(order_id: OrderIdType): bool acquires GlobalState {
        let open_orders = &borrow_global<GlobalState>(@0x1).open_orders;
        open_orders.contains(order_id)
    }

    public (package) fun bulk_order_exists(account: address): bool acquires GlobalState {
        let open_orders = &borrow_global<GlobalState>(@0x1).bulk_open_bids;
        open_orders.contains(account)
    }

    public(package) fun settle_trade_with_taker_cancelled(
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

    public(package) fun test_market_callbacks():
        MarketClearinghouseCallbacks<TestOrderMetadata, u64> acquires GlobalState {
        new_market_clearinghouse_callbacks(
            |_market, taker_order_info, maker_order_info, _fill_id, _price, size| {
                settle_trade(taker_order_info.get_account(), maker_order_info.get_account(), size, taker_order_info.is_bid())
            },
            |order_info, _is_taker, _size| {
                validate_order_placement(order_info.get_order_id())
            },
            |account, _bid_sizes, _bid_prices, _ask_sizes, _ask_prices, _order_metadata| {
                validate_bulk_order_placement(account)
            },
            |order_info, _size| {
                place_maker_order(order_info.get_order_id());
            },
            |order_info, _remaining_size| {
                cleanup_order(order_info.get_order_id());
            },
            |account, _order_id, _is_bid, _price, _size| {
                cleanup_bulk_order(account);
            },
            |_order_info, _size| {
                // decrease order size is not used in this test
            },
            |order_metadata| {
                get_order_metadata_bytes(order_metadata)
            }
        )
    }

    public(package) fun test_market_callbacks_with_taker_cancelled():
        MarketClearinghouseCallbacks<TestOrderMetadata, u64> acquires GlobalState {
        new_market_clearinghouse_callbacks(
            |_market, taker_order_info, maker_order_info, _fill_id, _price, size| {
                settle_trade_with_taker_cancelled(taker_order_info.get_account(), maker_order_info.get_account(), size, taker_order_info.is_bid())
            },
            |order_info, _is_taker, _size| {
                validate_order_placement(order_info.get_order_id())
            },
            |account, _bid_sizes, _bid_prices, _ask_sizes, _ask_prices, _order_metadata| {
                validate_bulk_order_placement(account)
            },
            |_order_info, _size| {
                // place_maker_order is not used in this test
            },
            |order_info, _remaining_size| {
                cleanup_order(order_info.get_order_id());
            },
            |account, _order_id, _is_bid, _price, _size| {
                cleanup_bulk_order(account);
            },
            |_order_info, _size| {
                // decrease order size is not used in this test
            },
            |order_metadata| {
                get_order_metadata_bytes(order_metadata)
            }
        )
    }
}
