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
        settle_trade_f: |address, address, bool, u64, u64| SettleTradeResult has drop + copy,
        // validate_settlement_update_f arguments: account, is_taker, is_long, price, size
        validate_settlement_update_f: |address, bool, bool, u64, u64| bool has drop + copy,
        // max_settlement_size_for_reduce_only_f arguments: account, is_long, orig_size
        max_settlement_size_f: |address, bool, u64, M| Option<u64> has drop + copy,
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
        settle_trade_f: |address, address, bool, u64, u64| SettleTradeResult has drop + copy,
        // validate_settlement_update_f arguments: accoun, is_taker, is_long, price, size
        validate_settlement_update_f: |address, bool, bool, u64, u64| bool has drop + copy,
        // max_settlement_size_for_reduce_only_f arguments: account, is_long, orig_size
        max_settlement_size: |address, bool, u64, M| Option<u64> has drop + copy,
    ): MarketClearinghouseCallbacks<M> {
        MarketClearinghouseCallbacks {
            settle_trade_f,
            validate_settlement_update_f,
            max_settlement_size_f: max_settlement_size,
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

    public fun settle_trade<M: store + copy + drop>(self: &MarketClearinghouseCallbacks<M>, taker: address, maker: address, is_taker_long: bool, price: u64, size: u64): SettleTradeResult {
        (self.settle_trade_f)(taker, maker, is_taker_long, price, size)
    }

    public fun validate_settlement_update<M: store + copy + drop>(self: &MarketClearinghouseCallbacks<M>, account: address, is_taker: bool, is_long: bool, price: u64, size: u64): bool {
        (self.validate_settlement_update_f)(account, is_taker, is_long, price, size)
    }

    public fun max_settlement_size<M: store + copy + drop>(self: &MarketClearinghouseCallbacks<M>, account: address, is_long: bool, orig_size: u64, metadata: M): Option<u64> {
        (self.max_settlement_size_f)(account, is_long, orig_size, metadata)
    }
}
