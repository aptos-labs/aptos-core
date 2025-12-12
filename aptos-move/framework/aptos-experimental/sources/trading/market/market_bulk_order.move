/// This module provides bulk order placement and cancellation APIs for the market.
/// Bulk orders allow users to place multiple bid and ask orders at different price levels
/// in a single transaction, improving efficiency for market makers.
module aptos_experimental::market_bulk_order {
    friend aptos_experimental::dead_mans_switch_operations;
    use std::signer;
    use aptos_experimental::bulk_order_book_types::{
        new_bulk_order_request, destroy_bulk_order_place_response
    };
    use aptos_experimental::market_types::{Self, MarketClearinghouseCallbacks, Market};
    use aptos_experimental::order_book_types::OrderIdType;

    const E_SEQUENCE_NUMBER_MISMATCH: u64 = 0;
    const E_CLEARINGHOUSE_VALIDATION_FAILED: u64 = 1;

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
    ): OrderIdType {
        let validation_result = callbacks.validate_bulk_order_placement(
            account,
            &bid_prices,
            &bid_sizes,
            &ask_prices,
            &ask_sizes,
            &metadata,
        );
        assert!(validation_result.is_validation_result_valid(), E_CLEARINGHOUSE_VALIDATION_FAILED);
        let request = new_bulk_order_request(
            account,
            sequence_number,
            bid_prices,
            bid_sizes,
            ask_prices,
            ask_sizes,
            metadata,
        );
        let response = market.get_order_book_mut().place_bulk_order(request);
        let (bulk_order, cancelled_bid_prices, cancelled_bid_sizes, cancelled_ask_prices, cancelled_ask_sizes, previous_seq_num_option) = destroy_bulk_order_place_response(response);
        let (order_id, _, _, order_sequence_number, _, bid_prices, bid_sizes, ask_prices, ask_sizes, _ ) = bulk_order.destroy_bulk_order(); // We don't need to keep the bulk order struct after placement
        assert!(sequence_number == order_sequence_number, E_SEQUENCE_NUMBER_MISMATCH);
        // Extract previous_seq_num from option, defaulting to 0 if none
        let previous_seq_num = previous_seq_num_option.destroy_with_default(0);
        // Emit an event for the placed bulk order
        market.emit_event_for_bulk_order_placed(
            order_id,
            order_sequence_number,
            account,
            bid_prices,
            bid_sizes,
            ask_prices,
            ask_sizes,
            cancelled_bid_prices,
            cancelled_bid_sizes,
            cancelled_ask_prices,
            cancelled_ask_sizes,
            previous_seq_num
        );
        order_id
    }

    /// Cancels all bulk orders for a given user.
    /// This will cancel all bid and ask orders that were placed as part of bulk orders
    /// for the specified user account.
    ///
    /// Parameters:
    /// - market: The market instance
    /// - user: The signer of the user whose bulk orders should be cancelled
    /// - cancellation_reason: The reason for cancelling the bulk order
    /// - callbacks: The market clearinghouse callbacks for cleanup operations
    public fun cancel_bulk_order<M: store + copy + drop, R: store + copy + drop>(
        market: &mut Market<M>,
        user: &signer,
        cancellation_reason: market_types::OrderCancellationReason,
        callbacks: &MarketClearinghouseCallbacks<M, R>
    ) {
        let account = signer::address_of(user);
        cancel_bulk_order_internal(market, account, cancellation_reason, callbacks);
    }

    public(friend) fun cancel_bulk_order_internal<M: store + copy + drop, R: store + copy + drop>(
        market: &mut Market<M>,
        user: address,
        cancellation_reason: market_types::OrderCancellationReason,
        callbacks: &MarketClearinghouseCallbacks<M, R>
    ) {
        let cancelled_bulk_order = market.get_order_book_mut().cancel_bulk_order(user);
        let (order_id, _, _, sequence_number, _, bid_prices, bid_sizes, ask_prices, ask_sizes, _ ) = cancelled_bulk_order.destroy_bulk_order();
        let i = 0;
        while (i < bid_sizes.length()) {
            callbacks.cleanup_bulk_order_at_price(user, order_id, true, bid_prices[i], bid_sizes[i]);
            i += 1;
        };
        let j = 0;
        while (j < ask_sizes.length()) {
            callbacks.cleanup_bulk_order_at_price(user, order_id, false, ask_prices[j], ask_sizes[j]);
            j += 1;
        };
        market.emit_event_for_bulk_order_cancelled(
            order_id,
            sequence_number,
            user,
            bid_prices,
            bid_sizes,
            ask_prices,
            ask_sizes,
            std::option::some(cancellation_reason)
        );
    }
}
