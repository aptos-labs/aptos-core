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
/// ## Usage Example:
/// ```move
/// // Create a new bulk order
/// let order = bulk_order_types::new_bulk_order(
///     order_request,
///     order_id,
///     unique_priority_idx,
///     creation_time_micros
/// );
/// ```
/// (work in progress)
module aptos_trading::bulk_order_types {
    use std::option;
    use std::option::Option;
    use std::vector;
    use aptos_trading::order_book_types::{OrderId, IncreasingIdx};
    use aptos_trading::order_match_types::{
        OrderMatch,
        new_bulk_order_match_details,
        new_order_match
    };

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
    /// Bulk orders do not support TimeInForce options and behave as maker orders only
    enum BulkOrderRequest<M: store + copy + drop> has store, copy, drop {
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
    /// - `unique_priority_idx`: Priority index for time-based ordering
    enum BulkOrder<M: store + copy + drop> has store, copy, drop {
        V1 {
            order_request: BulkOrderRequest<M>,
            order_id: OrderId,
            unique_priority_idx: IncreasingIdx,
            creation_time_micros: u64
        }
    }

    enum BulkOrderPlaceResponse<M: store + copy + drop> has copy, drop {
        Success_V1 {
            order: BulkOrder<M>,
            cancelled_bid_prices: vector<u64>,
            cancelled_bid_sizes: vector<u64>,
            cancelled_ask_prices: vector<u64>,
            cancelled_ask_sizes: vector<u64>,
            previous_seq_num: option::Option<u64>
        },
        Rejection_V1 {
            account: address,
            sequence_number: u64,
            existing_sequence_number: u64
        }
    }

