/// This module provides a core order book functionality for a trading system. On a high level, it has three major
/// components
/// 1. ActiveOrderBook: This is the main order book that keeps track of active orders and their states. The active order
/// book is backed by a BigOrderedMap, which is a data structure that allows for efficient insertion, deletion, and matching of the order
/// The orders are matched based on price-time priority.
/// 2. PendingOrderBookIndex: This keeps track of pending orders. The pending orders are those that are not active yet. Three
/// types of pending orders are supported.
///  - Price move up - Triggered when the price moves above a certain price level
/// - Price move down - Triggered when the price moves below a certain price level
/// - Time based - Triggered when a certain time has passed
/// 3. Orders: This is a BigOrderedMap of order id to order details.
///
module aptos_experimental::single_order_book {
    friend aptos_experimental::order_book;

    use std::vector;
    use std::error;
    use std::option::{Self, Option};
    use std::string::String;
    use aptos_framework::big_ordered_map::BigOrderedMap;
    use aptos_trading::order_book_types::{
        OrderId,
        AccountClientOrderId,
        new_account_client_order_id,
        IncreasingIdx,
        single_order_type,
        next_increasing_idx_type
    };
    use aptos_trading::order_match_types::{
        ActiveMatchedOrder,
        OrderMatchDetails,
        OrderMatch,
        new_order_match,
        new_single_order_match_details
    };
    use aptos_trading::single_order_types::{
        OrderWithState,
        new_single_order,
        new_order_with_state,
        SingleOrder,
        SingleOrderRequest,
        new_order_request_from_match_details
    };
    use aptos_experimental::price_time_index::PriceTimeIndex;
    use aptos_experimental::pending_order_book_index::{
        PendingOrderBookIndex,
        new_pending_order_book_index
    };
    use aptos_experimental::order_book_utils;

    #[test_only]
    use aptos_trading::order_book_types::{
        TriggerCondition,
        new_order_id_type,
        new_time_based_trigger_condition,
        price_move_up_condition,
        price_move_down_condition
    };
    #[test_only]
    use aptos_framework::timestamp;
    #[test_only]
    use aptos_framework::account;
    #[test_only]
    use aptos_trading::single_order_types::{
        create_simple_test_order_request,
        create_test_order_request,
        create_test_order_request_with_client_id
    };
    #[test_only]
    use aptos_experimental::price_time_index::new_price_time_idx;
    #[test_only]
    use aptos_trading::order_book_types::{TestMetadata, new_test_metadata};

    const EORDER_ALREADY_EXISTS: u64 = 1;
    const EPOST_ONLY_FILLED: u64 = 2;
    const EORDER_NOT_FOUND: u64 = 4;
    const EINVALID_INACTIVE_ORDER_STATE: u64 = 5;
    const EINVALID_ADD_SIZE_TO_ORDER: u64 = 6;
    const E_NOT_ACTIVE_ORDER: u64 = 7;
    const E_REINSERT_ORDER_MISMATCH: u64 = 8;
    const EORDER_CREATOR_MISMATCH: u64 = 9;
    const ENOT_SINGLE_ORDER_BOOK: u64 = 10;
    const ETRIGGER_COND_NOT_FOUND: u64 = 11;

    enum SingleOrderBook<M: store + copy + drop> has store {
        V1 {
            orders: BigOrderedMap<OrderId, OrderWithState<M>>,
            client_order_ids: BigOrderedMap<AccountClientOrderId, OrderId>,
            pending_orders: PendingOrderBookIndex
        }
    }

    public(friend) fun new_single_order_book<M: store + copy + drop>(): SingleOrderBook<M> {
        SingleOrderBook::V1 {
            orders: order_book_utils::new_default_big_ordered_map(),
            client_order_ids: order_book_utils::new_default_big_ordered_map(),
            pending_orders: new_pending_order_book_index()
        }
    }

    /// Cancels an order from the order book. If the order is active, it is removed from the active order book else
    /// it is removed from the pending order book.
    /// If order doesn't exist, it aborts with EORDER_NOT_FOUND.
    ///
    /// `order_creator` is passed to only verify order cancellation is authorized correctly
    public(friend) fun cancel_order<M: store + copy + drop>(
        self: &mut SingleOrderBook<M>,
        price_time_idx: &mut PriceTimeIndex,
        order_creator: address,
        order_id: OrderId
    ): SingleOrder<M> {
        let order_with_state_option = self.orders.remove_or_none(&order_id);
        assert!(order_with_state_option.is_some(), EORDER_NOT_FOUND);
        let order_with_state = order_with_state_option.destroy_some();
        let (order, is_active) = order_with_state.destroy_order_from_state();
        let order_request = order.get_order_request();
        assert!(order_creator == order_request.get_account(), EORDER_CREATOR_MISMATCH);
        if (is_active) {
            price_time_idx.cancel_active_order(
                order_request.get_price(),
                order.get_unique_priority_idx(),
                order_request.is_bid()
            );
            if (order_request.get_client_order_id().is_some()) {
                self.client_order_ids.remove(
                    &new_account_client_order_id(
                        order_request.get_account(),
                        order_request.get_client_order_id().destroy_some()
                    )
                );
            };
        } else {
            self.pending_orders.cancel_pending_order(
                order_request.get_trigger_condition().destroy_some(),
                order.get_unique_priority_idx()
            );
            if (order_request.get_client_order_id().is_some()) {
                self.client_order_ids.remove(
                    &new_account_client_order_id(
                        order_request.get_account(),
                        order_request.get_client_order_id().destroy_some()
                    )
                );
            };
        };
        return order
    }

    public(friend) fun try_cancel_order_with_client_order_id<M: store + copy + drop>(
        self: &mut SingleOrderBook<M>,
        price_time_idx: &mut PriceTimeIndex,
        order_creator: address,
        client_order_id: String
    ): Option<SingleOrder<M>> {
        let account_client_order_id =
            new_account_client_order_id(order_creator, client_order_id);
        let order_id = self.client_order_ids.get(&account_client_order_id);
        if (order_id.is_none()) {
            return option::none();
        };
        option::some(
            self.cancel_order(price_time_idx, order_creator, order_id.destroy_some())
        )
    }

    public(friend) fun try_cancel_order<M: store + copy + drop>(
        self: &mut SingleOrderBook<M>,
        price_time_idx: &mut PriceTimeIndex,
        order_creator: address,
        order_id: OrderId
    ): Option<SingleOrder<M>> {
        let is_creator =
            self.orders.get_and_map(
                &order_id,
                |order| order.get_order_from_state().get_order_request().get_account()
                    == order_creator
            );

        if (is_creator.is_none() || !is_creator.destroy_some()) {
            return option::none();
        };

        option::some(self.cancel_order(price_time_idx, order_creator, order_id))
    }

