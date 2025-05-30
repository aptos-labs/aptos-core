#[test_only]
module aptos_experimental::clearinghouse_test {
    use std::error;
    use std::option;
    use std::signer;
    use aptos_std::table;
    use aptos_std::table::Table;
    use aptos_experimental::market_types::{
        SettleTradeResult,
        new_settle_trade_result,
        MarketClearinghouseCallbacks,
        new_market_clearinghouse_callbacks
    };

    const EINVALID_ADDRESS: u64 = 1;

    struct TestOrderMetadata has store, copy, drop {}

    public fun new_test_order_metadata(): TestOrderMetadata {
        TestOrderMetadata {}
    }

    struct Position has store, drop {
        size: u64,
        is_long: bool
    }

    struct GlobalState has key {
        user_positions: Table<address, Position>
    }

    public(package) fun initialize(admin: &signer) {
        assert!(
            signer::address_of(admin) == @0x1,
            error::invalid_argument(EINVALID_ADDRESS)
        );
        move_to(admin, GlobalState { user_positions: table::new() });
    }

    public(package) fun validate_order_placement(): bool {
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
    ): SettleTradeResult acquires GlobalState {
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
        new_settle_trade_result(size, option::none(), option::none())
    }

    public(package) fun settle_trade_with_taker_cancelled(
        _taker: address,
        _maker: address,
        size: u64,
        _is_taker_long: bool
    ): SettleTradeResult {
        new_settle_trade_result(
            size / 2,
            option::none(),
            option::some(std::string::utf8(b"Max open interest violation"))
        )
    }

    public(package) fun test_market_callbacks():
        MarketClearinghouseCallbacks<TestOrderMetadata> acquires GlobalState {
        new_market_clearinghouse_callbacks(
            |taker, maker, _taker_order_id, _maker_order_id, is_taker_long, _price, size, _taker_metadata, _maker_metadata| {
                settle_trade(taker, maker, size, is_taker_long)
            },
            | _account, _order_id, _is_taker, _is_bid, _price, _size, _order_metadata| {
                validate_order_placement()
            },
            |_account, _order_id, _is_bid, _price, _size, _order_metadata| {
                // place_maker_order is not used in this test
            },
            | _account, _order_id, _is_bid, _remaining_size| {
                // cleanup_order is not used in this test
            },
            | _account, _order_id, _is_bid, _price, _size| {
                // decrease order size is not used in this test
            },
        )
    }

    public(package) fun test_market_callbacks_with_taker_cancelled():
        MarketClearinghouseCallbacks<TestOrderMetadata> {
        new_market_clearinghouse_callbacks(
            |taker, maker, _taker_order_id, _maker_order_id, is_taker_long, _price, size, _taker_metadata, _maker_metadata| {
                settle_trade_with_taker_cancelled(taker, maker, size, is_taker_long)
            },
            | _account, _order_id, _is_taker, _is_bid, _price, _size, _order_metadata| {
                validate_order_placement()
            },
            |_account, _order_id, _is_bid, _price, _size, _order_metadata| {
                // place_maker_order is not used in this test
            },
            | _account, _order_id, _is_bid, _remaining_size| {
                // cleanup_order is not used in this test
            },
            | _account, _order_id, _is_bid, _price, _size| {
                // decrease order size is not used in this test
            },
        )
    }
}
