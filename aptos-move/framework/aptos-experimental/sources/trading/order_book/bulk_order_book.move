/// # Bulk Order Book Module
///
/// This module implements a bulk order book system that allows traders to place orders with multiple
/// price levels simultaneously. The bulk order book supports both maker and taker orders, with
/// sophisticated order matching, cancellation, and reinsertion capabilities.
///
/// ## Key Features:
///
/// ### 1. Multi-Level Orders
/// - Traders can place orders with multiple price levels in a single transaction
/// - Bid orders: Prices must be in descending order (best price first)
/// - Ask orders: Prices must be in ascending order (best price first)
/// - Each price level has an associated size
///
/// ### 2. Order Matching
/// - Price-time priority: Orders are matched based on price first, then time
/// - Partial fills: Orders can be partially filled across multiple levels
/// - Automatic level progression: When a price level is fully consumed, the next level becomes active
///
/// ### 3. Order Management
/// - **Cancellation**: Orders can be cancelled, clearing all active levels
/// - **Reinsertion**: Matched portions of orders can be reinserted back into the order book
/// - **Order ID Reuse**: Cancelled orders allow the same account to place new orders with the same ID
///
/// ## Data Structures:
///
/// - `BulkOrderBook`: Main order book container
/// - `BulkOrderRequest`: Request structure for placing new orders
/// - `BulkOrder`: Internal representation of a multi-level order
/// - `BulkOrderResult`: Result of order matching operations
/// - `SingleBulkOrderMatch`: Single match result between orders
///
/// ## Error Codes:
/// - `EORDER_ALREADY_EXISTS`: Order already exists for the account
/// - `EPOST_ONLY_FILLED`: Post-only order was filled (crossed the spread)
/// - `EORDER_NOT_FOUND`: Order not found for cancellation or reinsertion
/// - `EINVALID_INACTIVE_ORDER_STATE`: Order is in an invalid inactive state
/// - `EINVALID_ADD_SIZE_TO_ORDER`: Invalid size addition to order
/// - `E_NOT_ACTIVE_ORDER`: Order is not active
/// - `E_REINSERT_ORDER_MISMATCH`: Reinsertion order validation failed
/// - `EORDER_CREATOR_MISMATCH`: Order creator mismatch
/// - `EINVLID_MM_ORDER_REQUEST`: Invalid bulk order request (price ordering, sizes, etc.)
/// - `EPRICE_CROSSING`: Price crossing is not allowed in bulk orders
///
module aptos_experimental::bulk_order_book {
    friend aptos_experimental::order_book;
    #[test_only]
    friend aptos_experimental::bulk_order_book_tests;

    use aptos_framework::big_ordered_map::BigOrderedMap;
    use aptos_trading::order_book_types::{
        bulk_order_type,
        OrderId,
        next_order_id,
        next_increasing_idx_type
    };
    use aptos_trading::order_match_types::{
        ActiveMatchedOrder,
        OrderMatch,
        OrderMatchDetails
    };
    use aptos_trading::bulk_order_types::{
        BulkOrder,
        BulkOrderPlaceResponse,
        BulkOrderRequest,
        new_bulk_order_match,
        new_bulk_order_place_response_success,
        new_bulk_order_place_response_rejection
    };
    use aptos_experimental::order_book_utils;
    use aptos_experimental::bulk_order_utils;

    // Error codes for various failure scenarios
    const EORDER_ALREADY_EXISTS: u64 = 1;
    const EPOST_ONLY_FILLED: u64 = 2;
    const EORDER_NOT_FOUND: u64 = 4;
    const EINVALID_INACTIVE_ORDER_STATE: u64 = 5;
    const EINVALID_ADD_SIZE_TO_ORDER: u64 = 6;
    const E_NOT_ACTIVE_ORDER: u64 = 7;
    const E_REINSERT_ORDER_MISMATCH: u64 = 8;
    const EORDER_CREATOR_MISMATCH: u64 = 9;
    const EINVLID_MM_ORDER_REQUEST: u64 = 10;
    const EPRICE_CROSSING: u64 = 11;
    const ENOT_BULK_ORDER: u64 = 12;
    const E_INVALID_SEQUENCE_NUMBER: u64 = 13;