    /// Creates a new bulk order with the specified parameters.
    ///
    /// # Arguments:
    /// - `order_request`: The bulk order request containing all order details
    /// - `order_id`: Unique identifier for the order
    /// - `unique_priority_idx`: Priority index for time-based ordering
    /// - `creation_time_micros`: Creation time of the order
    ///
    /// Does no validation itself.
    public fun new_bulk_order<M: store + copy + drop>(
        order_request: BulkOrderRequest<M>,
        order_id: OrderId,
        unique_priority_idx: IncreasingIdx,
        creation_time_micros: u64
    ): BulkOrder<M> {
        BulkOrder::V1 {
            order_request,
            order_id,
            unique_priority_idx,
            creation_time_micros
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
    /// Does no validation itself.
    public fun new_bulk_order_request<M: store + copy + drop>(
        account: address,
        sequence_number: u64,
        bid_prices: vector<u64>,
        bid_sizes: vector<u64>,
        ask_prices: vector<u64>,
        ask_sizes: vector<u64>,
        metadata: M
    ): BulkOrderRequest<M> {
        BulkOrderRequest::V1 {
            account,
            order_sequence_number: sequence_number,
            bid_prices,
            bid_sizes,
            ask_prices,
            ask_sizes,
            metadata
        }
    }

    public fun new_bulk_order_place_response_success<M: store + copy + drop>(
        order: BulkOrder<M>,
        cancelled_bid_prices: vector<u64>,
        cancelled_bid_sizes: vector<u64>,
        cancelled_ask_prices: vector<u64>,
        cancelled_ask_sizes: vector<u64>,
        previous_seq_num: option::Option<u64>
    ): BulkOrderPlaceResponse<M> {
        BulkOrderPlaceResponse::Success_V1 {
            order,
            cancelled_bid_prices,
            cancelled_bid_sizes,
            cancelled_ask_prices,
            cancelled_ask_sizes,
            previous_seq_num
        }
    }

    public fun new_bulk_order_place_response_rejection<M: store + copy + drop>(
        account: address, sequence_number: u64, existing_sequence_number: u64
    ): BulkOrderPlaceResponse<M> {
        BulkOrderPlaceResponse::Rejection_V1 {
            account,
            sequence_number,
            existing_sequence_number
        }
    }

    /// Gets the unique priority index of a bulk order.
    ///
    /// # Arguments:
    /// - `self`: Reference to the bulk order
    ///
    /// # Returns:
    /// The unique priority index for time-based ordering.
    public fun get_unique_priority_idx<M: store + copy + drop>(
        self: &BulkOrder<M>
    ): IncreasingIdx {
        self.unique_priority_idx
    }

    /// Gets the order ID of a bulk order.
    ///
    /// # Arguments:
    /// - `self`: Reference to the bulk order
    ///
    /// # Returns:
    /// The unique order identifier.
    public fun get_order_id<M: store + copy + drop>(self: &BulkOrder<M>): OrderId {
        self.order_id
    }

    public fun get_creation_time_micros<M: store + copy + drop>(
        self: &BulkOrder<M>
    ): u64 {
        self.creation_time_micros
    }

    public fun get_order_request<M: store + copy + drop>(self: &BulkOrder<M>)
        : &BulkOrderRequest<M> {
        &self.order_request
    }

    public fun get_order_request_mut<M: store + copy + drop>(
        self: &mut BulkOrder<M>
    ): &mut BulkOrderRequest<M> {
        &mut self.order_request
    }

    public fun get_account<M: store + copy + drop>(self: &BulkOrderRequest<M>): address {
        self.account
    }

    public fun get_sequence_number<M: store + copy + drop>(
        self: &BulkOrderRequest<M>
    ): u64 {
        self.order_sequence_number
    }

    public fun get_total_remaining_size<M: store + copy + drop>(
        self: &BulkOrderRequest<M>, is_bid: bool
    ): u64 {
        if (is_bid) {
            self.bid_sizes.fold(0, |acc, size| acc + size)
        } else {
            self.ask_sizes.fold(0, |acc, size| acc + size)
        }
    }

    /// Gets the active price for a specific side of a bulk order.
    ///
    /// # Arguments:
    /// - `self`: Reference to the bulk order
    /// - `is_bid`: True to get bid price, false for ask price
    ///
    /// # Returns:
    /// An option containing the active price if available, none otherwise.
    public fun get_active_price<M: store + copy + drop>(
        self: &BulkOrderRequest<M>, is_bid: bool
    ): Option<u64> {
        let prices =
            if (is_bid) {
                &self.bid_prices
            } else {
                &self.ask_prices
            };
        if (prices.length() == 0) {
            option::none() // No active price level
        } else {
            option::some(prices[0]) // Return the first price level
        }
    }

    public fun get_all_prices<M: store + copy + drop>(
        self: &BulkOrderRequest<M>, is_bid: bool
    ): vector<u64> {
        if (is_bid) {
            self.bid_prices
        } else {
            self.ask_prices
        }
    }

    public fun get_all_prices_mut<M: store + copy + drop>(
        self: &mut BulkOrderRequest<M>, is_bid: bool
    ): &mut vector<u64> {
        if (is_bid) {
            &mut self.bid_prices
        } else {
            &mut self.ask_prices
        }
    }

    public fun get_all_sizes<M: store + copy + drop>(
        self: &BulkOrderRequest<M>, is_bid: bool
    ): vector<u64> {
        if (is_bid) {
            self.bid_sizes
        } else {
            self.ask_sizes
        }
    }

    public fun get_all_sizes_mut<M: store + copy + drop>(
        self: &mut BulkOrderRequest<M>, is_bid: bool
    ): &mut vector<u64> {
        if (is_bid) {
            &mut self.bid_sizes
        } else {
            &mut self.ask_sizes
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
    public fun get_active_size<M: store + copy + drop>(
        self: &BulkOrderRequest<M>, is_bid: bool
    ): Option<u64> {
        let sizes =
            if (is_bid) {
                &self.bid_sizes
            } else {
                &self.ask_sizes
            };
        if (sizes.length() == 0) {
            option::none() // No active size level
        } else {
            option::some(sizes[0]) // Return the first size level
        }
    }

    public fun get_prices_and_sizes_mut<M: store + copy + drop>(
        self: &mut BulkOrderRequest<M>, is_bid: bool
    ): (&mut vector<u64>, &mut vector<u64>) {
        if (is_bid) {
            (&mut self.bid_prices, &mut self.bid_sizes)
        } else {
            (&mut self.ask_prices, &mut self.ask_sizes)
        }
    }

    public fun is_success_response<M: store + copy + drop>(
        self: &BulkOrderPlaceResponse<M>
    ): bool {
        self is BulkOrderPlaceResponse::Success_V1
    }

    public fun is_rejection_response<M: store + copy + drop>(
        self: &BulkOrderPlaceResponse<M>
    ): bool {
        self is BulkOrderPlaceResponse::Rejection_V1
    }

    public fun destroy_bulk_order_place_response_success<M: store + copy + drop>(
        self: BulkOrderPlaceResponse<M>
    ): (
        BulkOrder<M>,
        vector<u64>,
        vector<u64>,
        vector<u64>,
        vector<u64>,
        option::Option<u64>
    ) {
        let BulkOrderPlaceResponse::Success_V1 {
            order,
            cancelled_bid_prices,
            cancelled_bid_sizes,
            cancelled_ask_prices,
            cancelled_ask_sizes,
            previous_seq_num
        } = self;
        (
            order,
            cancelled_bid_prices,
            cancelled_bid_sizes,
            cancelled_ask_prices,
            cancelled_ask_sizes,
            previous_seq_num
        )
    }

    public fun destroy_bulk_order_place_response_rejection<M: store + copy + drop>(
        self: BulkOrderPlaceResponse<M>
    ): (address, u64, u64) {
        let BulkOrderPlaceResponse::Rejection_V1 {
            account,
            sequence_number,
            existing_sequence_number
        } = self;
        (account, sequence_number, existing_sequence_number)
    }

    // Creates a new single bulk order match result.
    //
    // Arguments:
    // - order: Reference to the bulk order being matched
    // - is_bid: True if matching against bid side, false for ask side
    // - matched_size: Size that was matched in this operation
    //
    // Returns:
    // A `SingleBulkOrderMatch` containing the match details.
    public fun new_bulk_order_match<M: store + copy + drop>(
        order: &BulkOrder<M>, is_bid: bool, matched_size: u64
    ): OrderMatch<M> {
        let order_request = &order.order_request;
        let (price, remaining_size) =
            if (is_bid) {
                (order_request.bid_prices[0], order_request.bid_sizes[0] - matched_size)
            } else {
                (order_request.ask_prices[0], order_request.ask_sizes[0] - matched_size)
            };
        new_order_match<M>(
            new_bulk_order_match_details<M>(
                order.order_id,
                order_request.account,
                order.unique_priority_idx,
                price,
                remaining_size,
                is_bid,
                order_request.order_sequence_number,
                order.creation_time_micros,
                order_request.metadata
            ),
            matched_size
        )
    }

    /// Sets the bulk order to empty state by clearing all sizes.
    ///
    /// This function is used during order cancellation to clear the order state
    /// while preserving the order ID for potential reuse.
    ///
    /// # Arguments:
    /// - `self`: Mutable reference to the bulk order
    public fun set_empty<M: store + copy + drop>(self: &mut BulkOrder<M>) {
        self.order_request.bid_sizes = vector::empty();
        self.order_request.ask_sizes = vector::empty();
        self.order_request.bid_prices = vector::empty();
        self.order_request.ask_prices = vector::empty();
    }

    public fun destroy_bulk_order<M: store + copy + drop>(
        self: BulkOrder<M>
    ): (BulkOrderRequest<M>, OrderId, IncreasingIdx, u64) {
        let BulkOrder::V1 {
            order_request,
            order_id,
            unique_priority_idx,
            creation_time_micros
        } = self;
        (order_request, order_id, unique_priority_idx, creation_time_micros)
    }

    public fun destroy_bulk_order_request<M: store + copy + drop>(
        self: BulkOrderRequest<M>
    ): (address, u64, vector<u64>, vector<u64>, vector<u64>, vector<u64>, M) {
        let BulkOrderRequest::V1 {
            account,
            order_sequence_number,
            bid_prices,
            bid_sizes,
            ask_prices,
            ask_sizes,
            metadata
        } = self;
        (
            account,
            order_sequence_number,
            bid_prices,
            bid_sizes,
            ask_prices,
            ask_sizes,
            metadata
        )
    }
}