    public(friend) fun client_order_id_exists<M: store + copy + drop>(
        self: &SingleOrderBook<M>, order_creator: address, client_order_id: String
    ): bool {
        let account_client_order_id =
            new_account_client_order_id(order_creator, client_order_id);
        self.client_order_ids.contains(&account_client_order_id)
    }

    /// Places a maker order to the order book. If the order is a pending order, it is added to the pending order book
    /// else it is added to the active order book. The API aborts if it's not a maker order or if the order already exists
    public(friend) fun place_maker_or_pending_order<M: store + copy + drop>(
        self: &mut SingleOrderBook<M>,
        price_time_idx: &mut PriceTimeIndex,
        order_req: SingleOrderRequest<M>
    ) {
        let ascending_idx = next_increasing_idx_type();
        if (order_req.get_trigger_condition().is_some()) {
            return self.place_pending_order_internal(order_req, ascending_idx);
        };
        self.place_ready_maker_order_with_unique_idx(
            price_time_idx, order_req, ascending_idx
        );
    }

    fun place_ready_maker_order_with_unique_idx<M: store + copy + drop>(
        self: &mut SingleOrderBook<M>,
        price_time_idx: &mut PriceTimeIndex,
        order_req: SingleOrderRequest<M>,
        ascending_idx: IncreasingIdx
    ) {
        let order = new_single_order(order_req, ascending_idx);
        assert!(
            self.orders.upsert(
                order_req.get_order_id(), new_order_with_state(order, true)
            ).is_none(),
            error::invalid_argument(EORDER_ALREADY_EXISTS)
        );
        if (order_req.get_client_order_id().is_some()) {
            self.client_order_ids.add(
                new_account_client_order_id(
                    order_req.get_account(),
                    order_req.get_client_order_id().destroy_some()
                ),
                order_req.get_order_id()
            );
        };
        price_time_idx.place_maker_order(
            order_req.get_order_id(),
            single_order_type(),
            order_req.get_price(),
            ascending_idx,
            order_req.get_remaining_size(),
            order_req.is_bid()
        );
    }

    /// Reinserts a maker order to the order book. This is used when the order is removed from the order book
    /// but the clearinghouse fails to settle all or part of the order. If the order doesn't exist in the order book,
    /// it is added to the order book, if it exists, its size is updated.
    public(friend) fun reinsert_order<M: store + copy + drop>(
        self: &mut SingleOrderBook<M>,
        price_time_idx: &mut PriceTimeIndex,
        reinsert_order: OrderMatchDetails<M>,
        original_order: &OrderMatchDetails<M>
    ) {
        assert!(
            reinsert_order.validate_single_order_reinsertion_request(original_order),
            E_REINSERT_ORDER_MISMATCH
        );
        let order_id = reinsert_order.get_order_id_from_match_details();
        let unique_idx = reinsert_order.get_unique_priority_idx_from_match_details();

        let reinsert_remaining_size =
            reinsert_order.get_remaining_size_from_match_details();
        let present =
            self.orders.modify_if_present(
                &order_id,
                |order_with_state| {
                    order_with_state.increase_remaining_size_from_state(
                        reinsert_remaining_size
                    );
                }
            );
        if (!present) {
            return self.place_ready_maker_order_with_unique_idx(
                price_time_idx,
                new_order_request_from_match_details(reinsert_order),
                unique_idx
            );
        };

        price_time_idx.increase_order_size(
            reinsert_order.get_price_from_match_details(),
            unique_idx,
            reinsert_order.get_remaining_size_from_match_details(),
            reinsert_order.is_bid_from_match_details()
        );
    }

    fun place_pending_order_internal<M: store + copy + drop>(
        self: &mut SingleOrderBook<M>,
        order_req: SingleOrderRequest<M>,
        ascending_idx: IncreasingIdx
    ) {
        let order_id = order_req.get_order_id();
        let order = new_single_order(order_req, ascending_idx);
        self.orders.add(order_id, new_order_with_state(order, false));

        if (order_req.get_client_order_id().is_some()) {
            self.client_order_ids.add(
                new_account_client_order_id(
                    order_req.get_account(),
                    order_req.get_client_order_id().destroy_some()
                ),
                order_req.get_order_id()
            );
        };

        self.pending_orders.place_pending_order(
            order_id,
            order_req.get_trigger_condition().destroy_some(),
            ascending_idx
        );
    }

    /// Returns a single match for a taker order. It is responsibility of the caller to first call the `is_taker_order`
    /// API to ensure that the order is a taker order before calling this API, otherwise it will abort.
    public(friend) fun get_single_match_for_taker<M: store + copy + drop>(
        self: &mut SingleOrderBook<M>, active_matched_order: ActiveMatchedOrder
    ): OrderMatch<M> {
        let (order_id, matched_size, remaining_size, order_book_type) =
            active_matched_order.destroy_active_matched_order();
        assert!(order_book_type == single_order_type(), ENOT_SINGLE_ORDER_BOOK);

        let order_with_state =
            if (remaining_size == 0) {
                let order = self.orders.remove(&order_id);
                order.set_remaining_size_from_state(0);
                order
            } else {
                self.orders.modify_and_return(
                    &order_id,
                    |order_with_state| {
                        aptos_trading::single_order_types::set_remaining_size_from_state(
                            order_with_state, remaining_size
                        );
                        // order_with_state.set_remaining_size_from_state(remaining_size);
                        *order_with_state
                    }
                )
            };

        let (order, is_active) = order_with_state.destroy_order_from_state();
        assert!(is_active, EINVALID_INACTIVE_ORDER_STATE);

        let (order_request, unique_priority_idx) = order.destroy_single_order();
        let (
            account,
            order_id,
            client_order_id,
            price,
            orig_size,
            size,
            is_bid,
            _trigger_condition,
            time_in_force,
            creation_time_micros,
            metadata
        ) = order_request.destroy_single_order_request();

        if (remaining_size == 0 && client_order_id.is_some()) {
            self.client_order_ids.remove(
                &new_account_client_order_id(
                    order.get_order_request().get_account(),
                    client_order_id.destroy_some()
                )
            );
        };
        new_order_match(
            new_single_order_match_details(
                order_id,
                account,
                client_order_id,
                unique_priority_idx,
                price,
                orig_size,
                size,
                is_bid,
                time_in_force,
                creation_time_micros,
                metadata
            ),
            matched_size
        )
    }