    /// Main bulk order book container that manages all orders and their matching.
    ///
    /// # Fields:
    /// - `orders`: Map of account addresses to their bulk orders
    /// - `order_id_to_address`: Map of order IDs to account addresses for lookup
    enum BulkOrderBook<M: store + copy + drop> has store {
        V1 {
            // TODO(skedia): Consider using a Table instead of BigOrderedMap so that each order has its own storage slot.
            orders: BigOrderedMap<address, BulkOrder<M>>,
            order_id_to_address: BigOrderedMap<OrderId, address>
        }
    }

    /// Creates a new empty bulk order book.
    ///
    /// # Returns:
    /// A new `BulkOrderBook` instance with empty order collections.
    public(friend) fun new_bulk_order_book<M: store + copy + drop>(): BulkOrderBook<M> {
        BulkOrderBook::V1 {
            orders: order_book_utils::new_default_big_ordered_map(),
            order_id_to_address: order_book_utils::new_default_big_ordered_map()
        }
    }

    /// Returns a single match for a taker order.
    ///
    /// This function should only be called after verifying that the order is a taker order
    /// using `is_taker_order()`. If called on a non-taker order, it will abort.
    ///
    /// # Arguments:
    /// - `self`: Mutable reference to the bulk order book
    /// - `price_time_idx`: Mutable reference to the price time index
    /// - `price`: The price of the taker order
    /// - `size`: The size of the taker order
    /// - `is_bid`: True if the taker order is a bid, false if ask
    ///
    /// # Returns:
    /// A `SingleBulkOrderMatch` containing the match details.
    ///
    /// # Side Effects:
    /// - Updates the matched order's remaining sizes
    /// - Activates the next price level if the current level is fully consumed
    /// - Updates the active order book
    public(friend) fun get_single_match_for_taker<M: store + copy + drop>(
        self: &mut BulkOrderBook<M>,
        price_time_idx: &mut aptos_experimental::price_time_index::PriceTimeIndex,
        active_matched_order: ActiveMatchedOrder,
        is_bid: bool
    ): OrderMatch<M> {
        let (order_id, matched_size, remaining_size, order_book_type) =
            active_matched_order.destroy_active_matched_order();
        assert!(order_book_type == bulk_order_type(), ENOT_BULK_ORDER);
        let order_address = self.order_id_to_address.get(&order_id).destroy_some();
        let order = self.orders.remove(&order_address);
        let order_match = new_bulk_order_match<M>(&order, !is_bid, matched_size);
        let (next_price, next_size) =
            bulk_order_utils::match_order_and_get_next_from_bulk_order(
                &mut order, !is_bid, matched_size
            );
        if (remaining_size == 0 && next_price.is_some()) {
            let price = next_price.destroy_some();
            let size = next_size.destroy_some();
            price_time_idx.place_maker_order(
                order_id,
                bulk_order_type(),
                price,
                order.get_unique_priority_idx(),
                size,
                !is_bid
            );
        };
        self.orders.add(order_address, order);
        return order_match
    }

    /// Cancels active orders for a specific side (bid or ask) of a bulk order.
    ///
    /// # Arguments:
    /// - `active_orders`: Reference to the active order book
    /// - `order`: The bulk order to cancel active orders for
    /// - `is_bid`: True to cancel bid orders, false for ask orders
    fun cancel_active_order_for_side<M: store + copy + drop>(
        price_time_idx: &mut aptos_experimental::price_time_index::PriceTimeIndex,
        order: &BulkOrder<M>,
        is_bid: bool
    ) {
        let active_price = order.get_order_request().get_active_price(is_bid);
        if (active_price.is_some()) {
            price_time_idx.cancel_active_order(
                active_price.destroy_some(),
                order.get_unique_priority_idx(),
                is_bid
            );
        };
    }

