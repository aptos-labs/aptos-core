module aptos_experimental::order_book {

    use std::option::Option;
    use aptos_std::table;
    use aptos_std::table::Table;
    use aptos_experimental::bulk_order_book::{BulkOrderBook, new_bulk_order_book};
    use aptos_experimental::single_order_book::{SingleOrderBook, new_single_order_book, SingleOrderRequest};
    use aptos_experimental::order_book_types::{AscendingIdGenerator, OrderIdType, new_ascending_id_generator,
        OrderMatch, OrderMatchDetails, single_order_book_type
    };
    use aptos_experimental::single_order_types::{SingleOrder};
    use aptos_experimental::order_book_types::TriggerCondition;
    use aptos_experimental::order_book_types::TimeInForce;
    use aptos_experimental::price_time_index::{PriceTimeIndex, new_price_time_idx};

    const E_REINSERT_ORDER_MISMATCH: u64 = 8;

    const SINGLE_ORDER_BOOK_KEY: u8 = 1;
    const BULK_ORDER_BOOK_KEY: u8 = 2;
    const PRICE_TIME_INDEX_KEY: u8 = 3;
    const ASCENDING_ID_GENERATOR_KEY: u8 = 4;

    enum OrderBook<M: store + copy + drop> has store {
        UnifiedV1 {
            single_order_book: Table<u8, SingleOrderBook<M>>,
            bulk_order_book: Table<u8, BulkOrderBook>,
            price_time_idx: Table<u8, PriceTimeIndex>,
            ascending_id_generator: Table<u8, AscendingIdGenerator>
        }
    }

    public fun new_order_book<M: store + copy + drop>(): OrderBook<M> {
        let single_order_book = table::new<u8, SingleOrderBook<M>>();
        single_order_book.add(SINGLE_ORDER_BOOK_KEY, new_single_order_book());
        let bulk_order_book = table::new<u8, BulkOrderBook>();
        bulk_order_book.add(BULK_ORDER_BOOK_KEY, new_bulk_order_book());
        let price_time_idx = table::new<u8, PriceTimeIndex>();
        price_time_idx.add(PRICE_TIME_INDEX_KEY, new_price_time_idx());
        let ascending_id_generator = table::new<u8, AscendingIdGenerator>();
        ascending_id_generator.add(ASCENDING_ID_GENERATOR_KEY, new_ascending_id_generator());
        OrderBook::UnifiedV1 {
            single_order_book,
            bulk_order_book,
            price_time_idx,
            ascending_id_generator,
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
        aptos_experimental::single_order_book::new_single_order_request(
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
        self.single_order_book.borrow_mut(SINGLE_ORDER_BOOK_KEY).cancel_order(
            self.price_time_idx.borrow_mut(PRICE_TIME_INDEX_KEY),
            order_creator,
            order_id
        )
    }

    public fun try_cancel_order_with_client_order_id<M: store + copy + drop>(
        self: &mut OrderBook<M>, order_creator: address, client_order_id: u64
    ): Option<SingleOrder<M>> {
        self.single_order_book.borrow_mut(SINGLE_ORDER_BOOK_KEY).try_cancel_order_with_client_order_id(
            self.price_time_idx.borrow_mut(PRICE_TIME_INDEX_KEY),
            order_creator,
            client_order_id
        )
    }

    public fun client_order_id_exists<M: store + copy + drop>(
        self: &OrderBook<M>, order_creator: address, client_order_id: u64
    ): bool {
        let OrderBook::UnifiedV1 { single_order_book, .. } = self;
        single_order_book.borrow(SINGLE_ORDER_BOOK_KEY).client_order_id_exists(order_creator, client_order_id)
    }

    public fun place_maker_order<M: store + copy + drop>(
        self: &mut OrderBook<M>, order_req: SingleOrderRequest<M>
    ) {
        self.single_order_book.borrow_mut(SINGLE_ORDER_BOOK_KEY).place_maker_order(
            self.price_time_idx.borrow_mut(PRICE_TIME_INDEX_KEY),
            self.ascending_id_generator.borrow_mut(ASCENDING_ID_GENERATOR_KEY),
            order_req
        );
    }

    public fun decrease_order_size<M: store + copy + drop>(
        self: &mut OrderBook<M>, order_creator: address, order_id: OrderIdType, size_delta: u64
    ) {
        self.single_order_book.borrow_mut(SINGLE_ORDER_BOOK_KEY).decrease_order_size(
            self.price_time_idx.borrow_mut(PRICE_TIME_INDEX_KEY),
            order_creator,
            order_id,
            size_delta
        )
    }

    public fun get_order_id_by_client_id<M: store + copy + drop>(
        self: &OrderBook<M>, order_creator: address, client_order_id: u64
    ): Option<OrderIdType> {
        self.single_order_book.borrow(SINGLE_ORDER_BOOK_KEY).get_order_id_by_client_id(order_creator, client_order_id)
    }

    public fun get_order_metadata<M: store + copy + drop>(
        self: &OrderBook<M>, order_id: OrderIdType
    ): Option<M> {
        self.single_order_book.borrow(SINGLE_ORDER_BOOK_KEY).get_order_metadata(order_id)
    }

    public fun set_order_metadata<M: store + copy + drop>(
        self: &mut OrderBook<M>, order_id: OrderIdType, metadata: M
    ) {
        self.single_order_book.borrow_mut(SINGLE_ORDER_BOOK_KEY).set_order_metadata(order_id, metadata)
    }

    public fun is_active_order<M: store + copy + drop>(
        self: &OrderBook<M>, order_id: OrderIdType
    ): bool {
        self.single_order_book.borrow(SINGLE_ORDER_BOOK_KEY).is_active_order(order_id)
    }

    public fun get_order<M: store + copy + drop>(
        self: &OrderBook<M>, order_id: OrderIdType
    ): Option<aptos_experimental::single_order_types::OrderWithState<M>> {
        self.single_order_book.borrow(SINGLE_ORDER_BOOK_KEY).get_order(order_id)
    }

    public fun get_remaining_size<M: store + copy + drop>(
        self: &OrderBook<M>, order_id: OrderIdType
    ): u64 {
        self.single_order_book.borrow(SINGLE_ORDER_BOOK_KEY).get_remaining_size(order_id)
    }

    public fun take_ready_price_based_orders<M: store + copy + drop>(
        self: &mut OrderBook<M>, oracle_price: u64, order_limit: u64
    ): vector<SingleOrder<M>> {
        self.single_order_book.borrow_mut(SINGLE_ORDER_BOOK_KEY).take_ready_price_based_orders(oracle_price, order_limit)
    }

    public fun take_ready_time_based_orders<M: store + copy + drop>(
        self: &mut OrderBook<M>, order_limit: u64
    ): vector<SingleOrder<M>> {
        self.single_order_book.borrow_mut(SINGLE_ORDER_BOOK_KEY).take_ready_time_based_orders(order_limit)
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
        let OrderBook::UnifiedV1 { price_time_idx, .. } = self;
        return price_time_idx.borrow(PRICE_TIME_INDEX_KEY).is_taker_order(price, is_bid)
    }

    public fun get_single_match_for_taker<M: store + copy + drop>(
        self: &mut OrderBook<M>,
        price: u64,
        size: u64,
        is_bid: bool
    ): OrderMatch<M> {
        let OrderBook::UnifiedV1 { single_order_book, bulk_order_book, price_time_idx, .. } = self;
        let result = price_time_idx.borrow_mut(PRICE_TIME_INDEX_KEY).get_single_match_result(price, size, is_bid);
        let book_type = result.get_active_matched_book_type();
        if (book_type == single_order_book_type()) {
            single_order_book.borrow_mut(SINGLE_ORDER_BOOK_KEY).get_single_match_for_taker(result)
        } else {
            bulk_order_book.borrow_mut(BULK_ORDER_BOOK_KEY).get_single_match_for_taker(
                price_time_idx.borrow_mut(PRICE_TIME_INDEX_KEY),
                result,
                is_bid
            )
        }
    }

    public fun reinsert_order<M: store + copy + drop>(
        self: &mut OrderBook<M>,
        reinsert_order: OrderMatchDetails<M>,
        original_order: &OrderMatchDetails<M>,
    ) {
        assert!(reinsert_order.get_book_type_from_match_details()
            == original_order.get_book_type_from_match_details(), E_REINSERT_ORDER_MISMATCH);
        let OrderBook::UnifiedV1 { single_order_book, bulk_order_book, price_time_idx, .. } = self;
        if (reinsert_order.get_book_type_from_match_details() == single_order_book_type()) {
            single_order_book.borrow_mut(SINGLE_ORDER_BOOK_KEY).reinsert_order(
                price_time_idx.borrow_mut(PRICE_TIME_INDEX_KEY), reinsert_order, original_order
            )
        } else {
            bulk_order_book.borrow_mut(BULK_ORDER_BOOK_KEY).reinsert_order(
                price_time_idx.borrow_mut(PRICE_TIME_INDEX_KEY), reinsert_order, original_order
            );
        }
    }

    public fun best_bid_price<M: store + copy + drop>(self: &OrderBook<M>): Option<u64> {
        let OrderBook::UnifiedV1 { price_time_idx, .. } = self;
        price_time_idx.borrow(PRICE_TIME_INDEX_KEY).best_bid_price()
    }

    public fun best_ask_price<M: store + copy + drop>(self: &OrderBook<M>): Option<u64> {
        let OrderBook::UnifiedV1 { price_time_idx, .. } = self;
        price_time_idx.borrow(PRICE_TIME_INDEX_KEY).best_ask_price()
    }

    public fun get_slippage_price<M: store + copy + drop>(
        self: &OrderBook<M>, is_bid: bool, slippage_pct: u64
    ): Option<u64> {
        let OrderBook::UnifiedV1 { price_time_idx, .. } = self;
        price_time_idx.borrow(PRICE_TIME_INDEX_KEY).get_slippage_price(is_bid, slippage_pct)
    }


    // ============================= APIs relevant to bulk order only ====================================
    public fun place_bulk_order<M: store + copy + drop>(
        self: &mut OrderBook<M>, order_req: aptos_experimental::bulk_order_book_types::BulkOrderRequest
    ) : OrderIdType {
        self.bulk_order_book.borrow_mut(BULK_ORDER_BOOK_KEY).place_bulk_order(
            self.price_time_idx.borrow_mut(PRICE_TIME_INDEX_KEY),
            self.ascending_id_generator.borrow_mut(ASCENDING_ID_GENERATOR_KEY),
            order_req
        )
    }

    public fun cancel_bulk_order<M: store + copy + drop>(
        self: &mut OrderBook<M>, order_creator: address
    ): (OrderIdType, u64, u64) {
        let OrderBook::UnifiedV1 { bulk_order_book, price_time_idx, .. } = self;
        bulk_order_book.borrow_mut(BULK_ORDER_BOOK_KEY).cancel_bulk_order(
            price_time_idx.borrow_mut(PRICE_TIME_INDEX_KEY),
            order_creator
        )
    }

    public fun get_bulk_order_remaining_size<M: store + copy + drop>(
        self: &OrderBook<M>,
        order_creator: address,
        is_bid: bool
    ): u64 {
        let OrderBook::UnifiedV1 { bulk_order_book, .. } = self;
        bulk_order_book.borrow(BULK_ORDER_BOOK_KEY).get_remaining_size(order_creator, is_bid)
    }

    // ============================= test_only APIs ====================================
    #[test_only]
    public fun destroy_order_book<M: store + copy + drop>(
        self: OrderBook<M>
    ) {
        let OrderBook::UnifiedV1 {
            single_order_book,
            bulk_order_book,
            price_time_idx,
            ascending_id_generator
        } = self;
        let single_order_book_val = single_order_book.remove(SINGLE_ORDER_BOOK_KEY);
        let bulk_order_book_val = bulk_order_book.remove(BULK_ORDER_BOOK_KEY);
        let price_time_idx_val = price_time_idx.remove(PRICE_TIME_INDEX_KEY);
        let _ = ascending_id_generator.remove(ASCENDING_ID_GENERATOR_KEY);
        bulk_order_book_val.destroy_bulk_order_book();
        single_order_book_val.destroy_single_order_book();
        price_time_idx_val.destroy_price_time_idx();
        single_order_book.drop_unchecked();
        bulk_order_book.drop_unchecked();
        price_time_idx.drop_unchecked();
        ascending_id_generator.drop_unchecked();
    }

    #[test_only]
    public fun set_up_test_with_id(): OrderBook<u64> {
        new_order_book()
    }
}
