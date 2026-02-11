
<a id="0x7_order_book"></a>

# Module `0x7::order_book`



-  [Enum `OrderBook`](#0x7_order_book_OrderBook)
-  [Constants](#@Constants_0)
-  [Function `new_order_book`](#0x7_order_book_new_order_book)
-  [Function `client_order_id_exists`](#0x7_order_book_client_order_id_exists)
-  [Function `get_single_order_metadata`](#0x7_order_book_get_single_order_metadata)
-  [Function `get_order_id_by_client_id`](#0x7_order_book_get_order_id_by_client_id)
-  [Function `get_single_order`](#0x7_order_book_get_single_order)
-  [Function `get_single_remaining_size`](#0x7_order_book_get_single_remaining_size)
-  [Function `cancel_single_order`](#0x7_order_book_cancel_single_order)
-  [Function `try_cancel_single_order`](#0x7_order_book_try_cancel_single_order)
-  [Function `try_cancel_single_order_with_client_order_id`](#0x7_order_book_try_cancel_single_order_with_client_order_id)
-  [Function `place_maker_order`](#0x7_order_book_place_maker_order)
-  [Function `decrease_single_order_size`](#0x7_order_book_decrease_single_order_size)
-  [Function `set_single_order_metadata`](#0x7_order_book_set_single_order_metadata)
-  [Function `take_ready_price_based_orders`](#0x7_order_book_take_ready_price_based_orders)
-  [Function `take_ready_time_based_orders`](#0x7_order_book_take_ready_time_based_orders)
-  [Function `best_bid_price`](#0x7_order_book_best_bid_price)
-  [Function `best_ask_price`](#0x7_order_book_best_ask_price)
-  [Function `get_slippage_price`](#0x7_order_book_get_slippage_price)
-  [Function `is_taker_order`](#0x7_order_book_is_taker_order)
-  [Function `get_single_match_for_taker`](#0x7_order_book_get_single_match_for_taker)
-  [Function `reinsert_order`](#0x7_order_book_reinsert_order)
-  [Function `get_bulk_order_remaining_size`](#0x7_order_book_get_bulk_order_remaining_size)
-  [Function `place_bulk_order`](#0x7_order_book_place_bulk_order)
-  [Function `get_bulk_order`](#0x7_order_book_get_bulk_order)
-  [Function `cancel_bulk_order`](#0x7_order_book_cancel_bulk_order)
-  [Function `cancel_bulk_order_at_price`](#0x7_order_book_cancel_bulk_order_at_price)


<pre><code><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;
<b>use</b> <a href="">0x5::bulk_order_types</a>;
<b>use</b> <a href="">0x5::order_book_types</a>;
<b>use</b> <a href="">0x5::order_match_types</a>;
<b>use</b> <a href="">0x5::single_order_types</a>;
<b>use</b> <a href="bulk_order_book.md#0x7_bulk_order_book">0x7::bulk_order_book</a>;
<b>use</b> <a href="price_time_index.md#0x7_price_time_index">0x7::price_time_index</a>;
<b>use</b> <a href="single_order_book.md#0x7_single_order_book">0x7::single_order_book</a>;
</code></pre>



<a id="0x7_order_book_OrderBook"></a>

## Enum `OrderBook`



<pre><code>enum <a href="order_book.md#0x7_order_book_OrderBook">OrderBook</a>&lt;M: <b>copy</b>, drop, store&gt; <b>has</b> store
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>UnifiedV1</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code><a href="single_order_book.md#0x7_single_order_book">single_order_book</a>: <a href="single_order_book.md#0x7_single_order_book_SingleOrderBook">single_order_book::SingleOrderBook</a>&lt;M&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code><a href="bulk_order_book.md#0x7_bulk_order_book">bulk_order_book</a>: <a href="bulk_order_book.md#0x7_bulk_order_book_BulkOrderBook">bulk_order_book::BulkOrderBook</a>&lt;M&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>price_time_idx: <a href="price_time_index.md#0x7_price_time_index_PriceTimeIndex">price_time_index::PriceTimeIndex</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x7_order_book_E_REINSERT_ORDER_MISMATCH"></a>



<pre><code><b>const</b> <a href="order_book.md#0x7_order_book_E_REINSERT_ORDER_MISMATCH">E_REINSERT_ORDER_MISMATCH</a>: u64 = 8;
</code></pre>



<a id="0x7_order_book_new_order_book"></a>

## Function `new_order_book`



<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_new_order_book">new_order_book</a>&lt;M: <b>copy</b>, drop, store&gt;(): <a href="order_book.md#0x7_order_book_OrderBook">order_book::OrderBook</a>&lt;M&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_new_order_book">new_order_book</a>&lt;M: store + <b>copy</b> + drop&gt;(): <a href="order_book.md#0x7_order_book_OrderBook">OrderBook</a>&lt;M&gt; {
    OrderBook::UnifiedV1 {
        <a href="single_order_book.md#0x7_single_order_book">single_order_book</a>: new_single_order_book(),
        <a href="bulk_order_book.md#0x7_bulk_order_book">bulk_order_book</a>: new_bulk_order_book(),
        price_time_idx: new_price_time_idx()
    }
}
</code></pre>



</details>

<a id="0x7_order_book_client_order_id_exists"></a>

## Function `client_order_id_exists`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book.md#0x7_order_book_client_order_id_exists">client_order_id_exists</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="order_book.md#0x7_order_book_OrderBook">order_book::OrderBook</a>&lt;M&gt;, order_creator: <b>address</b>, client_order_id: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book.md#0x7_order_book_client_order_id_exists">client_order_id_exists</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="order_book.md#0x7_order_book_OrderBook">OrderBook</a>&lt;M&gt;, order_creator: <b>address</b>, client_order_id: String
): bool {
    self.<a href="single_order_book.md#0x7_single_order_book">single_order_book</a>.<a href="order_book.md#0x7_order_book_client_order_id_exists">client_order_id_exists</a>(order_creator, client_order_id)
}
</code></pre>



</details>

<a id="0x7_order_book_get_single_order_metadata"></a>

## Function `get_single_order_metadata`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book.md#0x7_order_book_get_single_order_metadata">get_single_order_metadata</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="order_book.md#0x7_order_book_OrderBook">order_book::OrderBook</a>&lt;M&gt;, order_id: <a href="_OrderId">order_book_types::OrderId</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;M&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book.md#0x7_order_book_get_single_order_metadata">get_single_order_metadata</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="order_book.md#0x7_order_book_OrderBook">OrderBook</a>&lt;M&gt;, order_id: OrderId
): Option&lt;M&gt; {
    self.<a href="single_order_book.md#0x7_single_order_book">single_order_book</a>.get_order_metadata(order_id)
}
</code></pre>



</details>

<a id="0x7_order_book_get_order_id_by_client_id"></a>

## Function `get_order_id_by_client_id`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book.md#0x7_order_book_get_order_id_by_client_id">get_order_id_by_client_id</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="order_book.md#0x7_order_book_OrderBook">order_book::OrderBook</a>&lt;M&gt;, order_creator: <b>address</b>, client_order_id: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="_OrderId">order_book_types::OrderId</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book.md#0x7_order_book_get_order_id_by_client_id">get_order_id_by_client_id</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="order_book.md#0x7_order_book_OrderBook">OrderBook</a>&lt;M&gt;, order_creator: <b>address</b>, client_order_id: String
): Option&lt;OrderId&gt; {
    self.<a href="single_order_book.md#0x7_single_order_book">single_order_book</a>.<a href="order_book.md#0x7_order_book_get_order_id_by_client_id">get_order_id_by_client_id</a>(order_creator, client_order_id)
}
</code></pre>



</details>

<a id="0x7_order_book_get_single_order"></a>

## Function `get_single_order`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book.md#0x7_order_book_get_single_order">get_single_order</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="order_book.md#0x7_order_book_OrderBook">order_book::OrderBook</a>&lt;M&gt;, order_id: <a href="_OrderId">order_book_types::OrderId</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="_OrderWithState">single_order_types::OrderWithState</a>&lt;M&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book.md#0x7_order_book_get_single_order">get_single_order</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="order_book.md#0x7_order_book_OrderBook">OrderBook</a>&lt;M&gt;, order_id: OrderId
): Option&lt;aptos_trading::single_order_types::OrderWithState&lt;M&gt;&gt; {
    self.<a href="single_order_book.md#0x7_single_order_book">single_order_book</a>.get_order(order_id)
}
</code></pre>



</details>

<a id="0x7_order_book_get_single_remaining_size"></a>

## Function `get_single_remaining_size`



<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_get_single_remaining_size">get_single_remaining_size</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="order_book.md#0x7_order_book_OrderBook">order_book::OrderBook</a>&lt;M&gt;, order_id: <a href="_OrderId">order_book_types::OrderId</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_get_single_remaining_size">get_single_remaining_size</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="order_book.md#0x7_order_book_OrderBook">OrderBook</a>&lt;M&gt;, order_id: OrderId
): u64 {
    self.<a href="single_order_book.md#0x7_single_order_book">single_order_book</a>.get_remaining_size(order_id)
}
</code></pre>



</details>

<a id="0x7_order_book_cancel_single_order"></a>

## Function `cancel_single_order`



<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_cancel_single_order">cancel_single_order</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="order_book.md#0x7_order_book_OrderBook">order_book::OrderBook</a>&lt;M&gt;, order_creator: <b>address</b>, order_id: <a href="_OrderId">order_book_types::OrderId</a>): <a href="_SingleOrder">single_order_types::SingleOrder</a>&lt;M&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_cancel_single_order">cancel_single_order</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="order_book.md#0x7_order_book_OrderBook">OrderBook</a>&lt;M&gt;, order_creator: <b>address</b>, order_id: OrderId
): SingleOrder&lt;M&gt; {
    self.<a href="single_order_book.md#0x7_single_order_book">single_order_book</a>.cancel_order(
        &<b>mut</b> self.price_time_idx, order_creator, order_id
    )
}
</code></pre>



</details>

<a id="0x7_order_book_try_cancel_single_order"></a>

## Function `try_cancel_single_order`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book.md#0x7_order_book_try_cancel_single_order">try_cancel_single_order</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="order_book.md#0x7_order_book_OrderBook">order_book::OrderBook</a>&lt;M&gt;, order_creator: <b>address</b>, order_id: <a href="_OrderId">order_book_types::OrderId</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="_SingleOrder">single_order_types::SingleOrder</a>&lt;M&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book.md#0x7_order_book_try_cancel_single_order">try_cancel_single_order</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="order_book.md#0x7_order_book_OrderBook">OrderBook</a>&lt;M&gt;, order_creator: <b>address</b>, order_id: OrderId
): Option&lt;SingleOrder&lt;M&gt;&gt; {
    self.<a href="single_order_book.md#0x7_single_order_book">single_order_book</a>.try_cancel_order(
        &<b>mut</b> self.price_time_idx, order_creator, order_id
    )
}
</code></pre>



</details>

<a id="0x7_order_book_try_cancel_single_order_with_client_order_id"></a>

## Function `try_cancel_single_order_with_client_order_id`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book.md#0x7_order_book_try_cancel_single_order_with_client_order_id">try_cancel_single_order_with_client_order_id</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="order_book.md#0x7_order_book_OrderBook">order_book::OrderBook</a>&lt;M&gt;, order_creator: <b>address</b>, client_order_id: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="_SingleOrder">single_order_types::SingleOrder</a>&lt;M&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book.md#0x7_order_book_try_cancel_single_order_with_client_order_id">try_cancel_single_order_with_client_order_id</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="order_book.md#0x7_order_book_OrderBook">OrderBook</a>&lt;M&gt;, order_creator: <b>address</b>, client_order_id: String
): Option&lt;SingleOrder&lt;M&gt;&gt; {
    self.<a href="single_order_book.md#0x7_single_order_book">single_order_book</a>.try_cancel_order_with_client_order_id(
        &<b>mut</b> self.price_time_idx, order_creator, client_order_id
    )
}
</code></pre>



</details>

<a id="0x7_order_book_place_maker_order"></a>

## Function `place_maker_order`



<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_place_maker_order">place_maker_order</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="order_book.md#0x7_order_book_OrderBook">order_book::OrderBook</a>&lt;M&gt;, order_req: <a href="_SingleOrderRequest">single_order_types::SingleOrderRequest</a>&lt;M&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_place_maker_order">place_maker_order</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="order_book.md#0x7_order_book_OrderBook">OrderBook</a>&lt;M&gt;, order_req: SingleOrderRequest&lt;M&gt;
) {
    self.<a href="single_order_book.md#0x7_single_order_book">single_order_book</a>.place_maker_or_pending_order(
        &<b>mut</b> self.price_time_idx, order_req
    );
}
</code></pre>



</details>

<a id="0x7_order_book_decrease_single_order_size"></a>

## Function `decrease_single_order_size`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book.md#0x7_order_book_decrease_single_order_size">decrease_single_order_size</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="order_book.md#0x7_order_book_OrderBook">order_book::OrderBook</a>&lt;M&gt;, order_creator: <b>address</b>, order_id: <a href="_OrderId">order_book_types::OrderId</a>, size_delta: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book.md#0x7_order_book_decrease_single_order_size">decrease_single_order_size</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="order_book.md#0x7_order_book_OrderBook">OrderBook</a>&lt;M&gt;,
    order_creator: <b>address</b>,
    order_id: OrderId,
    size_delta: u64
) {
    self.<a href="single_order_book.md#0x7_single_order_book">single_order_book</a>.decrease_order_size(
        &<b>mut</b> self.price_time_idx,
        order_creator,
        order_id,
        size_delta
    )
}
</code></pre>



</details>

<a id="0x7_order_book_set_single_order_metadata"></a>

## Function `set_single_order_metadata`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book.md#0x7_order_book_set_single_order_metadata">set_single_order_metadata</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="order_book.md#0x7_order_book_OrderBook">order_book::OrderBook</a>&lt;M&gt;, order_id: <a href="_OrderId">order_book_types::OrderId</a>, metadata: M)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book.md#0x7_order_book_set_single_order_metadata">set_single_order_metadata</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="order_book.md#0x7_order_book_OrderBook">OrderBook</a>&lt;M&gt;, order_id: OrderId, metadata: M
) {
    self.<a href="single_order_book.md#0x7_single_order_book">single_order_book</a>.set_order_metadata(order_id, metadata)
}
</code></pre>



</details>

<a id="0x7_order_book_take_ready_price_based_orders"></a>

## Function `take_ready_price_based_orders`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book.md#0x7_order_book_take_ready_price_based_orders">take_ready_price_based_orders</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="order_book.md#0x7_order_book_OrderBook">order_book::OrderBook</a>&lt;M&gt;, oracle_price: u64, order_limit: u64): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="_SingleOrder">single_order_types::SingleOrder</a>&lt;M&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book.md#0x7_order_book_take_ready_price_based_orders">take_ready_price_based_orders</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="order_book.md#0x7_order_book_OrderBook">OrderBook</a>&lt;M&gt;, oracle_price: u64, order_limit: u64
): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;SingleOrder&lt;M&gt;&gt; {
    self.<a href="single_order_book.md#0x7_single_order_book">single_order_book</a>.<a href="order_book.md#0x7_order_book_take_ready_price_based_orders">take_ready_price_based_orders</a>(oracle_price, order_limit)
}
</code></pre>



</details>

<a id="0x7_order_book_take_ready_time_based_orders"></a>

## Function `take_ready_time_based_orders`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book.md#0x7_order_book_take_ready_time_based_orders">take_ready_time_based_orders</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="order_book.md#0x7_order_book_OrderBook">order_book::OrderBook</a>&lt;M&gt;, order_limit: u64): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="_SingleOrder">single_order_types::SingleOrder</a>&lt;M&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book.md#0x7_order_book_take_ready_time_based_orders">take_ready_time_based_orders</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="order_book.md#0x7_order_book_OrderBook">OrderBook</a>&lt;M&gt;, order_limit: u64
): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;SingleOrder&lt;M&gt;&gt; {
    self.<a href="single_order_book.md#0x7_single_order_book">single_order_book</a>.<a href="order_book.md#0x7_order_book_take_ready_time_based_orders">take_ready_time_based_orders</a>(order_limit)
}
</code></pre>



</details>

<a id="0x7_order_book_best_bid_price"></a>

## Function `best_bid_price`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book.md#0x7_order_book_best_bid_price">best_bid_price</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="order_book.md#0x7_order_book_OrderBook">order_book::OrderBook</a>&lt;M&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book.md#0x7_order_book_best_bid_price">best_bid_price</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="order_book.md#0x7_order_book_OrderBook">OrderBook</a>&lt;M&gt;
): Option&lt;u64&gt; {
    self.price_time_idx.<a href="order_book.md#0x7_order_book_best_bid_price">best_bid_price</a>()
}
</code></pre>



</details>

<a id="0x7_order_book_best_ask_price"></a>

## Function `best_ask_price`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book.md#0x7_order_book_best_ask_price">best_ask_price</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="order_book.md#0x7_order_book_OrderBook">order_book::OrderBook</a>&lt;M&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book.md#0x7_order_book_best_ask_price">best_ask_price</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="order_book.md#0x7_order_book_OrderBook">OrderBook</a>&lt;M&gt;
): Option&lt;u64&gt; {
    self.price_time_idx.<a href="order_book.md#0x7_order_book_best_ask_price">best_ask_price</a>()
}
</code></pre>



</details>

<a id="0x7_order_book_get_slippage_price"></a>

## Function `get_slippage_price`



<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_get_slippage_price">get_slippage_price</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="order_book.md#0x7_order_book_OrderBook">order_book::OrderBook</a>&lt;M&gt;, is_bid: bool, slippage_bps: u64): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_get_slippage_price">get_slippage_price</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="order_book.md#0x7_order_book_OrderBook">OrderBook</a>&lt;M&gt;, is_bid: bool, slippage_bps: u64
): Option&lt;u64&gt; {
    self.price_time_idx.<a href="order_book.md#0x7_order_book_get_slippage_price">get_slippage_price</a>(is_bid, slippage_bps)
}
</code></pre>



</details>

<a id="0x7_order_book_is_taker_order"></a>

## Function `is_taker_order`

Checks if the order is a taker order i.e., matched immediately with the active order book.


<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_is_taker_order">is_taker_order</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="order_book.md#0x7_order_book_OrderBook">order_book::OrderBook</a>&lt;M&gt;, price: u64, is_bid: bool, trigger_condition: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="_TriggerCondition">order_book_types::TriggerCondition</a>&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_is_taker_order">is_taker_order</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="order_book.md#0x7_order_book_OrderBook">OrderBook</a>&lt;M&gt;,
    price: u64,
    is_bid: bool,
    trigger_condition: Option&lt;TriggerCondition&gt;
): bool {
    <b>if</b> (trigger_condition.is_some()) {
        <b>return</b> <b>false</b>;
    };
    self.price_time_idx.<a href="order_book.md#0x7_order_book_is_taker_order">is_taker_order</a>(price, is_bid)
}
</code></pre>



</details>

<a id="0x7_order_book_get_single_match_for_taker"></a>

## Function `get_single_match_for_taker`



<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_get_single_match_for_taker">get_single_match_for_taker</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="order_book.md#0x7_order_book_OrderBook">order_book::OrderBook</a>&lt;M&gt;, price: u64, size: u64, is_bid: bool): <a href="_OrderMatch">order_match_types::OrderMatch</a>&lt;M&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_get_single_match_for_taker">get_single_match_for_taker</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="order_book.md#0x7_order_book_OrderBook">OrderBook</a>&lt;M&gt;,
    price: u64,
    size: u64,
    is_bid: bool
): OrderMatch&lt;M&gt; {
    <b>let</b> result = self.price_time_idx.get_single_match_result(price, size, is_bid);
    <b>if</b> (result.is_active_matched_book_type_single_order()) {
        self.<a href="single_order_book.md#0x7_single_order_book">single_order_book</a>.<a href="order_book.md#0x7_order_book_get_single_match_for_taker">get_single_match_for_taker</a>(result)
    } <b>else</b> {
        self.<a href="bulk_order_book.md#0x7_bulk_order_book">bulk_order_book</a>.<a href="order_book.md#0x7_order_book_get_single_match_for_taker">get_single_match_for_taker</a>(
            &<b>mut</b> self.price_time_idx, result, is_bid
        )
    }
}
</code></pre>



</details>

<a id="0x7_order_book_reinsert_order"></a>

## Function `reinsert_order`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book.md#0x7_order_book_reinsert_order">reinsert_order</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="order_book.md#0x7_order_book_OrderBook">order_book::OrderBook</a>&lt;M&gt;, reinsert_order: <a href="_OrderMatchDetails">order_match_types::OrderMatchDetails</a>&lt;M&gt;, original_order: &<a href="_OrderMatchDetails">order_match_types::OrderMatchDetails</a>&lt;M&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book.md#0x7_order_book_reinsert_order">reinsert_order</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="order_book.md#0x7_order_book_OrderBook">OrderBook</a>&lt;M&gt;,
    reinsert_order: OrderMatchDetails&lt;M&gt;,
    original_order: &OrderMatchDetails&lt;M&gt;
) {
    <b>if</b> (reinsert_order.is_single_order_from_match_details()) {
        self.<a href="single_order_book.md#0x7_single_order_book">single_order_book</a>.<a href="order_book.md#0x7_order_book_reinsert_order">reinsert_order</a>(
            &<b>mut</b> self.price_time_idx, reinsert_order, original_order
        )
    } <b>else</b> {
        self.<a href="bulk_order_book.md#0x7_bulk_order_book">bulk_order_book</a>.<a href="order_book.md#0x7_order_book_reinsert_order">reinsert_order</a>(
            &<b>mut</b> self.price_time_idx, reinsert_order, original_order
        );
    }
}
</code></pre>



</details>

<a id="0x7_order_book_get_bulk_order_remaining_size"></a>

## Function `get_bulk_order_remaining_size`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book.md#0x7_order_book_get_bulk_order_remaining_size">get_bulk_order_remaining_size</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="order_book.md#0x7_order_book_OrderBook">order_book::OrderBook</a>&lt;M&gt;, order_creator: <b>address</b>, is_bid: bool): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book.md#0x7_order_book_get_bulk_order_remaining_size">get_bulk_order_remaining_size</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="order_book.md#0x7_order_book_OrderBook">OrderBook</a>&lt;M&gt;, order_creator: <b>address</b>, is_bid: bool
): u64 {
    self.<a href="bulk_order_book.md#0x7_bulk_order_book">bulk_order_book</a>.get_remaining_size(order_creator, is_bid)
}
</code></pre>



</details>

<a id="0x7_order_book_place_bulk_order"></a>

## Function `place_bulk_order`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book.md#0x7_order_book_place_bulk_order">place_bulk_order</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="order_book.md#0x7_order_book_OrderBook">order_book::OrderBook</a>&lt;M&gt;, order_req: <a href="_BulkOrderRequest">bulk_order_types::BulkOrderRequest</a>&lt;M&gt;): <a href="_BulkOrderPlaceResponse">bulk_order_types::BulkOrderPlaceResponse</a>&lt;M&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book.md#0x7_order_book_place_bulk_order">place_bulk_order</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="order_book.md#0x7_order_book_OrderBook">OrderBook</a>&lt;M&gt;, order_req: BulkOrderRequest&lt;M&gt;
): BulkOrderPlaceResponse&lt;M&gt; {
    self.<a href="bulk_order_book.md#0x7_bulk_order_book">bulk_order_book</a>.<a href="order_book.md#0x7_order_book_place_bulk_order">place_bulk_order</a>(&<b>mut</b> self.price_time_idx, order_req)
}
</code></pre>



</details>

<a id="0x7_order_book_get_bulk_order"></a>

## Function `get_bulk_order`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book.md#0x7_order_book_get_bulk_order">get_bulk_order</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="order_book.md#0x7_order_book_OrderBook">order_book::OrderBook</a>&lt;M&gt;, order_creator: <b>address</b>): <a href="_BulkOrder">bulk_order_types::BulkOrder</a>&lt;M&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book.md#0x7_order_book_get_bulk_order">get_bulk_order</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="order_book.md#0x7_order_book_OrderBook">OrderBook</a>&lt;M&gt;, order_creator: <b>address</b>
): BulkOrder&lt;M&gt; {
    self.<a href="bulk_order_book.md#0x7_bulk_order_book">bulk_order_book</a>.<a href="order_book.md#0x7_order_book_get_bulk_order">get_bulk_order</a>(order_creator)
}
</code></pre>



</details>

<a id="0x7_order_book_cancel_bulk_order"></a>

## Function `cancel_bulk_order`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book.md#0x7_order_book_cancel_bulk_order">cancel_bulk_order</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="order_book.md#0x7_order_book_OrderBook">order_book::OrderBook</a>&lt;M&gt;, order_creator: <b>address</b>): <a href="_BulkOrder">bulk_order_types::BulkOrder</a>&lt;M&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book.md#0x7_order_book_cancel_bulk_order">cancel_bulk_order</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="order_book.md#0x7_order_book_OrderBook">OrderBook</a>&lt;M&gt;, order_creator: <b>address</b>
): BulkOrder&lt;M&gt; {
    self.<a href="bulk_order_book.md#0x7_bulk_order_book">bulk_order_book</a>.<a href="order_book.md#0x7_order_book_cancel_bulk_order">cancel_bulk_order</a>(&<b>mut</b> self.price_time_idx, order_creator)
}
</code></pre>



</details>

<a id="0x7_order_book_cancel_bulk_order_at_price"></a>

## Function `cancel_bulk_order_at_price`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book.md#0x7_order_book_cancel_bulk_order_at_price">cancel_bulk_order_at_price</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="order_book.md#0x7_order_book_OrderBook">order_book::OrderBook</a>&lt;M&gt;, order_creator: <b>address</b>, price: u64, is_bid: bool): (u64, <a href="_BulkOrder">bulk_order_types::BulkOrder</a>&lt;M&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book.md#0x7_order_book_cancel_bulk_order_at_price">cancel_bulk_order_at_price</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="order_book.md#0x7_order_book_OrderBook">OrderBook</a>&lt;M&gt;,
    order_creator: <b>address</b>,
    price: u64,
    is_bid: bool
): (u64, BulkOrder&lt;M&gt;) {
    self.<a href="bulk_order_book.md#0x7_bulk_order_book">bulk_order_book</a>.<a href="order_book.md#0x7_order_book_cancel_bulk_order_at_price">cancel_bulk_order_at_price</a>(
        &<b>mut</b> self.price_time_idx,
        order_creator,
        price,
        is_bid
    )
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
