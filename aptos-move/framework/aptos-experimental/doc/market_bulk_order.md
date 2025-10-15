
<a id="0x7_market_bulk_order"></a>

# Module `0x7::market_bulk_order`

This module provides bulk order placement and cancellation APIs for the market.
Bulk orders allow users to place multiple bid and ask orders at different price levels
in a single transaction, improving efficiency for market makers.


-  [Function `place_bulk_order`](#0x7_market_bulk_order_place_bulk_order)
-  [Function `cancel_bulk_order`](#0x7_market_bulk_order_cancel_bulk_order)
-  [Function `cancel_bulk_order_internal`](#0x7_market_bulk_order_cancel_bulk_order_internal)


<pre><code><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types">0x7::bulk_order_book_types</a>;
<b>use</b> <a href="market_types.md#0x7_market_types">0x7::market_types</a>;
<b>use</b> <a href="order_book.md#0x7_order_book">0x7::order_book</a>;
<b>use</b> <a href="order_book_types.md#0x7_order_book_types">0x7::order_book_types</a>;
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
- Option<OrderIdType>: The bulk order ID if successfully placed, None if validation failed


<pre><code><b>public</b> <b>fun</b> <a href="market_bulk_order.md#0x7_market_bulk_order_place_bulk_order">place_bulk_order</a>&lt;M: <b>copy</b>, drop, store, R: <b>copy</b>, drop, store&gt;(market: &<b>mut</b> <a href="market_types.md#0x7_market_types_Market">market_types::Market</a>&lt;M&gt;, <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>, sequence_number: u64, bid_prices: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, bid_sizes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, ask_prices: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, ask_sizes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, metadata: M, callbacks: &<a href="market_types.md#0x7_market_types_MarketClearinghouseCallbacks">market_types::MarketClearinghouseCallbacks</a>&lt;M, R&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>&gt;
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
): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;OrderIdType&gt; {
    // TODO(skedia) Add support for events for bulk orders
    <b>if</b> (!callbacks.validate_bulk_order_placement(
        <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>,
        bid_prices,
        bid_sizes,
        ask_prices,
        ask_sizes,
        metadata,
    )) {
        // If the bulk order is not valid, we simply <b>return</b> without placing the order.
        <b>return</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>();
    };
    <b>let</b> bulk_order = market.get_order_book_mut().<a href="market_bulk_order.md#0x7_market_bulk_order_place_bulk_order">place_bulk_order</a>(new_bulk_order_request(
        <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>,
        sequence_number,
        bid_prices,
        bid_sizes,
        ask_prices,
        ask_sizes,
        metadata,
    ));
    <b>let</b> (order_id, _, _, _, bid_sizes, bid_prices, ask_sizes, ask_prices, _ ) = bulk_order.destroy_bulk_order(); // We don't need <b>to</b> keep the bulk order <b>struct</b> after placement
    // Emit an <a href="../../aptos-framework/doc/event.md#0x1_event">event</a> for the placed bulk order
    market.emit_event_for_bulk_order_placed(order_id, <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>, bid_sizes, bid_prices, ask_sizes, ask_prices);
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
- callbacks: The market clearinghouse callbacks for cleanup operations


<pre><code><b>public</b> <b>fun</b> <a href="market_bulk_order.md#0x7_market_bulk_order_cancel_bulk_order">cancel_bulk_order</a>&lt;M: <b>copy</b>, drop, store, R: <b>copy</b>, drop, store&gt;(market: &<b>mut</b> <a href="market_types.md#0x7_market_types_Market">market_types::Market</a>&lt;M&gt;, user: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, callbacks: &<a href="market_types.md#0x7_market_types_MarketClearinghouseCallbacks">market_types::MarketClearinghouseCallbacks</a>&lt;M, R&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_bulk_order.md#0x7_market_bulk_order_cancel_bulk_order">cancel_bulk_order</a>&lt;M: store + <b>copy</b> + drop, R: store + <b>copy</b> + drop&gt;(
    market: &<b>mut</b> Market&lt;M&gt;,
    user: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    callbacks: &MarketClearinghouseCallbacks&lt;M, R&gt;
) {
    <b>let</b> <a href="../../aptos-framework/doc/account.md#0x1_account">account</a> = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(user);
    <a href="market_bulk_order.md#0x7_market_bulk_order_cancel_bulk_order_internal">cancel_bulk_order_internal</a>(market, <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>, callbacks);
}
</code></pre>



</details>

<a id="0x7_market_bulk_order_cancel_bulk_order_internal"></a>

## Function `cancel_bulk_order_internal`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="market_bulk_order.md#0x7_market_bulk_order_cancel_bulk_order_internal">cancel_bulk_order_internal</a>&lt;M: <b>copy</b>, drop, store, R: <b>copy</b>, drop, store&gt;(market: &<b>mut</b> <a href="market_types.md#0x7_market_types_Market">market_types::Market</a>&lt;M&gt;, user: <b>address</b>, callbacks: &<a href="market_types.md#0x7_market_types_MarketClearinghouseCallbacks">market_types::MarketClearinghouseCallbacks</a>&lt;M, R&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>package</b>) <b>fun</b> <a href="market_bulk_order.md#0x7_market_bulk_order_cancel_bulk_order_internal">cancel_bulk_order_internal</a>&lt;M: store + <b>copy</b> + drop, R: store + <b>copy</b> + drop&gt;(
    market: &<b>mut</b> Market&lt;M&gt;,
    user: <b>address</b>,
    callbacks: &MarketClearinghouseCallbacks&lt;M, R&gt;
) {
    <b>let</b> cancelled_bulk_order = market.get_order_book_mut().<a href="market_bulk_order.md#0x7_market_bulk_order_cancel_bulk_order">cancel_bulk_order</a>(user);
    <b>let</b> (order_id, _, _, _, bid_sizes, bid_prices, ask_sizes, ask_prices, _ ) = cancelled_bulk_order.destroy_bulk_order();
    <b>let</b> i = 0;
    <b>while</b> (i &lt; bid_sizes.length()) {
        callbacks.cleanup_bulk_order_at_price(user, order_id, <b>true</b>, bid_prices[i], bid_sizes[i]);
        i += 1;
    };
    <b>let</b> j = 0;
    <b>while</b> (j &lt; ask_sizes.length()) {
        callbacks.cleanup_bulk_order_at_price(user, order_id, <b>false</b>, ask_prices[j], ask_sizes[j]);
        j += 1;
    };
    market.emit_event_for_bulk_order_cancelled(order_id, user);
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
