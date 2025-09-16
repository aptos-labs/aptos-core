
<a id="0x7_order_operations"></a>

# Module `0x7::order_operations`

This module provides order cancellation and size reduction APIs for the market.
It includes functions for canceling orders by order ID, canceling orders by client order ID,
and reducing the size of existing orders.


-  [Constants](#@Constants_0)
-  [Function `cancel_order_with_client_id`](#0x7_order_operations_cancel_order_with_client_id)
-  [Function `cancel_order`](#0x7_order_operations_cancel_order)
-  [Function `decrease_order_size`](#0x7_order_operations_decrease_order_size)
-  [Function `cancel_single_order_helper`](#0x7_order_operations_cancel_single_order_helper)


<pre><code><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;
<b>use</b> <a href="market_types.md#0x7_market_types">0x7::market_types</a>;
<b>use</b> <a href="order_book.md#0x7_order_book">0x7::order_book</a>;
<b>use</b> <a href="order_book_types.md#0x7_order_book_types">0x7::order_book_types</a>;
<b>use</b> <a href="order_placement.md#0x7_order_placement">0x7::order_placement</a>;
<b>use</b> <a href="pre_cancellation_tracker.md#0x7_pre_cancellation_tracker">0x7::pre_cancellation_tracker</a>;
<b>use</b> <a href="single_order_types.md#0x7_single_order_types">0x7::single_order_types</a>;
</code></pre>



<a id="@Constants_0"></a>

## Constants


<a id="0x7_order_operations_EORDER_DOES_NOT_EXIST"></a>



<pre><code><b>const</b> <a href="order_operations.md#0x7_order_operations_EORDER_DOES_NOT_EXIST">EORDER_DOES_NOT_EXIST</a>: u64 = 6;
</code></pre>



<a id="0x7_order_operations_ENOT_ORDER_CREATOR"></a>



<pre><code><b>const</b> <a href="order_operations.md#0x7_order_operations_ENOT_ORDER_CREATOR">ENOT_ORDER_CREATOR</a>: u64 = 12;
</code></pre>



<a id="0x7_order_operations_cancel_order_with_client_id"></a>

## Function `cancel_order_with_client_id`

Cancels an order using the client order ID.
This function first tries to cancel an order that's already placed in the order book.
If the order is not found in the order book, it adds the order to the pre-cancellation tracker
so it can be cancelled when it's eventually placed.

Parameters:
- market: The market instance
- user: The signer of the user whose order should be cancelled
- client_order_id: The client order ID of the order to cancel
- callbacks: The market clearinghouse callbacks for cleanup operations


<pre><code><b>public</b> <b>fun</b> <a href="order_operations.md#0x7_order_operations_cancel_order_with_client_id">cancel_order_with_client_id</a>&lt;M: <b>copy</b>, drop, store&gt;(market: &<b>mut</b> <a href="market_types.md#0x7_market_types_Market">market_types::Market</a>&lt;M&gt;, user: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, client_order_id: u64, callbacks: &<a href="market_types.md#0x7_market_types_MarketClearinghouseCallbacks">market_types::MarketClearinghouseCallbacks</a>&lt;M&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_operations.md#0x7_order_operations_cancel_order_with_client_id">cancel_order_with_client_id</a>&lt;M: store + <b>copy</b> + drop&gt;(
    market: &<b>mut</b> Market&lt;M&gt;,
    user: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    client_order_id: u64,
    callbacks: &MarketClearinghouseCallbacks&lt;M&gt;
) {
    <b>let</b> order =
        market.get_order_book_mut().try_cancel_order_with_client_order_id(
            <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(user), client_order_id
        );
    <b>if</b> (order.is_some()) {
        // Order is already placed in the order book, so we can cancel it
        <b>return</b> <a href="order_operations.md#0x7_order_operations_cancel_single_order_helper">cancel_single_order_helper</a>(market, order.destroy_some(), callbacks);
    };
    pre_cancel_order_for_tracker(
        market.get_pre_cancellation_tracker_mut(),
        user,
        client_order_id,
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
- user: The signer of the user whose order should be cancelled
- order_id: The order ID of the order to cancel
- callbacks: The market clearinghouse callbacks for cleanup operations


<pre><code><b>public</b> <b>fun</b> <a href="order_operations.md#0x7_order_operations_cancel_order">cancel_order</a>&lt;M: <b>copy</b>, drop, store&gt;(market: &<b>mut</b> <a href="market_types.md#0x7_market_types_Market">market_types::Market</a>&lt;M&gt;, user: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, order_id: <a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>, callbacks: &<a href="market_types.md#0x7_market_types_MarketClearinghouseCallbacks">market_types::MarketClearinghouseCallbacks</a>&lt;M&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_operations.md#0x7_order_operations_cancel_order">cancel_order</a>&lt;M: store + <b>copy</b> + drop&gt;(
    market: &<b>mut</b> Market&lt;M&gt;,
    user: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    order_id: OrderIdType,
    callbacks: &MarketClearinghouseCallbacks&lt;M&gt;
) {
    <b>let</b> <a href="../../aptos-framework/doc/account.md#0x1_account">account</a> = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(user);
    <b>let</b> order = market.get_order_book_mut().<a href="order_operations.md#0x7_order_operations_cancel_order">cancel_order</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>, order_id);
    <b>assert</b>!(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a> == order.get_account(), <a href="order_operations.md#0x7_order_operations_ENOT_ORDER_CREATOR">ENOT_ORDER_CREATOR</a>);
    <a href="order_operations.md#0x7_order_operations_cancel_single_order_helper">cancel_single_order_helper</a>(market, order, callbacks);
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
- user: The signer of the user whose order size should be reduced
- order_id: The order ID of the order to reduce
- size_delta: The amount by which to reduce the order size
- callbacks: The market clearinghouse callbacks for cleanup operations


<pre><code><b>public</b> <b>fun</b> <a href="order_operations.md#0x7_order_operations_decrease_order_size">decrease_order_size</a>&lt;M: <b>copy</b>, drop, store&gt;(market: &<b>mut</b> <a href="market_types.md#0x7_market_types_Market">market_types::Market</a>&lt;M&gt;, user: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, order_id: <a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>, size_delta: u64, callbacks: &<a href="market_types.md#0x7_market_types_MarketClearinghouseCallbacks">market_types::MarketClearinghouseCallbacks</a>&lt;M&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_operations.md#0x7_order_operations_decrease_order_size">decrease_order_size</a>&lt;M: store + <b>copy</b> + drop&gt;(
    market: &<b>mut</b> Market&lt;M&gt;,
    user: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    order_id: OrderIdType,
    size_delta: u64,
    callbacks: &MarketClearinghouseCallbacks&lt;M&gt;
) {
    <b>let</b> <a href="../../aptos-framework/doc/account.md#0x1_account">account</a> = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(user);
    <b>let</b> <a href="order_book.md#0x7_order_book">order_book</a> = market.get_order_book_mut();
    <a href="order_book.md#0x7_order_book">order_book</a>.<a href="order_operations.md#0x7_order_operations_decrease_order_size">decrease_order_size</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>, order_id, size_delta);
    <b>let</b> maybe_order = <a href="order_book.md#0x7_order_book">order_book</a>.get_order(order_id);
    <b>assert</b>!(maybe_order.is_some(), <a href="order_operations.md#0x7_order_operations_EORDER_DOES_NOT_EXIST">EORDER_DOES_NOT_EXIST</a>);
    <b>let</b> (order, _) = maybe_order.destroy_some().destroy_order_from_state();
    <b>assert</b>!(order.get_account() == <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>, <a href="order_operations.md#0x7_order_operations_ENOT_ORDER_CREATOR">ENOT_ORDER_CREATOR</a>);
    <b>let</b> (
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
    callbacks.<a href="order_operations.md#0x7_order_operations_decrease_order_size">decrease_order_size</a>(
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
        <b>false</b>,
        aptos_experimental::market_types::order_status_size_reduced(),
        std::string::utf8(b"Order size reduced"),
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(metadata),
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(),
        time_in_force,
        callbacks
    );
}
</code></pre>



</details>

<a id="0x7_order_operations_cancel_single_order_helper"></a>

## Function `cancel_single_order_helper`

Internal helper function to cancel a single order.
This function handles the cleanup and event emission for order cancellation.

Parameters:
- market: The market instance
- order: The order to cancel
- callbacks: The market clearinghouse callbacks for cleanup operations


<pre><code><b>fun</b> <a href="order_operations.md#0x7_order_operations_cancel_single_order_helper">cancel_single_order_helper</a>&lt;M: <b>copy</b>, drop, store&gt;(market: &<b>mut</b> <a href="market_types.md#0x7_market_types_Market">market_types::Market</a>&lt;M&gt;, order: <a href="single_order_types.md#0x7_single_order_types_SingleOrder">single_order_types::SingleOrder</a>&lt;M&gt;, callbacks: &<a href="market_types.md#0x7_market_types_MarketClearinghouseCallbacks">market_types::MarketClearinghouseCallbacks</a>&lt;M&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="order_operations.md#0x7_order_operations_cancel_single_order_helper">cancel_single_order_helper</a>&lt;M: store + <b>copy</b> + drop&gt;(
    market: &<b>mut</b> Market&lt;M&gt;,
    order: SingleOrder&lt;M&gt;,
    callbacks: &MarketClearinghouseCallbacks&lt;M&gt;
) {
    <b>let</b> (
        <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>,
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
        <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>, order_id, single_order_book_type(), is_bid, remaining_size, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(metadata), callbacks
    );
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
        std::string::utf8(b"Order cancelled"),
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(metadata),
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(), // trigger_condition
        time_in_force,
        callbacks
    );
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
