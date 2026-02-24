module aptos_experimental::order_book {
    friend aptos_experimental::order_placement;
    friend aptos_experimental::order_operations;
    friend aptos_experimental::market_types;
    friend aptos_experimental::market_bulk_order;
    friend aptos_experimental::dead_mans_switch_operations;
    #[test_only]
    friend aptos_experimental::order_book_client_order_id;

    use std::option::Option;
    use std::string::String;
    use aptos_trading::order_book_types::{OrderId, TriggerCondition};
    use aptos_trading::bulk_order_types::{
        BulkOrder,
        BulkOrderRequest,
        BulkOrderPlaceResponse
    };
    use aptos_trading::order_match_types::{OrderMatch, OrderMatchDetails};
    use aptos_trading::single_order_types::{SingleOrder, SingleOrderRequest};
    use aptos_experimental::bulk_order_book::{BulkOrderBook, new_bulk_order_book};
    use aptos_experimental::single_order_book::{SingleOrderBook, new_single_order_book};
    use aptos_experimental::price_time_index::{PriceTimeIndex, new_price_time_idx};

    const E_REINSERT_ORDER_MISMATCH: u64 = 8;

    enum OrderBook<M: store + copy + drop> has store {
        UnifiedV1 {
            single_order_book: SingleOrderBook<M>,
            bulk_order_book: BulkOrderBook<M>,
            price_time_idx: PriceTimeIndex
        }
    }

    public fun new_order_book<M: store + copy + drop>(): OrderBook<M> {
        OrderBook::UnifiedV1 {
            single_order_book: new_single_order_book(),
            bulk_order_book: new_bulk_order_book(),
            price_time_idx: new_price_time_idx()
        }
    }

    // ============================= APIs relevant to single order only ====================================

    public(friend) fun client_order_id_exists<M: store + copy + drop>(
        self: &OrderBook<M>, order_creator: address, client_order_id: String
    ): bool {
        self.single_order_book.client_order_id_exists(order_creator, client_order_id)
    }

    public(friend) fun get_single_order_metadata<M: store + copy + drop>(
        self: &OrderBook<M>, order_id: OrderId
    ): Option<M> {
        self.single_order_book.get_order_metadata(order_id)
    }

    public(friend) fun get_order_id_by_client_id<M: store + copy + drop>(
        self: &OrderBook<M>, order_creator: address, client_order_id: String
    ): Option<OrderId> {
        self.single_order_book.get_order_id_by_client_id(order_creator, client_order_id)
    }

    public(friend) fun get_single_order<M: store + copy + drop>(
        self: &OrderBook<M>, order_id: OrderId
    ): Option<aptos_trading::single_order_types::OrderWithState<M>> {
        self.single_order_book.get_order(order_id)
    }

    public fun get_single_remaining_size<M: store + copy + drop>(
        self: &OrderBook<M>, order_id: OrderId
    ): u64 {
        self.single_order_book.get_remaining_size(order_id)
    }

    public fun cancel_single_order<M: store + copy + drop>(
        self: &mut OrderBook<M>, order_creator: address, order_id: OrderId
    ): SingleOrder<M> {
        self.single_order_book.cancel_order(
            &mut self.price_time_idx, order_creator, order_id
        )
    }

    public(friend) fun try_cancel_single_order<M: store + copy + drop>(
        self: &mut OrderBook<M>, order_creator: address, order_id: OrderId
    ): Option<SingleOrder<M>> {
        self.single_order_book.try_cancel_order(
            &mut self.price_time_idx, order_creator, order_id
        )
    }

    public(friend) fun try_cancel_single_order_with_client_order_id<M: store + copy + drop>(
        self: &mut OrderBook<M>, order_creator: address, client_order_id: String
    ): Option<SingleOrder<M>> {
        self.single_order_book.try_cancel_order_with_client_order_id(
            &mut self.price_time_idx, order_creator, client_order_id
        )
    }

    public fun place_maker_order<M: store + copy + drop>(
        self: &mut OrderBook<M>, order_req: SingleOrderRequest<M>
    ) {
        self.single_order_book.place_maker_or_pending_order(
            &mut self.price_time_idx, order_req
        );
    }

    public(friend) fun decrease_single_order_size<M: store + copy + drop>(
        self: &mut OrderBook<M>,
        order_creator: address,
        order_id: OrderId,
        size_delta: u64
    ) {
        self.single_order_book.decrease_order_size(
            &mut self.price_time_idx,
            order_creator,
            order_id,
            size_delta
        )
    }

    public(friend) fun set_single_order_metadata<M: store + copy + drop>(
        self: &mut OrderBook<M>, order_id: OrderId, metadata: M
    ) {
        self.single_order_book.set_order_metadata(order_id, metadata)
    }

    public(friend) fun take_ready_price_based_orders<M: store + copy + drop>(
        self: &mut OrderBook<M>, oracle_price: u64, order_limit: u64
    ): vector<SingleOrder<M>> {
        self.single_order_book.take_ready_price_based_orders(oracle_price, order_limit)
    }

    public(friend) fun take_ready_time_based_orders<M: store + copy + drop>(
        self: &mut OrderBook<M>, order_limit: u64
    ): vector<SingleOrder<M>> {
        self.single_order_book.take_ready_time_based_orders(order_limit)
    }

    // ============================= APIs relevant to both single and bulk order ====================================

    public(friend) fun best_bid_price<M: store + copy + drop>(
        self: &OrderBook<M>
    ): Option<u64> {
        self.price_time_idx.best_bid_price()
    }

    public(friend) fun best_ask_price<M: store + copy + drop>(
        self: &OrderBook<M>
    ): Option<u64> {
        self.price_time_idx.best_ask_price()
    }

    public fun get_slippage_price<M: store + copy + drop>(
        self: &OrderBook<M>, is_bid: bool, slippage_bps: u64
    ): Option<u64> {
        self.price_time_idx.get_slippage_price(is_bid, slippage_bps)
    }

    /// Checks if the order is a taker order i.e., matched immediately with the active order book.
    public fun is_taker_order<M: store + copy + drop>(
        self: &OrderBook<M>,
        price: u64,
        is_bid: bool,
        trigger_condition: Option<TriggerCondition>
    ): bool {
        if (trigger_condition.is_some()) {
            return false;
        };
        self.price_time_idx.is_taker_order(price, is_bid)
    }

    public fun get_single_match_for_taker<M: store + copy + drop>(
        self: &mut OrderBook<M>,
        price: u64,
        size: u64,
        is_bid: bool
    ): OrderMatch<M> {
        let result = self.price_time_idx.get_single_match_result(price, size, is_bid);
        if (result.is_active_matched_book_type_single_order()) {
            self.single_order_book.get_single_match_for_taker(result)
        } else {
            self.bulk_order_book.get_single_match_for_taker(
                &mut self.price_time_idx, result, is_bid
            )
        }
    }

    public(friend) fun reinsert_order<M: store + copy + drop>(
        self: &mut OrderBook<M>,
        reinsert_order: OrderMatchDetails<M>,
        original_order: &OrderMatchDetails<M>
    ) {
        if (reinsert_order.is_single_order_from_match_details()) {
            self.single_order_book.reinsert_order(
                &mut self.price_time_idx, reinsert_order, original_order
            )
        } else {
            self.bulk_order_book.reinsert_order(
                &mut self.price_time_idx, reinsert_order, original_order
            );
        }
    }

    // ============================= APIs relevant to bulk order only ====================================

    public(friend) fun get_bulk_order_remaining_size<M: store + copy + drop>(
        self: &OrderBook<M>, order_creator: address, is_bid: bool
    ): u64 {
        self.bulk_order_book.get_remaining_size(order_creator, is_bid)
    }

    public(friend) fun place_bulk_order<M: store + copy + drop>(
        self: &mut OrderBook<M>, order_req: BulkOrderRequest<M>
    ): BulkOrderPlaceResponse<M> {
        self.bulk_order_book.place_bulk_order(&mut self.price_time_idx, order_req)
    }

    public(friend) fun get_bulk_order<M: store + copy + drop>(
        self: &OrderBook<M>, order_creator: address
    ): BulkOrder<M> {
        self.bulk_order_book.get_bulk_order(order_creator)
    }

    public(friend) fun cancel_bulk_order<M: store + copy + drop>(
        self: &mut OrderBook<M>, order_creator: address
    ): BulkOrder<M> {
        self.bulk_order_book.cancel_bulk_order(&mut self.price_time_idx, order_creator)
    }

    public(friend) fun cancel_bulk_order_at_price<M: store + copy + drop>(
        self: &mut OrderBook<M>,
        order_creator: address,
        price: u64,
        is_bid: bool
    ): (u64, BulkOrder<M>) {
        self.bulk_order_book.cancel_bulk_order_at_price(
            &mut self.price_time_idx,
            order_creator,
            price,
            is_bid
        )
    }

    // ============================= test_only APIs ====================================
    #[test_only]
    public fun destroy_order_book<M: store + copy + drop>(self: OrderBook<M>) {
        let OrderBook::UnifiedV1 {
            single_order_book: retail_order_book,
            bulk_order_book,
            price_time_idx
        } = self;
        bulk_order_book.destroy_bulk_order_book();
        retail_order_book.destroy_single_order_book();
        price_time_idx.destroy_price_time_idx();
    }

    #[test_only]
    public fun set_up_test_with_id(): OrderBook<u64> {
        aptos_framework::timestamp::set_time_has_started_for_testing(
            &aptos_framework::account::create_signer_for_test(@0x1)
        );
        new_order_book()
    }
}
