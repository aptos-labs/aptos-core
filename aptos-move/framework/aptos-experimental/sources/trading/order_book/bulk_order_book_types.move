/// # Bulk Order Book Types Module
///
/// This module defines the core data structures and types used by the bulk order book system.
/// It provides the foundational types for representing multi-level orders, order results,
/// and match operations.
///
/// ## Key Data Structures:
///
/// ### 1. BulkOrder
/// Represents a multi-level order with both bid and ask sides. Each side can have multiple
/// price levels with associated sizes.
///
/// ### 2. BulkOrderResult
/// Represents the result of an order matching operation, containing details about the
/// matched order including price, size, and remaining quantities.
///
/// ### 3. SingleBulkOrderMatch
/// Represents a single match between a taker order and a maker order, containing the
/// matched order details and the size that was matched.
///
/// ## Core Functionality:
///
/// - **Order Creation**: Functions to create new bulk orders and order results
/// - **Order Matching**: Logic for matching orders and updating remaining quantities
/// - **Order Reinsertion**: Support for reinserting matched portions back into the order book
/// - **Order Validation**: Validation functions for order consistency and correctness
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
///
/// // Create a match result
/// let match_result = bulk_order_book_types::new_single_bulk_order_match(
///     &order,
///     true,  // is_bid
///     10     // matched_size
/// );
///
/// // Extract match details
/// let (order_result, matched_size) = match_result.destroy_single_bulk_order_match();
/// ```
/// (work in progress)
module aptos_experimental::bulk_order_book_types {
    use std::option;
    use std::option::Option;
    use std::vector;
    use aptos_experimental::order_book_types::{OrderIdType, UniqueIdxType};
    friend aptos_experimental::active_order_book;
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

    /// Represents the result of an order matching operation.
    ///
    /// Contains details about a matched order including the price, original size,
    /// remaining size, and whether it was a bid or ask order.
    ///
    /// # Fields:
    /// - `order_id`: Unique identifier for the order
    /// - `account`: Account that placed the order
    /// - `unique_priority_idx`: Priority index for time-based ordering
    /// - `price`: Price at which the order was matched
    /// - `orig_size`: Original size of the order
    /// - `remaining_size`: Remaining size after the match
    /// - `is_bid`: True if this was a bid order, false if ask order
    enum BulkOrderResult has copy, drop {
        V1 {
            order_id: OrderIdType,
            account: address,
            unique_priority_idx: UniqueIdxType,
            price: u64,
            orig_size: u64,
            remaining_size: u64,
            is_bid: bool,
        }
    }

    /// Represents a single match between a taker order and a maker order.
    ///
    /// Contains the matched order details and the size that was matched in this
    /// particular match operation.
    ///
    /// # Fields:
    /// - `order`: The matched order result
    /// - `matched_size`: The size that was matched in this operation
    enum SingleBulkOrderMatch has drop, copy {
        V1 {
            order: BulkOrderResult,
            matched_size: u64
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
        account: address,
        unique_priority_idx: UniqueIdxType,
        bid_prices: vector<u64>,
        bid_sizes: vector<u64>,
        ask_prices: vector<u64>,
        ask_sizes: vector<u64>
    ): BulkOrder {
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
            bid_prices,
            bid_sizes,
            ask_prices,
            ask_sizes
        }
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
    public(friend) fun new_single_bulk_order_match(
        order: &BulkOrder,
        is_bid: bool,
        matched_size: u64
    ): SingleBulkOrderMatch {
        let (price, orig_size, remaining_size) = if (is_bid) {
            (order.bid_prices[0], order.orig_bid_size, order.total_remaining_bid_size - matched_size)
        } else {
            (order.ask_prices[0], order.orig_ask_size, order.total_remaining_ask_size - matched_size)
        };
        SingleBulkOrderMatch::V1 {
            order: BulkOrderResult::V1 {
                order_id: order.order_id,
                account: order.account,
                unique_priority_idx: order.unique_priority_idx,
                price,
                orig_size,
                remaining_size,
                is_bid
            },
            matched_size
        }
    }

