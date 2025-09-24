/// This module provides a core order book functionality for a trading system. On a high level, it has three major
/// components
/// 1. ActiveOrderBook: This is the main order book that keeps track of active orders and their states. The active order
/// book is backed by a BigOrderedMap, which is a data structure that allows for efficient insertion, deletion, and matching of the order
/// The orders are matched based on time-price priority.
/// 2. PendingOrderBookIndex: This keeps track of pending orders. The pending orders are those that are not active yet. Three
/// types of pending orders are supported.
///  - Price move up - Trigggered when the price moves above a certain price level
/// - Price move down - Triggered when the price moves below a certain price level
/// - Time based - Triggered when a certain time has passed
/// 3. Orders: This is a BigOrderMap of order id to order details.
///
module aptos_experimental::single_order_book {
    friend aptos_experimental::order_book;
    use std::vector;
    use std::error;
    use std::option::{Self, Option};
    use aptos_framework::big_ordered_map::BigOrderedMap;

    use aptos_experimental::order_book_types::{
        OrderIdType,
        AscendingIdGenerator,
        AccountClientOrderId,
        new_unique_idx_type,
        new_account_client_order_id,
        new_default_big_ordered_map, OrderMatch, new_order_match, new_order_match_details, OrderMatchDetails,
        UniqueIdxType, single_order_book_type
    };
    use aptos_experimental::single_order_types::{
        OrderWithState,
        new_single_order,
        new_order_with_state,
        SingleOrder
    };
    use aptos_experimental::order_book_types::ActiveMatchedOrder;
    use aptos_experimental::order_book_types::{TimeInForce, TriggerCondition};
    use aptos_experimental::price_time_index::{PriceTimeIndex, new_price_time_idx};
    use aptos_experimental::pending_order_book_index::{
        PendingOrderBookIndex,
        new_pending_order_book_index
    };
    #[test_only]
    use aptos_experimental::order_book_types::{
        new_order_id_type,
        new_ascending_id_generator
    };
    #[test_only]
    use aptos_experimental::order_book_types::{good_till_cancelled, price_move_up_condition, price_move_down_condition};

    const EORDER_ALREADY_EXISTS: u64 = 1;
    const EPOST_ONLY_FILLED: u64 = 2;
    const EORDER_NOT_FOUND: u64 = 4;
    const EINVALID_INACTIVE_ORDER_STATE: u64 = 5;
    const EINVALID_ADD_SIZE_TO_ORDER: u64 = 6;
    const E_NOT_ACTIVE_ORDER: u64 = 7;
    const E_REINSERT_ORDER_MISMATCH: u64 = 8;
    const EORDER_CREATOR_MISMATCH: u64 = 9;
    const ENOT_SINGLE_ORDER_BOOK: u64 = 10;

    enum SingleOrderRequest<M: store + copy + drop> has copy, drop {
        V1 {
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
        }
    }

    enum SingleOrderBook<M: store + copy + drop> has store {
        V1 {
            orders: BigOrderedMap<OrderIdType, OrderWithState<M>>,
            client_order_ids: BigOrderedMap<AccountClientOrderId, OrderIdType>,
            pending_orders: PendingOrderBookIndex
        }
    }

    enum OrderType has store, drop, copy {
        GoodTilCancelled,
        PostOnly,
        FillOrKill
    }

    public(friend) fun new_single_order_request<M: store + copy + drop>(
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
        SingleOrderRequest::V1 {
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
        }
    }

    fun new_order_request_from_match_details<M: store + copy + drop>(
        order_match_details: OrderMatchDetails<M>
    ): SingleOrderRequest<M> {
        let (
            order_id,
            account,
            client_order_id,
            _unique_priority_idx,
            price,
            orig_size,
            remaining_size,
            is_bid,
            time_in_force,
            metadata,
            _single_order_book_type
        ) = order_match_details.destroy_order_match_details();
        SingleOrderRequest::V1 {
            account,
            order_id,
            client_order_id,
            price,
            orig_size,
            remaining_size,
            is_bid,
            trigger_condition: option::none(),
            time_in_force,
            metadata,
        }
    }

    public(friend) fun new_single_order_book<M: store + copy + drop>(): SingleOrderBook<M> {
        SingleOrderBook::V1 {
            orders: new_default_big_ordered_map(),
            client_order_ids: new_default_big_ordered_map(),
            pending_orders: new_pending_order_book_index()
        }
    }

    public(friend) fun new_price_time_index(): PriceTimeIndex {
        new_price_time_idx()
    }