    /// Decrease the size of the order by the given size delta. The API aborts if the order is not found in the order book or
    /// if the size delta is greater than or equal to the remaining size of the order. Please note that the API will abort and
    /// not cancel the order if the size delta is equal to the remaining size of the order, to avoid unintended
    /// cancellation of the order. Please use the `cancel_order` API to cancel the order.
    ///
    /// `order_creator` is passed to only verify order cancellation is authorized correctly
    public(friend) fun decrease_order_size<M: store + copy + drop>(
        self: &mut SingleOrderBook<M>,
        price_time_idx: &mut PriceTimeIndex,
        order_creator: address,
        order_id: OrderId,
        size_delta: u64
    ) {
        let order_opt =
            self.orders.modify_if_present_and_return(
                &order_id,
                |order_with_state| {
                    assert!(
                        order_creator
                            == order_with_state.get_order_from_state().get_order_request()
                            .get_account(),
                        EORDER_CREATOR_MISMATCH
                    );
                    // TODO should we be asserting that remaining size is greater than 0?
                    aptos_trading::single_order_types::decrease_remaining_size_from_state(
                        order_with_state, size_delta
                    );
                    // order_with_state.decrease_remaining_size(size_delta);
                    *order_with_state
                }
            );

        assert!(order_opt.is_some(), EORDER_NOT_FOUND);
        let order_with_state = order_opt.destroy_some();

        if (order_with_state.is_active_order()) {
            let order = order_with_state.get_order_from_state();
            price_time_idx.decrease_order_size(
                order.get_order_request().get_price(),
                order_with_state.get_unique_priority_idx_from_state(),
                size_delta,
                order.get_order_request().is_bid()
            );
        };
    }

    public(friend) fun get_order_id_by_client_id<M: store + copy + drop>(
        self: &SingleOrderBook<M>, order_creator: address, client_order_id: String
    ): Option<OrderId> {
        let account_client_order_id =
            new_account_client_order_id(order_creator, client_order_id);
        self.client_order_ids.get(&account_client_order_id)
    }

    public(friend) fun get_order_metadata<M: store + copy + drop>(
        self: &SingleOrderBook<M>, order_id: OrderId
    ): Option<M> {
        self.orders.get_and_map(&order_id, |order| order.get_metadata_from_state())
    }

    public(friend) fun set_order_metadata<M: store + copy + drop>(
        self: &mut SingleOrderBook<M>, order_id: OrderId, metadata: M
    ) {
        let present =
            self.orders.modify_if_present(
                &order_id,
                |order_with_state| {
                    order_with_state.set_metadata_in_state(metadata);
                }
            );
        assert!(present, EORDER_NOT_FOUND);
    }

    public(friend) fun is_active_order<M: store + copy + drop>(
        self: &SingleOrderBook<M>, order_id: OrderId
    ): bool {
        self.orders.get_and_map(&order_id, |order| order.is_active_order()).destroy_with_default(
            false
        )
    }

    public(friend) fun get_order<M: store + copy + drop>(
        self: &SingleOrderBook<M>, order_id: OrderId
    ): Option<OrderWithState<M>> {
        self.orders.get(&order_id)
    }

    public(friend) fun get_remaining_size<M: store + copy + drop>(
        self: &SingleOrderBook<M>, order_id: OrderId
    ): u64 {
        self.orders.get_and_map(
            &order_id, |order| order.get_remaining_size_from_state()
        ).destroy_with_default(0)
    }

    /// Removes and returns the orders that are ready to be executed based on the current price.
    public(friend) fun take_ready_price_based_orders<M: store + copy + drop>(
        self: &mut SingleOrderBook<M>, current_price: u64, order_limit: u64
    ): vector<SingleOrder<M>> {
        let self_orders = &mut self.orders;
        let self_client_order_ids = &mut self.client_order_ids;
        let order_ids =
            self.pending_orders.take_ready_price_based_orders(
                current_price, order_limit
            );
        let orders = vector::empty();

        order_ids.for_each(
            |order_id| {
                let order_with_state = self_orders.remove(&order_id);
                let (order, _) = order_with_state.destroy_order_from_state();
                let client_order_id = order.get_order_request().get_client_order_id();
                if (client_order_id.is_some()) {
                    self_client_order_ids.remove(
                        &new_account_client_order_id(
                            order.get_order_request().get_account(),
                            client_order_id.destroy_some()
                        )
                    );
                };
                orders.push_back(order);
            }
        );
        orders
    }

    /// Removes and returns the orders that are ready to be executed based on the time condition.
    public(friend) fun take_ready_time_based_orders<M: store + copy + drop>(
        self: &mut SingleOrderBook<M>, order_limit: u64
    ): vector<SingleOrder<M>> {
        let self_orders = &mut self.orders;
        let self_client_order_ids = &mut self.client_order_ids;
        let order_ids = self.pending_orders.take_ready_time_based_orders(order_limit);
        let orders = vector::empty();

        order_ids.for_each(
            |order_id| {
                let order_with_state = self_orders.remove(&order_id);
                let (order, _) = order_with_state.destroy_order_from_state();
                let client_order_id = order.get_order_request().get_client_order_id();
                if (client_order_id.is_some()) {
                    self_client_order_ids.remove(
                        &new_account_client_order_id(
                            order.get_order_request().get_account(),
                            client_order_id.destroy_some()
                        )
                    );
                };
                orders.push_back(order);
            }
        );
        orders
    }

    // ============================= test_only APIs ====================================

    #[test_only]
    public(friend) fun destroy_single_order_book<M: store + copy + drop>(
        self: SingleOrderBook<M>
    ) {
        let SingleOrderBook::V1 { orders, client_order_ids, pending_orders } = self;
        orders.destroy(|_v| {});
        client_order_ids.destroy(|_v| {});
        pending_orders.destroy_pending_order_book_index();
    }

    #[test_only]
    public(friend) fun get_unique_priority_idx<M: store + copy + drop>(
        self: &SingleOrderBook<M>, order_id: OrderId
    ): Option<IncreasingIdx> {
        self.orders.get_and_map(
            &order_id, |order| order.get_unique_priority_idx_from_state()
        )
    }

    #[test_only]
    public(friend) fun is_taker_order(
        price_time_idx: &PriceTimeIndex,
        price: u64,
        is_bid: bool,
        trigger_condition: Option<TriggerCondition>
    ): bool {
        if (trigger_condition.is_some()) {
            return false;
        };
        return price_time_idx.is_taker_order(price, is_bid)
    }

