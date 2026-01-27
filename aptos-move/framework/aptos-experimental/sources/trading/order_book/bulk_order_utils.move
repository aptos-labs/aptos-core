module aptos_experimental::bulk_order_utils {
    use aptos_trading::bulk_order_types::BulkOrder;
    use aptos_trading::order_match_types::OrderMatchDetails;
    use std::option::{Self, Option};

    friend aptos_experimental::bulk_order_book;

    const EUNEXPECTED_MATCH_SIZE: u64 = 2;


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
        let (prices, sizes) = order.get_order_request_mut().get_prices_and_sizes_mut(other.is_bid_from_match_details());
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
        let (prices, sizes) = order.get_order_request_mut().get_prices_and_sizes_mut(is_bid);
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
        order: &mut BulkOrder<M>,
        price: u64,
        is_bid: bool
    ): u64 {
        let (prices, sizes) = order.get_order_request_mut().get_prices_and_sizes_mut(is_bid);
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
