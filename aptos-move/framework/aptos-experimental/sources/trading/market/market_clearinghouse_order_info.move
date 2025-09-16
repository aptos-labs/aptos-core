module aptos_experimental::market_clearinghouse_order_info {
    use aptos_experimental::order_book_types::OrderIdType;

    friend aptos_experimental::market_types;
    friend aptos_experimental::order_operations;
    friend aptos_experimental::order_placement;

    enum MarketClearinghouseOrderInfo<M: copy + drop> has copy, drop {
        V1 {
            account: address,
            order_id: OrderIdType,
            is_bid: bool,
            price: u64,
            size: u64,
            metadata: M
        }
    }

    public(friend) fun new_clearinghouse_order_info<M: copy + drop>(
            account: address,
            order_id: OrderIdType,
            is_bid: bool,
            price: u64,
            size: u64,
            metadata: M
    ): MarketClearinghouseOrderInfo<M> {
        MarketClearinghouseOrderInfo::V1 {
            account, order_id, is_bid, price, size, metadata,
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

    public fun get_price<M: copy + drop>(self: &MarketClearinghouseOrderInfo<M>): u64 {
        self.price
    }

    public fun get_size<M: copy + drop>(self: &MarketClearinghouseOrderInfo<M>): u64 {
        self.size
    }

    public fun get_metadata<M: copy + drop>(self: &MarketClearinghouseOrderInfo<M>): &M {
        &self.metadata
    }

    public fun into_inner<M: copy + drop>(self: MarketClearinghouseOrderInfo<M>): (address, OrderIdType, bool, u64, u64, M) {
        (self.account, self.order_id, self.is_bid, self.price, self.size, self.metadata)
    }

}