    #[test_only]
    public(friend) fun place_order_and_get_matches<M: store + copy + drop>(
        self: &mut SingleOrderBook<M>,
        price_time_idx: &mut PriceTimeIndex,
        order_req: SingleOrderRequest<M>
    ): vector<OrderMatch<M>> {
        let match_results = vector::empty();
        let remaining_size = order_req.get_remaining_size();
        while (remaining_size > 0) {
            if (!is_taker_order(
                price_time_idx,
                order_req.get_price(),
                order_req.is_bid(),
                order_req.get_trigger_condition()
            )) {
                *order_req.get_remaining_size_mut() = remaining_size;
                self.place_maker_or_pending_order(price_time_idx, order_req);
                return match_results;
            };
            let result =
                price_time_idx.get_single_match_result(
                    order_req.get_price(), remaining_size, order_req.is_bid()
                );
            let match_result = self.get_single_match_for_taker(result);
            let matched_size = match_result.get_matched_size();
            match_results.push_back(match_result);
            remaining_size -= matched_size;
        };
        return match_results
    }

    #[test_only]
    public(friend) fun update_order_and_get_matches<M: store + copy + drop>(
        self: &mut SingleOrderBook<M>,
        price_time_idx: &mut PriceTimeIndex,
        order_req: SingleOrderRequest<M>
    ): vector<OrderMatch<M>> {
        let unique_priority_idx = self.get_unique_priority_idx(order_req.get_order_id());
        assert!(unique_priority_idx.is_some(), EORDER_NOT_FOUND);
        self.cancel_order(
            price_time_idx, order_req.get_account(), order_req.get_order_id()
        );
        self.place_order_and_get_matches(price_time_idx, order_req)
    }

    #[test_only]
    public(friend) fun trigger_pending_orders<M: store + copy + drop>(
        self: &mut SingleOrderBook<M>,
        price_time_idx: &mut PriceTimeIndex,
        oracle_price: u64
    ): vector<OrderMatch<M>> {
        let ready_orders = self.take_ready_price_based_orders(oracle_price, 1000);
        let all_matches = vector::empty();
        let i = 0;
        while (i < ready_orders.length()) {
            let order = ready_orders[i];
            let (order_request, _unique_priority_idx) = order.destroy_single_order();
            *order_request.get_trigger_condition_mut() = option::none();
            let match_results =
                self.place_order_and_get_matches(price_time_idx, order_request);
            all_matches.append(match_results);
            i += 1;
        };
        all_matches
    }

    #[test_only]
    public(friend) fun total_matched_size<M: store + copy + drop>(
        match_results: &vector<OrderMatch<M>>
    ): u64 {
        let total_matched_size = 0;
        let i = 0;
        while (i < match_results.length()) {
            total_matched_size += match_results[i].get_matched_size();
            i += 1;
        };
        total_matched_size
    }

    #[test_only]
    public fun set_up_test(): (SingleOrderBook<TestMetadata>, PriceTimeIndex) {
        timestamp::set_time_has_started_for_testing(
            &account::create_signer_for_test(@0x1)
        );
        let order_book = new_single_order_book<TestMetadata>();
        let price_time_idx = new_price_time_idx();
        (order_book, price_time_idx)
    }

    #[test_only]
    public fun set_up_test_with_id(): (SingleOrderBook<u64>, PriceTimeIndex) {
        timestamp::set_time_has_started_for_testing(
            &account::create_signer_for_test(@0x1)
        );
        let order_book = new_single_order_book<u64>();
        let price_time_idx = new_price_time_idx();
        (order_book, price_time_idx)
    }

    // ============================= Test Helper Functions ====================================

    #[test_only]
    public fun verify_order_state<M: store + copy + drop>(
        order_book: &SingleOrderBook<M>,
        order_id: OrderId,
        expected_account: address,
        expected_price: u64,
        expected_orig_size: u64,
        expected_remaining_size: u64,
        expected_is_bid: bool,
        expected_client_order_id: Option<String>
    ) {
        let order_state = *order_book.orders.borrow(&order_id);
        let (order, is_active) = order_state.destroy_order_from_state();
        let (order_request, _) = order.destroy_single_order();
        let (
            account,
            _order_id,
            client_order_id,
            price,
            orig_size,
            remaining_size,
            is_bid,
            _trigger_condition,
            _time_in_force,
            _creation_time_micros,
            _metadata
        ) = order_request.destroy_single_order_request();
        assert!(is_active == true);
        assert!(account == expected_account);
        assert!(price == expected_price);
        assert!(orig_size == expected_orig_size);
        assert!(remaining_size == expected_remaining_size);
        assert!(is_bid == expected_is_bid);
        assert!(client_order_id == expected_client_order_id);
    }

    #[test_only]
    public fun verify_match_result<M: store + copy + drop>(
        match_results: &vector<OrderMatch<M>>,
        expected_matched_size: u64,
        expected_maker_account: address,
        expected_maker_order_id: OrderId,
        expected_maker_matched_size: u64,
        expected_maker_orig_size: u64,
        expected_maker_remaining_size: u64
    ) {
        assert!(total_matched_size(match_results) == expected_matched_size);
        assert!(match_results.length() == 1);
        let maker_match = match_results[0];
        let (match_details, matched_size) = maker_match.destroy_order_match();
        assert!(
            match_details.get_account_from_match_details() == expected_maker_account
        );
        assert!(
            match_details.get_order_id_from_match_details() == expected_maker_order_id
        );
        assert!(matched_size == expected_maker_matched_size);
        assert!(
            match_details.get_orig_size_from_match_details()
                == expected_maker_orig_size
        );
        assert!(
            match_details.get_remaining_size_from_match_details()
                == expected_maker_remaining_size
        );
    }

    #[test_only]
    public fun cleanup_test<M: store + copy + drop>(
        order_book: SingleOrderBook<M>, price_time_idx: PriceTimeIndex
    ) {
        order_book.destroy_single_order_book();
        price_time_idx.destroy_price_time_idx();
    }

    // ============================= Tests ====================================

