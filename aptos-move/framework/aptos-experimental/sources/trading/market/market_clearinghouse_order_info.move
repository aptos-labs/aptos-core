module aptos_experimental::market_clearinghouse_order_info {
    friend aptos_experimental::market_types;
    friend aptos_experimental::order_operations;
    friend aptos_experimental::order_placement;

    use std::option::Option;
    use std::string::String;
    use aptos_trading::order_book_types::{
        OrderId,
        TimeInForce,
        OrderType,
        TriggerCondition
    };

    enum MarketClearinghouseOrderInfo<M: copy + drop> has copy, drop {
        V1 {
            account: address,
            order_id: OrderId,
            client_order_id: Option<String>,
            is_bid: bool,
            limit_price: u64,
            time_in_force: TimeInForce,
            order_type: OrderType,
            trigger_condition: Option<TriggerCondition>,
            metadata: M
        }
    }

    public fun new_clearinghouse_order_info<M: copy + drop>(
        account: address,
        order_id: OrderId,
        client_order_id: Option<String>,
        is_bid: bool,
        limit_price: u64,
        time_in_force: TimeInForce,
        order_type: OrderType,
        trigger_condition: Option<TriggerCondition>,
        metadata: M
    ): MarketClearinghouseOrderInfo<M> {
        MarketClearinghouseOrderInfo::V1 {
            account,
            order_id,
            client_order_id,
            is_bid,
            limit_price,
            time_in_force,
            order_type,
            trigger_condition,
            metadata
        }
    }

    public fun get_account<M: copy + drop>(
        self: &MarketClearinghouseOrderInfo<M>
    ): address {
        self.account
    }

    public fun get_order_id<M: copy + drop>(
        self: &MarketClearinghouseOrderInfo<M>
    ): OrderId {
        self.order_id
    }

    public fun is_bid<M: copy + drop>(
        self: &MarketClearinghouseOrderInfo<M>
    ): bool {
        self.is_bid
    }

    public fun get_client_order_id<M: copy + drop>(
        self: &MarketClearinghouseOrderInfo<M>
    ): Option<String> {
        self.client_order_id
    }

    public fun get_metadata<M: copy + drop>(
        self: &MarketClearinghouseOrderInfo<M>
    ): &M {
        &self.metadata
    }

    public fun into_inner<M: copy + drop>(
        self: MarketClearinghouseOrderInfo<M>
    ): (
        address,
        OrderId,
        Option<String>,
        bool,
        u64,
        TimeInForce,
        OrderType,
        Option<TriggerCondition>,
        M
    ) {
        let MarketClearinghouseOrderInfo::V1 {
            account,
            order_id,
            client_order_id,
            is_bid,
            limit_price,
            time_in_force,
            order_type,
            trigger_condition,
            metadata
        } = self;
        (
            account,
            order_id,
            client_order_id,
            is_bid,
            limit_price,
            time_in_force,
            order_type,
            trigger_condition,
            metadata
        )
    }
}
