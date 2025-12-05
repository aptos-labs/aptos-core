module aptos_experimental::orders_index {

    friend aptos_experimental::order_placement;
    friend aptos_experimental::order_operations;
    friend aptos_experimental::market_types;
    friend aptos_experimental::market_bulk_order;
    #[test_only] friend aptos_experimental::order_book_client_order_id;

    use std::option::Option;
    use std::string::String;
    use aptos_experimental::bulk_order_book_types::{BulkOrder, BulkOrderRequest, BulkOrderPlaceResponse};
    use aptos_experimental::bulk_order_book::{BulkOrderBook, new_bulk_order_book};
    use aptos_experimental::single_order_book::{SingleOrderBook, new_single_order_book, SingleOrderRequest};
    use aptos_experimental::order_book_types::{OrderIdType, OrderMatch, OrderMatchDetails};
    use aptos_experimental::single_order_types::SingleOrder;
    use aptos_experimental::order_book_types::TriggerCondition;
    use aptos_experimental::order_book_types::TimeInForce;
    use aptos_experimental::price_time_index::{PriceTimeIndex, new_price_time_idx};
    use aptos_framework::transaction_context;
    use aptos_std::table::{Self, Table};
    use aptos_experimental::order_book_types::UniqueIdxType;

    // ============================= APIs relevant to single order only ====================================

    enum IndexedOrder<M: store + copy + drop> {
        SingleOrder {
            unique_priority_idx: UniqueIdxType,
            order_request: SingleOrderRequest<M>
        },
        BulkOrder {
            order_id: OrderIdType,
            unique_priority_idx: UniqueIdxType,
            order_request: BulkOrderRequest<M>
        },
    }

    // change to enum
    enum OrdersIndex<M: store + copy + drop> has store {
        V1 {
            id: u128,
            version: u64,
            data: table::Table<OrderIdType, SingleOrder<M>>,
        }
    }

    public fun new_orders_index<M: store + copy + drop>(): OrdersIndex<M> {
        OrdersIndex::V1 {
            id: transaction_context::monotonically_increasing_counter(),
            version: 0,
            data: table::new(),
        }
    }

    //============================ Public Read APIs ============================
    // // public native fun client_order_id_exists<M: store + copy + drop>(
    // //     self: &OrdersIndex<M>, order_creator: address, client_order_id: String
    // // ): bool;

    // public native fun get_order_metadata<M: store + copy + drop>(
    //     self: &OrdersIndex<M>, order_id: OrderIdType
    // ): Option<M>;

    // // public native fun get_order_id_by_client_id<M: store + copy + drop>(
    // //     self: &OrdersIndex<M>, order_creator: address, client_order_id: String
    // // ): Option<OrderIdType>;

    // public native fun is_active_order<M: store + copy + drop>(
    //     self: &OrdersIndex<M>, order_id: OrderIdType
    // ): bool;

    // public native fun get_order<M: store + copy + drop>(
    //     self: &OrdersIndex<M>, order_id: OrderIdType
    // ): Option<aptos_experimental::single_order_types::OrderWithState<M>>;

    // public native fun get_remaining_size<M: store + copy + drop>(
    //     self: &OrdersIndex<M>, order_id: OrderIdType
    // ): u64;

    //============================ Public(package) Write APIs ============================
    // public native fun cancel_order<M: store + copy + drop>(
    //     self: &mut OrdersIndex<M>, order_creator: address, order_id: OrderIdType
    // ): SingleOrder<M>;

    // public(friend) native fun try_cancel_order<M: store + copy + drop>(
    //     self: &mut OrdersIndex<M>, order_creator: address, order_id: OrderIdType
    // ): Option<SingleOrder<M>>;

    // // public(friend) native fun try_cancel_order_with_client_order_id<M: store + copy + drop>(
    // //     self: &mut OrdersIndex<M>, order_creator: address, client_order_id: String
    // // ): Option<SingleOrder<M>>;

    public native fun place_maker_order<M: store + copy + drop>(
        self: &mut OrdersIndex<M>, order_req: SingleOrderRequest<M>
    );

    // public(friend) native fun decrease_order_size<M: store + copy + drop>(
    //     self: &mut OrdersIndex<M>, order_creator: address, order_id: OrderIdType, size_delta: u64
    // );

    // public(friend) native fun set_order_metadata<M: store + copy + drop>(
    //     self: &mut OrdersIndex<M>, order_id: OrderIdType, metadata: M
    // );

    // // public(friend) native fun take_ready_price_based_orders<M: store + copy + drop>(
    // //     self: &mut OrdersIndex<M>, oracle_price: u64, order_limit: u64
    // // ): vector<SingleOrder<M>>;

    // // public(friend) native fun take_ready_time_based_orders<M: store + copy + drop>(
    // //     self: &mut OrdersIndex<M>, order_limit: u64
    // // ): vector<SingleOrder<M>>;

    // ============================= APIs relevant to both single and bulk order ====================================

    // public native fun best_bid_price<M: store + copy + drop>(self: &OrdersIndex<M>): Option<u64>;

    // public native fun best_ask_price<M: store + copy + drop>(self: &OrdersIndex<M>): Option<u64>;

    // public native fun get_mid_price<M: store + copy + drop>(self: &OrdersIndex<M>): Option<u64>;

    // public native fun get_slippage_price<M: store + copy + drop>(
    //     self: &OrdersIndex<M>, is_bid: bool, slippage_pct: u64
    // ): Option<u64>;

    // public native fun get_bulk_order_remaining_size<M: store + copy + drop>(
    //     self: &OrdersIndex<M>,
    //     order_creator: address,
    //     is_bid: bool
    // ): u64;

    /// Checks if the order is a taker order i.e., matched immediately with the active order book.
    public native fun is_taker_order<M: store + copy + drop>(
        self: &OrdersIndex<M>,
        price: u64,
        is_bid: bool,
        trigger_condition: Option<TriggerCondition>
    ): bool;

    public native fun get_single_match_for_taker<M: store + copy + drop>(
        self: &mut OrdersIndex<M>,
        price: u64,
        size: u64,
        is_bid: bool
    ): OrderMatch<M>;

    // public(friend) native fun reinsert_order<M: store + copy + drop>(
    //     self: &mut OrdersIndex<M>,
    //     reinsert_order: OrderMatchDetails<M>,
    //     original_order: &OrderMatchDetails<M>,
    // );

    // // ============================= APIs relevant to bulk order only ====================================
    // public(friend) native fun place_bulk_order<M: store + copy + drop>(
    //     self: &mut OrderBook<M>, order_req: BulkOrderRequest<M>
    // ) : BulkOrderPlaceResponse<M>;

    // public(friend) native fun get_bulk_order<M: store + copy + drop>(
    //     self: &OrdersIndex<M>, order_creator: address
    // ): BulkOrder<M>;

    // public(friend) native fun cancel_bulk_order<M: store + copy + drop>(
    //     self: &mut OrdersIndex<M>, order_creator: address
    // ): BulkOrder<M>;

    #[test_only]
    fun destroy<M: store + copy + drop>(self: OrdersIndex<M>) {
        let OrdersIndex::V1 { id: _, version: _, data } = self;
        data.drop_unchecked();
    }

    #[test_only]
    struct None has copy, store, drop {}

    #[test]
    fun test_basic_orders_index() {
        let orders_index = new_orders_index<None>();
        assert!(!orders_index.is_taker_order(100, true, std::option::none()));
        orders_index.place_maker_order(aptos_experimental::order_book::new_single_order_request(
            @0x1234,
            aptos_experimental::order_book_types::next_order_id(),
            std::option::none(),
            100,
            100,
            100,
            true,
            std::option::none(),
            aptos_experimental::order_book_types::good_till_cancelled(),
            None{}
        ));

        let match_result = orders_index.get_single_match_for_taker(100, 20, false);
        let matched_size = match_result.get_matched_size();
        assert!(matched_size == 20);

        orders_index.destroy();
    }
}