    #[test]
    fun test_good_til_cancelled_order() {
        let (order_book, price_time_idx) = set_up_test();

        // Place a GTC sell order
        let order_req =
            create_test_order_request_with_client_id(
                @0xAA,
                new_order_id_type(1),
                std::string::utf8(b"1"),
                100,
                1000,
                false,
                new_test_metadata()
            );
        let match_results =
            order_book.place_order_and_get_matches(&mut price_time_idx, order_req);
        assert!(match_results.is_empty()); // No matches for first order

        // Verify order exists and is active
        verify_order_state(
            &order_book,
            new_order_id_type(1),
            @0xAA,
            100,
            1000,
            1000,
            false,
            option::some(std::string::utf8(b"1"))
        );

        // Place a matching buy order for partial fill
        let match_results =
            order_book.place_order_and_get_matches(
                &mut price_time_idx,
                create_test_order_request_with_client_id(
                    @0xBB,
                    new_order_id_type(1),
                    std::string::utf8(b"2"),
                    100,
                    400,
                    true,
                    new_test_metadata()
                )
            );

        // Verify taker match details
        assert!(total_matched_size(&match_results) == 400);
        assert!(order_book.get_remaining_size(new_order_id_type(2)) == 0);

        // Verify maker match details
        verify_match_result(
            &match_results,
            400,
            @0xAA,
            new_order_id_type(1),
            400,
            1000,
            600
        );

        // Verify original order still exists but with reduced size
        verify_order_state(
            &order_book,
            new_order_id_type(1),
            @0xAA,
            100,
            1000,
            600,
            false,
            option::some(std::string::utf8(b"1"))
        );

        // Cancel the remaining order
        order_book.cancel_order(&mut price_time_idx, @0xAA, new_order_id_type(1));

        // Verify order no longer exists
        assert!(order_book.get_remaining_size(new_order_id_type(1)) == 0);

        cleanup_test(order_book, price_time_idx);
    }

    #[test]
    fun test_update_buy_order() {
        let (order_book, price_time_idx) = set_up_test();

        // Place a GTC sell order
        let match_results =
            order_book.place_order_and_get_matches(
                &mut price_time_idx,
                create_simple_test_order_request(
                    @0xAA,
                    new_order_id_type(1),
                    101,
                    1000,
                    false,
                    new_test_metadata()
                )
            );
        assert!(match_results.is_empty());

        let match_results =
            order_book.place_order_and_get_matches(
                &mut price_time_idx,
                create_simple_test_order_request(
                    @0xBB,
                    new_order_id_type(2),
                    100,
                    500,
                    true,
                    new_test_metadata()
                )
            );
        assert!(match_results.is_empty());

        // Update the order so that it would match immediately
        let match_results =
            order_book.place_order_and_get_matches(
                &mut price_time_idx,
                create_simple_test_order_request(
                    @0xBB,
                    new_order_id_type(3),
                    101,
                    500,
                    true,
                    new_test_metadata()
                )
            );

        // Verify taker (buy order) was fully filled
        assert!(total_matched_size(&match_results) == 500);
        assert!(order_book.get_remaining_size(new_order_id_type(3)) == 0);

        verify_match_result(
            &match_results,
            500,
            @0xAA,
            new_order_id_type(1),
            500,
            1000,
            500
        );

        cleanup_test(order_book, price_time_idx);
    }

    #[test]
    fun test_update_sell_order() {
        let (order_book, price_time_idx) = set_up_test();

        // Place a GTC sell order
        let order_req =
            create_test_order_request_with_client_id(
                @0xAA,
                new_order_id_type(1),
                std::string::utf8(b"1"),
                100,
                1000,
                false,
                new_test_metadata()
            );
        let match_result =
            order_book.place_order_and_get_matches(&mut price_time_idx, order_req);
        assert!(match_result.is_empty()); // No matches for first order

        // Place a buy order at lower price
        let match_result =
            order_book.place_order_and_get_matches(
                &mut price_time_idx,
                create_test_order_request_with_client_id(
                    @0xBB,
                    new_order_id_type(2),
                    std::string::utf8(b"2"),
                    99,
                    500,
                    true,
                    new_test_metadata()
                )
            );
        assert!(match_result.is_empty());

        // Update sell order to match with buy order
        let match_results =
            order_book.update_order_and_get_matches(
                &mut price_time_idx,
                create_test_order_request_with_client_id(
                    @0xAA,
                    new_order_id_type(1),
                    std::string::utf8(b"3"),
                    99,
                    1000,
                    false,
                    new_test_metadata()
                )
            );

        // Verify taker (sell order) was partially filled
        assert!(total_matched_size(&match_results) == 500);

        verify_match_result(
            &match_results,
            500,
            @0xBB,
            new_order_id_type(2),
            500,
            500,
            0
        );

        cleanup_test(order_book, price_time_idx);
    }

    #[test]
    #[expected_failure(abort_code = EORDER_NOT_FOUND)]
    fun test_update_order_not_found() {
        let (order_book, price_time_idx) = set_up_test();

        // Place a GTC sell order
        let match_result =
            order_book.place_order_and_get_matches(
                &mut price_time_idx,
                create_simple_test_order_request(
                    @0xAA,
                    new_order_id_type(1),
                    101,
                    1000,
                    false,
                    new_test_metadata()
                )
            );
        assert!(match_result.is_empty()); // No matches for first order

        let match_result =
            order_book.place_order_and_get_matches(
                &mut price_time_idx,
                create_simple_test_order_request(
                    @0xBB,
                    new_order_id_type(2),
                    100,
                    500,
                    true,
                    new_test_metadata()
                )
            );
        assert!(match_result.is_empty());

        // Try to update non existant order
        let match_result =
            order_book.update_order_and_get_matches(
                &mut price_time_idx,
                create_simple_test_order_request(
                    @0xBB,
                    new_order_id_type(3),
                    100,
                    500,
                    true,
                    new_test_metadata()
                )
            );
        // This should fail with EORDER_NOT_FOUND
        assert!(match_result.is_empty());
        cleanup_test(order_book, price_time_idx);
    }