    /// Cancels an order from the order book. If the order is active, it is removed from the active order book else
    /// it is removed from the pending order book.
    /// If order doesn't exist, it aborts with EORDER_NOT_FOUND.
    ///
    /// `order_creator` is passed to only verify order cancellation is authorized correctly
    public(friend) fun cancel_order<M: store + copy + drop>(
        self: &mut SingleOrderBook<M>, price_time_idx: &mut PriceTimeIndex, order_creator: address, order_id: OrderIdType
    ): SingleOrder<M> {
        assert!(self.orders.contains(&order_id), EORDER_NOT_FOUND);
        let order_with_state = self.orders.remove(&order_id);
        let (order, is_active) = order_with_state.destroy_order_from_state();
        assert!(order_creator == order.get_account(), EORDER_CREATOR_MISMATCH);
        if (is_active) {
            let (
                account,
                _order_id,
                client_order_id,
                unique_priority_idx,
                bid_price,
                _orig_size,
                _size,
                is_bid,
                _,
                _,
                _
            ) = order.destroy_single_order();
            price_time_idx.cancel_active_order(bid_price, unique_priority_idx, is_bid);
            if (client_order_id.is_some()) {
                self.client_order_ids.remove(
                    &new_account_client_order_id(account, client_order_id.destroy_some())
                );
            };
        } else {
            let (
                _account,
                _order_id,
                client_order_id,
                unique_priority_idx,
                _bid_price,
                _orig_size,
                _size,
                _is_bid,
                trigger_condition,
                _,
                _
            ) = order.destroy_single_order();
            self.pending_orders.cancel_pending_order(
                trigger_condition.destroy_some(), unique_priority_idx
            );
            if (client_order_id.is_some()) {
                self.client_order_ids.remove(
                    &new_account_client_order_id(
                        order.get_account(), client_order_id.destroy_some()
                    )
                );
            };
        };
        return order
    }

    public(friend) fun try_cancel_order_with_client_order_id<M: store + copy + drop>(
        self: &mut SingleOrderBook<M>, price_time_idx: &mut PriceTimeIndex, order_creator: address, client_order_id: u64
    ): Option<SingleOrder<M>> {
        let account_client_order_id =
            new_account_client_order_id(order_creator, client_order_id);
        if (!self.client_order_ids.contains(&account_client_order_id)) {
            return option::none();
        };
        let order_id = self.client_order_ids.borrow(&account_client_order_id);
        option::some(self.cancel_order(price_time_idx, order_creator, *order_id))
    }

    public(friend) fun try_cancel_order<M: store + copy + drop>(
        self: &mut SingleOrderBook<M>, price_time_idx: &mut PriceTimeIndex, order_creator: address, order_id: OrderIdType
    ): Option<SingleOrder<M>> {
        if (!self.orders.contains(&order_id)) {
            return option::none();
        };
        let order = self.orders.borrow(&order_id);
        if (order.get_order_from_state().get_account() != order_creator) {
            return option::none();
        };
        option::some(self.cancel_order(price_time_idx, order_creator, order_id))
    }

    public(friend) fun client_order_id_exists<M: store + copy + drop>(
        self: &SingleOrderBook<M>, order_creator: address, client_order_id: u64
    ): bool {
        let account_client_order_id =
            new_account_client_order_id(order_creator, client_order_id);
        self.client_order_ids.contains(&account_client_order_id)
    }

    /// Places a maker order to the order book. If the order is a pending order, it is added to the pending order book
    /// else it is added to the active order book. The API aborts if its not a maker order or if the order already exists
    public(friend) fun place_maker_order<M: store + copy + drop>(
        self: &mut SingleOrderBook<M>, price_time_idx: &mut PriceTimeIndex, ascending_id_generator: &mut AscendingIdGenerator, order_req: SingleOrderRequest<M>
    ) {
        let ascending_idx =
            new_unique_idx_type(ascending_id_generator.next_ascending_id());
        if (order_req.trigger_condition.is_some()) {
            return self.place_pending_maker_order(ascending_id_generator, order_req);
        };
        self.place_ready_maker_order_with_unique_idx(price_time_idx, order_req, ascending_idx);

    }

    fun place_ready_maker_order_with_unique_idx<M: store + copy + drop>(
        self: &mut SingleOrderBook<M>,
        price_time_idx: &mut PriceTimeIndex,
        order_req: SingleOrderRequest<M>,
        ascending_idx: UniqueIdxType
    ) {
        assert!(
            !self.orders.contains(&order_req.order_id),
            error::invalid_argument(EORDER_ALREADY_EXISTS)
        );

        let order =
            new_single_order(
                order_req.order_id,
                order_req.account,
                ascending_idx,
                order_req.client_order_id,
                order_req.price,
                order_req.orig_size,
                order_req.remaining_size,
                order_req.is_bid,
                order_req.trigger_condition,
                order_req.time_in_force,
                order_req.metadata
            );
        self.orders.add(order_req.order_id, new_order_with_state(order, true));
        if (order_req.client_order_id.is_some()) {
            self.client_order_ids.add(
                new_account_client_order_id(
                    order_req.account, order_req.client_order_id.destroy_some()
                ),
                order_req.order_id
            );
        };
        price_time_idx.place_maker_order(
            order_req.order_id,
            single_order_book_type(),
            order_req.price,
            ascending_idx,
            order_req.remaining_size,
            order_req.is_bid
        );
    }


    /// Reinserts a maker order to the order book. This is used when the order is removed from the order book
    /// but the clearinghouse fails to settle all or part of the order. If the order doesn't exist in the order book,
    /// it is added to the order book, if it exists, it's size is updated.
    public(friend) fun reinsert_order<M: store + copy + drop>(
        self: &mut SingleOrderBook<M>,
        price_time_idx: &mut PriceTimeIndex,
        reinsert_order: OrderMatchDetails<M>,
        original_order: &OrderMatchDetails<M>,
    ) {
        assert!(reinsert_order.validate_reinsertion_request(original_order), E_REINSERT_ORDER_MISMATCH);
        let order_id = reinsert_order.get_order_id_from_match_details();
        let unique_idx = reinsert_order.get_unique_priority_idx_from_match_details();
        if (!self.orders.contains(&order_id)) {
            return self.place_ready_maker_order_with_unique_idx(price_time_idx, new_order_request_from_match_details(reinsert_order), unique_idx);
        };

        modify_order(&mut self.orders, &order_id, |order_with_state| {
            order_with_state.increase_remaining_size(reinsert_order.get_remaining_size_from_match_details());
        });
        price_time_idx.increase_order_size(
            reinsert_order.get_price_from_match_details(),
            unique_idx,
            reinsert_order.get_remaining_size_from_match_details(),
            reinsert_order.is_bid_from_match_details(),
        );
    }

