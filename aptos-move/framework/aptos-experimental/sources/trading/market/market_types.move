module aptos_experimental::market_types {
    use std::option::Option;
    use std::string::String;

    const EINVALID_ADDRESS: u64 = 1;
    const EINVALID_SETTLE_RESULT: u64 = 2;

    struct SettleTradeResult has drop {
        settled_size: u64,
        maker_cancellation_reason: Option<String>,
        taker_cancellation_reason: Option<String>
    }

    struct MarketClearinghouseCallbacks<M: store + copy + drop> has drop {
        // settle_trade_f arguments: taker, maker, is_taker_long, price, size
        settle_trade_f:  |address, address, bool, u64, u64, M, M| SettleTradeResult has drop + copy,
        // validate_settlement_update_f arguments: account, is_taker, is_long, price, size
        validate_order_placement_f: |address, bool, bool, u64, u64, M| bool has drop + copy,
    }

    public fun new_settle_trade_result(
        settled_size: u64,
        maker_cancellation_reason: Option<String>,
        taker_cancellation_reason: Option<String>
    ): SettleTradeResult {
        SettleTradeResult {
            settled_size,
            maker_cancellation_reason,
            taker_cancellation_reason
        }
    }

    public fun new_market_clearinghouse_callbacks<M: store + copy + drop>(
        // settle_trade_f arguments: taker, maker, is_taker_long, price, size
        settle_trade_f: |address, address, bool, u64, u64, M, M| SettleTradeResult has drop + copy,
        // validate_settlement_update_f arguments: accoun, is_taker, is_long, price, size
        validate_order_placement_f: |address, bool, bool, u64, u64, M| bool has drop + copy,
    ): MarketClearinghouseCallbacks<M> {
        MarketClearinghouseCallbacks {
            settle_trade_f,
            validate_order_placement_f,
        }
    }

    public fun get_settled_size(self: &SettleTradeResult): u64 {
        self.settled_size
    }

    public fun get_maker_cancellation_reason(self: &SettleTradeResult): Option<String> {
        self.maker_cancellation_reason
    }

    public fun get_taker_cancellation_reason(self: &SettleTradeResult): Option<String> {
        self.taker_cancellation_reason
    }

    public fun settle_trade<M: store + copy + drop>(self: &MarketClearinghouseCallbacks<M>, taker: address, maker: address, is_taker_long: bool, price: u64, size: u64, taker_metadata: M, maker_metadata: M): SettleTradeResult {
        (self.settle_trade_f)(taker, maker, is_taker_long, price, size, taker_metadata, maker_metadata)
    }

    public fun validate_order_placement<M: store + copy + drop>(self: &MarketClearinghouseCallbacks<M>, account: address, is_taker: bool, is_long: bool, price: u64, size: u64, order_metadata: M): bool {
        (self.validate_order_placement_f)(account, is_taker, is_long, price, size, order_metadata)
    }
}