    #[test]
    fun test_good_til_cancelled_partial_fill() {
        let (order_book, price_time_idx) = set_up_test();

        // Place a GTC sell order for 1000 units at price 100
        let match_result =
            order_book.place_order_and_get_matches(
                &mut price_time_idx,
                create_simple_test_order_request(
                    @0xAA,
                    new_order_id_type(1),
                    100,
                    1000,
                    false,
                    new_test_metadata()
                )
            );
        assert!(match_result.is_empty()); // No matches for first order

        // Place a smaller buy order (400 units) at the same price
        let match_results =
            order_book.place_order_and_get_matches(
                &mut price_time_idx,
                create_simple_test_order_request(
                    @0xBB,
                    new_order_id_type(2),
                    100,
                    400,
                    true,
                    new_test_metadata()
                )
            );

        // Verify taker (buy order) was fully filled
        assert!(total_matched_size(&match_results) == 400);

        // Verify maker (sell order) was partially filled
        verify_match_result(
            &match_results,
            400,
            @0xAA,
            new_order_id_type(1),
            400,
            1000,
            600
        );

        // Place another buy order for 300 units
        let match_results =
            order_book.place_order_and_get_matches(
                &mut price_time_idx,
                create_simple_test_order_request(
                    @0xBB,
                    new_order_id_type(3),
                    100,
                    300,
                    true,
                    new_test_metadata()
                )
            );
        assert!(match_results.length() == 1); // Should match with the sell order

        // Verify second taker was fully filled
        assert!(total_matched_size(&match_results) == 300);

        // Verify original maker was partially filled again
        verify_match_result(
            &match_results,
            300,
            @0xAA,
            new_order_id_type(1),
            300,
            1000,
            300
        );

        // Original sell order should still exist with 300 units remaining
        verify_order_state(
            &order_book,
            new_order_id_type(1),
            @0xAA,
            100,
            1000,
            300,
            false,
            option::none()
        );

        cleanup_test(order_book, price_time_idx);
    }

    #[test]
    fun test_good_til_cancelled_taker_partial_fill() {
        let (order_book, price_time_idx) = set_up_test();

        // Place a GTC sell order for 500 units at price 100
        let match_result =
            order_book.place_order_and_get_matches(
                &mut price_time_idx,
                create_simple_test_order_request(
                    @0xAA,
                    new_order_id_type(1),
                    100,
                    500,
                    false,
                    new_test_metadata()
                )
            );
        assert!(match_result.is_empty()); // No matches for first order

        // Place a larger buy order (800 units) at the same price
        // Should partially fill against the sell order and remain in book
        let match_results =
            order_book.place_order_and_get_matches(
                &mut price_time_idx,
                create_simple_test_order_request(
                    @0xBB,
                    new_order_id_type(2),
                    100,
                    800,
                    true,
                    new_test_metadata()
                )
            );

        // Verify taker (buy order) was partially filled
        assert!(total_matched_size(&match_results) == 500);

        // Verify maker (sell order) was fully filled
        verify_match_result(
            &match_results,
            500,
            @0xAA,
            new_order_id_type(1),
            500,
            500,
            0
        );

        // Verify original sell order no longer exists (fully filled)
        let order_id = new_order_id_type(1);
        assert!(!order_book.orders.contains(&order_id));

        // Verify buy order still exists with remaining size
        verify_order_state(
            &order_book,
            new_order_id_type(2),
            @0xBB,
            100,
            800,
            300,
            true,
            option::none()
        );

        cleanup_test(order_book, price_time_idx);
    }

    #[test]
    fun test_price_move_down_condition() {
        let (order_book, price_time_idx) = set_up_test();

        // Place a GTC sell order for 1000 units at price 100
        let match_result =
            order_book.place_order_and_get_matches(
                &mut price_time_idx,
                create_simple_test_order_request(
                    @0xAA,
                    new_order_id_type(1),
                    100,
                    1000,
                    false,
                    new_test_metadata()
                )
            );
        assert!(match_result.is_empty()); // No matches for first order

        assert!(order_book.trigger_pending_orders(&mut price_time_idx, 100).is_empty());

        // Place a smaller buy order (400 units) at the same price
        let match_result =
            order_book.place_order_and_get_matches(
                &mut price_time_idx,
                create_test_order_request(
                    @0xBB,
                    new_order_id_type(2),
                    option::none(),
                    100,
                    400,
                    400,
                    true,
                    option::some(price_move_down_condition(90)),
                    new_test_metadata()
                )
            );
        // Even if the price of 100 can be matched in the order book the trigger condition 90 should not trigger
        // the matching
        assert!(match_result.is_empty());
        assert!(
            order_book.pending_orders
                .get_price_move_down_index()
                .keys()
                .length() == 1
        );

        // Trigger the pending orders with a price of 90
        let match_results = order_book.trigger_pending_orders(&mut price_time_idx, 90);

        // Verify taker (buy order) was fully filled
        assert!(total_matched_size(&match_results) == 400);

        // Verify maker (sell order) was partially filled
        verify_match_result(
            &match_results,
            400,
            @0xAA,
            new_order_id_type(1),
            400,
            1000,
            600
        );
        cleanup_test(order_book, price_time_idx);
    }

    #[test_only]
    fun test_price_move_condition_time_priority_helper(is_move_up: bool) {
        let (order_book, price_time_idx) = set_up_test();

        let trigger_price = if (is_move_up) 110 else 90;
        let condition =
            if (is_move_up) {
                option::some(price_move_up_condition(trigger_price))
            } else {
                option::some(price_move_down_condition(trigger_price))
            };

        order_book.place_order_and_get_matches(
            &mut price_time_idx,
            create_test_order_request(
                @0xAA,
                new_order_id_type(1),
                option::none(),
                100,
                400,
                400,
                !is_move_up, // buy for move_down, sell for move_up
                condition,
                new_test_metadata()
            )
        );
        order_book.place_order_and_get_matches(
            &mut price_time_idx,
            create_test_order_request(
                @0xBB,
                new_order_id_type(2),
                option::none(),
                100,
                400,
                400,
                !is_move_up, // buy for move_down, sell for move_up
                condition, // Same condition but later time
                new_test_metadata()
            )
        );

        let ready_orders = order_book.take_ready_price_based_orders(trigger_price, 1);
        assert!(ready_orders.length() == 1);
        let (order_request, _) = ready_orders[0].destroy_single_order();
        let (
            account,
            order_id,
            _client_order_id,
            _price,
            _orig_size,
            _remaining_size,
            _is_bid,
            _trigger_condition,
            _time_in_force,
            _creation_time_micros,
            _metadata
        ) = order_request.destroy_single_order_request();
        // Verify that the first order placed is the one that gets triggered first (time priority)
        assert!(account == @0xAA);
        assert!(order_id == new_order_id_type(1));

        let ready_orders = order_book.take_ready_price_based_orders(trigger_price, 1);
        assert!(ready_orders.length() == 1);
        let (order_request, _) = ready_orders[0].destroy_single_order();
        let (
            account,
            order_id,
            _client_order_id,
            _price,
            _orig_size,
            _remaining_size,
            _is_bid,
            _trigger_condition,
            _time_in_force,
            _creation_time_micros,
            _metadata
        ) = order_request.destroy_single_order_request();
        // Verify that the second order placed is the one that gets triggered second
        assert!(account == @0xBB);
        assert!(order_id == new_order_id_type(2));
        cleanup_test(order_book, price_time_idx);
    }