    /// Cancels all active orders (both bid and ask) for a bulk order.
    ///
    /// # Arguments:
    /// - `active_orders`: Reference to the active order book
    /// - `order`: The bulk order to cancel active orders for
    fun cancel_active_orders<M: store + copy + drop>(
        price_time_idx: &mut aptos_experimental::price_time_index::PriceTimeIndex,
        order: &BulkOrder<M>
    ) {
        cancel_active_order_for_side(price_time_idx, order, true); // cancel bid
        cancel_active_order_for_side(price_time_idx, order, false); // cancel ask
    }

    /// Activates the first price level for a specific side of a bulk order.
    ///
    /// # Arguments:
    /// - `active_orders`: Reference to the active order book
    /// - `order`: The bulk order to activate levels for
    /// - `order_id`: The order ID for the bulk order
    /// - `is_bid`: True to activate bid levels, false for ask levels
    fun activate_first_price_level_for_side<M: store + copy + drop>(
        price_time_idx: &mut aptos_experimental::price_time_index::PriceTimeIndex,
        order: &BulkOrder<M>,
        order_id: OrderId,
        is_bid: bool
    ) {
        let order_request = order.get_order_request();
        let active_price = order_request.get_active_price(is_bid);
        let active_size = order_request.get_active_size(is_bid);
        if (active_price.is_some()) {
            price_time_idx.place_maker_order(
                order_id,
                bulk_order_type(),
                active_price.destroy_some(),
                order.get_unique_priority_idx(),
                active_size.destroy_some(),
                is_bid
            );
        }
    }

    /// Activates the first price levels for both bid and ask sides of a bulk order.
    ///
    /// # Arguments:
    /// - `active_orders`: Reference to the active order book
    /// - `order`: The bulk order to activate levels for
    /// - `order_id`: The order ID for the bulk order
    fun activate_first_price_levels<M: store + copy + drop>(
        price_time_idx: &mut aptos_experimental::price_time_index::PriceTimeIndex,
        order: &BulkOrder<M>,
        order_id: OrderId
    ) {
        activate_first_price_level_for_side(price_time_idx, order, order_id, true); // activate bid
        activate_first_price_level_for_side(price_time_idx, order, order_id, false); // activate ask
    }

    /// Reinserts a bulk order back into the order book after it has been matched.
    ///
    /// This function allows traders to reinsert portions of their orders that were matched,
    /// effectively allowing them to "reuse" matched liquidity.
    ///
    /// # Arguments:
    /// - `self`: Mutable reference to the bulk order book
    /// - `price_time_idx`: Mutable reference to the price time index
    /// - `reinsert_order`: The order result to reinsert
    /// - `original_order`: The original order result for validation
    ///
    /// # Aborts:
    /// - If the order account doesn't exist in the order book
    /// - If the reinsertion validation fails
    public(friend) fun reinsert_order<M: store + copy + drop>(
        self: &mut BulkOrderBook<M>,
        price_time_idx: &mut aptos_experimental::price_time_index::PriceTimeIndex,
        reinsert_order: OrderMatchDetails<M>,
        original_order: &OrderMatchDetails<M>
    ) {
        assert!(
            reinsert_order.validate_bulk_order_reinsertion_request(original_order),
            E_REINSERT_ORDER_MISMATCH
        );
        let account = reinsert_order.get_account_from_match_details();
        let order_option = self.orders.remove_or_none(&account);
        assert!(order_option.is_some(), EORDER_NOT_FOUND);
        let order = order_option.destroy_some();
        cancel_active_orders(price_time_idx, &order);
        bulk_order_utils::reinsert_order_into_bulk_order(&mut order, &reinsert_order);
        activate_first_price_levels(
            price_time_idx, &order, reinsert_order.get_order_id_from_match_details()
        );
        self.orders.add(account, order);
    }

