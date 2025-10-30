/// This module provides bulk order placement and cancellation APIs for the market.
/// Bulk orders allow users to place multiple bid and ask orders at different price levels
/// in a single transaction, improving efficiency for market makers.
module aptos_experimental::market_bulk_order {
    use std::option;
    use std::signer;
    use aptos_experimental::bulk_order_book_types::{
        new_bulk_order_request, destroy_bulk_order_request_response, is_bulk_order_success_response, destroy_bulk_order_place_success_response, destroy_bulk_order_place_reject_response
    };
    use aptos_experimental::market_types::{
        MarketClearinghouseCallbacks,
        Market,
    };
    use aptos_experimental::order_book_types::OrderIdType;

    const E_SEQUENCE_NUMBER_MISMATCH: u64 = 0;

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
        if (!callbacks.validate_bulk_order_placement(
            account,
            bid_prices,
            bid_sizes,
            ask_prices,
            ask_sizes,
            metadata,
        )) {
            // If the bulk order is not valid, emit rejection event and return without placing the order.
            market.emit_event_for_bulk_order_rejected(
                sequence_number,
                account,
                bid_prices,
                bid_sizes,
                ask_prices,
                ask_sizes,
                std::string::utf8(b"validation failed"),
            );
            return option::none();
        };
        let request_response = new_bulk_order_request(
            account,
            sequence_number,
            bid_prices,
            bid_sizes,
            ask_prices,
            ask_sizes,
            metadata,
        );
        let (request_option, request_rejection_reason_option) = destroy_bulk_order_request_response(request_response);
        if (request_option.is_none()) {
            // Bulk order request creation failed - emit rejection event
            let rejection_reason = request_rejection_reason_option.destroy_some();
            market.emit_event_for_bulk_order_rejected(
                sequence_number,
                account,
                bid_prices,
                bid_sizes,
                ask_prices,
                ask_sizes,
                rejection_reason,
            );
            return option::none();
        };
        let bulk_order_request = request_option.destroy_some();
        let response = market.get_order_book_mut().place_bulk_order(bulk_order_request);
        if (is_bulk_order_success_response(&response)) {
            let (bulk_order, cancelled_bid_prices, cancelled_bid_sizes, cancelled_ask_prices, cancelled_ask_sizes, previous_seq_num_option) = destroy_bulk_order_place_success_response(response);
            let (order_id, _, _, order_sequence_number, bid_prices, bid_sizes, ask_prices, ask_sizes, _ ) = bulk_order.destroy_bulk_order(); // We don't need to keep the bulk order struct after placement
            assert!(sequence_number == order_sequence_number, E_SEQUENCE_NUMBER_MISMATCH);
            // Extract previous_seq_num from option, defaulting to 0 if none
            let previous_seq_num = if (previous_seq_num_option.is_some()) {
                previous_seq_num_option.destroy_some()
            } else {
                0
            };
            // Emit an event for the placed bulk order
            market.emit_event_for_bulk_order_placed(order_id,
                order_sequence_number, account, bid_prices, bid_sizes, ask_prices, ask_sizes, cancelled_bid_prices, cancelled_bid_sizes, cancelled_ask_prices, cancelled_ask_sizes, previous_seq_num);
            option::some(order_id)
        } else {
            // Handle rejection from order book - emit rejection event
            let rejection_reason = destroy_bulk_order_place_reject_response(response);
            market.emit_event_for_bulk_order_rejected(
                sequence_number,
                account,
                bid_prices,
                bid_sizes,
                ask_prices,
                ask_sizes,
                rejection_reason,
            );
            option::none()
        }
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
        cancel_bulk_order_internal(market, account, callbacks);
    }

    public(friend) fun cancel_bulk_order_internal<M: store + copy + drop, R: store + copy + drop>(
        market: &mut Market<M>,
        user: address,
        callbacks: &MarketClearinghouseCallbacks<M, R>
    ) {
        let cancelled_bulk_order = market.get_order_book_mut().cancel_bulk_order(user);
        let (order_id, _, _, sequence_number, bid_prices, bid_sizes, ask_prices, ask_sizes, _ ) = cancelled_bulk_order.destroy_bulk_order();
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
            ask_sizes
        );
    }
}
