/// # Bulk Order Book Types Module
///
/// This module defines the core data structures and types used by the bulk order book system.
/// It provides the foundational types for representing multi-level orders and their management.
///
/// ## Key Data Structures:
///
/// ### 1. BulkOrder
/// Represents a multi-level order with both bid and ask sides. Each side can have multiple
/// price levels with associated sizes.
///
/// ## Core Functionality:
///
/// - **Order Creation**: Functions to create new bulk orders
/// - **Order Matching**: Logic for matching orders and updating remaining quantities
/// - **Order Reinsertion**: Support for reinserting matched portions back into the order book
/// - **Order Management**: Helper functions for order state management and cleanup
///
/// ## Error Codes:
/// - `EUNEXPECTED_MATCH_PRICE`: Unexpected price during order matching
/// - `EUNEXPECTED_MATCH_SIZE`: Unexpected size during order matching
/// - `E_REINSERT_ORDER_MISMATCH`: Order mismatch during reinsertion validation
///
/// ## Usage Example:
/// ```move
/// // Create a new bulk order
/// let order = bulk_order_book_types::new_bulk_order(
///     order_id,
///     @trader,
///     unique_priority_idx,
///     bid_prices,
///     bid_sizes,
///     ask_prices,
///     ask_sizes
/// );
/// ```
/// (work in progress)
module aptos_experimental::bulk_order_book_types {
    use std::option;
    use std::option::Option;
    use std::vector;
    use aptos_experimental::order_book_types::{OrderIdType, UniqueIdxType, OrderMatchDetails, OrderMatch,
        new_bulk_order_match_details, new_order_match
    };
    friend aptos_experimental::price_time_index;
    friend aptos_experimental::order_book;
    friend aptos_experimental::pending_order_book_index;
    friend aptos_experimental::order_placement;
    friend aptos_experimental::bulk_order_book;
    #[test_only]
    friend aptos_experimental::bulk_order_book_tests;

    // Error codes for various failure scenarios
    const EUNEXPECTED_MATCH_PRICE: u64 = 1;
    const EUNEXPECTED_MATCH_SIZE: u64 = 2;
    const E_REINSERT_ORDER_MISMATCH: u64 = 3;
    const EINVLID_MM_ORDER_REQUEST: u64 = 4;
    const EPRICE_CROSSING: u64 = 5;

    /// Request structure for placing a new bulk order with multiple price levels.
    ///
    /// # Fields:
    /// - `account`: The account placing the order
    /// - `bid_prices`: Vector of bid prices in descending order (best price first)
    /// - `bid_sizes`: Vector of bid sizes corresponding to each price level
    /// - `ask_prices`: Vector of ask prices in ascending order (best price first)
    /// - `ask_sizes`: Vector of ask sizes corresponding to each price level
    /// - `metadata`: Additional metadata for the order
    ///
    /// # Validation:
    /// - Bid prices must be in descending order
    /// - Ask prices must be in ascending order
    /// - All sizes must be greater than 0
    /// - Price and size vectors must have matching lengths.
    /// All bulk orders by default are post-only and will not cross the spread -
    /// GTC and non-reduce-only orders
    enum BulkOrderRequest<M: store + copy + drop> has copy, drop {
        V1 {
            account: address,
            order_sequence_number: u64, // sequence number for order validation
            bid_prices: vector<u64>, // prices for each levels of the order
            bid_sizes: vector<u64>, // sizes for each levels of the order
            ask_prices: vector<u64>, // prices for each levels of the order
            ask_sizes: vector<u64>, // sizes for each levels of the order
            metadata: M
        }
    }

    /// Represents a multi-level order with both bid and ask sides.
    ///
    /// Each side can have multiple price levels with associated sizes. The order maintains
    /// both original and remaining sizes for tracking purposes.
    ///
    /// # Fields:
    /// - `order_id`: Unique identifier for the order
    /// - `account`: Account that placed the order
    /// - `unique_priority_idx`: Priority index for time-based ordering
    /// - `orig_bid_size`: Original total size of all bid levels
    /// - `orig_ask_size`: Original total size of all ask levels
    /// - `total_remaining_bid_size`: Current remaining size of all bid levels
    /// - `total_remaining_ask_size`: Current remaining size of all ask levels
    /// - `bid_prices`: Vector of bid prices in descending order
    /// - `bid_sizes`: Vector of bid sizes corresponding to each price level
    /// - `ask_prices`: Vector of ask prices in ascending order
    /// - `ask_sizes`: Vector of ask sizes corresponding to each price level
    /// - `metadata`: Additional metadata for the order
    enum BulkOrder<M: store + copy + drop> has store, copy, drop {
        V1 {
            order_id: OrderIdType,
            account: address,
            unique_priority_idx: UniqueIdxType,
            order_sequence_number: u64, // sequence number for order validation
            bid_prices: vector<u64>, // prices for each levels of the order
            bid_sizes: vector<u64>, // sizes for each levels of the order
            ask_prices: vector<u64>, // prices for each levels of the order
            ask_sizes: vector<u64>, // sizes for each levels of the order
            metadata: M
        }
    }

