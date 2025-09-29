
module aptos_experimental::market_clearinghouse_order_info {
    use std::option::Option;
    use std::string::String;
    use aptos_experimental::order_book_types::{OrderIdType, TimeInForce};

    friend aptos_experimental::market_types;
    friend aptos_experimental::order_operations;
    friend aptos_experimental::order_placement;

    enum MarketClearinghouseOrderInfo<M: copy + drop> has copy, drop {
        V1 {
            account: address,
            order_id: OrderIdType,
            client_order_id: Option<String>,
            is_bid: bool,
            time_in_force: TimeInForce,
            metadata: M
        }
    }

    public(friend) fun new_clearinghouse_order_info<M: copy + drop>(
        account: address,
        order_id: OrderIdType,
        client_order_id: Option<String>,
        is_bid: bool,
        time_in_force: TimeInForce,
        metadata: M
    ): MarketClearinghouseOrderInfo<M> {
        MarketClearinghouseOrderInfo::V1 {
            account, order_id, client_order_id, is_bid, time_in_force, metadata,
        }
    }

    public fun get_account<M: copy + drop>(self: &MarketClearinghouseOrderInfo<M>): address {
        self.account
    }

    public fun get_order_id<M: copy + drop>(self: &MarketClearinghouseOrderInfo<M>): OrderIdType {
        self.order_id
    }

    public fun is_bid<M: copy + drop>(self: &MarketClearinghouseOrderInfo<M>): bool {
        self.is_bid
    }

    public fun get_client_order_id<M: copy + drop>(self: &MarketClearinghouseOrderInfo<M>): Option<String> {
        self.client_order_id
    }

    public fun get_metadata<M: copy + drop>(self: &MarketClearinghouseOrderInfo<M>): &M {
        &self.metadata
    }

    public fun into_inner<M: copy + drop>(self: MarketClearinghouseOrderInfo<M>): (address, OrderIdType, bool, Option<String>, M) {
        (self.account, self.order_id, self.is_bid, self.client_order_id, self.metadata)
    }

}
