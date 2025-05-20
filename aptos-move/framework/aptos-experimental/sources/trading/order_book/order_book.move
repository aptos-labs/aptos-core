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
module aptos_experimental::order_book {
    use std::vector;
    use std::error;
    use std::option::{Self, Option};
    use aptos_framework::big_ordered_map::BigOrderedMap;

    use aptos_experimental::order_book_types::{
        OrderIdType,
        OrderWithState,
        generate_unique_idx_fifo_tiebraker,
        new_order_id_type,
        new_order,
        new_order_with_state,
        new_single_order_match,
        new_default_big_ordered_map,
        TriggerCondition,
        UniqueIdxType,
        SingleOrderMatch,
        Order
    };
    use aptos_experimental::active_order_book::{ActiveOrderBook, new_active_order_book};
    use aptos_experimental::pending_order_book_index::{
        PendingOrderBookIndex,
        new_pending_order_book_index
    };
    #[test_only]
    use aptos_experimental::order_book_types::tp_trigger_condition;

    const U256_MAX: u256 =
        0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff;

    const EORDER_ALREADY_EXISTS: u64 = 1;
    const EPOST_ONLY_FILLED: u64 = 2;
    const EORDER_NOT_FOUND: u64 = 4;
    const EINVALID_INACTIVE_ORDER_STATE: u64 = 5;
    const EINVALID_ADD_SIZE_TO_ORDER: u64 = 6;
    const E_NOT_ACTIVE_ORDER: u64 = 7;

    struct OrderRequest<M: store + copy + drop> has copy, drop {
        account: address,
        account_order_id: u64,
        unique_priority_idx: Option<UniqueIdxType>,
        price: u64,
        orig_size: u64,
        remaining_size: u64,
        is_buy: bool,
        trigger_condition: Option<TriggerCondition>,
        metadata: M
    }

    enum OrderBook<M: store + copy + drop> has store {
        V1 {
            orders: BigOrderedMap<OrderIdType, OrderWithState<M>>,
            active_orders: ActiveOrderBook,
            pending_orders: PendingOrderBookIndex
        }
    }

    enum OrderType has store, drop, copy {
        GoodTilCancelled,
        PostOnly,
        FillOrKill
    }

    public fun new_order_request<M: store + copy + drop>(
        account: address,
        account_order_id: u64,
        unique_priority_idx: Option<UniqueIdxType>,
        price: u64,
        orig_size: u64,
        remaining_size: u64,
        is_buy: bool,
        trigger_condition: Option<TriggerCondition>,
        metadata: M
    ): OrderRequest<M> {
        OrderRequest {
            account,
            account_order_id,
            unique_priority_idx,
            price,
            orig_size,
            remaining_size,
            is_buy,
            trigger_condition,
            metadata
        }
    }

    public fun new_order_book<M: store + copy + drop>(): OrderBook<M> {
        OrderBook::V1 {
            orders: new_default_big_ordered_map(),
            active_orders: new_active_order_book(),
            pending_orders: new_pending_order_book_index()
        }
    }

    /// Cancels an order from the order book. If the order is active, it is removed from the active order book else
    /// it is removed from the pending order book. The API doesn't abort if the order is not found in the order book -
    /// this is a TODO for now.
    public fun cancel_order<M: store + copy + drop>(
        self: &mut OrderBook<M>, account: address, account_order_id: u64
    ): Option<Order<M>> {
        let order_id = new_order_id_type(account, account_order_id);
        assert!(self.orders.contains(&order_id), EORDER_NOT_FOUND);
        let order_with_state = self.orders.remove(&order_id);
        let (order, is_active) = order_with_state.destroy_order_from_state();
        if (is_active) {
            let (_, unique_priority_idx, bid_price, _orig_size, _size, is_buy, _, _) =
                order.destroy_order();
            self.active_orders.cancel_active_order(bid_price, unique_priority_idx, is_buy);
        } else {
            let (
                _,
                unique_priority_idx,
                _bid_price,
                _orig_size,
                _size,
                is_buy,
                trigger_condition,
                _
            ) = order.destroy_order();
            self.pending_orders.cancel_pending_order(
                trigger_condition.destroy_some(), unique_priority_idx, is_buy
            );
        };
        return option::some(order)
    }

    /// Checks if the order is a taker order i.e., matched immediatedly with the active order book.
    public fun is_taker_order<M: store + copy + drop>(
        self: &OrderBook<M>,
        price: u64,
        is_buy: bool,
        trigger_condition: Option<TriggerCondition>
    ): bool {
        if (trigger_condition.is_some()) {
            return false;
        };
        return self.active_orders.is_taker_order(price, is_buy)
    }

    /// Places a maker order to the order book. If the order is a pending order, it is added to the pending order book
    /// else it is added to the active order book. The API aborts if its not a maker order or if the order already exists
    public fun place_maker_order<M: store + copy + drop>(
        self: &mut OrderBook<M>, order_req: OrderRequest<M>
    ) {
        if (order_req.trigger_condition.is_some()) {
            return self.place_pending_maker_order(order_req);
        };

        let order_id = new_order_id_type(order_req.account, order_req.account_order_id);
        let unique_priority_idx =
            if (order_req.unique_priority_idx.is_some()) {
                order_req.unique_priority_idx.destroy_some()
            } else {
                generate_unique_idx_fifo_tiebraker()
            };

        assert!(
            !self.orders.contains(&order_id),
            error::invalid_argument(EORDER_ALREADY_EXISTS)
        );

        let order =
            new_order(
                order_id,
                unique_priority_idx,
                order_req.price,
                order_req.orig_size,
                order_req.remaining_size,
                order_req.is_buy,
                order_req.trigger_condition,
                order_req.metadata
            );
        self.orders.add(order_id, new_order_with_state(order, true));
        self.active_orders.place_maker_order(
            order_id,
            order_req.price,
            unique_priority_idx,
            order_req.remaining_size,
            order_req.is_buy
        );
    }

