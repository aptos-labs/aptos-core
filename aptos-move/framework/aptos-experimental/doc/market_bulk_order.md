
<a id="0x7_market_bulk_order"></a>

# Module `0x7::market_bulk_order`

This module provides bulk order placement and cancellation APIs for the market.
Bulk orders allow users to place multiple bid and ask orders at different price levels
in a single transaction, improving efficiency for market makers.


-  [Constants](#@Constants_0)
-  [Function `place_bulk_order`](#0x7_market_bulk_order_place_bulk_order)
-  [Function `cancel_bulk_order`](#0x7_market_bulk_order_cancel_bulk_order)
-  [Function `cancel_bulk_order_internal`](#0x7_market_bulk_order_cancel_bulk_order_internal)


<pre><code><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="">0x5::bulk_order_types</a>;
<b>use</b> <a href="">0x5::order_book_types</a>;
<b>use</b> <a href="market_types.md#0x7_market_types">0x7::market_types</a>;
<b>use</b> <a href="order_book.md#0x7_order_book">0x7::order_book</a>;
</code></pre>



<a id="@Constants_0"></a>

## Constants


<a id="0x7_market_bulk_order_E_CLEARINGHOUSE_VALIDATION_FAILED"></a>



<pre><code><b>const</b> <a href="market_bulk_order.md#0x7_market_bulk_order_E_CLEARINGHOUSE_VALIDATION_FAILED">E_CLEARINGHOUSE_VALIDATION_FAILED</a>: u64 = 1;
</code></pre>



<a id="0x7_market_bulk_order_E_SEQUENCE_NUMBER_MISMATCH"></a>



<pre><code><b>const</b> <a href="market_bulk_order.md#0x7_market_bulk_order_E_SEQUENCE_NUMBER_MISMATCH">E_SEQUENCE_NUMBER_MISMATCH</a>: u64 = 0;
</code></pre>



<a id="0x7_market_bulk_order_place_bulk_order"></a>

## Function `place_bulk_order`

Places a bulk order with multiple bid and ask price levels.
This allows market makers to place multiple orders at different price levels
in a single transaction, improving efficiency.

Parameters:
- market: The market instance
- account: The account address placing the bulk order
- bid_prices: Vector of bid prices
- bid_sizes: Vector of bid sizes (must match bid_prices length)
- ask_prices: Vector of ask prices
- ask_sizes: Vector of ask sizes (must match ask_prices length)
- callbacks: The market clearinghouse callbacks for validation and settlement

Returns:
- Option<OrderId>: The bulk order ID if successfully placed, None if rejected


<pre><code><b>public</b> <b>fun</b> <a href="market_bulk_order.md#0x7_market_bulk_order_place_bulk_order">place_bulk_order</a>&lt;M: <b>copy</b>, drop, store, R: <b>copy</b>, drop, store&gt;(market: &<b>mut</b> <a href="market_types.md#0x7_market_types_Market">market_types::Market</a>&lt;M&gt;, <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>, sequence_number: u64, bid_prices: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, bid_sizes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, ask_prices: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, ask_sizes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, metadata: M, callbacks: &<a href="market_types.md#0x7_market_types_MarketClearinghouseCallbacks">market_types::MarketClearinghouseCallbacks</a>&lt;M, R&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="_OrderId">order_book_types::OrderId</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_bulk_order.md#0x7_market_bulk_order_place_bulk_order">place_bulk_order</a>&lt;M: store + <b>copy</b> + drop, R: store + <b>copy</b> + drop&gt;(
    market: &<b>mut</b> Market&lt;M&gt;,
    <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>,
    sequence_number: u64,
    bid_prices: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    bid_sizes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    ask_prices: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    ask_sizes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    metadata: M,
    callbacks: &MarketClearinghouseCallbacks&lt;M, R&gt;
): Option&lt;OrderId&gt; {
    <b>let</b> validation_result =
        callbacks.validate_bulk_order_placement(
            <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>,
            &bid_prices,
            &bid_sizes,
            &ask_prices,
            &ask_sizes,
            &metadata
        );
    <b>assert</b>!(
        validation_result.is_validation_result_valid(),
        <a href="market_bulk_order.md#0x7_market_bulk_order_E_CLEARINGHOUSE_VALIDATION_FAILED">E_CLEARINGHOUSE_VALIDATION_FAILED</a>
    );
    <b>let</b> request =
        new_bulk_order_request(
            <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>,
            sequence_number,
            bid_prices,
            bid_sizes,
            ask_prices,
            ask_sizes,
            metadata
        );
    <b>let</b> response = market.get_order_book_mut().<a href="market_bulk_order.md#0x7_market_bulk_order_place_bulk_order">place_bulk_order</a>(request);

    // Check <b>if</b> the response is a rejection
    <b>if</b> (!response.is_success_response()) {
        <b>let</b> (rejected_account, rejected_seq_num, existing_seq_num) =
            response.destroy_bulk_order_place_response_rejection();
        // Emit rejection <a href="../../aptos-framework/doc/event.md#0x1_event">event</a>
        market.emit_event_for_bulk_order_rejection(
            rejected_account,
            rejected_seq_num,
            existing_seq_num
        );
        // Return None since the order was rejected
        <b>return</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
    };

    // Handle success response
    <b>let</b> (
        bulk_order,
        cancelled_bid_prices,
        cancelled_bid_sizes,
        cancelled_ask_prices,
        cancelled_ask_sizes,
        previous_seq_num_option
    ) = response.destroy_bulk_order_place_response_success();
    <b>let</b> (order_request, order_id, _unique_priority_idx, _creation_time_micros) =
        bulk_order.destroy_bulk_order();
    <b>let</b> (
        <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>,
        order_sequence_number,
        bid_prices,
        bid_sizes,
        ask_prices,
        ask_sizes,
        order_metadata
    ) = order_request.destroy_bulk_order_request();

    <b>assert</b>!(sequence_number == order_sequence_number, <a href="market_bulk_order.md#0x7_market_bulk_order_E_SEQUENCE_NUMBER_MISMATCH">E_SEQUENCE_NUMBER_MISMATCH</a>);
    // Extract previous_seq_num from <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">option</a>, defaulting <b>to</b> 0 <b>if</b> none
    <b>let</b> previous_seq_num = previous_seq_num_option.destroy_with_default(0);
    // Emit an <a href="../../aptos-framework/doc/event.md#0x1_event">event</a> for the placed bulk order
    market.emit_event_for_bulk_order_placed(
        order_id,
        order_sequence_number,
        <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>,
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
    // Invoke the place_bulk_order callback after successful placement
    callbacks.<a href="market_bulk_order.md#0x7_market_bulk_order_place_bulk_order">place_bulk_order</a>(
        <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>,
        order_id,
        &bid_prices,
        &bid_sizes,
        &ask_prices,
        &ask_sizes,
        &cancelled_bid_prices,
        &cancelled_bid_sizes,
        &cancelled_ask_prices,
        &cancelled_ask_sizes,
        &order_metadata
    );
    <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(order_id)
}
</code></pre>



</details>

<a id="0x7_market_bulk_order_cancel_bulk_order"></a>

## Function `cancel_bulk_order`

Cancels all bulk orders for a given user.
This will cancel all bid and ask orders that were placed as part of bulk orders
for the specified user account.

Parameters:
- market: The market instance
- user: The signer of the user whose bulk orders should be cancelled
- cancellation_reason: The reason for cancelling the bulk order
- callbacks: The market clearinghouse callbacks for cleanup operations


<pre><code><b>public</b> <b>fun</b> <a href="market_bulk_order.md#0x7_market_bulk_order_cancel_bulk_order">cancel_bulk_order</a>&lt;M: <b>copy</b>, drop, store, R: <b>copy</b>, drop, store&gt;(market: &<b>mut</b> <a href="market_types.md#0x7_market_types_Market">market_types::Market</a>&lt;M&gt;, user: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, cancellation_reason: <a href="market_types.md#0x7_market_types_OrderCancellationReason">market_types::OrderCancellationReason</a>, callbacks: &<a href="market_types.md#0x7_market_types_MarketClearinghouseCallbacks">market_types::MarketClearinghouseCallbacks</a>&lt;M, R&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_bulk_order.md#0x7_market_bulk_order_cancel_bulk_order">cancel_bulk_order</a>&lt;M: store + <b>copy</b> + drop, R: store + <b>copy</b> + drop&gt;(
    market: &<b>mut</b> Market&lt;M&gt;,
    user: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    cancellation_reason: <a href="market_types.md#0x7_market_types_OrderCancellationReason">market_types::OrderCancellationReason</a>,
    callbacks: &MarketClearinghouseCallbacks&lt;M, R&gt;
) {
    <b>let</b> <a href="../../aptos-framework/doc/account.md#0x1_account">account</a> = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(user);
    <a href="market_bulk_order.md#0x7_market_bulk_order_cancel_bulk_order_internal">cancel_bulk_order_internal</a>(
        market,
        <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>,
        cancellation_reason,
        callbacks
    );
}
</code></pre>



</details>

<a id="0x7_market_bulk_order_cancel_bulk_order_internal"></a>

## Function `cancel_bulk_order_internal`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="market_bulk_order.md#0x7_market_bulk_order_cancel_bulk_order_internal">cancel_bulk_order_internal</a>&lt;M: <b>copy</b>, drop, store, R: <b>copy</b>, drop, store&gt;(market: &<b>mut</b> <a href="market_types.md#0x7_market_types_Market">market_types::Market</a>&lt;M&gt;, user: <b>address</b>, cancellation_reason: <a href="market_types.md#0x7_market_types_OrderCancellationReason">market_types::OrderCancellationReason</a>, callbacks: &<a href="market_types.md#0x7_market_types_MarketClearinghouseCallbacks">market_types::MarketClearinghouseCallbacks</a>&lt;M, R&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="market_bulk_order.md#0x7_market_bulk_order_cancel_bulk_order_internal">cancel_bulk_order_internal</a>&lt;M: store + <b>copy</b> + drop, R: store + <b>copy</b> + drop&gt;(
    market: &<b>mut</b> Market&lt;M&gt;,
    user: <b>address</b>,
    cancellation_reason: <a href="market_types.md#0x7_market_types_OrderCancellationReason">market_types::OrderCancellationReason</a>,
    callbacks: &MarketClearinghouseCallbacks&lt;M, R&gt;
) {
    <b>let</b> cancelled_bulk_order = market.get_order_book_mut().<a href="market_bulk_order.md#0x7_market_bulk_order_cancel_bulk_order">cancel_bulk_order</a>(user);
    <b>let</b> (order_request, order_id, _unique_priority_idx, _creation_time_micros) =
        cancelled_bulk_order.destroy_bulk_order();
    <b>let</b> (
        _account,
        order_sequence_number,
        bid_prices,
        bid_sizes,
        ask_prices,
        ask_sizes,
        _metadata
    ) = order_request.destroy_bulk_order_request();
    <b>let</b> i = 0;
    <b>while</b> (i &lt; bid_sizes.length()) {
        callbacks.cleanup_bulk_order_at_price(
            user,
            order_id,
            <b>true</b>,
            bid_prices[i],
            bid_sizes[i]
        );
        i += 1;
    };
    <b>let</b> j = 0;
    <b>while</b> (j &lt; ask_sizes.length()) {
        callbacks.cleanup_bulk_order_at_price(
            user,
            order_id,
            <b>false</b>,
            ask_prices[j],
            ask_sizes[j]
        );
        j += 1;
    };
    market.emit_event_for_bulk_order_cancelled(
        order_id,
        order_sequence_number,
        user,
        bid_prices,
        bid_sizes,
        ask_prices,
        ask_sizes,
        std::option::some(cancellation_reason)
    );
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