    /// Creates a new bulk order result with the specified parameters.
    ///
    /// # Arguments:
    /// - `order_id`: Unique identifier for the order
    /// - `account`: Account that placed the order
    /// - `unique_priority_idx`: Priority index for time-based ordering
    /// - `price`: Price at which the order was matched
    /// - `orig_size`: Original size of the order
    /// - `remaining_size`: Remaining size after the match
    /// - `is_bid`: True if this was a bid order, false if ask order
    ///
    /// # Returns:
    /// A new `BulkOrderResult` instance.
    public(friend) fun new_bulk_order_result(
        order_id: OrderIdType,
        account: address,
        unique_priority_idx: UniqueIdxType,
        price: u64,
        orig_size: u64,
        remaining_size: u64,
        is_bid: bool
    ): BulkOrderResult {
        BulkOrderResult::V1 {
            order_id,
            account,
            unique_priority_idx,
            price,
            orig_size,
            remaining_size,
            is_bid
        }
    }

    /// Destroys a single bulk order match and returns its components.
    ///
    /// # Arguments:
    /// - `self`: The single bulk order match to destroy
    ///
    /// # Returns:
    /// A tuple containing the bulk order result and the matched size.
    public(friend) fun destroy_single_bulk_order_match(
        self: SingleBulkOrderMatch,
    ): (BulkOrderResult, u64) {
        let SingleBulkOrderMatch::V1 { order, matched_size } = self;
        (order, matched_size)
    }

    /// Destroys a bulk order result and returns its components.
    ///
    /// # Arguments:
    /// - `self`: The bulk order result to destroy
    ///
    /// # Returns:
    /// A tuple containing all the order result fields.
    public(friend) fun destroy_bulk_order_result(
        self: BulkOrderResult,
    ): (OrderIdType, address, UniqueIdxType, u64, u64, u64, bool) {
        let BulkOrderResult::V1 {
            order_id,
            account,
            unique_priority_idx,
            price,
            orig_size,
            remaining_size,
            is_bid
        } = self;
        (order_id, account, unique_priority_idx, price, orig_size, remaining_size, is_bid)
    }

    /// Gets the matched size from a single bulk order match.
    ///
    /// # Arguments:
    /// - `self`: Reference to the single bulk order match
    ///
    /// # Returns:
    /// The size that was matched in this operation.
    public(friend) fun get_matched_size(
        self: &SingleBulkOrderMatch,
    ): u64 {
        self.matched_size
    }