    #[test]
    fun test_price_move_down_condition_time_priority() {
        test_price_move_condition_time_priority_helper(false);
    }

    #[test]
    fun test_price_move_up_condition_time_priority() {
        test_price_move_condition_time_priority_helper(true);
    }

    #[test]
    fun test_price_move_up_condition() {
        let (order_book, price_time_idx) = set_up_test();

        // Place a GTC sell order for 1000 units at price 100
        let match_result =
            order_book.place_order_and_get_matches(
                &mut price_time_idx,
                create_simple_test_order_request(
                    @0xAA,
                    new_order_id_type(1),
                    100,
                    1000,
                    true,
                    new_test_metadata()
                )
            );
        assert!(match_result.is_empty()); // No matches for first order

        assert!(order_book.trigger_pending_orders(&mut price_time_idx, 100).is_empty());

        // Place a smaller buy order (400 units) at the same price
        let match_result =
            order_book.place_order_and_get_matches(
                &mut price_time_idx,
                create_test_order_request(
                    @0xBB,
                    new_order_id_type(2),
                    option::none(),
                    100,
                    400,
                    400,
                    false,
                    option::some(price_move_up_condition(110)),
                    new_test_metadata()
                )
            );
        // Even if the price of 100 can be matched in the order book the trigger condition 110 should not trigger
        // the matching
        assert!(match_result.is_empty());
        assert!(
            order_book.pending_orders
                .get_price_move_up_index()
                .keys()
                .length() == 1
        );

        // Trigger the pending orders with a price of 110
        let match_results = order_book.trigger_pending_orders(&mut price_time_idx, 110);
        assert!(match_results.length() == 1);

        // Verify taker (buy order) was fully filled
        assert!(total_matched_size(&match_results) == 400);

        // Verify maker (sell order) was partially filled
        verify_match_result(
            &match_results,
            400,
            @0xAA,
            new_order_id_type(1),
            400,
            1000,
            600
        );

        // Place another buy order for 300 units
        let match_result =
            order_book.place_order_and_get_matches(
                &mut price_time_idx,
                create_test_order_request(
                    @0xBB,
                    new_order_id_type(3),
                    option::none(),
                    100,
                    300,
                    300,
                    false,
                    option::some(price_move_up_condition(120)),
                    new_test_metadata()
                )
            );

        assert!(match_result.is_empty());
        assert!(
            order_book.pending_orders
                .get_price_move_up_index()
                .keys()
                .length() == 1
        );

        // Oracle price moves down to 100, this should not trigger any order
        let match_results = order_book.trigger_pending_orders(&mut price_time_idx, 100);
        assert!(match_results.is_empty());

        // Move the oracle price up to 120, this should trigger the order
        let match_results = order_book.trigger_pending_orders(&mut price_time_idx, 120);

        // Verify second taker was fully filled
        assert!(total_matched_size(&match_results) == 300);

        // Verify original maker was partially filled again
        verify_match_result(
            &match_results,
            300,
            @0xAA,
            new_order_id_type(1),
            300,
            1000,
            300
        );

        // Original sell order should still exist with 300 units remaining
        verify_order_state(
            &order_book,
            new_order_id_type(1),
            @0xAA,
            100,
            1000,
            300,
            true,
            option::none()
        );
        cleanup_test(order_book, price_time_idx);
    }

    #[test]
    fun test_duplicate_time_condition() {
        let (order_book, price_time_idx) = set_up_test();

        let match_result =
            order_book.place_order_and_get_matches(
                &mut price_time_idx,
                create_test_order_request(
                    @0xBB,
                    new_order_id_type(2),
                    option::none(),
                    100,
                    400,
                    400,
                    true,
                    option::some(new_time_based_trigger_condition(10000)),
                    new_test_metadata()
                )
            );
        assert!(match_result.is_empty());

        assert!(
            order_book.pending_orders
                .get_time_based_index()
                .keys()
                .length() == 1
        );

        let match_result =
            order_book.place_order_and_get_matches(
                &mut price_time_idx,
                create_test_order_request(
                    @0xCC,
                    new_order_id_type(3),
                    option::none(),
                    100,
                    300,
                    300,
                    true,
                    option::some(new_time_based_trigger_condition(10000)),
                    new_test_metadata()
                )
            );
        assert!(match_result.is_empty());
        assert!(
            order_book.pending_orders
                .get_time_based_index()
                .keys()
                .length() == 2
        );
        cleanup_test(order_book, price_time_idx);
    }

    #[test]
    fun test_maker_order_reinsert_already_exists() {
        let (order_book, price_time_idx) = set_up_test();

        // Place a GTC sell order
        let order_req =
            create_test_order_request_with_client_id(
                @0xAA,
                new_order_id_type(1),
                std::string::utf8(b"1"),
                100,
                1000,
                false,
                new_test_metadata()
            );
        order_book.place_maker_or_pending_order(&mut price_time_idx, order_req);
        assert!(order_book.get_remaining_size(new_order_id_type(1)) == 1000);

        assert!(order_book.client_order_id_exists(@0xAA, std::string::utf8(b"1")));

        // Taker order
        let order_req =
            create_simple_test_order_request(
                @0xBB,
                new_order_id_type(2),
                100,
                100,
                true,
                new_test_metadata()
            );

        let match_results =
            order_book.place_order_and_get_matches(&mut price_time_idx, order_req);
        assert!(total_matched_size(&match_results) == 100);

        assert!(order_book.client_order_id_exists(@0xAA, std::string::utf8(b"1")));

        let (matched_order, _) = match_results[0].destroy_order_match();
        let reinsert_request =
            matched_order.new_order_match_details_with_modified_size(50);
        // Assume half of the order was matched and remaining 50 size is reinserted back to the order book
        order_book.reinsert_order(&mut price_time_idx, reinsert_request, &matched_order);
        assert!(order_book.client_order_id_exists(@0xAA, std::string::utf8(b"1")));
        // Verify order was reinserted with updated size
        assert!(order_book.get_remaining_size(new_order_id_type(1)) == 950);
        cleanup_test(order_book, price_time_idx);
    }