    /// Reinserts a maker order to the order book. This is used when the order is removed from the order book
    /// but the clearinghouse fails to settle all or part of the order. If the order doesn't exist in the order book,
    /// it is added to the order book, if it exists, it's size is updated.
    public fun reinsert_maker_order<M: store + copy + drop>(
        self: &mut OrderBook<M>, order_req: OrderRequest<M>
    ) {
        assert!(order_req.trigger_condition.is_none(), E_NOT_ACTIVE_ORDER);
        let order_id = new_order_id_type(order_req.account, order_req.account_order_id);
        if (!self.orders.contains(&order_id)) {
            return self.place_maker_order(order_req);
        };
        let order_with_state = self.orders.remove(&order_id);
        order_with_state.increase_remaining_size(
            order_req.remaining_size
        );
        self.orders.add(order_id, order_with_state);
        self.active_orders.increase_order_size(
            order_req.price,
            order_req.unique_priority_idx.destroy_some(),
            order_req.remaining_size,
            order_req.is_buy
        );
    }

    fun place_pending_maker_order<M: store + copy + drop>(
        self: &mut OrderBook<M>, order_req: OrderRequest<M>
    ) {
        let order_id = new_order_id_type(order_req.account, order_req.account_order_id);
        let unique_priority_idx =
            if (order_req.unique_priority_idx.is_some()) {
                order_req.unique_priority_idx.destroy_some()
            } else {
                generate_unique_idx_fifo_tiebraker()
            };
        let order =
            new_order(
                order_id,
                unique_priority_idx,
                order_req.price,
                order_req.orig_size,
                order_req.remaining_size,
                order_req.is_buy,
                order_req.trigger_condition,
                order_req.metadata
            );

        self.orders.add(order_id, new_order_with_state(order, false));

        self.pending_orders.place_pending_maker_order(
            order_id,
            order_req.trigger_condition.destroy_some(),
            unique_priority_idx,
            order_req.is_buy
        );
    }

    /// Returns a single match for a taker order. It is responsibility of the caller to first call the `is_taker_order`
    /// API to ensure that the order is a taker order before calling this API, otherwise it will abort.
    public fun get_single_match_for_taker<M: store + copy + drop>(
        self: &mut OrderBook<M>,
        price: u64,
        size: u64,
        is_buy: bool
    ): SingleOrderMatch<M> {
        let result = self.active_orders.get_single_match_result(price, size, is_buy);
        let (order_id, matched_size, remaining_size) =
            result.destroy_active_matched_order();
        let order_with_state = self.orders.remove(&order_id);
        order_with_state.set_remaining_size(remaining_size);
        if (remaining_size > 0) {
            self.orders.add(order_id, order_with_state);
        };
        let (order, is_active) = order_with_state.destroy_order_from_state();
        assert!(is_active, EINVALID_INACTIVE_ORDER_STATE);
        new_single_order_match(order, matched_size)
    }

    /// Decrease the size of the order by the given size delta. The API aborts if the order is not found in the order book or
    /// if the size delta is greater than or equal to the remaining size of the order. Please note that the API will abort and
    /// not cancel the order if the size delta is equal to the remaining size of the order, to avoid unintended
    /// cancellation of the order. Please use the `cancel_order` API to cancel the order.
    public fun decrease_order_size<M: store + copy + drop>(
        self: &mut OrderBook<M>, account: address, account_order_id: u64, size_delta: u64
    ) {
        let order_id = new_order_id_type(account, account_order_id);
        assert!(self.orders.contains(&order_id), EORDER_NOT_FOUND);
        let order_with_state = self.orders.remove(&order_id);
        order_with_state.decrease_remaining_size(size_delta);
        if (order_with_state.is_active_order()) {
            let order = order_with_state.get_order_from_state();
            self.active_orders.decrease_order_size(
                order.get_price(),
                order_with_state.get_unique_priority_idx_from_state(),
                size_delta,
                order.is_buy()
            );
        };
        self.orders.add(order_id, order_with_state);
    }

    public fun is_active_order<M: store + copy + drop>(
        self: &OrderBook<M>, account: address, account_order_id: u64
    ): bool {
        let order_id = new_order_id_type(account, account_order_id);
        if (!self.orders.contains(&order_id)) {
            return false;
        };
        self.orders.borrow(&order_id).is_active_order()
    }

    public fun get_order<M: store + copy + drop>(
        self: &OrderBook<M>, account: address, account_order_id: u64
    ): Option<OrderWithState<M>> {
        let order_id = new_order_id_type(account, account_order_id);
        if (!self.orders.contains(&order_id)) {
            return option::none();
        };
        option::some(*self.orders.borrow(&order_id))
    }

    public fun get_remaining_size<M: store + copy + drop>(
        self: &OrderBook<M>, account: address, account_order_id: u64
    ): u64 {
        let order_id = new_order_id_type(account, account_order_id);
        if (!self.orders.contains(&order_id)) {
            return 0;
        };
        self.orders.borrow(&order_id).get_remaining_size_from_state()
    }