    // TODO move to big_ordered_map.move after function values are enabled in mainnet
    inline fun modify_order<M: store + copy + drop>(
        orders: &mut BigOrderedMap<OrderIdType, OrderWithState<M>>, order_id: &OrderIdType, modify_fn: |&mut  OrderWithState<M>|
    ) {
        let order = *orders.borrow(order_id);
        modify_fn(&mut order);
        orders.upsert(*order_id, order);
    }

    inline fun modify_and_copy_order<M: store + copy + drop>(
        orders: &mut BigOrderedMap<OrderIdType, OrderWithState<M>>, order_id: &OrderIdType, modify_fn: |&mut  OrderWithState<M>|
    ): OrderWithState<M> {
        let order = *orders.borrow(order_id);
        modify_fn(&mut order);
        orders.upsert(*order_id, order);
        order
    }

    inline fun modify_or_remove_order<M: store + copy + drop>(
        orders: &mut BigOrderedMap<OrderIdType, OrderWithState<M>>, order_id: &OrderIdType, modify_fn: |&mut  OrderWithState<M>| bool
    ): OrderWithState<M> {
        let order = orders.remove(order_id);
        let keep = modify_fn(&mut order);
        if (keep) {
            orders.add(*order_id, order);
        };
        order
    }


    fun place_pending_maker_order<M: store + copy + drop>(
        self: &mut SingleOrderBook<M>, ascending_id_generator: &mut AscendingIdGenerator, order_req: SingleOrderRequest<M>
    ) {
        let order_id = order_req.order_id;
        let ascending_idx =
            new_unique_idx_type(ascending_id_generator.next_ascending_id());
        let order =
            new_single_order(
                order_id,
                order_req.account,
                ascending_idx,
                order_req.client_order_id,
                order_req.price,
                order_req.orig_size,
                order_req.remaining_size,
                order_req.is_bid,
                order_req.trigger_condition,
                order_req.time_in_force,
                order_req.metadata
            );

        self.orders.add(order_id, new_order_with_state(order, false));

        self.pending_orders.place_pending_maker_order(
            order_id,
            order_req.trigger_condition.destroy_some(),
            ascending_idx,
        );
    }

    /// Returns a single match for a taker order. It is responsibility of the caller to first call the `is_taker_order`
    /// API to ensure that the order is a taker order before calling this API, otherwise it will abort.
    public(friend) fun get_single_match_for_taker<M: store + copy + drop>(
        self: &mut SingleOrderBook<M>,
        active_matched_order: ActiveMatchedOrder,
    ): OrderMatch<M> {
        let (order_id, matched_size, remaining_size, order_book_type) =
            active_matched_order.destroy_active_matched_order();
        assert!(order_book_type == single_order_book_type(), ENOT_SINGLE_ORDER_BOOK);

        let order_with_state = modify_or_remove_order(&mut self.orders, &order_id, |order_with_state| {
            order_with_state.set_remaining_size(remaining_size);
            remaining_size > 0
        });

        let (order, is_active) = order_with_state.destroy_order_from_state();
        if (remaining_size == 0 && order.get_client_order_id().is_some()) {
            self.client_order_ids.remove(
                &new_account_client_order_id(
                    order.get_account(), order.get_client_order_id().destroy_some()
                )
            );
        };
        let (
            account,
            order_id,
            client_order_id,
            unique_priority_idx,
            price,
            orig_size,
            size,
            is_bid,
            _trigger_condition,
            time_in_force,
            metadata
        ) = order.destroy_single_order();
        assert!(is_active, EINVALID_INACTIVE_ORDER_STATE);
        new_order_match(new_order_match_details(order_id, account, client_order_id, unique_priority_idx, price, orig_size, size, is_bid, time_in_force, metadata, single_order_book_type()), matched_size)
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
        order_id: OrderIdType,
        size_delta: u64
    ) {
        assert!(self.orders.contains(&order_id), EORDER_NOT_FOUND);

        let order_with_state = modify_and_copy_order(&mut self.orders, &order_id, |order_with_state| {
            assert!(
                order_creator == order_with_state.get_order_from_state().get_account(),
                EORDER_CREATOR_MISMATCH
            );
            order_with_state.decrease_remaining_size(size_delta);

            // TODO should we be asserting that remaining size is greater than 0?
        });

        if (order_with_state.is_active_order()) {
            let order = order_with_state.get_order_from_state();
            price_time_idx
                .decrease_order_size(
                order.get_price(),
                order_with_state.get_unique_priority_idx_from_state(),
                size_delta,
                order.is_bid()
            );
        };
    }

    public(friend) fun get_order_id_by_client_id<M: store + copy + drop>(
        self: &SingleOrderBook<M>, order_creator: address, client_order_id: u64
    ): Option<OrderIdType> {
        let account_client_order_id =
            new_account_client_order_id(order_creator, client_order_id);
        if (!self.client_order_ids.contains(&account_client_order_id)) {
            return option::none();
        };
        option::some(*self.client_order_ids.borrow(&account_client_order_id))
    }

