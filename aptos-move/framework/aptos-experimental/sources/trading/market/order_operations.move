/// This module provides order cancellation and size reduction APIs for the market.
/// It includes functions for canceling orders by order ID, canceling orders by client order ID,
/// and reducing the size of existing orders.
module aptos_experimental::order_operations {
    use std::option;
    use std::signer;
    use aptos_experimental::market_types::{
        MarketClearinghouseCallbacks,
        Market,
    };
    use aptos_experimental::order_book_types::{
        OrderIdType,
        single_order_book_type
    };
    use aptos_experimental::single_order_types::SingleOrder;
    use aptos_experimental::pre_cancellation_tracker::{
        pre_cancel_order_for_tracker
    };
    use aptos_experimental::order_placement::cleanup_order_internal;

    // Error codes
    const ENOT_ORDER_CREATOR: u64 = 12;
    const EORDER_DOES_NOT_EXIST: u64 = 6;

    /// Cancels an order using the client order ID.
    /// This function first tries to cancel an order that's already placed in the order book.
    /// If the order is not found in the order book, it adds the order to the pre-cancellation tracker
    /// so it can be cancelled when it's eventually placed.
    ///
    /// Parameters:
    /// - market: The market instance
    /// - user: The signer of the user whose order should be cancelled
    /// - client_order_id: The client order ID of the order to cancel
    /// - callbacks: The market clearinghouse callbacks for cleanup operations
    public fun cancel_order_with_client_id<M: store + copy + drop>(
        market: &mut Market<M>,
        user: &signer,
        client_order_id: u64,
        callbacks: &MarketClearinghouseCallbacks<M>
    ) {
        let order =
            market.get_order_book_mut().try_cancel_order_with_client_order_id(
                signer::address_of(user), client_order_id
            );
        if (order.is_some()) {
            // Order is already placed in the order book, so we can cancel it
            return cancel_single_order_helper(market, order.destroy_some(), callbacks);
        };
        pre_cancel_order_for_tracker(
            market.get_pre_cancellation_tracker_mut(),
            user,
            client_order_id,
        );
    }

    /// Cancels an order by order ID.
    /// This will cancel the order and emit an event for the order cancellation.
    ///
    /// Parameters:
    /// - market: The market instance
    /// - user: The signer of the user whose order should be cancelled
    /// - order_id: The order ID of the order to cancel
    /// - callbacks: The market clearinghouse callbacks for cleanup operations
    public fun cancel_order<M: store + copy + drop>(
        market: &mut Market<M>,
        user: &signer,
        order_id: OrderIdType,
        callbacks: &MarketClearinghouseCallbacks<M>
    ) {
        let account = signer::address_of(user);
        let order = market.get_order_book_mut().cancel_order(account, order_id);
        assert!(account == order.get_account(), ENOT_ORDER_CREATOR);
        cancel_single_order_helper(market, order, callbacks);
    }

    /// Reduces the size of an existing order.
    /// This function decreases the size of an order by the specified amount and emits
    /// an event for the size reduction.
    ///
    /// Parameters:
    /// - market: The market instance
    /// - user: The signer of the user whose order size should be reduced
    /// - order_id: The order ID of the order to reduce
    /// - size_delta: The amount by which to reduce the order size
    /// - callbacks: The market clearinghouse callbacks for cleanup operations
    public fun decrease_order_size<M: store + copy + drop>(
        market: &mut Market<M>,
        user: &signer,
        order_id: OrderIdType,
        size_delta: u64,
        callbacks: &MarketClearinghouseCallbacks<M>
    ) {
        let account = signer::address_of(user);
        let order_book = market.get_order_book_mut();
        order_book.decrease_order_size(account, order_id, size_delta);
        let maybe_order = order_book.get_order(order_id);
        assert!(maybe_order.is_some(), EORDER_DOES_NOT_EXIST);
        let (order, _) = maybe_order.destroy_some().destroy_order_from_state();
        assert!(order.get_account() == account, ENOT_ORDER_CREATOR);
        let (
            user,
            order_id,
            client_order_id,
            _,
            price,
            orig_size,
            remaining_size,
            is_bid,
            _trigger_condition,
            time_in_force,
            metadata
        ) = order.destroy_single_order();
        callbacks.decrease_order_size(
            user, order_id, is_bid, price, remaining_size
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
            option::some(metadata),
            option::none(),
            time_in_force,
            callbacks
        );
    }

    /// Internal helper function to cancel a single order.
    /// This function handles the cleanup and event emission for order cancellation.
    ///
    /// Parameters:
    /// - market: The market instance
    /// - order: The order to cancel
    /// - callbacks: The market clearinghouse callbacks for cleanup operations
    fun cancel_single_order_helper<M: store + copy + drop>(
        market: &mut Market<M>,
        order: SingleOrder<M>,
        callbacks: &MarketClearinghouseCallbacks<M>
    ) {
        let (
            account,
            order_id,
            client_order_id,
            _,
            price,
            orig_size,
            remaining_size,
            is_bid,
            _trigger_condition,
            time_in_force,
            metadata
        ) = order.destroy_single_order();
        cleanup_order_internal(
            account, order_id, single_order_book_type(), is_bid, remaining_size, option::some(metadata), callbacks
        );
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
            std::string::utf8(b"Order cancelled"),
            option::some(metadata),
            option::none(), // trigger_condition
            time_in_force,
            callbacks
        );
    }

}
