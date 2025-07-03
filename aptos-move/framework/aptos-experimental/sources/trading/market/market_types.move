module aptos_experimental::market_types {
    use std::option::Option;
    use std::string::String;

    friend aptos_experimental::market;

    const EINVALID_ADDRESS: u64 = 1;
    const EINVALID_SETTLE_RESULT: u64 = 2;
    const EINVALID_TIME_IN_FORCE: u64 = 3;

    /// Order time in force
    enum TimeInForce has drop, copy, store {
        /// Good till cancelled order type
        GTC,
        /// Post Only order type - ensures that the order is not a taker order
        POST_ONLY,
        /// Immediate or Cancel order type - ensures that the order is a taker order. Try to match as much of the
        /// order as possible as taker order and cancel the rest.
        IOC
    }

    public fun time_in_force_from_index(index: u8): TimeInForce {
        if (index == 0) {
            TimeInForce::GTC
        } else if (index == 1) {
            TimeInForce::POST_ONLY
        } else if (index == 2) {
            TimeInForce::IOC
        } else {
            abort EINVALID_TIME_IN_FORCE
        }
    }

    public fun good_till_cancelled(): TimeInForce {
        TimeInForce::GTC
    }

    public fun post_only(): TimeInForce {
        TimeInForce::POST_ONLY
    }

    public fun immediate_or_cancel(): TimeInForce {
        TimeInForce::IOC
    }

    enum OrderStatus has drop, copy, store {
        /// Order has been accepted by the engine.
        OPEN,
        /// Order has been fully or partially filled.
        FILLED,
        /// Order has been cancelled by the user or engine.
        CANCELLED,
        /// Order has been rejected by the engine. Unlike cancelled orders, rejected
        /// orders are invalid orders. Rejection reasons:
        /// 1. Insufficient margin
        /// 2. Order is reduce_only but does not reduce
        REJECTED,
        SIZE_REDUCED
    }

    public fun order_status_open(): OrderStatus {
        OrderStatus::OPEN
    }

    public fun order_status_filled(): OrderStatus {
        OrderStatus::FILLED
    }

    public fun order_status_cancelled(): OrderStatus {
        OrderStatus::CANCELLED
    }

    public fun order_status_rejected(): OrderStatus {
        OrderStatus::REJECTED
    }

    public fun order_status_size_reduced(): OrderStatus {
        OrderStatus::SIZE_REDUCED
    }

    enum SettleTradeResult has drop {
        V1 {
            settled_size: u64,
            maker_cancellation_reason: Option<String>,
            taker_cancellation_reason: Option<String>,
        }
    }

    enum MarketClearinghouseCallbacks<M: store + copy + drop> has drop {
        V1 {
            // settle_trade_f arguments: taker, maker, taker_order_id, maker_order_id, fill_id, is_taker_long, price, size
            settle_trade_f:  |address, address, u64, u64, u64, bool, u64, u64, M, M| SettleTradeResult has drop + copy,
            // validate_settlement_update_f arguments: account, is_taker, is_long, price, size
            validate_order_placement_f: |address, u64, bool, bool, Option<u64>, u64, M| bool has drop + copy,
            // place_maker_order_f arguments: account, order_id, is_bid, price, size, order_metadata
            place_maker_order_f: |address, u64, bool, u64, u64, M| has drop + copy,
            // cleanup_order_f arguments: account, order_id, is_bid, remaining_size
            cleanup_order_f: |address, u64, bool, u64| has drop + copy,
            // decrease_order_size_f arguments: account, order_id, is_bid, price, size
            decrease_order_size_f: |address, u64, bool, u64, u64| has drop + copy,
        }
    }

    public fun new_settle_trade_result(
        settled_size: u64,
        maker_cancellation_reason: Option<String>,
        taker_cancellation_reason: Option<String>
    ): SettleTradeResult {
        SettleTradeResult::V1 {
            settled_size,
            maker_cancellation_reason,
            taker_cancellation_reason
        }
    }

    public fun new_market_clearinghouse_callbacks<M: store + copy + drop>(
        // settle_trade_f arguments: taker, maker, taker_order_id, maker_order_id, fill_id, is_taker_long, price, size
        settle_trade_f: |address, address, u64, u64, u64, bool, u64, u64, M, M| SettleTradeResult has drop + copy,
        // validate_settlement_update_f arguments: accoun, is_taker, is_long, price, size
        validate_order_placement_f: |address, u64, bool, bool, Option<u64>, u64, M| bool has drop + copy,
        place_maker_order_f: |address, u64, bool, u64, u64, M| has drop + copy,
        cleanup_order_f: |address, u64, bool, u64| has drop + copy,
        decrease_order_size_f: |address, u64, bool, u64, u64| has drop + copy,
    ): MarketClearinghouseCallbacks<M> {
        MarketClearinghouseCallbacks::V1 {
            settle_trade_f,
            validate_order_placement_f,
            place_maker_order_f,
            cleanup_order_f,
            decrease_order_size_f
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

    public(friend) fun settle_trade<M: store + copy + drop>(
        self: &MarketClearinghouseCallbacks<M>,
        taker: address,
        maker: address,
        taker_order_id: u64,
        maker_order_id:u64,
        fill_id: u64,
        is_taker_long: bool,
        price: u64,
        size: u64,
        taker_metadata: M,
        maker_metadata: M): SettleTradeResult {
        (self.settle_trade_f)(taker, maker, taker_order_id, maker_order_id, fill_id, is_taker_long, price, size, taker_metadata, maker_metadata)
    }

    public(friend) fun validate_order_placement<M: store + copy + drop>(
        self: &MarketClearinghouseCallbacks<M>,
        account: address,
        order_id: u64,
        is_taker: bool,
        is_bid: bool,
        price: Option<u64>,
        size: u64,
        order_metadata: M): bool {
        (self.validate_order_placement_f)(account, order_id, is_taker, is_bid, price, size, order_metadata)
    }

    public(friend) fun place_maker_order<M: store + copy + drop>(
        self: &MarketClearinghouseCallbacks<M>,
        account: address,
        order_id: u64,
        is_bid: bool,
        price: u64,
        size: u64,
        order_metadata: M) {
        (self.place_maker_order_f)(account, order_id, is_bid, price, size, order_metadata)
    }

    public(friend) fun cleanup_order<M: store + copy + drop>(
        self: &MarketClearinghouseCallbacks<M>,
        account: address,
        order_id: u64,
        is_bid: bool,
        remaining_size: u64) {
        (self.cleanup_order_f)(account, order_id, is_bid, remaining_size)
    }

    public(friend) fun decrease_order_size<M: store + copy + drop>(
        self: &MarketClearinghouseCallbacks<M>,
        account: address,
        order_id: u64,
        is_bid: bool,
        price: u64,
        size: u64,) {
        (self.decrease_order_size_f)(account, order_id, is_bid, price, size)
    }
}