    public(friend) fun get_order_metadata<M: store + copy + drop>(
        self: &SingleOrderBook<M>, order_id: OrderIdType
    ): Option<M> {
        if (!self.orders.contains(&order_id)) {
            return option::none();
        };
        option::some(self.orders.borrow(&order_id).get_metadata_from_state())
    }

    public(friend) fun set_order_metadata<M: store + copy + drop>(
        self: &mut SingleOrderBook<M>, order_id: OrderIdType, metadata: M
    ) {
        assert!(self.orders.contains(&order_id), EORDER_NOT_FOUND);

        modify_order(&mut self.orders, &order_id, |order_with_state| {
            order_with_state.set_metadata_in_state(metadata);
        });
    }

    public(friend) fun is_active_order<M: store + copy + drop>(
        self: &SingleOrderBook<M>, order_id: OrderIdType
    ): bool {
        if (!self.orders.contains(&order_id)) {
            return false;
        };
        self.orders.borrow(&order_id).is_active_order()
    }

    public(friend) fun get_order<M: store + copy + drop>(
        self: &SingleOrderBook<M>, order_id: OrderIdType
    ): Option<OrderWithState<M>> {
        if (!self.orders.contains(&order_id)) {
            return option::none();
        };
        option::some(*self.orders.borrow(&order_id))
    }

    public(friend) fun get_remaining_size<M: store + copy + drop>(
        self: &SingleOrderBook<M>, order_id: OrderIdType
    ): u64 {
        if (!self.orders.contains(&order_id)) {
            return 0;
        };
        self.orders.borrow(&order_id).get_remaining_size_from_state()
    }

    /// Removes and returns the orders that are ready to be executed based on the current price.
    public(friend) fun take_ready_price_based_orders<M: store + copy + drop>(
        self: &mut SingleOrderBook<M>, current_price: u64, order_limit: u64
    ): vector<SingleOrder<M>> {
        let self_orders = &mut self.orders;
        let order_ids = self.pending_orders.take_ready_price_based_orders(current_price, order_limit);
        let orders = vector::empty();

        order_ids.for_each(|order_id| {
            let order_with_state = self_orders.remove(&order_id);
            let (order, _) = order_with_state.destroy_order_from_state();
            orders.push_back(order);
        });
        orders
    }

    /// Removes and returns the orders that are ready to be executed based on the time condition.
    public(friend) fun take_ready_time_based_orders<M: store + copy + drop>(
        self: &mut SingleOrderBook<M>, order_limit: u64
    ): vector<SingleOrder<M>> {
        let self_orders = &mut self.orders;
        let order_ids = self.pending_orders.take_time_time_based_orders(order_limit);
        let orders = vector::empty();

        order_ids.for_each(|order_id| {
            let order_with_state = self_orders.remove(&order_id);
            let (order, _) = order_with_state.destroy_order_from_state();
            orders.push_back(order);
        });
        orders
    }

    // ============================= test_only APIs ====================================

    #[test_only]
    public(friend) fun destroy_single_order_book<M: store + copy + drop>(self: SingleOrderBook<M>) {
        let SingleOrderBook::V1 {
            orders,
            client_order_ids,
            pending_orders
        } = self;
        orders.destroy(|_v| {});
        client_order_ids.destroy(|_v| {});
        pending_orders.destroy_pending_order_book_index();
    }

    #[test_only]
    public(friend) fun get_unique_priority_idx<M: store + copy + drop>(
        self: &SingleOrderBook<M>, order_id: OrderIdType
    ): Option<UniqueIdxType> {
        if (!self.orders.contains(&order_id)) {
            return option::none();
        };
        option::some(self.orders.borrow(&order_id).get_unique_priority_idx_from_state())
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
        self: &mut SingleOrderBook<M>, price_time_idx: &mut PriceTimeIndex, ascending_id_generator: &mut AscendingIdGenerator, order_req: SingleOrderRequest<M>
    ): vector<OrderMatch<M>> {
        let match_results = vector::empty();
        let remaining_size = order_req.remaining_size;
        while (remaining_size > 0) {
            if (!is_taker_order(
                price_time_idx, order_req.price, order_req.is_bid, order_req.trigger_condition
            )) {
                self.place_maker_order(
                    price_time_idx,
                    ascending_id_generator,
                    SingleOrderRequest::V1 {
                        account: order_req.account,
                        order_id: order_req.order_id,
                        client_order_id: order_req.client_order_id,
                        price: order_req.price,
                        orig_size: order_req.orig_size,
                        remaining_size,
                        is_bid: order_req.is_bid,
                        trigger_condition: order_req.trigger_condition,
                        time_in_force: order_req.time_in_force,
                        metadata: order_req.metadata
                    }
                );
                return match_results;
            };
            let result = price_time_idx.get_single_match_result( order_req.price, remaining_size, order_req.is_bid);
            let match_result =
                self.get_single_match_for_taker(result);
            let matched_size = match_result.get_matched_size();
            match_results.push_back(match_result);
            remaining_size -= matched_size;
        };
        return match_results
    }

