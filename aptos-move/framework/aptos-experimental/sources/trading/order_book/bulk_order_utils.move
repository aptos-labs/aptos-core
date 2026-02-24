module aptos_experimental::bulk_order_utils {
    use std::option::{Self, Option};
    use std::vector;
    use aptos_std::timestamp;
    use aptos_trading::bulk_order_types::{Self, BulkOrder, BulkOrderRequest};
    use aptos_trading::order_book_types::{OrderId, IncreasingIdx};
    use aptos_trading::order_match_types::OrderMatchDetails;

    friend aptos_experimental::bulk_order_book;
    friend aptos_experimental::market_bulk_order;
    #[test_only]
    friend aptos_experimental::bulk_order_book_tests;
    #[test_only]
    friend aptos_experimental::bulk_order_types_tests;

    // Error codes for various failure scenarios
    const EPRICE_CROSSING: u64 = 1;
    const E_BID_LENGTH_MISMATCH: u64 = 2;
    const E_ASK_LENGTH_MISMATCH: u64 = 3;
    const E_EMPTY_ORDER: u64 = 4;
    const E_BID_SIZE_ZERO: u64 = 5;
    const E_ASK_SIZE_ZERO: u64 = 6;
    const E_BID_ORDER_INVALID: u64 = 7;
    const E_ASK_ORDER_INVALID: u64 = 8;
    const E_BULK_ORDER_DEPTH_EXCEEDED: u64 = 9;
    const E_INVALID_SEQUENCE_NUMBER: u64 = 10;
    const EUNEXPECTED_MATCH_SIZE: u64 = 11;

    /// Maximum number of price levels per side (bid or ask) in a bulk order.
    /// This limit prevents gas DoS scenarios when cancelling bulk orders.
    const MAX_BULK_ORDER_DEPTH_PER_SIDE: u64 = 30;

    /// # Aborts:
    /// - If sequence_number is 0 (reserved to avoid ambiguity in events)
    /// - If bid_prices and bid_sizes have different lengths
    /// - If ask_prices and ask_sizes have different lengths
    /// - If bid_prices or ask_prices exceeds MAX_BULK_ORDER_DEPTH_PER_SIDE (30) levels
    public(friend) fun new_bulk_order_request_with_sanitization<M: store + copy + drop>(
        account: address,
        sequence_number: u64,
        bid_prices: vector<u64>,
        bid_sizes: vector<u64>,
        ask_prices: vector<u64>,
        ask_sizes: vector<u64>,
        metadata: M
    ): BulkOrderRequest<M> {
        // Sequence number 0 is reserved to avoid ambiguity in events
        assert!(sequence_number > 0, E_INVALID_SEQUENCE_NUMBER);

        let num_bids = bid_prices.length();
        let num_asks = ask_prices.length();

        // Basic length validation
        assert!(num_bids == bid_sizes.length(), E_BID_LENGTH_MISMATCH);
        assert!(num_asks == ask_sizes.length(), E_ASK_LENGTH_MISMATCH);
        assert!(num_bids > 0 || num_asks > 0, E_EMPTY_ORDER);
        // Depth validation to prevent gas DoS when cancelling
        assert!(num_bids <= MAX_BULK_ORDER_DEPTH_PER_SIDE, E_BULK_ORDER_DEPTH_EXCEEDED);
        assert!(num_asks <= MAX_BULK_ORDER_DEPTH_PER_SIDE, E_BULK_ORDER_DEPTH_EXCEEDED);
        assert!(validate_not_zero_sizes(&bid_sizes), E_BID_SIZE_ZERO);
        assert!(validate_not_zero_sizes(&ask_sizes), E_ASK_SIZE_ZERO);
        assert!(validate_price_ordering(&bid_prices, true), E_BID_ORDER_INVALID);
        assert!(validate_price_ordering(&ask_prices, false), E_ASK_ORDER_INVALID);

        if (num_bids > 0 && num_asks > 0) {
            // First element in bids is the highest (descending order), first element in asks is the lowest (ascending
            // order).
            assert!(bid_prices[0] < ask_prices[0], EPRICE_CROSSING);
        };
        bulk_order_types::new_bulk_order_request(
            account,
            sequence_number,
            bid_prices,
            bid_sizes,
            ask_prices,
            ask_sizes,
            metadata
        )
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
    public(friend) fun new_bulk_order_with_sanitization<M: store + copy + drop>(
        order_id: OrderId,
        unique_priority_idx: IncreasingIdx,
        order_req: BulkOrderRequest<M>,
        best_bid_price: Option<u64>,
        best_ask_price: Option<u64>
    ): (BulkOrder<M>, vector<u64>, vector<u64>, vector<u64>, vector<u64>) {
        let creation_time_micros = timestamp::now_microseconds();
        let bid_price_crossing_idx =
            discard_price_crossing_levels(
                &order_req.get_all_prices(true), best_ask_price, true
            );
        let ask_price_crossing_idx =
            discard_price_crossing_levels(
                &order_req.get_all_prices(false), best_bid_price, false
            );

        // Extract cancelled levels (the ones that were discarded)
        let (cancelled_bid_prices, cancelled_bid_sizes) =
            if (bid_price_crossing_idx > 0) {
                let cancelled_bid_prices =
                    trim_start(
                        order_req.get_all_prices_mut(true), bid_price_crossing_idx
                    );
                let cancelled_bid_sizes =
                    trim_start(
                        order_req.get_all_sizes_mut(true), bid_price_crossing_idx
                    );
                (cancelled_bid_prices, cancelled_bid_sizes)
            } else {
                (vector::empty<u64>(), vector::empty<u64>())
            };
        let (cancelled_ask_prices, cancelled_ask_sizes) =
            if (ask_price_crossing_idx > 0) {
                let cancelled_ask_prices =
                    trim_start(
                        order_req.get_all_prices_mut(false), ask_price_crossing_idx
                    );
                let cancelled_ask_sizes =
                    trim_start(
                        order_req.get_all_sizes_mut(false), ask_price_crossing_idx
                    );
                (cancelled_ask_prices, cancelled_ask_sizes)
            } else {
                (vector::empty<u64>(), vector::empty<u64>())
            };
        let bulk_order =
            bulk_order_types::new_bulk_order(
                order_req,
                order_id,
                unique_priority_idx,
                creation_time_micros
            );
        (
            bulk_order,
            cancelled_bid_prices,
            cancelled_bid_sizes,
            cancelled_ask_prices,
            cancelled_ask_sizes
        )
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
        prices: &vector<u64>, is_descending: bool
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

    fun trim_start<Element>(v: &mut vector<Element>, new_start: u64): vector<Element> {
        let other = vector::empty();
        vector::move_range(v, 0, new_start, &mut other, 0);
        other
    }

    fun discard_price_crossing_levels(
        prices: &vector<u64>, best_price: Option<u64>, is_bid: bool
    ): u64 {
        // Discard bid levels that are >= best ask price
        let i = 0;
        if (best_price.is_some()) {
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

    /// Reinserts an order into a bulk order.
    ///
    /// This function adds the reinserted order's price and size to the appropriate side
    /// of the bulk order. If the price already exists at the first level, it increases
    /// the size; otherwise, it inserts the new price level at the front.
    ///
    /// # Arguments:
    /// - `self`: Mutable reference to the bulk order
    /// - `other`: Reference to the order result to reinsert
    public(friend) fun reinsert_order_into_bulk_order<M: store + copy + drop>(
        order: &mut BulkOrder<M>, other: &OrderMatchDetails<M>
    ) {
        // Reinsert the order into the bulk order
        let (prices, sizes) =
            order.get_order_request_mut().get_prices_and_sizes_mut(
                other.is_bid_from_match_details()
            );
        // Reinsert the price and size at the front of the respective vectors - if the price already exists, we ensure that
        // it is same as the reinsertion price and we just increase the size
        // If the price does not exist, we insert it at the front.
        let other_price = other.get_price_from_match_details();
        if (prices.length() > 0 && prices[0] == other_price) {
            sizes[0] += other.get_remaining_size_from_match_details(); // Increase the size at the first price level
        } else {
            prices.insert(0, other_price); // Insert the new price at the front
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
    public(friend) fun match_order_and_get_next_from_bulk_order<M: store + copy + drop>(
        order: &mut BulkOrder<M>, is_bid: bool, matched_size: u64
    ): (Option<u64>, Option<u64>) {
        let (prices, sizes) =
            order.get_order_request_mut().get_prices_and_sizes_mut(is_bid);
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

    /// Cancels a specific price level in a bulk order by setting its size to 0 and removing it.
    ///
    /// This function finds the price level matching the specified price and removes it from
    /// the order, keeping other price levels intact.
    ///
    /// # Arguments:
    /// - `order`: Mutable reference to the bulk order
    /// - `price`: The price level to cancel
    /// - `is_bid`: True to cancel from bid side, false for ask side
    ///
    /// # Returns:
    /// The size that was cancelled at that price level, or 0 if the price wasn't found
    public(friend) fun cancel_at_price_level<M: store + copy + drop>(
        order: &mut BulkOrder<M>, price: u64, is_bid: bool
    ): u64 {
        let (prices, sizes) =
            order.get_order_request_mut().get_prices_and_sizes_mut(is_bid);
        let i = 0;
        while (i < prices.length()) {
            if (prices[i] == price) {
                // Found the price level, remove it
                let cancelled_size = sizes[i];
                prices.remove(i);
                sizes.remove(i);
                return cancelled_size
            };
            i = i + 1;
        };
        0 // Price not found
    }
}