    /// Checks if a bulk order has remaining orders on the specified side.
    ///
    /// # Arguments:
    /// - `self`: Reference to the bulk order
    /// - `is_bid`: True to check bid side, false for ask side
    ///
    /// # Returns:
    /// True if there are remaining orders on the specified side, false otherwise.
    public(friend) fun is_remaining_order(
        self: &BulkOrder,
        is_bid: bool,
    ): bool {
        let sizes = if (is_bid) { self.bid_sizes } else { self.ask_sizes };
        return sizes.length() > 0 && sizes[0] > 0 // Check if the first price level has a non-zero size
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
    ///
    /// # Arguments:
    /// - `self`: Mutable reference to the bulk order
    public(friend) fun set_empty(
        self: &mut BulkOrder,
    ) {
        self.bid_prices = vector::empty<u64>();
        self.bid_sizes = vector::empty<u64>();
        self.ask_prices = vector::empty<u64>();
        self.ask_sizes = vector::empty<u64>();
        self.total_remaining_bid_size = 0;
        self.total_remaining_ask_size = 0;
        self.orig_bid_size = 0;
        self.orig_ask_size = 0;
    }

    /// Validates that a reinsertion request is consistent with the original order.
    ///
    /// This function ensures that the order being reinserted matches the original order
    /// in terms of order ID, account, priority index, bid/ask status, and price.
    ///
    /// # Arguments:
    /// - `self`: Reference to the reinsertion order result
    /// - `other`: Reference to the original order result
    ///
    /// # Aborts:
    /// - If any validation fails (order ID, account, priority index, bid/ask status, price mismatch)
    public(friend) fun validate_reinsertion_request(
        self: &BulkOrderResult,
        other: &BulkOrderResult,
    ) {
        assert!(self.order_id == other.order_id, E_REINSERT_ORDER_MISMATCH); // Ensure the order IDs match
        assert!(self.account == other.account, E_REINSERT_ORDER_MISMATCH); // Ensure the accounts match
        assert!(self.unique_priority_idx == other.unique_priority_idx, E_REINSERT_ORDER_MISMATCH); // Ensure the unique priority indices match
        assert!(self.is_bid == other.is_bid, E_REINSERT_ORDER_MISMATCH); // Ensure the bid/ask status matches
        assert!(self.price == other.price, E_REINSERT_ORDER_MISMATCH); // Ensure the prices match
    }

    /// Gets the account from a bulk order result.
    ///
    /// # Arguments:
    /// - `self`: Reference to the bulk order result
    ///
    /// # Returns:
    /// The account that placed the order.
    public(friend) fun get_account_from_order_result(
        self: &BulkOrderResult,
    ): address {
        self.account
    }

    /// Gets the order ID from a bulk order result.
    ///
    /// # Arguments:
    /// - `self`: Reference to the bulk order result
    ///
    /// # Returns:
    /// The unique order identifier.
    public(friend) fun get_order_id_from_order_result(
        self: &BulkOrderResult,
    ): OrderIdType {
        self.order_id
    }

    /// Reinserts an order into a bulk order.
    ///
    /// This function adds the reinserted order's price and size to the appropriate side
    /// of the bulk order. If the price already exists at the first level, it increases
    /// the size; otherwise, it inserts the new price level at the front.
    ///
    /// # Arguments:
    /// - `self`: Mutable reference to the bulk order
    /// - `other`: Reference to the order result to reinsert
    public(friend) fun reinsert_order(
        self: &mut BulkOrder,
        other: &BulkOrderResult,
    ) {
        // Reinsert the order into the bulk order
        let (prices, sizes, total_remaining) = if (other.is_bid) {
            (&mut self.bid_prices, &mut self.bid_sizes, &mut self.total_remaining_bid_size)
        } else {
            (&mut self.ask_prices, &mut self.ask_sizes, &mut self.total_remaining_ask_size)
        };
        // Reinsert the price and size at the front of the respective vectors - if the price already exists, we ensure that
        // it is same as the reinsertion price and we just increase the size
        // If the price does not exist, we insert it at the front.
        if (prices.length() > 0 && prices[0] == other.price) {
            sizes[0] += other.remaining_size; // Increase the size at the first price level
            *total_remaining += other.remaining_size; // Increase the total remaining size
        } else {
            prices.insert(0, other.price); // Insert the new price at the front
            sizes.insert(0, other.remaining_size); // Insert the new size at the front
            *total_remaining += other.remaining_size; // Set the total remaining size to the new size
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

    // Getter functions for BulkOrderResult
    /// Gets the order ID from a bulk order result.
    ///
    /// # Arguments:
    /// - `self`: Reference to the bulk order result
    ///
    /// # Returns:
    /// The unique order identifier.
    public(friend) fun get_order_id_from_result(
        self: &BulkOrderResult
    ): OrderIdType {
        self.order_id
    }

    /// Gets the account from a bulk order result.
    ///
    /// # Arguments:
    /// - `self`: Reference to the bulk order result
    ///
    /// # Returns:
    /// The account that placed the order.
    public(friend) fun get_account_from_result(
        self: &BulkOrderResult
    ): address {
        self.account
    }

    /// Gets the unique priority index from a bulk order result.
    ///
    /// # Arguments:
    /// - `self`: Reference to the bulk order result
    ///
    /// # Returns:
    /// The unique priority index for time-based ordering.
    public(friend) fun get_unique_priority_idx_from_result(
        self: &BulkOrderResult
    ): UniqueIdxType {
        self.unique_priority_idx
    }

    /// Gets the price from a bulk order result.
    ///
    /// # Arguments:
    /// - `self`: Reference to the bulk order result
    ///
    /// # Returns:
    /// The price at which the order was matched.
    public(friend) fun get_price_from_result(
        self: &BulkOrderResult
    ): u64 {
        self.price
    }

    /// Gets the original size from a bulk order result.
    ///
    /// # Arguments:
    /// - `self`: Reference to the bulk order result
    ///
    /// # Returns:
    /// The original size of the order.
    public(friend) fun get_orig_size_from_result(
        self: &BulkOrderResult
    ): u64 {
        self.orig_size
    }

    /// Gets the remaining size from a bulk order result.
    ///
    /// # Arguments:
    /// - `self`: Reference to the bulk order result
    ///
    /// # Returns:
    /// The remaining size after the match.
    public(friend) fun get_remaining_size_from_result(
        self: &BulkOrderResult
    ): u64 {
        self.remaining_size
    }

    /// Checks if a bulk order result represents a bid order.
    ///
    /// # Arguments:
    /// - `self`: Reference to the bulk order result
    ///
    /// # Returns:
    /// True if this was a bid order, false if ask order.
    public(friend) fun is_bid_from_result(
        self: &BulkOrderResult
    ): bool {
        self.is_bid
    }
}