    /// Creates a new bulk order with the specified parameters.
    ///
    /// # Arguments:
    /// - `order_id`: Unique identifier for the order
    /// - `unique_priority_idx`: Priority index for time-based ordering
    /// - `order_req`: The bulk order request containing all order details
    /// - `best_bid_price`: Current best bid price in the market
    /// - `best_ask_price`: Current best ask price in the market
    ///
    /// # Returns:
    /// A tuple containing:
    /// - `BulkOrder<M>`: The created bulk order with non-crossing levels
    /// - `vector<u64>`: Cancelled bid prices (levels that crossed the spread)
    /// - `vector<u64>`: Cancelled bid sizes corresponding to cancelled prices
    /// - `vector<u64>`: Cancelled ask prices (levels that crossed the spread)
    /// - `vector<u64>`: Cancelled ask sizes corresponding to cancelled prices
    public(friend) fun new_bulk_order<M: store + copy + drop>(
        order_id: OrderIdType,
        unique_priority_idx: UniqueIdxType,
        order_req: BulkOrderRequest<M>,
        best_bid_price: Option<u64>,
        best_ask_price: Option<u64>,
    ): (BulkOrder<M>, vector<u64>, vector<u64>, vector<u64>, vector<u64>) {
        let BulkOrderRequest::V1 { account, order_sequence_number, bid_prices, bid_sizes, ask_prices, ask_sizes, metadata } = order_req;
        let bid_price_crossing_idx = discard_price_crossing_levels(&bid_prices, best_ask_price, true);
        let ask_price_crossing_idx = discard_price_crossing_levels(&ask_prices, best_bid_price, false);

        // Extract cancelled levels (the ones that were discarded)
        let (cancelled_bid_prices, cancelled_bid_sizes, post_only_bid_prices, post_only_bid_sizes) = if (bid_price_crossing_idx > 0) {
            let post_only_bid_prices = bid_prices.trim(bid_price_crossing_idx);
            let post_only_bid_sizes = bid_sizes.trim(bid_price_crossing_idx);
            (bid_prices.slice(0, bid_price_crossing_idx), bid_sizes.slice(0, bid_price_crossing_idx), post_only_bid_prices, post_only_bid_sizes)
        } else {
            (vector::empty<u64>(), vector::empty<u64>(), bid_prices, bid_sizes)
        };
        let (cancelled_ask_prices, cancelled_ask_sizes, post_only_ask_prices, post_only_ask_sizes) = if (ask_price_crossing_idx > 0) {
            let post_only_ask_prices = ask_prices.trim(ask_price_crossing_idx);
            let post_only_ask_sizes = ask_sizes.trim(ask_price_crossing_idx);
            (ask_prices.slice(0, ask_price_crossing_idx), ask_sizes.slice(0, ask_price_crossing_idx), post_only_ask_prices, post_only_ask_sizes)
        } else {
            (vector::empty<u64>(), vector::empty<u64>(), ask_prices, ask_sizes)
        };
        let bulk_order = BulkOrder::V1 {
            order_id,
            account,
            unique_priority_idx,
            order_sequence_number,
            bid_prices: post_only_bid_prices,
            bid_sizes: post_only_bid_sizes,
            ask_prices: post_only_ask_prices,
            ask_sizes: post_only_ask_sizes,
            metadata
        };
        (bulk_order, cancelled_bid_prices, cancelled_bid_sizes, cancelled_ask_prices, cancelled_ask_sizes)
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
    public fun new_bulk_order_request<M: store + copy + drop>(
        account: address,
        sequence_number: u64,
        bid_prices: vector<u64>,
        bid_sizes: vector<u64>,
        ask_prices: vector<u64>,
        ask_sizes: vector<u64>,
        metadata: M
    ): BulkOrderRequestResponse<M> {
        // Basic length validation
        if (bid_prices.length() != bid_sizes.length()) {
            return new_bulk_order_request_response_rejection(
                std::string::utf8(b"Bid length mismatch")
            );
        };
        if (ask_prices.length() != ask_sizes.length()) {
            return new_bulk_order_request_response_rejection(
                std::string::utf8(b"Ask length mismatch")
            );
        };

        let req = BulkOrderRequest::V1 {
            account,
            order_sequence_number: sequence_number,
            bid_prices,
            bid_sizes,
            ask_prices,
            ask_sizes,
            metadata
        };

        // Ensure bid prices are in descending order and ask prices are in ascending order
        // Check if at least one side has orders
        if (req.bid_sizes.length() == 0 && req.ask_sizes.length() == 0) {
            return new_bulk_order_request_response_rejection(
                std::string::utf8(b"No orders")
            );
        };

        // Check for zero sizes
        if (!validate_not_zero_sizes(&req.bid_sizes)) {
            return new_bulk_order_request_response_rejection(
                std::string::utf8(b"Zero bid size")
            );
        };
        if (!validate_not_zero_sizes(&req.ask_sizes)) {
            return new_bulk_order_request_response_rejection(
                std::string::utf8(b"Zero ask size")
            );
        };

        // Check price ordering
        if (!validate_price_ordering(&req.bid_prices, true)) {
            return new_bulk_order_request_response_rejection(
                std::string::utf8(b"Bid order invalid")
            );
        };
        if (!validate_price_ordering(&req.ask_prices, false)) {
            return new_bulk_order_request_response_rejection(
                std::string::utf8(b"Ask order invalid")
            );
        };

        // Check for price crossing
        if (!validate_no_price_crossing(&req.bid_prices, &req.ask_prices)) {
            return new_bulk_order_request_response_rejection(
                std::string::utf8(b"Price crossing")
            );
        };

        new_bulk_order_request_response_success(req)
    }

    public fun get_account_from_order_request<M: store + copy + drop>(
        order_req: &BulkOrderRequest<M>
    ): address {
        let BulkOrderRequest::V1 { account, .. } = order_req;
        *account
    }

    public fun get_sequence_number_from_order_request<M: store + copy + drop>(
        order_req: &BulkOrderRequest<M>
    ): u64 {
        let BulkOrderRequest::V1 { order_sequence_number: sequence_number, .. } = order_req;
        *sequence_number
    }

    public fun get_sequence_number_from_bulk_order<M: store + copy + drop>(
        order: &BulkOrder<M>
    ): u64 {
        let BulkOrder::V1 { order_sequence_number: sequence_number, .. } = order;
        *sequence_number
    }

    enum BulkOrderPlaceResponse<M: store + copy + drop> has copy, drop {
        Success {
            order: BulkOrder<M>,
            cancelled_bid_prices: vector<u64>,
            cancelled_bid_sizes: vector<u64>,
            cancelled_ask_prices: vector<u64>,
            cancelled_ask_sizes: vector<u64>,
            previous_seq_num: option::Option<u64>,
        },
        Rejection {
            reason: std::string::String,
        }
    }

    struct BulkOrderRequestResponse<M: store + copy + drop> has copy, drop {
        request: option::Option<BulkOrderRequest<M>>,
        rejection_reason: option::Option<std::string::String>,
    }

    public fun new_bulk_order_place_response_success<M: store + copy + drop>(
        order: BulkOrder<M>,
        cancelled_bid_prices: vector<u64>,
        cancelled_bid_sizes: vector<u64>,
        cancelled_ask_prices: vector<u64>,
        cancelled_ask_sizes: vector<u64>,
        previous_seq_num: option::Option<u64>
    ): BulkOrderPlaceResponse<M> {
        BulkOrderPlaceResponse::Success {
            order,
            cancelled_bid_prices,
            cancelled_bid_sizes,
            cancelled_ask_prices,
            cancelled_ask_sizes,
            previous_seq_num,
        }
    }

    public fun new_bulk_order_place_response_rejection<M: store + copy + drop>(
        rejection_reason: std::string::String
    ): BulkOrderPlaceResponse<M> {
        BulkOrderPlaceResponse::Rejection {
            reason: rejection_reason,
        }
    }

    public fun is_success<M: store + copy + drop>(
        response: &BulkOrderPlaceResponse<M>
    ): bool {
        if (response is BulkOrderPlaceResponse::Success) {
            true
        } else {
            false
        }
    }

    public fun is_rejection<M: store + copy + drop>(
        response: &BulkOrderPlaceResponse<M>
    ): bool {
        if (response is BulkOrderPlaceResponse::Rejection) {
            true
        } else {
            false
        }
    }

    public fun is_bulk_order_success_response<M: store + copy + drop>(
        response: &BulkOrderPlaceResponse<M>
    ): bool {
        response is BulkOrderPlaceResponse::Success
    }

    public fun destroy_bulk_order_place_success_response<M: store + copy + drop>(
        response: BulkOrderPlaceResponse<M>
    ): (BulkOrder<M>, vector<u64>, vector<u64>, vector<u64>, vector<u64>, option::Option<u64>) {
        let BulkOrderPlaceResponse::Success { order, cancelled_bid_prices, cancelled_bid_sizes, cancelled_ask_prices, cancelled_ask_sizes, previous_seq_num } = response;
        (order, cancelled_bid_prices, cancelled_bid_sizes, cancelled_ask_prices, cancelled_ask_sizes, previous_seq_num)
    }

    public fun destroy_bulk_order_place_reject_response<M: store + copy + drop>(
        response: BulkOrderPlaceResponse<M>
    ): std::string::String {
        let BulkOrderPlaceResponse::Rejection { reason } = response;
        reason
    }

    public fun new_bulk_order_request_response_success<M: store + copy + drop>(
        request: BulkOrderRequest<M>
    ): BulkOrderRequestResponse<M> {
        BulkOrderRequestResponse {
            request: option::some(request),
            rejection_reason: option::none(),
        }
    }

    public fun new_bulk_order_request_response_rejection<M: store + copy + drop>(
        rejection_reason: std::string::String
    ): BulkOrderRequestResponse<M> {
        BulkOrderRequestResponse {
            request: option::none(),
            rejection_reason: option::some(rejection_reason),
        }
    }

    public fun destroy_bulk_order_request_response<M: store + copy + drop>(
        response: BulkOrderRequestResponse<M>
    ): (option::Option<BulkOrderRequest<M>>, option::Option<std::string::String>) {
        let BulkOrderRequestResponse { request, rejection_reason } = response;
        (request, rejection_reason)
    }

    /// Validates that all sizes in the vector are greater than 0.
    ///
    /// # Arguments:
    /// - `sizes`: Vector of sizes to validate
    ///
    fun validate_not_zero_sizes(sizes: &vector<u64>): bool {
        let i = 0;
        while (i < sizes.length()) {
            if (sizes[i] == 0) {
                return false;
            };
            i += 1;
        };
        true
    }

    /// Validates that prices are in the correct order (descending for bids, ascending for asks).
    ///
    /// # Arguments:
    /// - `prices`: Vector of prices to validate
    /// - `is_descending`: True if prices should be in descending order, false for ascending
    ///
    fun validate_price_ordering(
        prices: &vector<u64>,
        is_descending: bool
    ): bool {
        let i = 0;
        if (prices.length() == 0) {
            return true; // No prices to validate
        };
        while (i < prices.length() - 1) {
            if (is_descending) {
                if (prices[i] <= prices[i + 1]) {
                    return false;
                };
            } else {
                if (prices[i] >= prices[i + 1]) {
                    return false;
                };
            };
            i += 1;
        };
        true
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
    fun validate_no_price_crossing(
        bid_prices: &vector<u64>,
        ask_prices: &vector<u64>
    ): bool {
        if (bid_prices.length() > 0 && ask_prices.length() > 0) {
            let highest_bid = bid_prices[0]; // First element is highest (descending order)
            let lowest_ask = ask_prices[0];  // First element is lowest (ascending order)
            return highest_bid < lowest_ask;
        };
        true
    }

    fun discard_price_crossing_levels(
        prices: &vector<u64>,
        best_price: Option<u64>,
        is_bid: bool,
    ): u64 {
        // Discard bid levels that are >= best ask price
        let i = 0;
        if (best_price != option::none()) {
            let best_price = best_price.destroy_some();
            while (i < prices.length()) {
                if (is_bid && prices[i] < best_price) {
                    break;
                } else if (!is_bid && prices[i] > best_price) {
                    break;
                };
                i += 1;
            };
        };
        i // Return the index of the first non-crossing level
    }

    #[lint::skip(needless_mutable_reference)]
    // Creates a new single bulk order match result.
    //
    // Arguments:
    // - order: Reference to the bulk order being matched
    // - is_bid: True if matching against bid side, false for ask side
    // - matched_size: Size that was matched in this operation
    //
    // Returns:
    // A `SingleBulkOrderMatch` containing the match details.
    public(friend) fun new_bulk_order_match<M: store + copy + drop>(
        order: &mut BulkOrder<M>,
        is_bid: bool,
        matched_size: u64
    ): OrderMatch<M> {
        let (price, remaining_size) = if (is_bid) {
            (order.bid_prices[0], order.bid_sizes[0]  - matched_size)
        } else {
            (order.ask_prices[0], order.ask_sizes[0] - matched_size)
        };
        new_order_match<M>(
            new_bulk_order_match_details<M>(
                order.get_order_id(),
                order.get_account(),
                order.get_unique_priority_idx(),
                price,
                remaining_size,
                is_bid,
                get_sequence_number_from_bulk_order(order),
                order.metadata,
            ),
            matched_size
        )
    }

    public(friend)  fun get_total_remaining_size<M: store + copy + drop>(
        self: &BulkOrder<M>,
        is_bid: bool,
    ): u64 {
        if (is_bid) {
            self.bid_sizes.fold(0, |acc, size| acc + size)
        } else {
            self.ask_sizes.fold(0, |acc, size| acc + size)
        }
    }

    /// Gets the unique priority index of a bulk order.
    ///
    /// # Arguments:
    /// - `self`: Reference to the bulk order
    ///
    /// # Returns:
    /// The unique priority index for time-based ordering.
    public(friend) fun get_unique_priority_idx<M: store + copy + drop>(
        self: &BulkOrder<M>,
    ): UniqueIdxType {
        self.unique_priority_idx
    }

    /// Gets the order ID of a bulk order.
    ///
    /// # Arguments:
    /// - `self`: Reference to the bulk order
    ///
    /// # Returns:
    /// The unique order identifier.
    public(friend) fun get_order_id<M: store + copy + drop>(
        self: &BulkOrder<M>,
    ): OrderIdType {
        self.order_id
    }

    /// Gets the account of a bulk order.
    ///
    /// # Arguments:
    /// - `self`: Reference to the bulk order
    ///
    /// # Returns:
    /// The account that placed the order.
    public(friend) fun get_account<M: store + copy + drop>(
        self: &BulkOrder<M>,
    ): address {
        self.account
    }

    public(friend) fun get_sequence_number<M: store + copy + drop>(
        self: &BulkOrder<M>,
    ): u64 {
        self.order_sequence_number
    }

    /// Gets the active price for a specific side of a bulk order.
    ///
    /// # Arguments:
    /// - `self`: Reference to the bulk order
    /// - `is_bid`: True to get bid price, false for ask price
    ///
    /// # Returns:
    /// An option containing the active price if available, none otherwise.
    public(friend) fun get_active_price<M: store + copy + drop>(
        self: &BulkOrder<M>,
        is_bid: bool,
    ): Option<u64> {
        let prices = if (is_bid) { self.bid_prices } else { self.ask_prices };
        if (prices.length() == 0) {
            option::none() // No active price level
        } else {
            option::some(prices[0]) // Return the first price level
        }
    }

    public(friend) fun get_all_prices<M: store + copy + drop>(
        self: &BulkOrder<M>,
        is_bid: bool,
    ): vector<u64> {
        if (is_bid) {
            self.bid_prices
        } else {
            self.ask_prices
        }
    }

    public(friend) fun get_all_sizes<M: store + copy + drop>(
        self: &BulkOrder<M>,
        is_bid: bool,
    ): vector<u64> {
        if (is_bid) {
            self.bid_sizes
        } else {
            self.ask_sizes
        }
    }

    /// Gets the active size for a specific side of a bulk order.
    ///
    /// # Arguments:
    /// - `self`: Reference to the bulk order
    /// - `is_bid`: True to get bid size, false for ask size
    ///
    /// # Returns:
    /// An option containing the active size if available, none otherwise.
    public(friend) fun get_active_size<M: store + copy + drop>(
        self: &BulkOrder<M>,
        is_bid: bool,
    ): Option<u64> {
        let sizes = if (is_bid) { self.bid_sizes } else { self.ask_sizes };
        if (sizes.length() == 0) {
            option::none() // No active size level
        } else {
            option::some(sizes[0]) // Return the first size level
        }
    }

    /// Sets a bulk order to empty state, clearing all price levels and sizes.
    ///
    /// This function is used during order cancellation to clear the order state
    /// while preserving the order ID for potential reuse.


    /// Validates that a reinsertion request is consistent with the original order.
    ///


    /// Reinserts an order into a bulk order.
    ///
    /// This function adds the reinserted order's price and size to the appropriate side
    /// of the bulk order. If the price already exists at the first level, it increases
    /// the size; otherwise, it inserts the new price level at the front.
    ///
    /// # Arguments:
    /// - `self`: Mutable reference to the bulk order
    /// - `other`: Reference to the order result to reinsert
    public(friend) fun reinsert_order<M: store + copy + drop>(
        self: &mut BulkOrder<M>,
        other: &OrderMatchDetails<M>,
    ) {
        // Reinsert the order into the bulk order
        let (prices, sizes) = if (other.is_bid_from_match_details()) {
            (&mut self.bid_prices, &mut self.bid_sizes)
        } else {
            (&mut self.ask_prices, &mut self.ask_sizes)
        };
        // Reinsert the price and size at the front of the respective vectors - if the price already exists, we ensure that
        // it is same as the reinsertion price and we just increase the size
        // If the price does not exist, we insert it at the front.
        if (prices.length() > 0 && prices[0] == other.get_price_from_match_details()) {
            sizes[0] += other.get_remaining_size_from_match_details(); // Increase the size at the first price level
        } else {
            prices.insert(0, other.get_price_from_match_details()); // Insert the new price at the front
            sizes.insert(0, other.get_remaining_size_from_match_details()); // Insert the new size at the front
        }
    }

    /// Matches an order and returns the next active price and size.
    ///
    /// This function reduces the size at the first price level by the matched size.
    /// If the first level becomes empty, it is removed and the next level becomes active.
    ///
    /// # Arguments:
    /// - `self`: Mutable reference to the bulk order
    /// - `is_bid`: True if matching against bid side, false for ask side
    /// - `matched_size`: Size that was matched in this operation
    ///
    /// # Returns:
    /// A tuple containing the next active price and size as options.
    ///
    /// # Aborts:
    /// - If the matched size exceeds the available size at the first level
    public(friend) fun match_order_and_get_next<M: store + copy + drop>(
        self: &mut BulkOrder<M>,
        is_bid: bool,
        matched_size: u64,
    ): (Option<u64>, Option<u64>) {
        let (prices, sizes) = if (is_bid) {
            (&mut self.bid_prices, &mut self.bid_sizes)
        } else {
            (&mut self.ask_prices, &mut self.ask_sizes)
        };
        assert!(matched_size <= sizes[0], EUNEXPECTED_MATCH_SIZE); // Ensure the remaining size is not more than the size at the first price level
        sizes[0] -= matched_size; // Decrease the size at the first price level by the matched size
        if (sizes[0] == 0) {
            // If the size at the first price level is now 0, remove this price level
            prices.remove(0);
            sizes.remove(0);
        };
        if (sizes.length() == 0) {
            (option::none(), option::none()) // No active price or size left
        } else {
            (option::some(prices[0]), option::some(sizes[0])) // Return the next active price and size
        }
    }

    /// Sets the bulk order to empty state by clearing all sizes.
    ///
    /// # Arguments:
    /// - `self`: Mutable reference to the bulk order
    public(friend) fun set_empty<M: store + copy + drop>(
        self: &mut BulkOrder<M>
    ) {
        self.bid_sizes = vector::empty();
        self.ask_sizes = vector::empty();
        self.bid_prices = vector::empty();
        self.ask_prices = vector::empty();
    }

    public fun destroy_bulk_order<M: store + copy + drop>(
        self: BulkOrder<M>
    ): (
        OrderIdType,
        address,
        UniqueIdxType,
        u64,
        vector<u64>,
        vector<u64>,
        vector<u64>,
        vector<u64>,
        M
    ) {
        let BulkOrder::V1 {
            order_id,
            account,
            unique_priority_idx,
            order_sequence_number,
            bid_prices,
            bid_sizes,
            ask_prices,
            ask_sizes,
            metadata
        } = self;
        (
            order_id,
            account,
            unique_priority_idx,
            order_sequence_number,
            bid_prices,
            bid_sizes,
            ask_prices,
            ask_sizes,
            metadata
        )
    }
}
