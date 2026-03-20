
<a id="0x7_dead_mans_switch_operations"></a>

# Module `0x7::dead_mans_switch_operations`

This module provides dead man's switch operations for the market.
It includes functions for cleaning up expired orders based on keep-alive timeouts.


-  [Constants](#@Constants_0)
-  [Function `cleanup_expired_orders`](#0x7_dead_mans_switch_operations_cleanup_expired_orders)
-  [Function `cleanup_expired_bulk_order`](#0x7_dead_mans_switch_operations_cleanup_expired_bulk_order)
-  [Function `keep_alive`](#0x7_dead_mans_switch_operations_keep_alive)


<pre><code><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;
<b>use</b> <a href="">0x5::bulk_order_types</a>;
<b>use</b> <a href="">0x5::order_book_types</a>;
<b>use</b> <a href="">0x5::single_order_types</a>;
<b>use</b> <a href="dead_mans_switch_tracker.md#0x7_dead_mans_switch_tracker">0x7::dead_mans_switch_tracker</a>;
<b>use</b> <a href="market_types.md#0x7_market_types">0x7::market_types</a>;
<b>use</b> <a href="order_book.md#0x7_order_book">0x7::order_book</a>;
<b>use</b> <a href="order_operations.md#0x7_order_operations">0x7::order_operations</a>;
</code></pre>



<a id="@Constants_0"></a>

## Constants


<a id="0x7_dead_mans_switch_operations_E_DEAD_MANS_SWITCH_NOT_ENABLED"></a>



<pre><code><b>const</b> <a href="dead_mans_switch_operations.md#0x7_dead_mans_switch_operations_E_DEAD_MANS_SWITCH_NOT_ENABLED">E_DEAD_MANS_SWITCH_NOT_ENABLED</a>: u64 = 0;
</code></pre>



<a id="0x7_dead_mans_switch_operations_E_TOO_MANY_ORDERS"></a>



<pre><code><b>const</b> <a href="dead_mans_switch_operations.md#0x7_dead_mans_switch_operations_E_TOO_MANY_ORDERS">E_TOO_MANY_ORDERS</a>: u64 = 1;
</code></pre>



<a id="0x7_dead_mans_switch_operations_MAX_ORDERS_CLEANED_PER_CALL"></a>



<pre><code><b>const</b> <a href="dead_mans_switch_operations.md#0x7_dead_mans_switch_operations_MAX_ORDERS_CLEANED_PER_CALL">MAX_ORDERS_CLEANED_PER_CALL</a>: u64 = 100;
</code></pre>



<a id="0x7_dead_mans_switch_operations_MICROS_PER_SECOND"></a>



<pre><code><b>const</b> <a href="dead_mans_switch_operations.md#0x7_dead_mans_switch_operations_MICROS_PER_SECOND">MICROS_PER_SECOND</a>: u64 = 1000000;
</code></pre>



<a id="0x7_dead_mans_switch_operations_cleanup_expired_orders"></a>

## Function `cleanup_expired_orders`

Cleans up expired orders based on dead man's switch rules.

This function validates that each order's creation timestamp is valid according to
the dead man's switch tracker. If an order was created before the current keep-alive
session or if the session has expired, the order will be cancelled.

Parameters:
- market: The market instance
- order_ids: Vector of order IDs to check and potentially cancel
- callbacks: The market clearinghouse callbacks for cleanup operations

Aborts:
- E_DEAD_MANS_SWITCH_NOT_ENABLED: If dead man's switch is not enabled for this market
- E_TOO_MANY_ORDERS: If more than MAX_ORDERS_CLEANED_PER_CALL order IDs are provided


<pre><code><b>public</b> <b>fun</b> <a href="dead_mans_switch_operations.md#0x7_dead_mans_switch_operations_cleanup_expired_orders">cleanup_expired_orders</a>&lt;M: <b>copy</b>, drop, store, R: <b>copy</b>, drop, store&gt;(market: &<b>mut</b> <a href="market_types.md#0x7_market_types_Market">market_types::Market</a>&lt;M&gt;, order_ids: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="_OrderId">order_book_types::OrderId</a>&gt;, callbacks: &<a href="market_types.md#0x7_market_types_MarketClearinghouseCallbacks">market_types::MarketClearinghouseCallbacks</a>&lt;M, R&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="dead_mans_switch_operations.md#0x7_dead_mans_switch_operations_cleanup_expired_orders">cleanup_expired_orders</a>&lt;M: store + <b>copy</b> + drop, R: store + <b>copy</b> + drop&gt;(
    market: &<b>mut</b> Market&lt;M&gt;,
    order_ids: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;OrderId&gt;,
    callbacks: &MarketClearinghouseCallbacks&lt;M, R&gt;
) {
    // Check <b>if</b> dead man's switch is enabled
    <b>assert</b>!(market.is_dead_mans_switch_enabled(), <a href="dead_mans_switch_operations.md#0x7_dead_mans_switch_operations_E_DEAD_MANS_SWITCH_NOT_ENABLED">E_DEAD_MANS_SWITCH_NOT_ENABLED</a>);
    // Cap the number of orders that can be cleaned in a single call
    <b>assert</b>!(order_ids.length() &lt;= <a href="dead_mans_switch_operations.md#0x7_dead_mans_switch_operations_MAX_ORDERS_CLEANED_PER_CALL">MAX_ORDERS_CLEANED_PER_CALL</a>, <a href="dead_mans_switch_operations.md#0x7_dead_mans_switch_operations_E_TOO_MANY_ORDERS">E_TOO_MANY_ORDERS</a>);

    // Loop through each order ID
    <b>let</b> i = 0;
    <b>while</b> (i &lt; order_ids.length()) {
        <b>let</b> order_id = order_ids[i];

        // Get the order from the order book
        <b>let</b> order_opt = market.get_order_book().get_single_order(order_id);

        <b>if</b> (order_opt.is_some()) {
            <b>let</b> order_with_state = order_opt.destroy_some();
            <b>let</b> (order, is_active) = order_with_state.destroy_order_from_state();

            <b>if</b> (!is_active) {
                // Order is already inactive, skip
                i += 1;
                <b>continue</b>;
            };
            // Get <a href="../../aptos-framework/doc/account.md#0x1_account">account</a> from the order
            <b>let</b> <a href="../../aptos-framework/doc/account.md#0x1_account">account</a> = order.get_order_request().get_account();

            // Get creation <a href="../../aptos-framework/doc/timestamp.md#0x1_timestamp">timestamp</a> in microseconds and convert <b>to</b> seconds
            <b>let</b> creation_time_micros =
                order.get_order_request().get_creation_time_micros();
            <b>let</b> creation_time_secs = creation_time_micros / <a href="dead_mans_switch_operations.md#0x7_dead_mans_switch_operations_MICROS_PER_SECOND">MICROS_PER_SECOND</a>;

            // Check <b>if</b> order is valid according <b>to</b> dead man's switch
            // We get tracker each time <b>to</b> avoid borrowing conflicts
            <b>let</b> tracker = market.get_dead_mans_switch_tracker();
            <b>let</b> is_valid =
                is_order_valid(tracker, <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(creation_time_secs));

            <b>if</b> (!is_valid) {
                // Cancel the order
                <a href="order_operations.md#0x7_order_operations_cancel_order">order_operations::cancel_order</a>(
                    market,
                    <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>,
                    order_id,
                    <b>true</b>, // emit_event
                    <a href="market_types.md#0x7_market_types_order_cancellation_reason_dead_mans_switch_expired">market_types::order_cancellation_reason_dead_mans_switch_expired</a>(),
                    <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b"Dead man's switch: Order expired"),
                    callbacks
                );
            }
        };
        i += 1;
    };
}
</code></pre>



</details>

<a id="0x7_dead_mans_switch_operations_cleanup_expired_bulk_order"></a>

## Function `cleanup_expired_bulk_order`

Cleans up an expired bulk order for a given account based on dead man's switch rules.

This function checks if the bulk order's creation timestamp is valid according to
the dead man's switch tracker. If the order was created before the current keep-alive
session or if the session has expired, the bulk order will be cancelled.

Parameters:
- market: The market instance
- account: The account whose bulk order should be checked and cleaned up
- callbacks: The market clearinghouse callbacks for cleanup operations

Aborts:
- E_DEAD_MANS_SWITCH_NOT_ENABLED: If dead man's switch is not enabled for this market


<pre><code><b>public</b> <b>fun</b> <a href="dead_mans_switch_operations.md#0x7_dead_mans_switch_operations_cleanup_expired_bulk_order">cleanup_expired_bulk_order</a>&lt;M: <b>copy</b>, drop, store, R: <b>copy</b>, drop, store&gt;(market: &<b>mut</b> <a href="market_types.md#0x7_market_types_Market">market_types::Market</a>&lt;M&gt;, <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>, callbacks: &<a href="market_types.md#0x7_market_types_MarketClearinghouseCallbacks">market_types::MarketClearinghouseCallbacks</a>&lt;M, R&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="dead_mans_switch_operations.md#0x7_dead_mans_switch_operations_cleanup_expired_bulk_order">cleanup_expired_bulk_order</a>&lt;M: store + <b>copy</b> + drop, R: store + <b>copy</b> + drop&gt;(
    market: &<b>mut</b> Market&lt;M&gt;,
    <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>,
    callbacks: &MarketClearinghouseCallbacks&lt;M, R&gt;
) {
    // Check <b>if</b> dead man's switch is enabled
    <b>assert</b>!(market.is_dead_mans_switch_enabled(), <a href="dead_mans_switch_operations.md#0x7_dead_mans_switch_operations_E_DEAD_MANS_SWITCH_NOT_ENABLED">E_DEAD_MANS_SWITCH_NOT_ENABLED</a>);

    // Get the bulk order from the order book
    <b>let</b> bulk_order = market.get_order_book().get_bulk_order(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>);

    // Get creation <a href="../../aptos-framework/doc/timestamp.md#0x1_timestamp">timestamp</a> in microseconds and convert <b>to</b> seconds
    <b>let</b> creation_time_micros = bulk_order.get_creation_time_micros();
    <b>let</b> creation_time_secs = creation_time_micros / <a href="dead_mans_switch_operations.md#0x7_dead_mans_switch_operations_MICROS_PER_SECOND">MICROS_PER_SECOND</a>;

    // Check <b>if</b> order is valid according <b>to</b> dead man's switch
    <b>let</b> tracker = market.get_dead_mans_switch_tracker();
    <b>let</b> is_valid = is_order_valid(
        tracker, <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(creation_time_secs)
    );

    <b>if</b> (!is_valid) {
        // Cancel the bulk order
        <a href="market_bulk_order.md#0x7_market_bulk_order_cancel_bulk_order_internal">market_bulk_order::cancel_bulk_order_internal</a>(
            market,
            <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>,
            <a href="market_types.md#0x7_market_types_order_cancellation_reason_dead_mans_switch_expired">market_types::order_cancellation_reason_dead_mans_switch_expired</a>(),
            callbacks
        );
    }
}
</code></pre>



</details>

<a id="0x7_dead_mans_switch_operations_keep_alive"></a>

## Function `keep_alive`

Updates the keep-alive state for a trader in the dead man's switch.
This function should be called periodically by traders to keep their orders active.

This function does not validate the account parameter. It is the caller's responsibility
to ensure proper signer validation is performed before calling this function if needed.

Behavior:
- First update: Creates a new session starting at time 0 (all existing orders remain valid)
- Subsequent updates before expiration: Extends the current session
- Update after expiration: Starts a new session (invalidates all orders placed before now)

Parameters:
- market: The market instance
- account: The trader's address
- timeout_seconds: Duration in seconds until the session expires.
Must be >= min_keep_alive_time_secs or 0 to disable.
Pass 0 to disable the dead man's switch for this account.

Aborts:
- E_DEAD_MANS_SWITCH_NOT_ENABLED: If dead man's switch is not enabled for this market
- E_KEEP_ALIVE_TIMEOUT_TOO_SHORT: If timeout is less than minimum and not zero

```


<pre><code><b>public</b> <b>fun</b> <a href="dead_mans_switch_operations.md#0x7_dead_mans_switch_operations_keep_alive">keep_alive</a>&lt;M: <b>copy</b>, drop, store&gt;(market: &<b>mut</b> <a href="market_types.md#0x7_market_types_Market">market_types::Market</a>&lt;M&gt;, <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>, timeout_seconds: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="dead_mans_switch_operations.md#0x7_dead_mans_switch_operations_keep_alive">keep_alive</a>&lt;M: store + <b>copy</b> + drop&gt;(
    market: &<b>mut</b> Market&lt;M&gt;, <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>, timeout_seconds: u64
) {
    // Check <b>if</b> dead man's switch is enabled
    <b>assert</b>!(market.is_dead_mans_switch_enabled(), <a href="dead_mans_switch_operations.md#0x7_dead_mans_switch_operations_E_DEAD_MANS_SWITCH_NOT_ENABLED">E_DEAD_MANS_SWITCH_NOT_ENABLED</a>);

    <b>let</b> parent = market.get_parent();
    <b>let</b> market_addr = market.get_market();
    <b>let</b> tracker = market.get_dead_mans_switch_tracker_mut();
    <a href="dead_mans_switch_tracker.md#0x7_dead_mans_switch_tracker_keep_alive">dead_mans_switch_tracker::keep_alive</a>(
        tracker,
        parent,
        market_addr,
        <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>,
        timeout_seconds
    );
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