    /// Cancels a bulk order for the specified account.
    ///
    /// Instead of removing the order entirely, this function clears all active levels
    /// and sets the order to empty state, allowing the same account to place new orders
    /// with the same order ID in the future.
    ///
    /// # Arguments:
    /// - `self`: Mutable reference to the bulk order book
    /// - `price_time_idx`: Mutable reference to the price time index
    /// - `account`: The account whose order should be cancelled
    ///
    /// # Aborts:
    /// - If no order exists for the specified account
    public(friend) fun cancel_bulk_order<M: store + copy + drop>(
        self: &mut BulkOrderBook<M>,
        price_time_idx: &mut aptos_experimental::price_time_index::PriceTimeIndex,
        account: address
    ): BulkOrder<M> {
        // For cancellation, instead of removing the order, we will just cancel the active orders and set the sizes to 0.
        // This allows us to reuse the order id for the same account in the future without creating a new order.
        let order_opt = self.orders.remove_or_none(&account);
        assert!(order_opt.is_some(), EORDER_NOT_FOUND);
        let order = order_opt.destroy_some();
        let order_copy = order;
        cancel_active_orders(price_time_idx, &order);
        order.set_empty();
        self.orders.add(account, order);
        order_copy
    }

    /// Cancels a specific price level in a bulk order.
    ///
    /// This function removes only the specified price level from the bulk order,
    /// keeping all other price levels intact. If the cancelled price level was active,
    /// it will be removed from the active order book and the next price level (if any)
    /// will be activated.
    ///
    /// # Arguments:
    /// - `self`: Mutable reference to the bulk order book
    /// - `price_time_idx`: Mutable reference to the price time index
    /// - `account`: The account whose order contains the price level to cancel
    /// - `price`: The price level to cancel
    /// - `is_bid`: True to cancel from bid side, false for ask side
    ///
    /// # Returns:
    /// A tuple containing:
    /// - The cancelled size at that price level
    /// - The updated bulk order (copy for event emission)
    ///
    /// # Aborts:
    /// - If no order exists for the specified account
    public(friend) fun cancel_bulk_order_at_price<M: store + copy + drop>(
        self: &mut BulkOrderBook<M>,
        price_time_idx: &mut aptos_experimental::price_time_index::PriceTimeIndex,
        account: address,
        price: u64,
        is_bid: bool
    ): (u64, BulkOrder<M>) {
        let order_opt = self.orders.remove_or_none(&account);
        assert!(order_opt.is_some(), EORDER_NOT_FOUND);
        let order = order_opt.destroy_some();

        // Check if the price to cancel is the currently active price
        let active_price = order.get_order_request().get_active_price(is_bid);
        let was_active = active_price.is_some() && active_price.destroy_some() == price;

        // If this was the active price level, we need to cancel it from the active order book first
        if (was_active) {
            cancel_active_order_for_side(price_time_idx, &order, is_bid);
        };

        // Cancel the specific price level
        let cancelled_size =
            bulk_order_utils::cancel_at_price_level(&mut order, price, is_bid);

        // If this was the active price level, activate the next price level if available
        if (was_active) {
            let order_id = order.get_order_id();
            activate_first_price_level_for_side(price_time_idx, &order, order_id, is_bid);
        };

        let order_copy = order;
        self.orders.add(account, order);
        (cancelled_size, order_copy)
    }

    public(friend) fun get_bulk_order<M: store + copy + drop>(
        self: &BulkOrderBook<M>, account: address
    ): BulkOrder<M> {
        let result = self.orders.get(&account);
        assert!(result.is_some(), EORDER_NOT_FOUND);
        result.destroy_some()
    }

