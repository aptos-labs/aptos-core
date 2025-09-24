/// This module provides bulk order placement and cancellation APIs for the market.
/// Bulk orders allow users to place multiple bid and ask orders at different price levels
/// in a single transaction, improving efficiency for market makers.
module aptos_experimental::market_bulk_order {
    use std::option;
    use std::signer;
    use aptos_experimental::bulk_order_book_types::new_bulk_order_request;
    use aptos_experimental::market_types::{
        MarketClearinghouseCallbacks,
        Market,
    };
    use aptos_experimental::order_book_types::OrderIdType;

    /// Places a bulk order with multiple bid and ask price levels.
    /// This allows market makers to place multiple orders at different price levels
    /// in a single transaction, improving efficiency.
    ///
    /// Parameters:
    /// - market: The market instance
    /// - account: The account address placing the bulk order
    /// - bid_prices: Vector of bid prices
    /// - bid_sizes: Vector of bid sizes (must match bid_prices length)
    /// - ask_prices: Vector of ask prices
    /// - ask_sizes: Vector of ask sizes (must match ask_prices length)
    /// - callbacks: The market clearinghouse callbacks for validation and settlement
    ///
    /// Returns:
    /// - Option<OrderIdType>: The bulk order ID if successfully placed, None if validation failed
    public fun place_bulk_order<M: store + copy + drop, R: store + copy + drop>(
        market: &mut Market<M>,
        account: address,
        sequence_number: u64,
        bid_prices: vector<u64>,
        bid_sizes: vector<u64>,
        ask_prices: vector<u64>,
        ask_sizes: vector<u64>,
        metadata: M,
        callbacks: &MarketClearinghouseCallbacks<M, R>
    ): option::Option<OrderIdType> {
        // TODO(skedia) Add support for events for bulk orders
        if (!callbacks.validate_bulk_order_placement(
            account,
            bid_prices,
            bid_sizes,
            ask_prices,
            ask_sizes,
            metadata,
        )) {
            // If the bulk order is not valid, we simply return without placing the order.
            return option::none();
        };
        option::some(market.get_order_book_mut().place_bulk_order(new_bulk_order_request(
            account,
            sequence_number,
            bid_prices,
            bid_sizes,
            ask_prices,
            ask_sizes,
            metadata,
        )))
    }

    /// Cancels all bulk orders for a given user.
    /// This will cancel all bid and ask orders that were placed as part of bulk orders
    /// for the specified user account.
    ///
    /// Parameters:
    /// - market: The market instance
    /// - user: The signer of the user whose bulk orders should be cancelled
    /// - callbacks: The market clearinghouse callbacks for cleanup operations
    public fun cancel_bulk_order<M: store + copy + drop, R: store + copy + drop>(
        market: &mut Market<M>,
        user: &signer,
        callbacks: &MarketClearinghouseCallbacks<M, R>
    ) {
        let account = signer::address_of(user);
        let (_order_id, remaining_bid_size, remaining_ask_size) = market.get_order_book_mut().cancel_bulk_order(account);
        if (remaining_ask_size > 0) {
            callbacks.cleanup_bulk_orders(account, false, remaining_ask_size);
        };
        if (remaining_bid_size > 0) {
            callbacks.cleanup_bulk_orders(account, true, remaining_bid_size);
        }
    }
}
