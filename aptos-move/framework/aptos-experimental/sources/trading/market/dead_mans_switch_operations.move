/// This module provides dead man's switch operations for the market.
/// It includes functions for cleaning up expired orders based on keep-alive timeouts.
module aptos_experimental::dead_mans_switch_operations {
    use std::option;
    use std::string;
    use aptos_experimental::market_types::{Self, MarketClearinghouseCallbacks, Market};
    use aptos_experimental::order_book_types::OrderIdType;
    use aptos_experimental::dead_mans_switch_tracker::{Self, is_order_valid};
    use aptos_experimental::order_operations;
    use aptos_experimental::single_order_types;
    use aptos_experimental::bulk_order_book_types;
    use aptos_experimental::market_bulk_order;

    // Error codes
    const E_DEAD_MANS_SWITCH_NOT_ENABLED: u64 = 0;

    const MICROS_PER_SECOND: u64 = 1000000;

    /// Cleans up expired orders based on dead man's switch rules.
    ///
    /// This function validates that each order's creation timestamp is valid according to
    /// the dead man's switch tracker. If an order was created before the current keep-alive
    /// session or if the session has expired, the order will be cancelled.
    ///
    /// Parameters:
    /// - market: The market instance
    /// - order_ids: Vector of order IDs to check and potentially cancel
    /// - callbacks: The market clearinghouse callbacks for cleanup operations
    ///
    /// Aborts:
    /// - E_DEAD_MANS_SWITCH_NOT_ENABLED: If dead man's switch is not enabled for this market
    public fun cleanup_expired_orders<M: store + copy + drop, R: store + copy + drop>(
        market: &mut Market<M>,
        order_ids: vector<OrderIdType>,
        callbacks: &MarketClearinghouseCallbacks<M, R>
    ) {
        // Check if dead man's switch is enabled
        assert!(market.is_dead_mans_switch_enabled(), E_DEAD_MANS_SWITCH_NOT_ENABLED);

        // Loop through each order ID
        let i = 0;
        while (i < order_ids.length()) {
            let order_id = order_ids[i];

            // Get the order from the order book
            let order_opt = market.get_order_book().get_single_order(order_id);

            if (order_opt.is_some()) {
                let order_with_state = order_opt.destroy_some();
                let (order, is_active) = order_with_state.destroy_order_from_state();

                if (!is_active) {
                    // Order is already inactive, skip
                    i += 1;
                    continue;
                };
                // Get account from the order
                let account = single_order_types::get_account(&order);

                // Get creation timestamp in microseconds and convert to seconds
                let creation_time_micros = single_order_types::get_creation_time_micros(&order);
                let creation_time_secs = creation_time_micros / MICROS_PER_SECOND;

                // Check if order is valid according to dead man's switch
                // We get tracker each time to avoid borrowing conflicts
                let tracker = market.get_dead_mans_switch_tracker();
                let is_valid = is_order_valid(tracker, account, option::some(creation_time_secs));

                if (!is_valid) {
                    // Cancel the order
                    order_operations::cancel_order(
                        market,
                        account,
                        order_id,
                        true, // emit_event
                        market_types::order_cancellation_reason_dead_mans_switch_expired(),
                        string::utf8(b"Dead man's switch: Order expired"),
                        callbacks
                    );
                }
            };
            i += 1;
        };
    }

    /// Cleans up an expired bulk order for a given account based on dead man's switch rules.
    ///
    /// This function checks if the bulk order's creation timestamp is valid according to
    /// the dead man's switch tracker. If the order was created before the current keep-alive
    /// session or if the session has expired, the bulk order will be cancelled.
    ///
    /// Parameters:
    /// - market: The market instance
    /// - account: The account whose bulk order should be checked and cleaned up
    /// - callbacks: The market clearinghouse callbacks for cleanup operations
    ///
    /// Aborts:
    /// - E_DEAD_MANS_SWITCH_NOT_ENABLED: If dead man's switch is not enabled for this market
    public fun cleanup_expired_bulk_order<M: store + copy + drop, R: store + copy + drop>(
        market: &mut Market<M>,
        account: address,
        callbacks: &MarketClearinghouseCallbacks<M, R>
    ) {
        // Check if dead man's switch is enabled
        assert!(market.is_dead_mans_switch_enabled(), E_DEAD_MANS_SWITCH_NOT_ENABLED);

        // Get the bulk order from the order book
        let bulk_order = market.get_order_book().get_bulk_order(account);

        // Get creation timestamp in microseconds and convert to seconds
        let creation_time_micros = bulk_order_book_types::get_creation_time_micros(&bulk_order);
        let creation_time_secs = creation_time_micros / MICROS_PER_SECOND;

        // Check if order is valid according to dead man's switch
        let tracker = market.get_dead_mans_switch_tracker();
        let is_valid = is_order_valid(tracker, account, option::some(creation_time_secs));

        if (!is_valid) {
            // Cancel the bulk order
            market_bulk_order::cancel_bulk_order_internal(
                market,
                account,
                market_types::order_cancellation_reason_dead_mans_switch_expired(),
                callbacks
            );
        }
    }

    /// Updates the keep-alive state for a trader in the dead man's switch.
    /// This function should be called periodically by traders to keep their orders active.
    ///
    /// Behavior:
    /// - First update: Creates a new session starting at time 0 (all existing orders remain valid)
    /// - Subsequent updates before expiration: Extends the current session
    /// - Update after expiration: Starts a new session (invalidates all orders placed before now)
    ///
    /// Parameters:
    /// - market: The market instance
    /// - account: The trader's address
    /// - timeout_seconds: Duration in seconds until the session expires.
    ///   Must be >= min_keep_alive_time_secs or 0 to disable.
    ///   Pass 0 to disable the dead man's switch for this account.
    ///
    /// Aborts:
    /// - E_DEAD_MANS_SWITCH_NOT_ENABLED: If dead man's switch is not enabled for this market
    /// - E_KEEP_ALIVE_TIMEOUT_TOO_SHORT: If timeout is less than minimum and not zero
    ///
    /// ```
    public fun keep_alive<M: store + copy + drop>(
        market: &mut Market<M>,
        account: address,
        timeout_seconds: u64
    ) {
        // Check if dead man's switch is enabled
        assert!(market.is_dead_mans_switch_enabled(), E_DEAD_MANS_SWITCH_NOT_ENABLED);

        let tracker = market_types::get_dead_mans_switch_tracker_mut(market);
        dead_mans_switch_tracker::keep_alive(tracker, account, timeout_seconds);
    }
}
