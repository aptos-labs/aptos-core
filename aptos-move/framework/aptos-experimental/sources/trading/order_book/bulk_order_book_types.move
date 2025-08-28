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
        good_till_cancelled, bulk_order_book_type, new_order_match_details, new_order_match
    };
    friend aptos_experimental::price_time_index;
    friend aptos_experimental::order_book;
    friend aptos_experimental::pending_order_book_index;
    friend aptos_experimental::market;
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
    ///
    /// # Validation:
    /// - Bid prices must be in descending order
    /// - Ask prices must be in ascending order
    /// - All sizes must be greater than 0
    /// - Price and size vectors must have matching lengths.
    /// All bulk orders by default are post-only and will not cross the spread -
    /// GTC and non-reduce-only orders
    enum BulkOrderRequest has copy, drop {
        V1 {
            account: address,
            bid_prices: vector<u64>, // prices for each levels of the order
            bid_sizes: vector<u64>, // sizes for each levels of the order
            ask_prices: vector<u64>, // prices for each levels of the order
            ask_sizes: vector<u64>, // sizes for each levels of the order
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
    enum BulkOrder has store, copy, drop {
        V1 {
            order_id: OrderIdType,
            account: address,
            unique_priority_idx: UniqueIdxType,
            orig_bid_size: u64, // original size of the bid order
            orig_ask_size: u64, // original size of the ask order
            total_remaining_bid_size: u64, // remaining size of the bid order
            total_remaining_ask_size: u64, // remaining size of the ask order
            bid_prices: vector<u64>, // prices for each levels of the order
            bid_sizes: vector<u64>, // sizes for each levels of the order
            ask_prices: vector<u64>, // prices for each levels of the order
            ask_sizes: vector<u64>, // sizes for each levels of the order
        }
    }

    /// Creates a new bulk order with the specified parameters.
    ///
    /// # Arguments:
    /// - `order_id`: Unique identifier for the order
    /// - `account`: Account placing the order
    /// - `unique_priority_idx`: Priority index for time-based ordering
    /// - `bid_prices`: Vector of bid prices in descending order
    /// - `bid_sizes`: Vector of bid sizes corresponding to each price level
    /// - `ask_prices`: Vector of ask prices in ascending order
    /// - `ask_sizes`: Vector of ask sizes corresponding to each price level
    ///
    /// # Returns:
    /// A new `BulkOrder` instance with calculated original and remaining sizes.
    public(friend) fun new_bulk_order(
        order_id: OrderIdType,
        unique_priority_idx: UniqueIdxType,
        order_req: BulkOrderRequest, best_bid_price: Option<u64>, best_ask_price: Option<u64>,
    ): BulkOrder {
        let BulkOrderRequest::V1 { account, bid_prices, bid_sizes, ask_prices, ask_sizes } = order_req;
        let bid_price_crossing_idx = discard_price_crossing_levels(&bid_prices, best_ask_price, true);
        let ask_price_crossing_idx = discard_price_crossing_levels(&ask_prices, best_bid_price, false);
        let (post_only_bid_prices, post_only_bid_sizes) = if (bid_price_crossing_idx > 0) {
            (bid_prices.slice(bid_price_crossing_idx, bid_prices.length()),
            bid_sizes.slice(bid_price_crossing_idx, bid_sizes.length()))
        } else {
            (bid_prices, bid_sizes)
        };
        let (post_only_ask_prices, post_only_ask_sizes) = if (ask_price_crossing_idx > 0) {
            (ask_prices.slice(ask_price_crossing_idx, ask_prices.length()),
            ask_sizes.slice(ask_price_crossing_idx, ask_sizes.length()))
        } else {
            (ask_prices, ask_sizes)
        };
        // Original bid and ask sizes are the sum of the sizes at each price level
        let orig_bid_size = bid_sizes.fold(0, |acc, size| acc + size);
        let orig_ask_size = ask_sizes.fold(0, |acc, size| acc + size);
        BulkOrder::V1 {
            order_id,
            account,
            unique_priority_idx,
            orig_bid_size,
            orig_ask_size,
            total_remaining_bid_size: orig_bid_size, // Initially, the remaining size is the original size
            total_remaining_ask_size: orig_ask_size, // Initially, the remaining size is the original size
            bid_prices: post_only_bid_prices,
            bid_sizes: post_only_bid_sizes,
            ask_prices: post_only_ask_prices,
            ask_sizes: post_only_ask_sizes
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
        let req = BulkOrderRequest::V1 {
            account,
            bid_prices,
            bid_sizes,
            ask_prices,
            ask_sizes
        };
        validate_bulk_order_request(&req);
        req
    }

    public fun get_account_from_order_request(
        order_req: &BulkOrderRequest
    ): address {
        let BulkOrderRequest::V1 { account, .. } = order_req;
        *account
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
    fun validate_bulk_order_request(
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
                    break; // All remaining levels are removed
                } else if (!is_bid && prices[i] > best_price) {
                    break; // All remaining levels are removed
                };
                i += 1;
            };
        };
        i // Return the index of the first non-crossing level
    }


    /// Creates a new single bulk order match result.
    ///
    /// # Arguments:
    /// - `order`: Reference to the bulk order being matched
    /// - `is_bid`: True if matching against bid side, false for ask side
    /// - `matched_size`: Size that was matched in this operation
    ///
    /// # Returns:
    /// A `SingleBulkOrderMatch` containing the match details.
    public(friend) fun new_bulk_order_match<M: store + copy + drop>(
        order: &mut BulkOrder,
        is_bid: bool,
        matched_size: u64
    ): OrderMatch<M> {
        // print( &order.total_remaining_bid_size);
        let (price, orig_size, remaining_size) = if (is_bid) {
            (order.bid_prices[0], order.orig_bid_size, order.total_remaining_bid_size - matched_size)
        } else {
            (order.ask_prices[0], order.orig_ask_size, order.total_remaining_ask_size - matched_size)
        };
        new_order_match<M>(
            new_order_match_details<M>(
                order.get_order_id(),
                order.get_account(),
                option::none(),
                order.get_unique_priority_idx(),
                price,
                orig_size,
                remaining_size,
                is_bid,
                good_till_cancelled(),
                option::none(),
                bulk_order_book_type(),
            ),
            matched_size
        )
    }

    public(friend)  fun get_total_remaining_size(
        self: &BulkOrder,
        is_bid: bool,
    ): u64 {
        if (is_bid) {
            self.total_remaining_bid_size
        } else {
            self.total_remaining_ask_size
        }
    }

    /// Gets the unique priority index of a bulk order.
    ///
    /// # Arguments:
    /// - `self`: Reference to the bulk order
    ///
    /// # Returns:
    /// The unique priority index for time-based ordering.
    public(friend) fun get_unique_priority_idx(
        self: &BulkOrder,
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
    public(friend) fun get_order_id(
        self: &BulkOrder,
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
    public(friend) fun get_account(
        self: &BulkOrder,
    ): address {
        self.account
    }

    /// Gets the active price for a specific side of a bulk order.
    ///
    /// # Arguments:
    /// - `self`: Reference to the bulk order
    /// - `is_bid`: True to get bid price, false for ask price
    ///
    /// # Returns:
    /// An option containing the active price if available, none otherwise.
    public(friend) fun get_active_price(
        self: &BulkOrder,
        is_bid: bool,
    ): Option<u64> {
        let prices = if (is_bid) { self.bid_prices } else { self.ask_prices };
        if (prices.length() == 0) {
            option::none() // No active price level
        } else {
            option::some(prices[0]) // Return the first price level
        }
    }

    public(friend) fun get_all_prices(
        self: &BulkOrder,
        is_bid: bool,
    ): vector<u64> {
        if (is_bid) {
            self.bid_prices
        } else {
            self.ask_prices
        }
    }

    public(friend) fun get_all_sizes(
        self: &BulkOrder,
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
    public(friend) fun get_active_size(
        self: &BulkOrder,
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
        self: &mut BulkOrder,
        other: &OrderMatchDetails<M>,
    ) {
        // Reinsert the order into the bulk order
        let (prices, sizes, total_remaining) = if (other.is_bid_from_match_details()) {
            (&mut self.bid_prices, &mut self.bid_sizes, &mut self.total_remaining_bid_size)
        } else {
            (&mut self.ask_prices, &mut self.ask_sizes, &mut self.total_remaining_ask_size)
        };
        // Reinsert the price and size at the front of the respective vectors - if the price already exists, we ensure that
        // it is same as the reinsertion price and we just increase the size
        // If the price does not exist, we insert it at the front.
        if (prices.length() > 0 && prices[0] == other.get_price_from_match_details()) {
            sizes[0] += other.get_remaining_size_from_match_details(); // Increase the size at the first price level
            *total_remaining += other.get_remaining_size_from_match_details(); // Increase the total remaining size
        } else {
            prices.insert(0, other.get_price_from_match_details()); // Insert the new price at the front
            sizes.insert(0, other.get_remaining_size_from_match_details()); // Insert the new size at the front
            *total_remaining += other.get_remaining_size_from_match_details(); // Set the total remaining size to the new size
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
    public(friend) fun match_order_and_get_next(
        self: &mut BulkOrder,
        is_bid: bool,
        matched_size: u64,
    ): (Option<u64>, Option<u64>) {
        let (prices, sizes, total_remaining) = if (is_bid) {
            (&mut self.bid_prices, &mut self.bid_sizes, &mut self.total_remaining_bid_size)
        } else {
            (&mut self.ask_prices, &mut self.ask_sizes, &mut self.total_remaining_ask_size)
        };
        assert!(matched_size <= sizes[0], EUNEXPECTED_MATCH_SIZE); // Ensure the remaining size is not more than the size at the first price level
        sizes[0] -= matched_size; // Decrease the size at the first price level by the matched size
        *total_remaining -= matched_size; // Decrease the total remaining size
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
    public(friend) fun set_empty(
        self: &mut BulkOrder
    ) {
        self.total_remaining_bid_size = 0;
        self.total_remaining_ask_size = 0;
        self.bid_sizes = vector::empty();
        self.ask_sizes = vector::empty();
        self.bid_prices = vector::empty();
        self.ask_prices = vector::empty();
    }
}