    /// Removes and returns the orders that are ready to be executed based on the current price.
    public fun take_ready_price_based_orders<M: store + copy + drop>(
        self: &mut OrderBook<M>, current_price: u64
    ): vector<Order<M>> {
        let self_orders = &mut self.orders;
        let order_ids = self.pending_orders.take_ready_price_based_orders(current_price);
        let orders = vector::empty();

        order_ids.for_each(|order_id| {
            let order_with_state = self_orders.remove(&order_id);
            let (order, _) = order_with_state.destroy_order_from_state();
            orders.push_back(order);
        });
        orders
    }

    public fun best_bid_price<M: store + copy + drop>(self: &OrderBook<M>): Option<u64> {
        self.active_orders.best_bid_price()
    }

    public fun best_ask_price<M: store + copy + drop>(self: &OrderBook<M>): Option<u64> {
        self.active_orders.best_ask_price()
    }

    public fun get_slippage_price<M: store + copy + drop>(
        self: &OrderBook<M>, is_buy: bool, slippage_pct: u64
    ): Option<u64> {
        self.active_orders.get_slippage_price(is_buy, slippage_pct)
    }

    /// Removes and returns the orders that are ready to be executed based on the time condition.
    public fun take_ready_time_based_orders<M: store + copy + drop>(
        self: &mut OrderBook<M>
    ): vector<Order<M>> {
        let self_orders = &mut self.orders;
        let order_ids = self.pending_orders.take_time_time_based_orders();
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
    public fun destroy_order_book<M: store + copy + drop>(self: OrderBook<M>) {
        let OrderBook::V1 { orders, active_orders, pending_orders } = self;
        orders.destroy(|_v| {});
        active_orders.destroy_active_order_book();
        pending_orders.destroy_pending_order_book_index();
    }

    #[test_only]
    public fun get_unique_priority_idx<M: store + copy + drop>(
        self: &OrderBook<M>, account: address, account_order_id: u64
    ): Option<UniqueIdxType> {
        let order_id = new_order_id_type(account, account_order_id);
        if (!self.orders.contains(&order_id)) {
            return option::none();
        };
        option::some(self.orders.borrow(&order_id).get_unique_priority_idx_from_state())
    }

    public fun place_order_and_get_matches<M: store + copy + drop>(
        self: &mut OrderBook<M>, order_req: OrderRequest<M>
    ): vector<SingleOrderMatch<M>> {
        let match_results = vector::empty();
        let remainig_size = order_req.remaining_size;
        while (remainig_size > 0) {
            if (!self.is_taker_order(order_req.price, order_req.is_buy, order_req.trigger_condition)) {
                self.place_maker_order(
                    OrderRequest {
                        account: order_req.account,
                        account_order_id: order_req.account_order_id,
                        unique_priority_idx: option::none(),
                        price: order_req.price,
                        orig_size: order_req.orig_size,
                        remaining_size: remainig_size,
                        is_buy: order_req.is_buy,
                        trigger_condition: order_req.trigger_condition,
                        metadata: order_req.metadata
                    }
                );
                return match_results;
            };
            let match_result =
                self.get_single_match_for_taker(
                    order_req.price, remainig_size, order_req.is_buy
                );
            let matched_size = match_result.get_matched_size();
            match_results.push_back(match_result);
            remainig_size -= matched_size;
        };
        return match_results
    }

    #[test_only]
    public fun update_order_and_get_matches<M: store + copy + drop>(
        self: &mut OrderBook<M>, order_req: OrderRequest<M>
    ): vector<SingleOrderMatch<M>> {
        let unique_priority_idx =
            self.get_unique_priority_idx(order_req.account, order_req.account_order_id);
        assert!(unique_priority_idx.is_some(), EORDER_NOT_FOUND);
        let unique_priority_idx = unique_priority_idx.destroy_some();
        self.cancel_order(order_req.account, order_req.account_order_id);
        let order_req = OrderRequest {
            account: order_req.account,
            account_order_id: order_req.account_order_id,
            unique_priority_idx: option::some(unique_priority_idx),
            price: order_req.price,
            orig_size: order_req.orig_size,
            remaining_size: order_req.remaining_size,
            is_buy: order_req.is_buy,
            trigger_condition: order_req.trigger_condition,
            metadata: order_req.metadata
        };
        self.place_order_and_get_matches(order_req)
    }

    #[test_only]
    public fun trigger_pending_orders<M: store + copy + drop>(
        self: &mut OrderBook<M>, oracle_price: u64
    ): vector<SingleOrderMatch<M>> {
        let ready_orders = self.take_ready_price_based_orders(oracle_price);
        let all_matches = vector::empty();
        let i = 0;
        while (i < ready_orders.length()) {
            let order = ready_orders[i];
            let (
                order_id,
                unique_priority_idx,
                price,
                orig_size,
                remaining_size,
                is_buy,
                _,
                metadata
            ) = order.destroy_order();
            let (account, account_order_id) = order_id.destroy_order_id_type();
            let order_req = OrderRequest {
                account,
                account_order_id,
                unique_priority_idx: option::some(unique_priority_idx),
                price,
                orig_size,
                remaining_size,
                is_buy,
                trigger_condition: option::none(),
                metadata
            };
            let match_results = self.place_order_and_get_matches(order_req);
            all_matches.append(match_results);
            i = i + 1;
        };
        all_matches
    }

    #[test_only]
    public fun total_matched_size<M: store + copy + drop>(
        match_results: &vector<SingleOrderMatch<M>>
    ): u64 {
        let total_matched_size = 0;
        let i = 0;
        while (i < match_results.length()) {
            total_matched_size = total_matched_size
                + match_results[i].get_matched_size();
            i = i + 1;
        };
        total_matched_size
    }

    struct TestMetadata has store, copy, drop {}

    // ============================= Tests ====================================

    #[test]
    fun test_good_til_cancelled_order() {
        let order_book = new_order_book<TestMetadata>();

        // Place a GTC sell order
        let order_req = OrderRequest {
            account: @0xAA,
            account_order_id: 1,
            unique_priority_idx: option::none(),
            price: 100,
            orig_size: 1000,
            remaining_size: 1000,
            is_buy: false,
            trigger_condition: option::none(),
            metadata: TestMetadata {}
        };
        let match_results = order_book.place_order_and_get_matches(order_req);
        assert!(match_results.is_empty()); // No matches for first order

        // Verify order exists and is active
        let order_id = new_order_id_type(@0xAA, 1);
        let order_state = *order_book.orders.borrow(&order_id);
        let (order, is_active) = order_state.destroy_order_from_state();
        let (_order_id, _unique_priority_idx, price, orig_size, size, is_buy, _, _) =
            order.destroy_order();
        assert!(is_active == true);
        assert!(price == 100);
        assert!(orig_size == 1000);
        assert!(size == 1000);
        assert!(is_buy == false);

        // Place a matching buy order for partial fill
        let match_results =
            order_book.place_order_and_get_matches(
                OrderRequest {
                    account: @0xBB,
                    account_order_id: 1,
                    unique_priority_idx: option::none(),
                    price: 100,
                    orig_size: 400,
                    remaining_size: 400,
                    is_buy: true,
                    trigger_condition: option::none(),
                    metadata: TestMetadata {}
                }
            );
        // // Verify taker match details
        assert!(total_matched_size(&match_results) == 400);
        assert!(order_book.get_remaining_size(@0xBB, 1) == 0);

        // Verify maker match details
        assert!(match_results.length() == 1); // One match result
        let maker_match = match_results[0];
        let (order, matched_size) = maker_match.destroy_single_order_match();
        assert!(order.get_order_id() == new_order_id_type(@0xAA, 1));
        assert!(matched_size == 400);
        assert!(order.get_orig_size() == 1000);
        assert!(order.get_remaining_size() == 600); // Maker order partially filled

        // Verify original order still exists but with reduced size
        let order_state = *order_book.orders.borrow(&order_id);
        let (order, is_active) = order_state.destroy_order_from_state();
        let (_, _unique_priority_idx, price, orig_size, size, is_buy, _, _) =
            order.destroy_order();
        assert!(is_active == true);
        assert!(price == 100);
        assert!(orig_size == 1000);
        assert!(size == 600);
        assert!(is_buy == false);

        // Cancel the remaining order
        order_book.cancel_order(@0xAA, 1);

        // Verify order no longer exists
        assert!(order_book.get_remaining_size(@0xAA, 1) == 0);

        // Since we cannot drop the order book, we move it to a test struct
        order_book.destroy_order_book();
    }

    #[test]
    fun test_update_buy_order() {
        let order_book = new_order_book<TestMetadata>();

        // Place a GTC sell order
        let match_results =
            order_book.place_order_and_get_matches(
                OrderRequest {
                    account: @0xAA,
                    account_order_id: 1,
                    unique_priority_idx: option::none(),
                    price: 101,
                    orig_size: 1000,
                    remaining_size: 1000,
                    is_buy: false,
                    trigger_condition: option::none(),
                    metadata: TestMetadata {}
                }
            );
        assert!(match_results.is_empty());

        let match_results =
            order_book.place_order_and_get_matches(
                OrderRequest {
                    account: @0xBB,
                    account_order_id: 1,
                    unique_priority_idx: option::none(),
                    price: 100,
                    orig_size: 500,
                    remaining_size: 500,
                    is_buy: true,
                    trigger_condition: option::none(),
                    metadata: TestMetadata {}
                }
            );
        assert!(match_results.is_empty());

        // Update the order so that it would match immediately
        let match_results =
            order_book.place_order_and_get_matches(
                OrderRequest {
                    account: @0xBB,
                    account_order_id: 2,
                    unique_priority_idx: option::none(),
                    price: 101,
                    orig_size: 500,
                    remaining_size: 500,
                    is_buy: true,
                    trigger_condition: option::none(),
                    metadata: TestMetadata {}
                }
            );

        // Verify taker (buy order) was fully filled
        assert!(total_matched_size(&match_results) == 500);
        assert!(order_book.get_remaining_size(@0xBB, 2) == 0);

        assert!(match_results.length() == 1);
        let maker_match = match_results[0];
        let (order, matched_size) = maker_match.destroy_single_order_match();
        assert!(order.get_order_id() == new_order_id_type(@0xAA, 1));
        assert!(matched_size == 500);
        assert!(order.get_orig_size() == 1000);
        assert!(order.get_remaining_size() == 500); // Partial fill

        order_book.destroy_order_book();
    }

    #[test]
    fun test_update_sell_order() {
        let order_book = new_order_book<TestMetadata>();

        // Place a GTC sell order
        let order_req = OrderRequest {
            account: @0xAA,
            account_order_id: 1,
            unique_priority_idx: option::none(),
            price: 100,
            orig_size: 1000,
            remaining_size: 1000,
            is_buy: false,
            trigger_condition: option::none(),
            metadata: TestMetadata {}
        };
        let match_result = order_book.place_order_and_get_matches(order_req);
        assert!(match_result.is_empty()); // No matches for first order

        // Place a buy order at lower price
        let match_result =
            order_book.place_order_and_get_matches(
                OrderRequest {
                    account: @0xBB,
                    account_order_id: 1,
                    unique_priority_idx: option::none(),
                    price: 99,
                    orig_size: 500,
                    remaining_size: 500,
                    is_buy: true,
                    trigger_condition: option::none(),
                    metadata: TestMetadata {}
                }
            );
        assert!(match_result.is_empty());

        // Update sell order to match with buy order
        let match_results =
            order_book.update_order_and_get_matches(
                OrderRequest {
                    account: @0xAA,
                    account_order_id: 1,
                    unique_priority_idx: option::none(),
                    price: 99,
                    orig_size: 1000,
                    remaining_size: 1000,
                    is_buy: false,
                    trigger_condition: option::none(),
                    metadata: TestMetadata {}
                }
            );

        // Verify taker (sell order) was partially filled
        assert!(total_matched_size(&match_results) == 500);

        assert!(match_results.length() == 1); // One match result
        let maker_match = match_results[0];
        let (order, matched_size) = maker_match.destroy_single_order_match();
        assert!(order.get_order_id() == new_order_id_type(@0xBB, 1));
        assert!(matched_size == 500);
        assert!(order.get_orig_size() == 500);
        assert!(order.get_remaining_size() == 0); // Fully filled

        order_book.destroy_order_book();
    }

    #[test]
    #[expected_failure(abort_code = EORDER_NOT_FOUND)]
    fun test_update_order_not_found() {
        let order_book = new_order_book<TestMetadata>();

        // Place a GTC sell order
        let match_result =
            order_book.place_order_and_get_matches(
                OrderRequest {
                    account: @0xAA,
                    account_order_id: 1,
                    unique_priority_idx: option::none(),
                    price: 101,
                    orig_size: 1000,
                    remaining_size: 1000,
                    is_buy: false,
                    trigger_condition: option::none(),
                    metadata: TestMetadata {}
                }
            );
        assert!(match_result.is_empty()); // No matches for first order

        let match_result =
            order_book.place_order_and_get_matches(
                OrderRequest {
                    account: @0xBB,
                    account_order_id: 1,
                    unique_priority_idx: option::none(),
                    price: 100,
                    orig_size: 500,
                    remaining_size: 500,
                    is_buy: true,
                    trigger_condition: option::none(),
                    metadata: TestMetadata {}
                }
            );
        assert!(match_result.is_empty());

        // Try to update non existant order
        let match_result =
            order_book.update_order_and_get_matches(
                OrderRequest {
                    account: @0xBB,
                    account_order_id: 3,
                    unique_priority_idx: option::none(),
                    price: 100,
                    orig_size: 500,
                    remaining_size: 500,
                    is_buy: true,
                    trigger_condition: option::none(),
                    metadata: TestMetadata {}
                }
            );
        // This should fail with EORDER_NOT_FOUND
        assert!(match_result.is_empty());
        order_book.destroy_order_book();
    }

    #[test]
    fun test_good_til_cancelled_partial_fill() {
        let order_book = new_order_book<TestMetadata>();

        // Place a GTC sell order for 1000 units at price 100
        let match_result =
            order_book.place_order_and_get_matches(
                OrderRequest {
                    account: @0xAA,
                    account_order_id: 1,
                    unique_priority_idx: option::none(),
                    price: 100,
                    orig_size: 1000,
                    remaining_size: 1000,
                    is_buy: false,
                    trigger_condition: option::none(),
                    metadata: TestMetadata {}
                }
            );
        assert!(match_result.is_empty()); // No matches for first order

        // Place a smaller buy order (400 units) at the same price
        let match_results =
            order_book.place_order_and_get_matches(
                OrderRequest {
                    account: @0xBB,
                    account_order_id: 1,
                    unique_priority_idx: option::none(),
                    price: 100,
                    orig_size: 400,
                    remaining_size: 400,
                    is_buy: true,
                    trigger_condition: option::none(),
                    metadata: TestMetadata {}
                }
            );

        // Verify taker (buy order) was fully filled
        assert!(total_matched_size(&match_results) == 400);

        // Verify maker (sell order) was partially filled
        assert!(match_results.length() == 1);
        let maker_match = match_results[0];
        let (order, matched_size) = maker_match.destroy_single_order_match();
        assert!(order.get_order_id() == new_order_id_type(@0xAA, 1));
        assert!(matched_size == 400);
        assert!(order.get_orig_size() == 1000);
        assert!(order.get_remaining_size() == 600); // Partial fill

        // Place another buy order for 300 units
        let match_results =
            order_book.place_order_and_get_matches(
                OrderRequest {
                    account: @0xBB,
                    account_order_id: 2,
                    unique_priority_idx: option::none(),
                    price: 100,
                    orig_size: 300,
                    remaining_size: 300,
                    is_buy: true,
                    trigger_condition: option::none(),
                    metadata: TestMetadata {}
                }
            );
        assert!(match_results.length() == 1); // Should match with the sell order

        // Verify second taker was fully filled
        assert!(total_matched_size(&match_results) == 300);

        // Verify original maker was partially filled again
        assert!(match_results.length() == 1);
        let maker_match = match_results[0];
        let (order, matched_size) = maker_match.destroy_single_order_match();
        assert!(order.get_order_id() == new_order_id_type(@0xAA, 1));
        assert!(matched_size == 300);
        assert!(order.get_orig_size() == 1000);
        assert!(order.get_remaining_size() == 300); // Still partial as 300 units remain

        // Original sell order should still exist with 300 units remaining
        let order_id = new_order_id_type(@0xAA, 1);
        let order_state = *order_book.orders.borrow(&order_id);
        let (order, is_active) = order_state.destroy_order_from_state();
        let (_order_id, _unique_priority_idx, price, orig_size, size, is_buy, _, _) =
            order.destroy_order();
        assert!(is_active == true);
        assert!(price == 100);
        assert!(orig_size == 1000);
        assert!(size == 300); // 1000 - 400 - 300 = 300 remaining
        assert!(is_buy == false);

        order_book.destroy_order_book();
    }

    #[test]
    fun test_good_til_cancelled_taker_partial_fill() {
        let order_book = new_order_book<TestMetadata>();

        // Place a GTC sell order for 500 units at price 100
        let match_result =
            order_book.place_order_and_get_matches(
                OrderRequest {
                    account: @0xAA,
                    account_order_id: 1,
                    unique_priority_idx: option::none(),
                    price: 100,
                    orig_size: 500,
                    remaining_size: 500,
                    is_buy: false,
                    trigger_condition: option::none(),
                    metadata: TestMetadata {}
                }
            );
        assert!(match_result.is_empty()); // No matches for first order

        // Place a larger buy order (800 units) at the same price
        // Should partially fill against the sell order and remain in book
        let match_results =
            order_book.place_order_and_get_matches(
                OrderRequest {
                    account: @0xBB,
                    account_order_id: 1,
                    unique_priority_idx: option::none(),
                    price: 100,
                    orig_size: 800,
                    remaining_size: 800,
                    is_buy: true,
                    trigger_condition: option::none(),
                    metadata: TestMetadata {}
                }
            );

        // Verify taker (buy order) was partially filled
        assert!(total_matched_size(&match_results) == 500);

        // Verify maker (sell order) was fully filled
        assert!(match_results.length() == 1);
        let maker_match = match_results[0];
        let (order, matched_size) = maker_match.destroy_single_order_match();
        assert!(order.get_order_id() == new_order_id_type(@0xAA, 1));
        assert!(matched_size == 500);
        assert!(order.get_orig_size() == 500);
        assert!(order.get_remaining_size() == 0); // Fully filled

        // Verify original sell order no longer exists (fully filled)
        let order_id = new_order_id_type(@0xAA, 1);
        assert!(!order_book.orders.contains(&order_id));

        // Verify buy order still exists with remaining size
        let order_id = new_order_id_type(@0xBB, 1);
        let order_state = *order_book.orders.borrow(&order_id);
        let (order, is_active) = order_state.destroy_order_from_state();
        let (_order_id, _unique_priority_idx, price, orig_size, size, is_buy, _, _) =
            order.destroy_order();
        assert!(is_active == true);
        assert!(price == 100);
        assert!(orig_size == 800);
        assert!(size == 300); // 800 - 500 = 300 remaining
        assert!(is_buy == true);

        order_book.destroy_order_book();
    }

    #[test]
    fun test_TP_order() {
        let order_book = new_order_book<TestMetadata>();

        // Place a GTC sell order for 1000 units at price 100
        let match_result =
            order_book.place_order_and_get_matches(
                OrderRequest {
                    account: @0xAA,
                    account_order_id: 1,
                    unique_priority_idx: option::none(),
                    price: 100,
                    orig_size: 1000,
                    remaining_size: 1000,
                    is_buy: false,
                    trigger_condition: option::none(),
                    metadata: TestMetadata {}
                }
            );
        assert!(match_result.is_empty()); // No matches for first order

        assert!(order_book.trigger_pending_orders(100).is_empty());

        // Place a smaller buy order (400 units) at the same price
        let match_result =
            order_book.place_order_and_get_matches(
                OrderRequest {
                    account: @0xBB,
                    account_order_id: 1,
                    unique_priority_idx: option::none(),
                    price: 100,
                    orig_size: 400,
                    remaining_size: 400,
                    is_buy: true,
                    trigger_condition: option::some(tp_trigger_condition(90)),
                    metadata: TestMetadata {}
                }
            );
        // Even if the price of 100 can be matched in the order book the trigger condition 90 should not trigger
        // the matching
        assert!(match_result.is_empty());
        assert!(
            order_book.pending_orders.get_price_move_down_index().keys().length() == 1
        );

        // Trigger the pending orders with a price of 90
        let match_results = order_book.trigger_pending_orders(90);

        // Verify taker (buy order) was fully filled
        assert!(total_matched_size(&match_results) == 400);

        // Verify maker (sell order) was partially filled
        assert!(match_results.length() == 1);
        let maker_match = match_results[0];
        let (order, matched_size) = maker_match.destroy_single_order_match();
        assert!(order.get_order_id() == new_order_id_type(@0xAA, 1));
        assert!(matched_size == 400);
        assert!(order.get_orig_size() == 1000);
        assert!(order.get_remaining_size() == 600); // Partial fill

        // Place another buy order for 300 units
        let match_result =
            order_book.place_order_and_get_matches(
                OrderRequest {
                    account: @0xBB,
                    account_order_id: 2,
                    unique_priority_idx: option::none(),
                    price: 100,
                    orig_size: 300,
                    remaining_size: 300,
                    is_buy: true,
                    trigger_condition: option::some(tp_trigger_condition(80)),
                    metadata: TestMetadata {}
                }
            );

        assert!(match_result.is_empty());
        assert!(
            order_book.pending_orders.get_price_move_down_index().keys().length() == 1
        );

        // Oracle price moves up to 95, this should not trigger any order
        let match_results = order_book.trigger_pending_orders(95);
        assert!(match_results.length() == 0);

        // Move the oracle price down to 80, this should trigger the order
        let match_results = order_book.trigger_pending_orders(80);
        // Verify second taker was fully filled
        assert!(total_matched_size(&match_results) == 300);

        // Verify original maker was partially filled again
        assert!(match_results.length() == 1);
        let maker_match = match_results[0];
        let (order, matched_size) = maker_match.destroy_single_order_match();
        assert!(order.get_order_id() == new_order_id_type(@0xAA, 1));
        assert!(matched_size == 300);
        assert!(order.get_orig_size() == 1000);
        assert!(order.get_remaining_size() == 300); // Still partial as 300 units remain

        // Original sell order should still exist with 300 units remaining
        let order_id = new_order_id_type(@0xAA, 1);
        let order_state = *order_book.orders.borrow(&order_id);
        let (order, is_active) = order_state.destroy_order_from_state();
        let (_order_id, _unique_priority_idx, price, orig_size, size, is_buy, _, _) =
            order.destroy_order();
        assert!(is_active == true);
        assert!(price == 100);
        assert!(orig_size == 1000);
        assert!(size == 300); // 1000 - 400 - 300 = 300 remaining
        assert!(is_buy == false);

        order_book.destroy_order_book();
    }

    #[test]
    fun test_SL_order() {
        let order_book = new_order_book<TestMetadata>();

        // Place a GTC sell order for 1000 units at price 100
        let match_result =
            order_book.place_order_and_get_matches(
                OrderRequest {
                    account: @0xAA,
                    account_order_id: 1,
                    unique_priority_idx: option::none(),
                    price: 100,
                    orig_size: 1000,
                    remaining_size: 1000,
                    is_buy: true,
                    trigger_condition: option::none(),
                    metadata: TestMetadata {}
                }
            );
        assert!(match_result.is_empty()); // No matches for first order

        assert!(order_book.trigger_pending_orders(100).is_empty());

        // Place a smaller buy order (400 units) at the same price
        let match_result =
            order_book.place_order_and_get_matches(
                OrderRequest {
                    account: @0xBB,
                    account_order_id: 1,
                    unique_priority_idx: option::none(),
                    price: 100,
                    orig_size: 400,
                    remaining_size: 400,
                    is_buy: false,
                    trigger_condition: option::some(tp_trigger_condition(110)),
                    metadata: TestMetadata {}
                }
            );
        // Even if the price of 100 can be matched in the order book the trigger condition 110 should not trigger
        // the matching
        assert!(match_result.is_empty());
        assert!(
            order_book.pending_orders.get_price_move_up_index().keys().length() == 1
        );

        // Trigger the pending orders with a price of 110
        let match_results = order_book.trigger_pending_orders(110);
        assert!(match_results.length() == 1);

        // Verify taker (buy order) was fully filled
        assert!(total_matched_size(&match_results) == 400);

        // Verify maker (sell order) was partially filled
        assert!(match_results.length() == 1);
        let maker_match = match_results[0];
        let (order, matched_size) = maker_match.destroy_single_order_match();
        assert!(order.get_order_id() == new_order_id_type(@0xAA, 1));
        assert!(matched_size == 400);
        assert!(order.get_orig_size() == 1000);
        assert!(order.get_remaining_size() == 600); // Partial fill

        // Place another buy order for 300 units
        let match_result =
            order_book.place_order_and_get_matches(
                OrderRequest {
                    account: @0xBB,
                    account_order_id: 2,
                    unique_priority_idx: option::none(),
                    price: 100,
                    orig_size: 300,
                    remaining_size: 300,
                    is_buy: false,
                    trigger_condition: option::some(tp_trigger_condition(120)),
                    metadata: TestMetadata {}
                }
            );

        assert!(match_result.is_empty());
        assert!(
            order_book.pending_orders.get_price_move_up_index().keys().length() == 1
        );

        // Oracle price moves down to 100, this should not trigger any order
        let match_results = order_book.trigger_pending_orders(100);
        assert!(match_results.is_empty());

        // Move the oracle price up to 120, this should trigger the order
        let match_results = order_book.trigger_pending_orders(120);

        // Verify second taker was fully filled
        assert!(total_matched_size(&match_results) == 300);

        // Verify original maker was partially filled again
        assert!(match_results.length() == 1);
        let maker_match = match_results[0];
        let (order, matched_size) = maker_match.destroy_single_order_match();
        assert!(order.get_order_id() == new_order_id_type(@0xAA, 1));
        assert!(matched_size == 300);
        assert!(order.get_orig_size() == 1000);
        assert!(order.get_remaining_size() == 300); // Still partial as 300 units remain

        // Original sell order should still exist with 300 units remaining
        let order_id = new_order_id_type(@0xAA, 1);
        let order_state = *order_book.orders.borrow(&order_id);
        let (order, is_active) = order_state.destroy_order_from_state();
        let (_order_id, _unique_priority_idx, price, orig_size, size, is_buy, _, _) =
            order.destroy_order();
        assert!(is_active == true);
        assert!(price == 100);
        assert!(orig_size == 1000);
        assert!(size == 300); // 1000 - 400 - 300 = 300 remaining
        assert!(is_buy == true);
        order_book.destroy_order_book();
    }

    #[test]
    fun test_maker_order_reinsert_already_exists() {
        let order_book = new_order_book<TestMetadata>();

        // Place a GTC sell order
        let order_req = OrderRequest {
            account: @0xAA,
            account_order_id: 1,
            unique_priority_idx: option::none(),
            price: 100,
            orig_size: 1000,
            remaining_size: 1000,
            is_buy: false,
            trigger_condition: option::none(),
            metadata: TestMetadata {}
        };
        order_book.place_maker_order(order_req);
        assert!(order_book.get_remaining_size(@0xAA, 1) ==  1000);

        // Taker order
        let order_req = OrderRequest {
            account: @0xBB,
            account_order_id: 1,
            unique_priority_idx: option::none(),
            price: 100,
            orig_size: 100,
            remaining_size: 100,
            is_buy: true,
            trigger_condition: option::none(),
            metadata: TestMetadata {}
        };

        let match_results = order_book.place_order_and_get_matches(order_req);
        assert!(total_matched_size(&match_results) == 100);

        let (matched_order, _) = match_results[0].destroy_single_order_match();
        let (_order_id, unique_idx, price, orig_size, _remaining_size, is_buy, _trigger_condition, metadata  ) = matched_order.destroy_order();
        // Assume half of the order was matched and remaining 50 size is reinserted back to the order book
        let order_req = OrderRequest {
            account: @0xAA,
            account_order_id: 1,
            unique_priority_idx: option::some(unique_idx),
            price,
            orig_size,
            remaining_size: 50,
            is_buy,
            trigger_condition: option::none(),
            metadata
        };
        order_book.reinsert_maker_order(order_req);
        // Verify order was reinserted with updated size
        assert!(order_book.get_remaining_size(@0xAA, 1) ==  950);
        order_book.destroy_order_book();
    }

    #[test]
    fun test_maker_order_reinsert_not_exists() {
        let order_book = new_order_book<TestMetadata>();

        // Place a GTC sell order
        let order_req = OrderRequest {
            account: @0xAA,
            account_order_id: 1,
            unique_priority_idx: option::none(),
            price: 100,
            orig_size: 1000,
            remaining_size: 1000,
            is_buy: false,
            trigger_condition: option::none(),
            metadata: TestMetadata {}
        };
        order_book.place_maker_order(order_req);
        assert!(order_book.get_remaining_size(@0xAA, 1) ==  1000);

        // Taker order
        let order_req = OrderRequest {
            account: @0xBB,
            account_order_id: 1,
            unique_priority_idx: option::none(),
            price: 100,
            orig_size: 1000,
            remaining_size: 1000,
            is_buy: true,
            trigger_condition: option::none(),
            metadata: TestMetadata {}
        };

        let match_results = order_book.place_order_and_get_matches(order_req);
        assert!(total_matched_size(&match_results) == 1000);

        let (matched_order, _) = match_results[0].destroy_single_order_match();
        let (_order_id, unique_idx, price, orig_size, _remaining_size, is_buy, _trigger_condition, metadata  ) = matched_order.destroy_order();
        // Assume half of the order was matched and remaining 50 size is reinserted back to the order book
        let order_req = OrderRequest {
            account: @0xAA,
            account_order_id: 1,
            unique_priority_idx: option::some(unique_idx),
            price,
            orig_size,
            remaining_size: 500,
            is_buy,
            trigger_condition: option::none(),
            metadata
        };
        order_book.reinsert_maker_order(order_req);
        // Verify order was reinserted with updated size
        assert!(order_book.get_remaining_size(@0xAA, 1) == 500);
        order_book.destroy_order_book();
    }


    #[test]
    fun test_decrease_order_size() {
        let order_book = new_order_book<TestMetadata>();

        // Place an active order
        let order_req = OrderRequest {
            account: @0xAA,
            account_order_id: 1,
            unique_priority_idx: option::none(),
            price: 100,
            orig_size: 1000,
            remaining_size: 1000,
            is_buy: false,
            trigger_condition: option::none(),
            metadata: TestMetadata {}
        };
        order_book.place_maker_order(order_req);
        assert!(order_book.get_remaining_size(@0xAA, 1) ==  1000);

        order_book.decrease_order_size(@0xAA, 1, 700);
        // Verify order was decreased with updated size
        assert!(order_book.get_remaining_size(@0xAA, 1) == 300);

        let order_req = OrderRequest {
            account: @0xBB,
            account_order_id: 1,
            unique_priority_idx: option::none(),
            price: 100,
            orig_size: 1000,
            remaining_size: 1000,
            is_buy: false,
            trigger_condition: option::some(tp_trigger_condition(90)),
            metadata: TestMetadata {}
        };
        order_book.place_maker_order(order_req);
        assert!(order_book.get_remaining_size(@0xBB, 1) == 1000);
        order_book.decrease_order_size(@0xBB, 1, 600);
        // Verify order was decreased with updated size
        assert!(order_book.get_remaining_size(@0xBB, 1) == 400);

        order_book.destroy_order_book();
    }

}
