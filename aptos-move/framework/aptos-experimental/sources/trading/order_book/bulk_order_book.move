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
/// - `EPOST_ONLY_FILLED`: Post-only order was filled (not implemented in bulk orders)
/// - `EORDER_NOT_FOUND`: Order not found for cancellation or reinsertion
/// - `EINVALID_INACTIVE_ORDER_STATE`: Order is in an invalid inactive state
/// - `EINVALID_ADD_SIZE_TO_ORDER`: Invalid size addition to order
/// - `E_NOT_ACTIVE_ORDER`: Order is not active
/// - `E_REINSERT_ORDER_MISMATCH`: Reinsertion order validation failed
/// - `EORDER_CREATOR_MISMATCH`: Order creator mismatch
/// - `EINVLID_MM_ORDER_REQUEST`: Invalid bulk order request (price ordering, sizes, etc.)
/// - `EPRICE_CROSSING`: Price crossing is not allowed in bulk orders
///
/// ## Usage Example:
/// ```move
/// // Create a new bulk order book
/// let order_book = bulk_order_book::new_bulk_order_book();
///
/// // Create a bulk order request with multiple price levels
/// let bid_prices = vector[100, 99, 98];
/// let bid_sizes = vector[10, 20, 30];
/// let ask_prices = vector[101, 102, 103];
/// let ask_sizes = vector[15, 25, 35];
///
/// let order_request = bulk_order_book::new_bulk_order_request(
///     @trader,
///     bid_prices,
///     bid_sizes,
///     ask_prices,
///     ask_sizes
/// );
///
/// // Place the maker order
/// order_book.place_maker_order(order_request);
///
/// // Check if a taker order would match
/// if (order_book.is_taker_order(101, true)) {
///     // Get the match
///     let match_result = order_book.get_single_match_for_taker(101, 10, true);
///     // Process the match...
/// }
///
/// // Cancel the order
/// order_book.cancel_order(@trader);
/// ```
module aptos_experimental::bulk_order_book {
    use aptos_framework::big_ordered_map::BigOrderedMap;
    use aptos_experimental::order_book_types::ActiveMatchedOrder;
    use aptos_experimental::order_book_types;
    use aptos_experimental::bulk_order_book_types::{BulkOrder, new_bulk_order,
        new_bulk_order_match
    };
    use aptos_experimental::order_book_types::{OrderMatch, OrderMatchDetails, bulk_order_book_type};
    use aptos_experimental::order_book_types::{
        OrderIdType,
        AscendingIdGenerator, new_order_id_type, new_unique_idx_type
    };
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
    /// Request structure for placing a new bulk order with multiple price levels.
    ///
    /// # Fields:
    /// - `account`: The account placing the order
    /// - `bid_prices`: Vector of bid prices in descending order (best price first)
    /// - `bid_sizes`: Vector of bid sizes corresponding to each price level
    /// - `ask_prices`: Vector of ask prices in ascending order (best price first)
    /// - `ask_sizes`: Vector of ask sizes corresponding to each price level
    ///
    /// # Validation:
    /// - Bid prices must be in descending order
    /// - Ask prices must be in ascending order
    /// - All sizes must be greater than 0
    /// - Price and size vectors must have matching lengths
    enum BulkOrderRequest has copy, drop {
        V1 {
            account: address,
            bid_prices: vector<u64>, // prices for each levels of the order
            bid_sizes: vector<u64>, // sizes for each levels of the order
            ask_prices: vector<u64>, // prices for each levels of the order
            ask_sizes: vector<u64>, // sizes for each levels of the order
        }
    }

    /// Main bulk order book container that manages all orders and their matching.
    ///
    /// # Fields:
    /// - `orders`: Map of account addresses to their bulk orders
    /// - `order_id_to_address`: Map of order IDs to account addresses for lookup
    enum BulkOrderBook has store {
        V1 {
            orders: BigOrderedMap<address, BulkOrder>,
            order_id_to_address: BigOrderedMap<OrderIdType, address>
        }
    }