    #[test_only]
    public(friend) fun update_order_and_get_matches<M: store + copy + drop>(
        self: &mut SingleOrderBook<M>, price_time_idx: &mut PriceTimeIndex, ascending_id_generator: &mut AscendingIdGenerator, order_req: SingleOrderRequest<M>
    ): vector<OrderMatch<M>> {
        let unique_priority_idx = self.get_unique_priority_idx(order_req.order_id);
        assert!(unique_priority_idx.is_some(), EORDER_NOT_FOUND);
        self.cancel_order(price_time_idx, order_req.account, order_req.order_id);
        let order_req = SingleOrderRequest::V1 {
            account: order_req.account,
            order_id: order_req.order_id,
            client_order_id: order_req.client_order_id,
            price: order_req.price,
            orig_size: order_req.orig_size,
            remaining_size: order_req.remaining_size,
            is_bid: order_req.is_bid,
            trigger_condition: order_req.trigger_condition,
            time_in_force: order_req.time_in_force,
            metadata: order_req.metadata
        };
        self.place_order_and_get_matches(price_time_idx, ascending_id_generator, order_req)
    }

    #[test_only]
    public(friend) fun trigger_pending_orders<M: store + copy + drop>(
        self: &mut SingleOrderBook<M>, price_time_idx: &mut PriceTimeIndex, ascending_id_generator: &mut AscendingIdGenerator, oracle_price: u64
    ): vector<OrderMatch<M>> {
        let ready_orders = self.take_ready_price_based_orders(oracle_price, 1000);
        let all_matches = vector::empty();
        let i = 0;
        while (i < ready_orders.length()) {
            let order = ready_orders[i];
            let (
                account,
                order_id,
                client_order_id,
                _unique_priority_idx,
                price,
                orig_size,
                remaining_size,
                is_bid,
                _,
                time_in_force,
                metadata
            ) = order.destroy_single_order();
            let order_req = SingleOrderRequest::V1 {
                account,
                order_id,
                client_order_id,
                price,
                orig_size,
                remaining_size,
                is_bid,
                trigger_condition: option::none(),
                time_in_force,
                metadata
            };
            let match_results = self.place_order_and_get_matches(price_time_idx, ascending_id_generator, order_req);
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

    struct TestMetadata has store, copy, drop {}

    #[test_only]
    public fun new_test_metadata(): TestMetadata {
        TestMetadata {}
    }

    #[test_only]
    public fun set_up_test(): (SingleOrderBook<TestMetadata>, PriceTimeIndex, AscendingIdGenerator) {
        let order_book = new_single_order_book<TestMetadata>();
        let price_time_idx = new_price_time_idx();
        let ascending_id_generator = new_ascending_id_generator();
        (order_book, price_time_idx, ascending_id_generator)
    }

    #[test_only]
    public fun set_up_test_with_id(): (SingleOrderBook<u64>, PriceTimeIndex, AscendingIdGenerator) {
        let order_book = new_single_order_book<u64>();
        let price_time_idx = new_price_time_idx();
        let ascending_id_generator = new_ascending_id_generator();
        (order_book, price_time_idx, ascending_id_generator)
    }

    // ============================= Test Helper Functions ====================================

    #[test_only]
    public fun create_test_order_request<M: store + copy + drop>(
        account: address,
        order_id: OrderIdType,
        client_order_id: Option<u64>,
        price: u64,
        orig_size: u64,
        remaining_size: u64,
        is_bid: bool,
        trigger_condition: Option<TriggerCondition>,
        metadata: M
    ): SingleOrderRequest<M> {
        SingleOrderRequest::V1 {
            account,
            order_id,
            client_order_id,
            price,
            orig_size,
            remaining_size,
            is_bid,
            trigger_condition,
            time_in_force: good_till_cancelled(),
            metadata
        }
    }

    #[test_only]
    public fun create_simple_test_order_request<M: store + copy + drop>(
        account: address,
        order_id: OrderIdType,
        price: u64,
        size: u64,
        is_bid: bool,
        metadata: M
    ): SingleOrderRequest<M> {
        create_test_order_request(
            account,
            order_id,
            option::none(),
            price,
            size,
            size,
            is_bid,
            option::none(),
            metadata
        )
    }

    #[test_only]
    public fun create_test_order_request_with_client_id<M: store + copy + drop>(
        account: address,
        order_id: OrderIdType,
        client_order_id: u64,
        price: u64,
        size: u64,
        is_bid: bool,
        metadata: M
    ): SingleOrderRequest<M> {
        create_test_order_request(
            account,
            order_id,
            option::some(client_order_id),
            price,
            size,
            size,
            is_bid,
            option::none(),
            metadata
        )
    }

    #[test_only]
    public fun verify_order_state<M: store + copy + drop>(
        order_book: &SingleOrderBook<M>,
        order_id: OrderIdType,
        expected_account: address,
        expected_price: u64,
        expected_orig_size: u64,
        expected_remaining_size: u64,
        expected_is_bid: bool,
        expected_client_order_id: Option<u64>
    ) {
        let order_state = *order_book.orders.borrow(&order_id);
        let (order, is_active) = order_state.destroy_order_from_state();
        let (account, _order_id, client_order_id, _, price, orig_size, size, is_bid,_, _, _) =
            order.destroy_single_order();
        assert!(is_active == true);
        assert!(account == expected_account);
        assert!(price == expected_price);
        assert!(orig_size == expected_orig_size);
        assert!(size == expected_remaining_size);
        assert!(is_bid == expected_is_bid);
        assert!(client_order_id == expected_client_order_id);
    }

    #[test_only]
    public fun verify_match_result<M: store + copy + drop>(
        match_results: &vector<OrderMatch<M>>,
        expected_matched_size: u64,
        expected_maker_account: address,
        expected_maker_order_id: OrderIdType,
        expected_maker_matched_size: u64,
        expected_maker_orig_size: u64,
        expected_maker_remaining_size: u64
    ) {
        assert!(total_matched_size(match_results) == expected_matched_size);
        assert!(match_results.length() == 1);
        let maker_match = match_results[0];
        let (match_details, matched_size) = maker_match.destroy_order_match();
        assert!(match_details.get_account_from_match_details() == expected_maker_account);
        assert!(match_details.get_order_id_from_match_details() == expected_maker_order_id);
        assert!(matched_size == expected_maker_matched_size);
        assert!(match_details.get_orig_size_from_match_details() == expected_maker_orig_size);
        assert!(match_details.get_remaining_size_from_match_details() == expected_maker_remaining_size);
    }

    #[test_only]
    public fun cleanup_test<M: store + copy + drop>(
        order_book: SingleOrderBook<M>,
        price_time_idx: PriceTimeIndex
    ) {
        order_book.destroy_single_order_book();
        price_time_idx.destroy_price_time_idx();
    }

    // ============================= Tests ====================================

    #[test]
    fun test_good_til_cancelled_order() {
        let (order_book, price_time_idx, ascending_id_generator) = set_up_test();

        // Place a GTC sell order
        let order_req = create_test_order_request_with_client_id(
            @0xAA,
            new_order_id_type(1),
            1,
            100,
            1000,
            false,
            TestMetadata {}
        );
        let match_results = order_book.place_order_and_get_matches(&mut price_time_idx, &mut ascending_id_generator, order_req);
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
            option::some(1)
        );

        // Place a matching buy order for partial fill
        let match_results = order_book.place_order_and_get_matches(
            &mut price_time_idx,
            &mut ascending_id_generator,
            create_test_order_request_with_client_id(
                @0xBB,
                new_order_id_type(1),
                2,
                100,
                400,
                true,
                TestMetadata {}
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
            option::some(1)
        );

        // Cancel the remaining order
        order_book.cancel_order(&mut price_time_idx, @0xAA, new_order_id_type(1));

        // Verify order no longer exists
        assert!(order_book.get_remaining_size(new_order_id_type(1)) == 0);

        cleanup_test(order_book, price_time_idx);
    }

    #[test]
    fun test_update_buy_order() {
        let (order_book, price_time_idx, ascending_id_generator) = set_up_test();

        // Place a GTC sell order
        let match_results = order_book.place_order_and_get_matches(
            &mut price_time_idx,
            &mut ascending_id_generator,
            create_simple_test_order_request(
                @0xAA,
                new_order_id_type(1),
                101,
                1000,
                false,
                TestMetadata {}
            )
        );
        assert!(match_results.is_empty());

        let match_results = order_book.place_order_and_get_matches(
            &mut price_time_idx,
            &mut ascending_id_generator,
            create_simple_test_order_request(
                @0xBB,
                new_order_id_type(2),
                100,
                500,
                true,
                TestMetadata {}
            )
        );
        assert!(match_results.is_empty());

        // Update the order so that it would match immediately
        let match_results = order_book.place_order_and_get_matches(
            &mut price_time_idx,
            &mut ascending_id_generator,
            create_simple_test_order_request(
                @0xBB,
                new_order_id_type(3),
                101,
                500,
                true,
                TestMetadata {}
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
        let (order_book, price_time_idx, ascending_id_generator) = set_up_test();

        // Place a GTC sell order
        let order_req = create_test_order_request_with_client_id(
            @0xAA,
            new_order_id_type(1),
            1,
            100,
            1000,
            false,
            TestMetadata {}
        );
        let match_result = order_book.place_order_and_get_matches(&mut price_time_idx, &mut ascending_id_generator, order_req);
        assert!(match_result.is_empty()); // No matches for first order

        // Place a buy order at lower price
        let match_result = order_book.place_order_and_get_matches(
            &mut price_time_idx,
            &mut ascending_id_generator,
            create_test_order_request_with_client_id(
                @0xBB,
                new_order_id_type(2),
                2,
                99,
                500,
                true,
                TestMetadata {}
            )
        );
        assert!(match_result.is_empty());

        // Update sell order to match with buy order
        let match_results = order_book.update_order_and_get_matches(
            &mut price_time_idx,
            &mut ascending_id_generator,
            create_test_order_request_with_client_id(
                @0xAA,
                new_order_id_type(1),
                3,
                99,
                1000,
                false,
                TestMetadata {}
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
        let (order_book, price_time_idx, ascending_id_generator) = set_up_test();

        // Place a GTC sell order
        let match_result = order_book.place_order_and_get_matches(
            &mut price_time_idx,
            &mut ascending_id_generator,
            create_simple_test_order_request(
                @0xAA,
                new_order_id_type(1),
                101,
                1000,
                false,
                TestMetadata {}
            )
        );
        assert!(match_result.is_empty()); // No matches for first order

        let match_result = order_book.place_order_and_get_matches(
            &mut price_time_idx,
            &mut ascending_id_generator,
            create_simple_test_order_request(
                @0xBB,
                new_order_id_type(2),
                100,
                500,
                true,
                TestMetadata {}
            )
        );
        assert!(match_result.is_empty());

        // Try to update non existant order
        let match_result = order_book.update_order_and_get_matches(
            &mut price_time_idx,
            &mut ascending_id_generator,
            create_simple_test_order_request(
                @0xBB,
                new_order_id_type(3),
                100,
                500,
                true,
                TestMetadata {}
            )
        );
        // This should fail with EORDER_NOT_FOUND
        assert!(match_result.is_empty());
        cleanup_test(order_book, price_time_idx);
    }

    #[test]
    fun test_good_til_cancelled_partial_fill() {
        let (order_book, price_time_idx, ascending_id_generator) = set_up_test();

        // Place a GTC sell order for 1000 units at price 100
        let match_result = order_book.place_order_and_get_matches(
            &mut price_time_idx,
            &mut ascending_id_generator,
            create_simple_test_order_request(
                @0xAA,
                new_order_id_type(1),
                100,
                1000,
                false,
                TestMetadata {}
            )
        );
        assert!(match_result.is_empty()); // No matches for first order

        // Place a smaller buy order (400 units) at the same price
        let match_results = order_book.place_order_and_get_matches(
            &mut price_time_idx,
            &mut ascending_id_generator,
            create_simple_test_order_request(
                @0xBB,
                new_order_id_type(2),
                100,
                400,
                true,
                TestMetadata {}
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
        let match_results = order_book.place_order_and_get_matches(
            &mut price_time_idx,
            &mut ascending_id_generator,
            create_simple_test_order_request(
                @0xBB,
                new_order_id_type(3),
                100,
                300,
                true,
                TestMetadata {}
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
        let (order_book, price_time_idx, ascending_id_generator) = set_up_test();

        // Place a GTC sell order for 500 units at price 100
        let match_result = order_book.place_order_and_get_matches(
            &mut price_time_idx,
            &mut ascending_id_generator,
            create_simple_test_order_request(
                @0xAA,
                new_order_id_type(1),
                100,
                500,
                false,
                TestMetadata {}
            )
        );
        assert!(match_result.is_empty()); // No matches for first order

        // Place a larger buy order (800 units) at the same price
        // Should partially fill against the sell order and remain in book
        let match_results = order_book.place_order_and_get_matches(
            &mut price_time_idx,
            &mut ascending_id_generator,
            create_simple_test_order_request(
                @0xBB,
                new_order_id_type(2),
                100,
                800,
                true,
                TestMetadata {}
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
        let (order_book, price_time_idx, ascending_id_generator) = set_up_test();

        // Place a GTC sell order for 1000 units at price 100
        let match_result = order_book.place_order_and_get_matches(
            &mut price_time_idx,
            &mut ascending_id_generator,
            create_simple_test_order_request(
                @0xAA,
                new_order_id_type(1),
                100,
                1000,
                false,
                TestMetadata {}
            )
        );
        assert!(match_result.is_empty()); // No matches for first order

        assert!(order_book.trigger_pending_orders(&mut price_time_idx, &mut ascending_id_generator, 100).is_empty());

        // Place a smaller buy order (400 units) at the same price
        let match_result = order_book.place_order_and_get_matches(
            &mut price_time_idx,
            &mut ascending_id_generator,
            create_test_order_request(
                @0xBB,
                new_order_id_type(2),
                option::none(),
                100,
                400,
                400,
                true,
                option::some(price_move_down_condition(90)),
                TestMetadata {}
            )
        );
        // Even if the price of 100 can be matched in the order book the trigger condition 90 should not trigger
        // the matching
        assert!(match_result.is_empty());
        assert!(
            order_book.pending_orders.get_price_move_down_index().keys().length() == 1
        );

        // Trigger the pending orders with a price of 90
        let match_results = order_book.trigger_pending_orders(&mut price_time_idx, &mut ascending_id_generator, 90);

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

    #[test]
    fun test_price_move_up_condition() {
        let (order_book, price_time_idx, ascending_id_generator) = set_up_test();

        // Place a GTC sell order for 1000 units at price 100
        let match_result = order_book.place_order_and_get_matches(
            &mut price_time_idx,
            &mut ascending_id_generator,
            create_simple_test_order_request(
                @0xAA,
                new_order_id_type(1),
                100,
                1000,
                true,
                TestMetadata {}
            )
        );
        assert!(match_result.is_empty()); // No matches for first order

        assert!(order_book.trigger_pending_orders(&mut price_time_idx, &mut ascending_id_generator, 100).is_empty());

        // Place a smaller buy order (400 units) at the same price
        let match_result = order_book.place_order_and_get_matches(
            &mut price_time_idx,
            &mut ascending_id_generator,
            create_test_order_request(
                @0xBB,
                new_order_id_type(2),
                option::none(),
                100,
                400,
                400,
                false,
                option::some(price_move_up_condition(110)),
                TestMetadata {}
            )
        );
        // Even if the price of 100 can be matched in the order book the trigger condition 110 should not trigger
        // the matching
        assert!(match_result.is_empty());
        assert!(
            order_book.pending_orders.get_price_move_up_index().keys().length() == 1
        );

        // Trigger the pending orders with a price of 110
        let match_results = order_book.trigger_pending_orders(&mut price_time_idx, &mut ascending_id_generator, 110);
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
        let match_result = order_book.place_order_and_get_matches(
            &mut price_time_idx,
            &mut ascending_id_generator,
            create_test_order_request(
                @0xBB,
                new_order_id_type(3),
                option::none(),
                100,
                300,
                300,
                false,
                option::some(price_move_up_condition(120)),
                TestMetadata {}
            )
        );

        assert!(match_result.is_empty());
        assert!(
            order_book.pending_orders.get_price_move_up_index().keys().length() == 1
        );

        // Oracle price moves down to 100, this should not trigger any order
        let match_results = order_book.trigger_pending_orders(&mut price_time_idx, &mut ascending_id_generator, 100);
        assert!(match_results.is_empty());

        // Move the oracle price up to 120, this should trigger the order
        let match_results = order_book.trigger_pending_orders(&mut price_time_idx, &mut ascending_id_generator, 120);

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
    fun test_maker_order_reinsert_already_exists() {
        let (order_book, price_time_idx, ascending_id_generator) = set_up_test();

        // Place a GTC sell order
        let order_req = create_test_order_request_with_client_id(
            @0xAA,
            new_order_id_type(1),
            1,
            100,
            1000,
            false,
            TestMetadata {}
        );
        order_book.place_maker_order(&mut price_time_idx, &mut ascending_id_generator, order_req);
        assert!(order_book.get_remaining_size(new_order_id_type(1)) == 1000);

        assert!(order_book.client_order_id_exists(@0xAA, 1));

        // Taker order
        let order_req = create_simple_test_order_request(
            @0xBB,
            new_order_id_type(2),
            100,
            100,
            true,
            TestMetadata {}
        );

        let match_results = order_book.place_order_and_get_matches(&mut price_time_idx, &mut ascending_id_generator, order_req);
        assert!(total_matched_size(&match_results) == 100);

        assert!(order_book.client_order_id_exists(@0xAA, 1));

        let (matched_order, _) = match_results[0].destroy_order_match();
        let reinsert_request = matched_order.new_order_match_details_with_modified_size(50);
        // Assume half of the order was matched and remaining 50 size is reinserted back to the order book
        order_book.reinsert_order(&mut price_time_idx, reinsert_request,  &matched_order);
        assert!(order_book.client_order_id_exists(@0xAA, 1));
        // Verify order was reinserted with updated size
        assert!(order_book.get_remaining_size(new_order_id_type(1)) == 950);
        cleanup_test(order_book, price_time_idx);
    }

    #[test]
    fun test_maker_order_reinsert_not_exists() {
        let (order_book, price_time_idx, ascending_id_generator) = set_up_test();

        // Place a GTC sell order
        let order_req = create_test_order_request_with_client_id(
            @0xAA,
            new_order_id_type(1),
            1,
            100,
            1000,
            false,
            TestMetadata {}
        );
        order_book.place_maker_order(&mut price_time_idx, &mut ascending_id_generator, order_req);
        assert!(order_book.get_remaining_size(new_order_id_type(1)) == 1000);

        // Taker order
        let order_req = create_test_order_request_with_client_id(
            @0xBB,
            new_order_id_type(2),
            1,
            100,
            1000,
            true,
            TestMetadata {}
        );

        assert!(order_book.client_order_id_exists(@0xAA, 1));

        let match_results = order_book.place_order_and_get_matches(&mut price_time_idx, &mut ascending_id_generator, order_req);
        assert!(total_matched_size(&match_results) == 1000);

        assert!(!order_book.client_order_id_exists(@0xAA, 1));

        let (matched_order, _) = match_results[0].destroy_order_match();
        let reinsert_request = matched_order.new_order_match_details_with_modified_size(500);

        order_book.reinsert_order(&mut price_time_idx,  reinsert_request, &matched_order);
        assert!(order_book.client_order_id_exists(@0xAA, 1));
        // Verify order was reinserted with updated size
        assert!(order_book.get_remaining_size(new_order_id_type(1)) == 500);
        cleanup_test(order_book, price_time_idx);
    }

    #[test]
    fun test_decrease_order_size() {
        let (order_book, price_time_idx, ascending_id_generator) = set_up_test();

        // Place an active order
        let order_req = create_simple_test_order_request(
            @0xAA,
            new_order_id_type(1),
            100,
            1000,
            false,
            TestMetadata {}
        );
        order_book.place_maker_order(&mut price_time_idx, &mut ascending_id_generator, order_req);
        assert!(order_book.get_remaining_size(new_order_id_type(1)) == 1000);

        order_book.decrease_order_size(&mut price_time_idx, @0xAA, new_order_id_type(1), 700);
        // Verify order was decreased with updated size
        assert!(order_book.get_remaining_size(new_order_id_type(1)) == 300);

        let order_req = create_test_order_request(
            @0xBB,
            new_order_id_type(2),
            option::none(),
            100,
            1000,
            1000,
            false,
            option::some(price_move_up_condition(90)),
            TestMetadata {}
        );
        order_book.place_maker_order(&mut price_time_idx, &mut ascending_id_generator, order_req);
        assert!(order_book.get_remaining_size(new_order_id_type(2)) == 1000);
        order_book.decrease_order_size(&mut price_time_idx, @0xBB, new_order_id_type(2), 600);
        // Verify order was decreased with updated size
        assert!(order_book.get_remaining_size(new_order_id_type(2)) == 400);

        cleanup_test(order_book, price_time_idx);
    }

    #[test]
    fun test_get_and_set_order_metadata() {
        let (order_book, price_time_idx, ascending_id_generator) = set_up_test_with_id();

        // Place an active order
        let order_req = create_simple_test_order_request(
            @0xAA,
            new_order_id_type(1),
            100,
            1000,
            false,
            1
        );
        order_book.place_maker_order(&mut price_time_idx, &mut ascending_id_generator, order_req);
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