    #[test]
    fun test_maker_order_reinsert_not_exists() {
        let (order_book, price_time_idx) = set_up_test();

        // Place a GTC sell order
        let order_req =
            create_test_order_request_with_client_id(
                @0xAA,
                new_order_id_type(1),
                std::string::utf8(b"1"),
                100,
                1000,
                false,
                new_test_metadata()
            );
        order_book.place_maker_or_pending_order(&mut price_time_idx, order_req);
        assert!(order_book.get_remaining_size(new_order_id_type(1)) == 1000);

        // Taker order
        let order_req =
            create_test_order_request_with_client_id(
                @0xBB,
                new_order_id_type(2),
                std::string::utf8(b"1"),
                100,
                1000,
                true,
                new_test_metadata()
            );

        assert!(order_book.client_order_id_exists(@0xAA, std::string::utf8(b"1")));

        let match_results =
            order_book.place_order_and_get_matches(&mut price_time_idx, order_req);
        assert!(total_matched_size(&match_results) == 1000);

        assert!(!order_book.client_order_id_exists(@0xAA, std::string::utf8(b"1")));

        let (matched_order, _) = match_results[0].destroy_order_match();
        let reinsert_request =
            matched_order.new_order_match_details_with_modified_size(500);

        order_book.reinsert_order(&mut price_time_idx, reinsert_request, &matched_order);
        assert!(order_book.client_order_id_exists(@0xAA, std::string::utf8(b"1")));
        // Verify order was reinserted with updated size
        assert!(order_book.get_remaining_size(new_order_id_type(1)) == 500);
        cleanup_test(order_book, price_time_idx);
    }

    #[test]
    fun test_reinserted_order_preserves_timestamp() {
        let (order_book, price_time_idx) = set_up_test();

        // Place a GTC sell order (maker)
        let order_req =
            create_test_order_request_with_client_id(
                @0xAA,
                new_order_id_type(1),
                std::string::utf8(b"1"),
                100,
                1000,
                false,
                new_test_metadata()
            );
        order_book.place_maker_or_pending_order(&mut price_time_idx, order_req);
        assert!(order_book.get_remaining_size(new_order_id_type(1)) == 1000);

        // Get the original creation timestamp
        let order_opt = order_book.get_order(new_order_id_type(1));
        assert!(order_opt.is_some());
        let order_with_state = order_opt.destroy_some();
        let original_timestamp =
            order_with_state.get_order_from_state().get_order_request().get_creation_time_micros();

        // Fast forward time
        timestamp::fast_forward_seconds(100);

        // Taker order comes and fully fills the maker order
        let order_req =
            create_simple_test_order_request(
                @0xBB,
                new_order_id_type(2),
                100,
                1000, // Full fill: 1000 out of 1000
                true,
                new_test_metadata()
            );

        let match_results =
            order_book.place_order_and_get_matches(&mut price_time_idx, order_req);
        assert!(total_matched_size(&match_results) == 1000);

        // After full fill, order should be removed from the order book
        assert!(order_book.get_remaining_size(new_order_id_type(1)) == 0);

        // Get the matched order details
        let (matched_order, _) = match_results[0].destroy_order_match();

        // Verify the matched order has the original timestamp
        let matched_timestamp =
            matched_order.get_creation_time_micros_from_match_details();
        assert!(matched_timestamp == original_timestamp);

        // Reinsert partial amount back into the order book (simulating a failed settlement)
        let reinsert_request =
            matched_order.new_order_match_details_with_modified_size(500);
        order_book.reinsert_order(&mut price_time_idx, reinsert_request, &matched_order);

        // Verify the order was reinserted with the specified size
        assert!(order_book.get_remaining_size(new_order_id_type(1)) == 500);

        // Check that the reinserted order still has the original timestamp
        let reinserted_order_opt = order_book.get_order(new_order_id_type(1));
        assert!(reinserted_order_opt.is_some());
        let reinserted_order_with_state = reinserted_order_opt.destroy_some();
        let reinserted_timestamp =
            reinserted_order_with_state.get_order_from_state().get_order_request().get_creation_time_micros();

        // The critical assertion: reinserted order must preserve the original timestamp
        assert!(reinserted_timestamp == original_timestamp);

        cleanup_test(order_book, price_time_idx);
    }

    #[test]
    fun test_decrease_order_size() {
        let (order_book, price_time_idx) = set_up_test();

        // Place an active order
        let order_req =
            create_simple_test_order_request(
                @0xAA,
                new_order_id_type(1),
                100,
                1000,
                false,
                new_test_metadata()
            );
        order_book.place_maker_or_pending_order(&mut price_time_idx, order_req);
        assert!(order_book.get_remaining_size(new_order_id_type(1)) == 1000);

        order_book.decrease_order_size(
            &mut price_time_idx,
            @0xAA,
            new_order_id_type(1),
            700
        );
        // Verify order was decreased with updated size
        assert!(order_book.get_remaining_size(new_order_id_type(1)) == 300);

        let order_req =
            create_test_order_request(
                @0xBB,
                new_order_id_type(2),
                option::none(),
                100,
                1000,
                1000,
                false,
                option::some(price_move_up_condition(90)),
                new_test_metadata()
            );
        order_book.place_maker_or_pending_order(&mut price_time_idx, order_req);
        assert!(order_book.get_remaining_size(new_order_id_type(2)) == 1000);
        order_book.decrease_order_size(
            &mut price_time_idx,
            @0xBB,
            new_order_id_type(2),
            600
        );
        // Verify order was decreased with updated size
        assert!(order_book.get_remaining_size(new_order_id_type(2)) == 400);

        cleanup_test(order_book, price_time_idx);
    }

    #[test]
    fun test_get_and_set_order_metadata() {
        let (order_book, price_time_idx) = set_up_test_with_id();

        // Place an active order
        let order_req =
            create_simple_test_order_request(
                @0xAA, new_order_id_type(1), 100, 1000, false, 1
            );
        order_book.place_maker_or_pending_order(&mut price_time_idx, order_req);
        // Verify order was placed with correct metadata
        let metadata = order_book.get_order_metadata(new_order_id_type(1));
        assert!(metadata.is_some());
        assert!(metadata.destroy_some() == 1);

        // Update order metadata
        order_book.set_order_metadata(new_order_id_type(1), 2);
        // Verify order metadata was updated
        let metadata = order_book.get_order_metadata(new_order_id_type(1));
        assert!(metadata.is_some());
        assert!(metadata.destroy_some() == 2);

        // Try to get metadata for non-existing order
        let non_existing_metadata = order_book.get_order_metadata(new_order_id_type(999));
        assert!(non_existing_metadata.is_none());
        cleanup_test(order_book, price_time_idx);
    }
}