    /// Creates a new empty bulk order book.
    ///
    /// # Returns:
    /// A new `BulkOrderBook` instance with empty order collections.
    public fun new_bulk_order_book(): BulkOrderBook {
        BulkOrderBook::V1 {
            orders:  order_book_types::new_default_big_ordered_map(),
            order_id_to_address:  order_book_types::new_default_big_ordered_map()
        }
    }

    /// Creates a new bulk order request with the specified price levels and sizes.
    ///
    /// # Arguments:
    /// - `account`: The account placing the order
    /// - `bid_prices`: Vector of bid prices in descending order
    /// - `bid_sizes`: Vector of bid sizes corresponding to each price level
    /// - `ask_prices`: Vector of ask prices in ascending order
    /// - `ask_sizes`: Vector of ask sizes corresponding to each price level
    /// - `metadata`: Additional metadata for the order
    ///
    /// # Returns:
    /// A `BulkOrderRequest` instance.
    ///
    /// # Aborts:
    /// - If bid_prices and bid_sizes have different lengths
    /// - If ask_prices and ask_sizes have different lengths
    public fun new_bulk_order_request(
        account: address,
        bid_prices: vector<u64>,
        bid_sizes: vector<u64>,
        ask_prices: vector<u64>,
        ask_sizes: vector<u64>
    ): BulkOrderRequest {
        assert!(bid_prices.length() == bid_sizes.length(), EINVLID_MM_ORDER_REQUEST);
        assert!(ask_prices.length() == ask_sizes.length(), EINVLID_MM_ORDER_REQUEST);
        return BulkOrderRequest::V1 {
            account,
            bid_prices,
            bid_sizes,
            ask_prices,
            ask_sizes
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
    public fun get_single_match_for_taker<M: store + copy + drop>(
        self: &mut BulkOrderBook,
        price_time_idx: &mut aptos_experimental::price_time_index::PriceTimeIndex,
        active_matched_order: ActiveMatchedOrder,
        is_bid: bool
    ): OrderMatch<M> {
        let (order_id, matched_size, remaining_size, order_book_type) =
            active_matched_order.destroy_active_matched_order();
        assert!(order_book_type == bulk_order_book_type(), ENOT_BULK_ORDER);
        let order_address = self.order_id_to_address.get(&order_id).destroy_some();
        let order = self.orders.remove(&order_address);
        let order_match = new_bulk_order_match<M>(
            &mut order,
            !is_bid,
            matched_size,
        );
        let (next_price, next_size) = order.match_order_and_get_next(!is_bid, matched_size);
        if (remaining_size == 0 && next_price.is_some()) {
            let price = next_price.destroy_some();
            let size = next_size.destroy_some();
            price_time_idx.place_maker_order(
                order_id,
                bulk_order_book_type(),
                price,
                order.get_unique_priority_idx(),
                size,
                !is_bid,
            );
        };
        self.orders.add(order_address, order);
        return order_match
    }

    /// Validates that prices are in the correct order (descending for bids, ascending for asks).
    ///
    /// # Arguments:
    /// - `prices`: Vector of prices to validate
    /// - `is_descending`: True if prices should be in descending order, false for ascending
    ///
    /// # Aborts:
    /// - If prices are not in the correct order
    fun validate_price_ordering(
        prices: &vector<u64>,
        is_descending: bool
    ) {
        let i = 0;
        if (prices.length() == 0) {
            return ; // No prices to validate
        };
        while (i < prices.length() - 1) {
            if (is_descending) {
                assert!(prices[i] > prices[i + 1], EINVLID_MM_ORDER_REQUEST);
            } else {
                assert!(prices[i] < prices[i + 1], EINVLID_MM_ORDER_REQUEST);
            };
            i += 1;
        };
    }

    /// Validates that all sizes in the vector are greater than 0.
    ///
    /// # Arguments:
    /// - `sizes`: Vector of sizes to validate
    ///
    /// # Aborts:
    /// - If the vector is empty
    /// - If any size is 0
    fun validate_not_zero_sizes(
        sizes: &vector<u64>
    ) {
        let i = 0;
        while (i < sizes.length()) {
            assert!(sizes[i] > 0, EINVLID_MM_ORDER_REQUEST);
            i += 1;
        };
    }

    /// Validates that bid and ask prices don't cross.
    ///
    /// This ensures that the highest bid price is lower than the lowest ask price,
    /// preventing self-matching within a single order.
    ///
    /// # Arguments:
    /// - `bid_prices`: Vector of bid prices (should be in descending order)
    /// - `ask_prices`: Vector of ask prices (should be in ascending order)
    ///
    /// # Aborts:
    /// - If the highest bid price is greater than or equal to the lowest ask price
    fun validate_no_price_crossing(
        bid_prices: &vector<u64>,
        ask_prices: &vector<u64>
    ) {
        if (bid_prices.length() > 0 && ask_prices.length() > 0) {
            let highest_bid = bid_prices[0]; // First element is highest (descending order)
            let lowest_ask = ask_prices[0];  // First element is lowest (ascending order)
            assert!(highest_bid < lowest_ask, EPRICE_CROSSING);
        };
    }

    /// Validates a bulk order request for correctness.
    ///
    /// # Arguments:
    /// - `order_req`: The bulk order request to validate
    ///
    /// # Aborts:
    /// - If any validation fails (price ordering, sizes, vector lengths, price crossing)
    fun validate_mm_order_request(
        order_req: &BulkOrderRequest,
    ) {
        // Ensure bid prices are in descending order and ask prices are in ascending order
        assert!(order_req.bid_sizes.length() > 0 || order_req.ask_sizes.length() > 0, EINVLID_MM_ORDER_REQUEST);
        validate_not_zero_sizes(&order_req.bid_sizes);
        validate_not_zero_sizes(&order_req.ask_sizes);
        assert!(order_req.bid_prices.length() == order_req.bid_sizes.length(), EINVLID_MM_ORDER_REQUEST);
        assert!(order_req.ask_prices.length() == order_req.ask_sizes.length(), EINVLID_MM_ORDER_REQUEST);
        validate_price_ordering(&order_req.bid_prices, true);  // descending
        validate_price_ordering(&order_req.ask_prices, false); // ascending
        validate_no_price_crossing(&order_req.bid_prices, &order_req.ask_prices);
    }

    /// Cancels active orders for a specific side (bid or ask) of a bulk order.
    ///
    /// # Arguments:
    /// - `active_orders`: Reference to the active order book
    /// - `order`: The bulk order to cancel active orders for
    /// - `is_bid`: True to cancel bid orders, false for ask orders
    fun cancel_active_order_for_side(
        price_time_idx: &mut aptos_experimental::price_time_index::PriceTimeIndex,
        order: &BulkOrder,
        is_bid: bool
    ) {
        let active_price = order.get_active_price(is_bid);
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
    fun cancel_active_orders(
        price_time_idx: &mut aptos_experimental::price_time_index::PriceTimeIndex, order: &BulkOrder
    ) {
        cancel_active_order_for_side(price_time_idx, order, true);  // cancel bid
        cancel_active_order_for_side(price_time_idx, order, false); // cancel ask
    }

    /// Activates the first price level for a specific side of a bulk order.
    ///
    /// # Arguments:
    /// - `active_orders`: Reference to the active order book
    /// - `order`: The bulk order to activate levels for
    /// - `order_id`: The order ID for the bulk order
    /// - `is_bid`: True to activate bid levels, false for ask levels
    fun activate_first_price_level_for_side(
        price_time_idx: &mut aptos_experimental::price_time_index::PriceTimeIndex,
        order: &BulkOrder,
        order_id: OrderIdType,
        is_bid: bool
    ) {
        let active_price = order.get_active_price(is_bid);
        let active_size = order.get_active_size(is_bid);
        if (active_price.is_some()) {
            price_time_idx.place_maker_order(
                order_id,
                bulk_order_book_type(),
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
    fun activate_first_price_levels(
        price_time_idx: &mut aptos_experimental::price_time_index::PriceTimeIndex, order: &BulkOrder, order_id: OrderIdType
    ) {
        activate_first_price_level_for_side(price_time_idx, order, order_id, true);  // activate bid
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
    public fun reinsert_order<M: store + copy + drop>(
        self: &mut BulkOrderBook,
        price_time_idx: &mut aptos_experimental::price_time_index::PriceTimeIndex,
        reinsert_order: OrderMatchDetails<M>,
        original_order: &OrderMatchDetails<M>
    ) {
        assert!(reinsert_order.validate_reinsertion_request(original_order), E_REINSERT_ORDER_MISMATCH);
        let account = reinsert_order.get_account_from_match_details();
        assert!(self.orders.contains(&account), EORDER_NOT_FOUND);
        let order = self.orders.remove(&account);
        cancel_active_orders(price_time_idx, &order);
        order.reinsert_order(&reinsert_order);
        activate_first_price_levels(price_time_idx, &order, reinsert_order.get_order_id_from_match_details());
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
    public fun cancel_bulk_order(
        self: &mut BulkOrderBook,
        price_time_idx: &mut aptos_experimental::price_time_index::PriceTimeIndex,
        account: address
    ): (OrderIdType, u64, u64) {
        if (!self.orders.contains(&account)) {
            abort EORDER_NOT_FOUND;
        };
        // For cancellation, instead of removing the order, we will just cancel the active orders and set the sizes to 0.
        // This allows us to reuse the order id for the same account in the future without creating a new order.
        let order = self.orders.remove(&account);
        let order_id = order.get_order_id();
        let remaining_bid_size = order.get_total_remaining_size(true);
        let remaining_ask_size = order.get_total_remaining_size(false);
        cancel_active_orders(price_time_idx, &order);
        order.set_empty();
        self.orders.add(account, order);
        (order_id, remaining_bid_size, remaining_ask_size)
    }


    public fun get_remaining_size(
        self: &BulkOrderBook,
        account: address,
        is_bid: bool
    ): u64 {
        if (!self.orders.contains(&account)) {
            abort EORDER_NOT_FOUND;
        };

        self.orders.get(&account).destroy_some().get_total_remaining_size(is_bid)
    }

    /// Places a new maker order in the bulk order book.
    ///
    /// If an order already exists for the account, it will be replaced with the new order.
    /// The first price levels of both bid and ask sides will be activated in the active order book.
    ///
    /// # Arguments:
    /// - `self`: Mutable reference to the bulk order book
    /// - `price_time_idx`: Mutable reference to the price time index
    /// - `ascending_id_generator`: Mutable reference to the ascending id generator
    /// - `order_req`: The bulk order request to place
    ///
    /// # Aborts:
    /// - If the order request validation fails
    public fun place_bulk_order(
        self: &mut BulkOrderBook,
        price_time_idx: &mut aptos_experimental::price_time_index::PriceTimeIndex,
        ascending_id_generator: &mut AscendingIdGenerator,
        order_req: BulkOrderRequest
    ) : OrderIdType {
        validate_mm_order_request(&order_req);
        let existing_order = self.orders.contains(&order_req.account);
        let order_id = if (existing_order) {
            let old_order = self.orders.remove(&order_req.account);
            cancel_active_orders(price_time_idx, &old_order);
            old_order.get_order_id()
        } else {
            let order_id = new_order_id_type(ascending_id_generator.next_ascending_id());
            self.order_id_to_address.add(order_id, order_req.account);
            order_id
        };
        let BulkOrderRequest::V1 { account, bid_prices, bid_sizes, ask_prices, ask_sizes } = order_req;
        let new_order = new_bulk_order(
            order_id,
            account,
            new_unique_idx_type(ascending_id_generator.next_ascending_id()),
            bid_prices,
            bid_sizes,
            ask_prices,
            ask_sizes
        );
        self.orders.add(order_req.account, new_order);
        // Activate the first price levels in the active order book
        activate_first_price_levels(price_time_idx, &new_order, order_id);
        order_id
    }

    #[test_only]
    public fun destroy_bulk_order_book(
        self: BulkOrderBook
    ) {
        let BulkOrderBook::V1 {
            orders,
            order_id_to_address
        } = self;
        orders.destroy(|_v| {});
        order_id_to_address.destroy(|_v| {});
    }
}