    public(friend) fun get_remaining_size<M: store + copy + drop>(
        self: &BulkOrderBook<M>, account: address, is_bid: bool
    ): u64 {
        let result_option =
            self.orders.get_and_map(
                &account,
                |order| order.get_order_request().get_total_remaining_size(is_bid)
            );
        assert!(result_option.is_some(), EORDER_NOT_FOUND);
        result_option.destroy_some()
    }

    public(friend) fun get_prices<M: store + copy + drop>(
        self: &BulkOrderBook<M>, account: address, is_bid: bool
    ): vector<u64> {
        let result_option =
            self.orders.get_and_map(
                &account,
                |order| order.get_order_request().get_all_prices(is_bid)
            );
        assert!(result_option.is_some(), EORDER_NOT_FOUND);
        result_option.destroy_some()
    }

    public(friend) fun get_sizes<M: store + copy + drop>(
        self: &BulkOrderBook<M>, account: address, is_bid: bool
    ): vector<u64> {
        let result_option =
            self.orders.get_and_map(
                &account,
                |order| order.get_order_request().get_all_sizes(is_bid)
            );
        assert!(result_option.is_some(), EORDER_NOT_FOUND);
        result_option.destroy_some()
    }

    /// Places a new maker order in the bulk order book.
    ///
    /// If an order already exists for the account, it will be replaced with the new order.
    /// The first price levels of both bid and ask sides will be activated in the active order book.
    ///
    /// # Arguments:
    /// - `self`: Mutable reference to the bulk order book
    /// - `price_time_idx`: Mutable reference to the price time index
    /// - `order_req`: The bulk order request to place
    ///
    /// # Aborts:
    /// - If the order request validation fails
    public(friend) fun place_bulk_order<M: store + copy + drop>(
        self: &mut BulkOrderBook<M>,
        price_time_idx: &mut aptos_experimental::price_time_index::PriceTimeIndex,
        order_req: BulkOrderRequest<M>
    ): BulkOrderPlaceResponse<M> {
        let account = order_req.get_account();
        let new_sequence_number = order_req.get_sequence_number();
        let order_option = self.orders.remove_or_none(&account);
        let (order_id, previous_seq_num) =
            if (order_option.is_some()) {
                let old_order = order_option.destroy_some();
                let existing_sequence_number =
                    old_order.get_order_request().get_sequence_number();
                // Return rejection response instead of aborting
                if (new_sequence_number <= existing_sequence_number) {
                    // Put the old order back
                    self.orders.add(account, old_order);
                    return new_bulk_order_place_response_rejection(
                        account,
                        new_sequence_number,
                        existing_sequence_number
                    )
                };
                cancel_active_orders(price_time_idx, &old_order);
                (old_order.get_order_id(), std::option::some(existing_sequence_number))
            } else {
                let order_id = next_order_id();
                self.order_id_to_address.add(order_id, account);
                (order_id, std::option::none())
            };
        let (
            bulk_order,
            cancelled_bid_prices,
            cancelled_bid_sizes,
            cancelled_ask_prices,
            cancelled_ask_sizes
        ) =
            bulk_order_utils::new_bulk_order_with_sanitization(
                order_id,
                next_increasing_idx_type(),
                order_req,
                price_time_idx.best_bid_price(),
                price_time_idx.best_ask_price()
            );
        self.orders.add(account, bulk_order);
        // Activate the first price levels in the active order book
        activate_first_price_levels(price_time_idx, &bulk_order, order_id);
        new_bulk_order_place_response_success(
            bulk_order,
            cancelled_bid_prices,
            cancelled_bid_sizes,
            cancelled_ask_prices,
            cancelled_ask_sizes,
            previous_seq_num
        )
    }

    #[test_only]
    public fun destroy_bulk_order_book<M: store + copy + drop>(
        self: BulkOrderBook<M>
    ) {
        let BulkOrderBook::V1 { orders, order_id_to_address } = self;
        orders.destroy(|_v| {});
        order_id_to_address.destroy(|_v| {});
    }
}
