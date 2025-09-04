module velor_experimental::order_book {

    use std::option::Option;
    use velor_experimental::bulk_order_book::{BulkOrderBook, new_bulk_order_book};
    use velor_experimental::single_order_book::{SingleOrderBook, new_single_order_book, SingleOrderRequest};
    use velor_experimental::order_book_types::{AscendingIdGenerator, OrderIdType, new_ascending_id_generator,
        OrderMatch, OrderMatchDetails, single_order_book_type
    };
    use velor_experimental::single_order_types::{SingleOrder};
    use velor_experimental::order_book_types::TriggerCondition;
    use velor_experimental::order_book_types::TimeInForce;
    use velor_experimental::price_time_index::{PriceTimeIndex, new_price_time_idx};

    const E_REINSERT_ORDER_MISMATCH: u64 = 8;

    enum OrderBook<M: store + copy + drop> has store {
        UnifiedV1 {
            single_order_book: SingleOrderBook<M>,
            bulk_order_book: BulkOrderBook,
            price_time_idx: PriceTimeIndex,
            ascending_id_generator: AscendingIdGenerator
        }
    }

    public fun new_order_book<M: store + copy + drop>(): OrderBook<M> {
        OrderBook::UnifiedV1 {
            single_order_book: new_single_order_book(),
            bulk_order_book: new_bulk_order_book(),
            price_time_idx: new_price_time_idx(),
            ascending_id_generator: new_ascending_id_generator(),
        }
    }

    public fun new_single_order_request<M: store + copy + drop>(
        account: address,
        order_id: OrderIdType,
        client_order_id: Option<u64>,
        price: u64,
        orig_size: u64,
        remaining_size: u64,
        is_bid: bool,
        trigger_condition: Option<TriggerCondition>,
        time_in_force: TimeInForce,
        metadata: M
    ): SingleOrderRequest<M> {
        velor_experimental::single_order_book::new_single_order_request(
            account,
            order_id,
            client_order_id,
            price,
            orig_size,
            remaining_size,
            is_bid,
            trigger_condition,
            time_in_force,
            metadata
        )
    }

    // ============================= APIs relevant to single order only ====================================

    public fun cancel_order<M: store + copy + drop>(
        self: &mut OrderBook<M>, order_creator: address, order_id: OrderIdType
    ): SingleOrder<M> {
        self.single_order_book.cancel_order(&mut self.price_time_idx, order_creator, order_id)
    }

    public fun try_cancel_order_with_client_order_id<M: store + copy + drop>(
        self: &mut OrderBook<M>, order_creator: address, client_order_id: u64
    ): Option<SingleOrder<M>> {
        self.single_order_book.try_cancel_order_with_client_order_id(&mut self.price_time_idx, order_creator, client_order_id)
    }

    public fun client_order_id_exists<M: store + copy + drop>(
        self: &OrderBook<M>, order_creator: address, client_order_id: u64
    ): bool {
        self.single_order_book.client_order_id_exists(order_creator, client_order_id)
    }

    public fun place_maker_order<M: store + copy + drop>(
        self: &mut OrderBook<M>, order_req: SingleOrderRequest<M>
    ) {
        self.single_order_book.place_maker_order(
            &mut self.price_time_idx,
            &mut self.ascending_id_generator,
            order_req
        );
    }

    public fun decrease_order_size<M: store + copy + drop>(
        self: &mut OrderBook<M>, order_creator: address, order_id: OrderIdType, size_delta: u64
    ) {
        self.single_order_book.decrease_order_size(&mut self.price_time_idx, order_creator, order_id, size_delta)
    }

    public fun get_order_id_by_client_id<M: store + copy + drop>(
        self: &OrderBook<M>, order_creator: address, client_order_id: u64
    ): Option<OrderIdType> {
        self.single_order_book.get_order_id_by_client_id(order_creator, client_order_id)
    }

    public fun get_order_metadata<M: store + copy + drop>(
        self: &OrderBook<M>, order_id: OrderIdType
    ): Option<M> {
        self.single_order_book.get_order_metadata(order_id)
    }

    public fun set_order_metadata<M: store + copy + drop>(
        self: &mut OrderBook<M>, order_id: OrderIdType, metadata: M
    ) {
        self.single_order_book.set_order_metadata(order_id, metadata)
    }

    public fun is_active_order<M: store + copy + drop>(
        self: &OrderBook<M>, order_id: OrderIdType
    ): bool {
        self.single_order_book.is_active_order(order_id)
    }

    public fun get_order<M: store + copy + drop>(
        self: &OrderBook<M>, order_id: OrderIdType
    ): Option<velor_experimental::single_order_types::OrderWithState<M>> {
        self.single_order_book.get_order(order_id)
    }

    public fun get_remaining_size<M: store + copy + drop>(
        self: &OrderBook<M>, order_id: OrderIdType
    ): u64 {
        self.single_order_book.get_remaining_size(order_id)
    }

    public fun take_ready_price_based_orders<M: store + copy + drop>(
        self: &mut OrderBook<M>, oracle_price: u64, order_limit: u64
    ): vector<SingleOrder<M>> {
        self.single_order_book.take_ready_price_based_orders(oracle_price, order_limit)
    }

    public fun take_ready_time_based_orders<M: store + copy + drop>(
        self: &mut OrderBook<M>, order_limit: u64
    ): vector<SingleOrder<M>> {
        self.single_order_book.take_ready_time_based_orders(order_limit)
    }

    // ============================= APIs relevant to both single and bulk order ====================================

    /// Checks if the order is a taker order i.e., matched immediatedly with the active order book.
    public fun is_taker_order<M: store + copy + drop>(
        self: &OrderBook<M>,
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
        self: &mut OrderBook<M>,
        price: u64,
        size: u64,
        is_bid: bool
    ): OrderMatch<M> {
        let result = self.price_time_idx.get_single_match_result(price, size, is_bid);
        let book_type = result.get_active_matched_book_type();
        if (book_type == single_order_book_type()) {
            self.single_order_book.get_single_match_for_taker(result)
        } else {
            self.bulk_order_book.get_single_match_for_taker(&mut self.price_time_idx, result, is_bid)
        }
    }

    public fun reinsert_order<M: store + copy + drop>(
        self: &mut OrderBook<M>,
        reinsert_order: OrderMatchDetails<M>,
        original_order: &OrderMatchDetails<M>,
    ) {
        assert!(reinsert_order.get_book_type_from_match_details()
            == original_order.get_book_type_from_match_details(), E_REINSERT_ORDER_MISMATCH);
        if (reinsert_order.get_book_type_from_match_details() == single_order_book_type()) {
            self.single_order_book.reinsert_order(
                &mut self.price_time_idx, reinsert_order, original_order
            )
        } else {
            self.bulk_order_book.reinsert_order(
                &mut self.price_time_idx, reinsert_order, original_order
            );
        }
    }

    public fun best_bid_price<M: store + copy + drop>(self: &OrderBook<M>): Option<u64> {
        self.price_time_idx.best_bid_price()
    }

    public fun best_ask_price<M: store + copy + drop>(self: &OrderBook<M>): Option<u64> {
        self.price_time_idx.best_ask_price()
    }

    public fun get_slippage_price<M: store + copy + drop>(
        self: &OrderBook<M>, is_bid: bool, slippage_pct: u64
    ): Option<u64> {
        self.price_time_idx.get_slippage_price(is_bid, slippage_pct)
    }


    // ============================= APIs relevant to bulk order only ====================================
    public fun place_bulk_order<M: store + copy + drop>(
        self: &mut OrderBook<M>, order_req: velor_experimental::bulk_order_book_types::BulkOrderRequest
    ) : OrderIdType {
        self.bulk_order_book.place_bulk_order(
            &mut self.price_time_idx,
            &mut self.ascending_id_generator,
            order_req
        )
    }

    public fun cancel_bulk_order<M: store + copy + drop>(
        self: &mut OrderBook<M>, order_creator: address
    ): (OrderIdType, u64, u64) {
        self.bulk_order_book.cancel_bulk_order(&mut self.price_time_idx, order_creator)
    }

    public fun get_bulk_order_remaining_size<M: store + copy + drop>(
        self: &OrderBook<M>,
        order_creator: address,
        is_bid: bool
    ): u64 {
        self.bulk_order_book.get_remaining_size(order_creator, is_bid)
    }

    // ============================= test_only APIs ====================================
    #[test_only]
    public fun destroy_order_book<M: store + copy + drop>(
        self: OrderBook<M>
    ) {
        let OrderBook::UnifiedV1 {
            single_order_book: retail_order_book,
            bulk_order_book,
            price_time_idx,
            ascending_id_generator: _
        } = self;
        bulk_order_book.destroy_bulk_order_book();
        retail_order_book.destroy_single_order_book();
        price_time_idx.destroy_price_time_idx();
    }

    #[test_only]
    public fun set_up_test_with_id(): OrderBook<u64> {
        new_order_book()
    }
}
