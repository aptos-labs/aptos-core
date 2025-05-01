#[test_only]
module aptos_experimental::clearinghouse_test {
    use std::error;
    use std::option;
    use std::option::Option;
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

    struct GlobalState has key {
        user_positions: Table<address, u64>
    }

    public(package) fun initialize(admin: &signer) {
        assert!(
            signer::address_of(admin) == @0x1,
            error::invalid_argument(EINVALID_ADDRESS)
        );
        move_to(admin, GlobalState { user_positions: table::new() });
    }

    public(package) fun max_settlement_size<M: store + copy + drop>(
        orig_size: u64
    ): Option<u64> {
        return option::some(orig_size)
    }

    public(package) fun validate_settlement_update(): bool {
        return true
    }

    public(package) fun get_position_size(user: address): u64 acquires GlobalState {
        let user_positions = &borrow_global<GlobalState>(@0x1).user_positions;
        if (!user_positions.contains(user)) {
            return 0;
        };
        *user_positions.borrow(user)
    }

    public(package) fun settle_trade(
        taker: address, maker: address, size: u64
    ): SettleTradeResult acquires GlobalState {
        let user_positions =
            &mut borrow_global_mut<GlobalState>(@0x1).user_positions;
        let taker_position = user_positions.borrow_mut_with_default(taker, 0);
        *taker_position += size;
        let maker_position = user_positions.borrow_mut_with_default(maker, 0);
        // TODO(skedia): Track the position direction and reduce the size if applicable.
        *maker_position += size;
        new_settle_trade_result(size, option::none(), option::none())
    }

    public(package) fun test_market_callbacks():
        MarketClearinghouseCallbacks<TestOrderMetadata> acquires GlobalState {
        new_market_clearinghouse_callbacks(
            |taker, maker, _is_taker_long, _price, size| {
                settle_trade(taker, maker, size)
            },
            |_account, _is_taker, _is_long, _price, _size| { validate_settlement_update() },
            |_user_addr, _is_buy, orig_size, _metadata| {
                max_settlement_size<TestOrderMetadata>(orig_size)
            }
        )
    }
}
