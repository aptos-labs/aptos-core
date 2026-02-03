module aptos_experimental::bulk_order_utils {
    use std::option::{Self, Option};
    use std::vector;
    use aptos_std::timestamp;
    use aptos_trading::bulk_order_types::{Self, BulkOrder, BulkOrderRequest};
    use aptos_trading::order_book_types::{OrderId, IncreasingIdx};
    use aptos_trading::order_match_types::OrderMatchDetails;

    friend aptos_experimental::bulk_order_book;

    const EUNEXPECTED_MATCH_SIZE: u64 = 2;

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
