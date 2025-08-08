module aptos_experimental::unified_order_book {

    use std::option::Option;
    use aptos_experimental::retail_order_book::{RetailOrderBook, new_retail_order_book, OrderRequest};
    use aptos_experimental::retail_order_types::{AscendingIdGenerator, OrderIdType, Order, TriggerCondition,
        new_ascending_id_generator, SingleOrderMatch
    };
    use aptos_experimental::price_time_index::{PriceTimeIndex, new_price_time_idx};
    enum UnifiedOrderBook<M: store + copy + drop> has store {
        V1 {
            retail_order_book: RetailOrderBook<M>,
            price_time_idx: PriceTimeIndex,
            ascending_id_generator: AscendingIdGenerator
        }
    }

    public fun new_unified_order_book<M: store + copy + drop>(): UnifiedOrderBook<M> {
        UnifiedOrderBook::V1 {
            retail_order_book: new_retail_order_book(),
            price_time_idx: new_price_time_idx(),
            ascending_id_generator: new_ascending_id_generator(),
        }
    }

    public fun cancel_order<M: store + copy + drop>(
        self: &mut UnifiedOrderBook<M>, order_creator: address, order_id: OrderIdType
    ): Order<M> {
        self.retail_order_book.cancel_order(&mut self.price_time_idx, order_creator, order_id)
    }

    public fun try_cancel_order_with_client_order_id<M: store + copy + drop>(
        self: &mut UnifiedOrderBook<M>, order_creator: address, client_order_id: u64
    ): Option<Order<M>> {
        self.retail_order_book.try_cancel_order_with_client_order_id(&mut self.price_time_idx, order_creator, client_order_id)
    }

    public fun client_order_id_exists<M: store + copy + drop>(
        self: &UnifiedOrderBook<M>, order_creator: address, client_order_id: u64
    ): bool {
        self.retail_order_book.client_order_id_exists(order_creator, client_order_id)
    }

    /// Checks if the order is a taker order i.e., matched immediatedly with the active order book.
    public fun is_taker_order<M: store + copy + drop>(
        self: &UnifiedOrderBook<M>,
        price: u64,
        is_bid: bool,
        trigger_condition: Option<TriggerCondition>
    ): bool {
        if (trigger_condition.is_some()) {
            return false;
        };
        return self.price_time_idx.is_taker_order(price, is_bid)
    }

    public fun get_single_match_for_taker<M: store + copy + drop>(
        self: &mut UnifiedOrderBook<M>,
        price: u64,
        size: u64,
        is_bid: bool
    ): SingleOrderMatch<M> {
        self.retail_order_book.get_single_match_for_taker(
            &mut self.price_time_idx, price, size, is_bid
        )
    }

    public fun reinsert_maker_order<M: store + copy + drop>(
        self: &mut UnifiedOrderBook<M>,  order_req: OrderRequest<M>, original_order: Order<M>
    ) {
        self.retail_order_book.reinsert_maker_order(
            &mut self.price_time_idx, &mut self.ascending_id_generator, order_req, original_order
        );
    }

    public fun place_maker_order<M: store + copy + drop>(
        self: &mut UnifiedOrderBook<M>,  order_req: OrderRequest<M>
    ) {
        self.retail_order_book.place_maker_order(
            &mut self.price_time_idx,
            &mut self.ascending_id_generator,
            order_req
        );
    }

    public fun decrease_order_size<M: store + copy + drop>(
        self: &mut UnifiedOrderBook<M>, order_creator: address, order_id: OrderIdType, size_delta: u64
    ) {
        self.retail_order_book.decrease_order_size(&mut self.price_time_idx, order_creator, order_id, size_delta)
    }

    public fun get_order_id_by_client_id<M: store + copy + drop>(
        self: &UnifiedOrderBook<M>, order_creator: address, client_order_id: u64
    ): Option<OrderIdType> {
        self.retail_order_book.get_order_id_by_client_id(order_creator, client_order_id)
    }

    public fun get_order_metadata<M: store + copy + drop>(
        self: &UnifiedOrderBook<M>, order_id: OrderIdType
    ): Option<M> {
        self.retail_order_book.get_order_metadata(order_id)
    }

    public fun set_order_metadata<M: store + copy + drop>(
        self: &mut UnifiedOrderBook<M>, order_id: OrderIdType, metadata: M
    ) {
        self.retail_order_book.set_order_metadata(order_id, metadata)
    }

    public fun is_active_order<M: store + copy + drop>(
        self: &UnifiedOrderBook<M>, order_id: OrderIdType
    ): bool {
        self.retail_order_book.is_active_order(order_id)
    }

    public fun get_order<M: store + copy + drop>(
        self: &UnifiedOrderBook<M>, order_id: OrderIdType
    ): Option<aptos_experimental::retail_order_types::OrderWithState<M>> {
        self.retail_order_book.get_order(order_id)
    }

    public fun get_remaining_size<M: store + copy + drop>(
        self: &UnifiedOrderBook<M>, order_id: OrderIdType
    ): u64 {
        self.retail_order_book.get_remaining_size(order_id)
    }

    public fun take_ready_price_based_orders<M: store + copy + drop>(
        self: &mut UnifiedOrderBook<M>, oracle_price: u64, order_limit: u64
    ): vector<Order<M>> {
        self.retail_order_book.take_ready_price_based_orders(oracle_price, order_limit)
    }

    public fun take_ready_time_based_orders<M: store + copy + drop>(
        self: &mut UnifiedOrderBook<M>, order_limit: u64
    ): vector<Order<M>> {
        self.retail_order_book.take_ready_time_based_orders(order_limit)
    }

    public fun best_bid_price<M: store + copy + drop>(self: &UnifiedOrderBook<M>): Option<u64> {
        self.price_time_idx.best_bid_price()
    }

    public fun best_ask_price<M: store + copy + drop>(self: &UnifiedOrderBook<M>): Option<u64> {
        self.price_time_idx.best_ask_price()
    }

    public fun get_slippage_price<M: store + copy + drop>(
        self: &UnifiedOrderBook<M>, is_bid: bool, slippage_pct: u64
    ): Option<u64> {
        self.price_time_idx.get_slippage_price(is_bid, slippage_pct)
    }

    #[test_only]
    public fun destroy_unified_order_book<M: store + copy + drop>(
        self: UnifiedOrderBook<M>
    ) {
        let UnifiedOrderBook::V1 {
            retail_order_book,
            price_time_idx,
            ascending_id_generator: _
        } = self;
        retail_order_book.destroy_order_book();
        price_time_idx.destroy_price_time_idx();
    }

}
