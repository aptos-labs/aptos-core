/// This module provides order cancellation and size reduction APIs for the market.
/// It includes functions for canceling orders by order ID, canceling orders by client order ID,
/// and reducing the size of existing orders.
module aptos_experimental::order_operations {
    friend aptos_experimental::dead_mans_switch_operations;

    use std::option;
    use std::string::String;
    use aptos_trading::order_book_types::{OrderId, single_order_type};
    use aptos_trading::single_order_types::SingleOrder;
    use aptos_experimental::market_types::{Self, MarketClearinghouseCallbacks, Market};
    use aptos_experimental::pre_cancellation_tracker::{pre_cancel_order_for_tracker};
    use aptos_experimental::order_placement::cleanup_order_internal;
    use aptos_experimental::market_clearinghouse_order_info::new_clearinghouse_order_info;

    // Error codes
    const EORDER_DOES_NOT_EXIST: u64 = 6;

    /// Cancels an order using the client order ID.
    /// This function first tries to cancel an order that's already placed in the order book.
    /// If the order is not found in the order book, it adds the order to the pre-cancellation tracker
    /// so it can be cancelled when it's eventually placed.
    ///
    /// Parameters:
    /// - market: The market instance
    /// - account: address of the account that owns the order - please note that no signer validation is done here.
    ///   It it the caller's responsibility to ensure that the account is authorized to cancel the order.
    /// - user: The signer of the user whose order should be cancelled
    /// - client_order_id: The client order ID of the order to cancel
    /// - cancellation_reason: The reason for cancellation
    /// - cancel_reason: String description of the cancellation
    /// - callbacks: The market clearinghouse callbacks for cleanup operations
    public fun cancel_order_with_client_id<M: store + copy + drop, R: store + copy + drop>(
        market: &mut Market<M>,
        user: address,
        client_order_id: String,
        cancellation_reason: market_types::OrderCancellationReason,
        cancel_reason: String,
        callbacks: &MarketClearinghouseCallbacks<M, R>
    ) {
        let order =
            market.get_order_book_mut().try_cancel_single_order_with_client_order_id(
                user, client_order_id
            );
        if (order.is_some()) {
            // Order is already placed in the order book, so we can cancel it
            return cancel_single_order_helper(
                market,
                order.destroy_some(),
                true,
                cancellation_reason,
                cancel_reason,
                callbacks
            );
        };
        pre_cancel_order_for_tracker(
            market.get_pre_cancellation_tracker_mut(),
            user,
            client_order_id
        );
    }

    /// Cancels an order by order ID.
    /// This will cancel the order and emit an event for the order cancellation.
    ///
    /// Parameters:
    /// - market: The market instance
    /// - account: address of the account that owns the order - please note that no signer validation is done here.
    ///   It it the caller's responsibility to ensure that the account is authorized to cancel the order.
    /// - user: The signer of the user whose order should be cancelled
    /// - order_id: The order ID of the order to cancel
    /// - cancellation_reason: The reason for cancellation
    /// - cancel_reason: String description of the cancellation
    /// - callbacks: The market clearinghouse callbacks for cleanup operations
    public fun cancel_order<M: store + copy + drop, R: store + copy + drop>(
        market: &mut Market<M>,
        account: address,
        order_id: OrderId,
        emit_event: bool,
        cancellation_reason: market_types::OrderCancellationReason,
        cancel_reason: String,
        callbacks: &MarketClearinghouseCallbacks<M, R>
    ): SingleOrder<M> {
        let order = market.get_order_book_mut().cancel_single_order(account, order_id);
        cancel_single_order_helper(
            market,
            order,
            emit_event,
            cancellation_reason,
            cancel_reason,
            callbacks
        );
        order
    }

    /// Tries to cancel an order by order ID.
    /// This function attempts to cancel the order and returns an option containing the order
    /// if it was successfully cancelled, or None if the order does not exist.
    public fun try_cancel_order<M: store + copy + drop, R: store + copy + drop>(
        market: &mut Market<M>,
        account: address,
        order_id: OrderId,
        emit_event: bool,
        cancellation_reason: market_types::OrderCancellationReason,
        cancel_reason: String,
        callbacks: &MarketClearinghouseCallbacks<M, R>
    ): option::Option<SingleOrder<M>> {
        let maybe_order =
            market.get_order_book_mut().try_cancel_single_order(account, order_id);
        if (maybe_order.is_some()) {
            let order = maybe_order.destroy_some();
            cancel_single_order_helper(
                market,
                order,
                emit_event,
                cancellation_reason,
                cancel_reason,
                callbacks
            );
            option::some(order)
        } else {
            option::none()
        }
    }

