
<a id="0x7_order_operations"></a>

# Module `0x7::order_operations`

This module provides order cancellation and size reduction APIs for the market.
It includes functions for canceling orders by order ID, canceling orders by client order ID,
and reducing the size of existing orders.


-  [Constants](#@Constants_0)
-  [Function `cancel_order_with_client_id`](#0x7_order_operations_cancel_order_with_client_id)
-  [Function `cancel_order`](#0x7_order_operations_cancel_order)
-  [Function `try_cancel_order`](#0x7_order_operations_try_cancel_order)
-  [Function `decrease_order_size`](#0x7_order_operations_decrease_order_size)
-  [Function `cancel_single_order_helper`](#0x7_order_operations_cancel_single_order_helper)


<pre><code><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;
<b>use</b> <a href="">0x5::order_book_types</a>;
<b>use</b> <a href="">0x5::single_order_types</a>;
<b>use</b> <a href="market_clearinghouse_order_info.md#0x7_market_clearinghouse_order_info">0x7::market_clearinghouse_order_info</a>;
<b>use</b> <a href="market_types.md#0x7_market_types">0x7::market_types</a>;
<b>use</b> <a href="order_book.md#0x7_order_book">0x7::order_book</a>;
<b>use</b> <a href="pre_cancellation_tracker.md#0x7_pre_cancellation_tracker">0x7::pre_cancellation_tracker</a>;
</code></pre>



<a id="@Constants_0"></a>

## Constants


<a id="0x7_order_operations_EORDER_DOES_NOT_EXIST"></a>



<pre><code><b>const</b> <a href="order_operations.md#0x7_order_operations_EORDER_DOES_NOT_EXIST">EORDER_DOES_NOT_EXIST</a>: u64 = 6;
</code></pre>



<a id="0x7_order_operations_cancel_order_with_client_id"></a>

## Function `cancel_order_with_client_id`

Cancels an order using the client order ID.
This function first tries to cancel an order that's already placed in the order book.
If the order is not found in the order book, it adds the order to the pre-cancellation tracker
so it can be cancelled when it's eventually placed.

Parameters:
- market: The market instance
- account: address of the account that owns the order - please note that no signer validation is done here.
It it the caller's responsibility to ensure that the account is authorized to cancel the order.
- user: The signer of the user whose order should be cancelled
- client_order_id: The client order ID of the order to cancel
- cancellation_reason: The reason for cancellation
- cancel_reason: String description of the cancellation
- callbacks: The market clearinghouse callbacks for cleanup operations


<pre><code><b>public</b> <b>fun</b> <a href="order_operations.md#0x7_order_operations_cancel_order_with_client_id">cancel_order_with_client_id</a>&lt;M: <b>copy</b>, drop, store, R: <b>copy</b>, drop, store&gt;(market: &<b>mut</b> <a href="market_types.md#0x7_market_types_Market">market_types::Market</a>&lt;M&gt;, user: <b>address</b>, client_order_id: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, cancellation_reason: <a href="market_types.md#0x7_market_types_OrderCancellationReason">market_types::OrderCancellationReason</a>, cancel_reason: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, callbacks: &<a href="market_types.md#0x7_market_types_MarketClearinghouseCallbacks">market_types::MarketClearinghouseCallbacks</a>&lt;M, R&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_operations.md#0x7_order_operations_cancel_order_with_client_id">cancel_order_with_client_id</a>&lt;M: store + <b>copy</b> + drop, R: store + <b>copy</b> + drop&gt;(
    market: &<b>mut</b> Market&lt;M&gt;,
    user: <b>address</b>,
    client_order_id: String,
    cancellation_reason: <a href="market_types.md#0x7_market_types_OrderCancellationReason">market_types::OrderCancellationReason</a>,
    cancel_reason: String,
    callbacks: &MarketClearinghouseCallbacks&lt;M, R&gt;
) {
    <b>let</b> order =
        market.get_order_book_mut().try_cancel_single_order_with_client_order_id(
            user, client_order_id
        );
    <b>if</b> (order.is_some()) {
        // Order is already placed in the order book, so we can cancel it
        <b>return</b> <a href="order_operations.md#0x7_order_operations_cancel_single_order_helper">cancel_single_order_helper</a>(
            market,
            order.destroy_some(),
            <b>true</b>,
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
</code></pre>



</details>

<a id="0x7_order_operations_cancel_order"></a>

## Function `cancel_order`

Cancels an order by order ID.
This will cancel the order and emit an event for the order cancellation.

Parameters:
- market: The market instance
- account: address of the account that owns the order - please note that no signer validation is done here.
It it the caller's responsibility to ensure that the account is authorized to cancel the order.
- user: The signer of the user whose order should be cancelled
- order_id: The order ID of the order to cancel
- cancellation_reason: The reason for cancellation
- cancel_reason: String description of the cancellation
- callbacks: The market clearinghouse callbacks for cleanup operations


<pre><code><b>public</b> <b>fun</b> <a href="order_operations.md#0x7_order_operations_cancel_order">cancel_order</a>&lt;M: <b>copy</b>, drop, store, R: <b>copy</b>, drop, store&gt;(market: &<b>mut</b> <a href="market_types.md#0x7_market_types_Market">market_types::Market</a>&lt;M&gt;, <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>, order_id: <a href="_OrderId">order_book_types::OrderId</a>, emit_event: bool, cancellation_reason: <a href="market_types.md#0x7_market_types_OrderCancellationReason">market_types::OrderCancellationReason</a>, cancel_reason: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, callbacks: &<a href="market_types.md#0x7_market_types_MarketClearinghouseCallbacks">market_types::MarketClearinghouseCallbacks</a>&lt;M, R&gt;): <a href="_SingleOrder">single_order_types::SingleOrder</a>&lt;M&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_operations.md#0x7_order_operations_cancel_order">cancel_order</a>&lt;M: store + <b>copy</b> + drop, R: store + <b>copy</b> + drop&gt;(
    market: &<b>mut</b> Market&lt;M&gt;,
    <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>,
    order_id: OrderId,
    emit_event: bool,
    cancellation_reason: <a href="market_types.md#0x7_market_types_OrderCancellationReason">market_types::OrderCancellationReason</a>,
    cancel_reason: String,
    callbacks: &MarketClearinghouseCallbacks&lt;M, R&gt;
): SingleOrder&lt;M&gt; {
    <b>let</b> order = market.get_order_book_mut().cancel_single_order(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>, order_id);
    <a href="order_operations.md#0x7_order_operations_cancel_single_order_helper">cancel_single_order_helper</a>(
        market,
        order,
        emit_event,
        cancellation_reason,
        cancel_reason,
        callbacks
    );
    order
}
</code></pre>



</details>

<a id="0x7_order_operations_try_cancel_order"></a>

## Function `try_cancel_order`

Tries to cancel an order by order ID.
This function attempts to cancel the order and returns an option containing the order
if it was successfully cancelled, or None if the order does not exist.


<pre><code><b>public</b> <b>fun</b> <a href="order_operations.md#0x7_order_operations_try_cancel_order">try_cancel_order</a>&lt;M: <b>copy</b>, drop, store, R: <b>copy</b>, drop, store&gt;(market: &<b>mut</b> <a href="market_types.md#0x7_market_types_Market">market_types::Market</a>&lt;M&gt;, <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>, order_id: <a href="_OrderId">order_book_types::OrderId</a>, emit_event: bool, cancellation_reason: <a href="market_types.md#0x7_market_types_OrderCancellationReason">market_types::OrderCancellationReason</a>, cancel_reason: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, callbacks: &<a href="market_types.md#0x7_market_types_MarketClearinghouseCallbacks">market_types::MarketClearinghouseCallbacks</a>&lt;M, R&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="_SingleOrder">single_order_types::SingleOrder</a>&lt;M&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_operations.md#0x7_order_operations_try_cancel_order">try_cancel_order</a>&lt;M: store + <b>copy</b> + drop, R: store + <b>copy</b> + drop&gt;(
    market: &<b>mut</b> Market&lt;M&gt;,
    <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>,
    order_id: OrderId,
    emit_event: bool,
    cancellation_reason: <a href="market_types.md#0x7_market_types_OrderCancellationReason">market_types::OrderCancellationReason</a>,
    cancel_reason: String,
    callbacks: &MarketClearinghouseCallbacks&lt;M, R&gt;
): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;SingleOrder&lt;M&gt;&gt; {
    <b>let</b> maybe_order =
        market.get_order_book_mut().try_cancel_single_order(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>, order_id);
    <b>if</b> (maybe_order.is_some()) {
        <b>let</b> order = maybe_order.destroy_some();
        <a href="order_operations.md#0x7_order_operations_cancel_single_order_helper">cancel_single_order_helper</a>(
            market,
            order,
            emit_event,
            cancellation_reason,
            cancel_reason,
            callbacks
        );
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(order)
    } <b>else</b> {
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
    }
}
</code></pre>



</details>

<a id="0x7_order_operations_decrease_order_size"></a>

## Function `decrease_order_size`

Reduces the size of an existing order.
This function decreases the size of an order by the specified amount and emits
an event for the size reduction.

Parameters:
- market: The market instance
- account: address of the account that owns the order - please note that no signer validation is done here.
It it the caller's responsibility to ensure that the account is authorized to modify the order.
- order_id: The order ID of the order to reduce
- size_delta: The amount by which to reduce the order size
- callbacks: The market clearinghouse callbacks for cleanup operations


<pre><code><b>public</b> <b>fun</b> <a href="order_operations.md#0x7_order_operations_decrease_order_size">decrease_order_size</a>&lt;M: <b>copy</b>, drop, store, R: <b>copy</b>, drop, store&gt;(market: &<b>mut</b> <a href="market_types.md#0x7_market_types_Market">market_types::Market</a>&lt;M&gt;, <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>, order_id: <a href="_OrderId">order_book_types::OrderId</a>, size_delta: u64, callbacks: &<a href="market_types.md#0x7_market_types_MarketClearinghouseCallbacks">market_types::MarketClearinghouseCallbacks</a>&lt;M, R&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_operations.md#0x7_order_operations_decrease_order_size">decrease_order_size</a>&lt;M: store + <b>copy</b> + drop, R: store + <b>copy</b> + drop&gt;(
    market: &<b>mut</b> Market&lt;M&gt;,
    <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>,
    order_id: OrderId,
    size_delta: u64,
    callbacks: &MarketClearinghouseCallbacks&lt;M, R&gt;
) {
    <b>let</b> <a href="order_book.md#0x7_order_book">order_book</a> = market.get_order_book_mut();
    <a href="order_book.md#0x7_order_book">order_book</a>.decrease_single_order_size(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>, order_id, size_delta);
    <b>let</b> (order, _) =
        <a href="order_book.md#0x7_order_book">order_book</a>.get_single_order(order_id).destroy_some().destroy_order_from_state();
    <b>let</b> (order_request, _unique_priority_idx) = order.destroy_single_order();
    <b>let</b> (
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
    callbacks.<a href="order_operations.md#0x7_order_operations_decrease_order_size">decrease_order_size</a>(
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
        <b>false</b>,
        aptos_experimental::market_types::order_status_size_reduced(),
        std::string::utf8(b"Order size reduced"),
        metadata,
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(),
        time_in_force,
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(),
        callbacks
    );
}
</code></pre>



</details>

<a id="0x7_order_operations_cancel_single_order_helper"></a>

## Function `cancel_single_order_helper`



<pre><code>#[lint::skip(#[needless_mutable_reference])]
<b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_operations.md#0x7_order_operations_cancel_single_order_helper">cancel_single_order_helper</a>&lt;M: <b>copy</b>, drop, store, R: <b>copy</b>, drop, store&gt;(market: &<b>mut</b> <a href="market_types.md#0x7_market_types_Market">market_types::Market</a>&lt;M&gt;, order: <a href="_SingleOrder">single_order_types::SingleOrder</a>&lt;M&gt;, emit_event: bool, cancellation_reason: <a href="market_types.md#0x7_market_types_OrderCancellationReason">market_types::OrderCancellationReason</a>, cancel_reason: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, callbacks: &<a href="market_types.md#0x7_market_types_MarketClearinghouseCallbacks">market_types::MarketClearinghouseCallbacks</a>&lt;M, R&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_operations.md#0x7_order_operations_cancel_single_order_helper">cancel_single_order_helper</a>&lt;M: store + <b>copy</b> + drop, R: store + <b>copy</b> + drop&gt;(
    market: &<b>mut</b> Market&lt;M&gt;,
    order: SingleOrder&lt;M&gt;,
    emit_event: bool,
    cancellation_reason: <a href="market_types.md#0x7_market_types_OrderCancellationReason">market_types::OrderCancellationReason</a>,
    cancel_reason: String,
    callbacks: &MarketClearinghouseCallbacks&lt;M, R&gt;
) {
    <b>let</b> (order_request, _unique_priority_idx) = order.destroy_single_order();
    <b>let</b> (
        <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>,
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
        <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>,
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
        <b>false</b>
    );
    <b>if</b> (emit_event) {
        market.emit_event_for_order(
            order_id,
            client_order_id,
            <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>,
            orig_size,
            0,
            remaining_size,
            price,
            is_bid,
            <b>false</b>,
            aptos_experimental::market_types::order_status_cancelled(),
            cancel_reason,
            metadata,
            <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(), // trigger_condition
            time_in_force,
            <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(cancellation_reason),
            callbacks
        );
    }
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