    /// Reduces the size of an existing order.
    /// This function decreases the size of an order by the specified amount and emits
    /// an event for the size reduction.
    ///
    /// Parameters:
    /// - market: The market instance
    /// - account: address of the account that owns the order - please note that no signer validation is done here.
    /// It it the caller's responsibility to ensure that the account is authorized to modify the order.
    /// - order_id: The order ID of the order to reduce
    /// - size_delta: The amount by which to reduce the order size
    /// - callbacks: The market clearinghouse callbacks for cleanup operations
    public fun decrease_order_size<M: store + copy + drop, R: store + copy + drop>(
        market: &mut Market<M>,
        account: address,
        order_id: OrderId,
        size_delta: u64,
        callbacks: &MarketClearinghouseCallbacks<M, R>
    ) {
        let order_book = market.get_order_book_mut();
        order_book.decrease_single_order_size(account, order_id, size_delta);
        let (order, _) =
            order_book.get_single_order(order_id).destroy_some().destroy_order_from_state();
        let (order_request, _unique_priority_idx) = order.destroy_single_order();
        let (
            user,
            order_id,
            client_order_id,
            price,
            orig_size,
            remaining_size,
            is_bid,
            trigger_condition,
            time_in_force,
            _creation_time_micros,
            metadata
        ) = order_request.destroy_single_order_request();
        callbacks.decrease_order_size(
            new_clearinghouse_order_info(
                user,
                order_id,
                client_order_id,
                is_bid,
                price,
                time_in_force,
                single_order_type(),
                trigger_condition,
                metadata
            ),
            remaining_size
        );

        market.emit_event_for_order(
            order_id,
            client_order_id,
            user,
            orig_size,
            remaining_size,
            size_delta,
            price,
            is_bid,
            false,
            aptos_experimental::market_types::order_status_size_reduced(),
            std::string::utf8(b"Order size reduced"),
            metadata,
            option::none(),
            time_in_force,
            option::none(),
            callbacks
        );
    }

    #[lint::skip(needless_mutable_reference)]
    // Internal helper function to cancel a single order.
    // This function handles the cleanup and event emission for order cancellation.
    //
    // Parameters:
    // - market: The market instance
    // - order: The order to cancel
    // - cancellation_reason: The reason for cancellation
    // - cancel_reason: String description of the cancellation
    // - callbacks: The market clearinghouse callbacks for cleanup operations
    public(friend) fun cancel_single_order_helper<M: store + copy + drop, R: store + copy + drop>(
        market: &mut Market<M>,
        order: SingleOrder<M>,
        emit_event: bool,
        cancellation_reason: market_types::OrderCancellationReason,
        cancel_reason: String,
        callbacks: &MarketClearinghouseCallbacks<M, R>
    ) {
        let (order_request, _unique_priority_idx) = order.destroy_single_order();
        let (
            account,
            order_id,
            client_order_id,
            price,
            orig_size,
            remaining_size,
            is_bid,
            trigger_condition,
            time_in_force,
            _creation_time_micros,
            metadata
        ) = order_request.destroy_single_order_request();
        cleanup_order_internal(
            account,
            order_id,
            client_order_id,
            single_order_type(),
            is_bid,
            time_in_force,
            remaining_size,
            price,
            trigger_condition,
            metadata,
            callbacks,
            false
        );
        if (emit_event) {
            market.emit_event_for_order(
                order_id,
                client_order_id,
                account,
                orig_size,
                0,
                remaining_size,
                price,
                is_bid,
                false,
                aptos_experimental::market_types::order_status_cancelled(),
                cancel_reason,
                metadata,
                option::none(), // trigger_condition
                time_in_force,
                option::some(cancellation_reason),
                callbacks
            );
        }
    }
}
