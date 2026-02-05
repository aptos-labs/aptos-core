
<a id="0x7_single_order_book"></a>

# Module `0x7::single_order_book`

This module provides a core order book functionality for a trading system. On a high level, it has three major
components
1. ActiveOrderBook: This is the main order book that keeps track of active orders and their states. The active order
book is backed by a BigOrderedMap, which is a data structure that allows for efficient insertion, deletion, and matching of the order
The orders are matched based on price-time priority.
2. PendingOrderBookIndex: This keeps track of pending orders. The pending orders are those that are not active yet. Three
types of pending orders are supported.
- Price move up - Triggered when the price moves above a certain price level
- Price move down - Triggered when the price moves below a certain price level
- Time based - Triggered when a certain time has passed
3. Orders: This is a BigOrderedMap of order id to order details.


-  [Enum `SingleOrderBook`](#0x7_single_order_book_SingleOrderBook)
-  [Constants](#@Constants_0)
-  [Function `__lambda__1__get_single_match_for_taker`](#0x7_single_order_book___lambda__1__get_single_match_for_taker)
-  [Function `__lambda__1__reinsert_order`](#0x7_single_order_book___lambda__1__reinsert_order)
-  [Function `__lambda__1__set_order_metadata`](#0x7_single_order_book___lambda__1__set_order_metadata)
-  [Function `__lambda__1__decrease_order_size`](#0x7_single_order_book___lambda__1__decrease_order_size)
-  [Function `new_single_order_book`](#0x7_single_order_book_new_single_order_book)
-  [Function `cancel_order`](#0x7_single_order_book_cancel_order)
-  [Function `try_cancel_order_with_client_order_id`](#0x7_single_order_book_try_cancel_order_with_client_order_id)
-  [Function `try_cancel_order`](#0x7_single_order_book_try_cancel_order)
-  [Function `client_order_id_exists`](#0x7_single_order_book_client_order_id_exists)
-  [Function `place_maker_or_pending_order`](#0x7_single_order_book_place_maker_or_pending_order)
-  [Function `place_ready_maker_order_with_unique_idx`](#0x7_single_order_book_place_ready_maker_order_with_unique_idx)
-  [Function `reinsert_order`](#0x7_single_order_book_reinsert_order)
-  [Function `place_pending_order_internal`](#0x7_single_order_book_place_pending_order_internal)
-  [Function `get_single_match_for_taker`](#0x7_single_order_book_get_single_match_for_taker)
-  [Function `decrease_order_size`](#0x7_single_order_book_decrease_order_size)
-  [Function `get_order_id_by_client_id`](#0x7_single_order_book_get_order_id_by_client_id)
-  [Function `get_order_metadata`](#0x7_single_order_book_get_order_metadata)
-  [Function `set_order_metadata`](#0x7_single_order_book_set_order_metadata)
-  [Function `is_active_order`](#0x7_single_order_book_is_active_order)
-  [Function `get_order`](#0x7_single_order_book_get_order)
-  [Function `get_remaining_size`](#0x7_single_order_book_get_remaining_size)
-  [Function `take_ready_price_based_orders`](#0x7_single_order_book_take_ready_price_based_orders)
-  [Function `take_ready_time_based_orders`](#0x7_single_order_book_take_ready_time_based_orders)


<pre><code><b>use</b> <a href="../../aptos-framework/doc/big_ordered_map.md#0x1_big_ordered_map">0x1::big_ordered_map</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
<b>use</b> <a href="">0x5::order_book_types</a>;
<b>use</b> <a href="">0x5::order_match_types</a>;
<b>use</b> <a href="">0x5::single_order_types</a>;
<b>use</b> <a href="pending_order_book_index.md#0x7_pending_order_book_index">0x7::pending_order_book_index</a>;
<b>use</b> <a href="price_time_index.md#0x7_price_time_index">0x7::price_time_index</a>;
</code></pre>



<a id="0x7_single_order_book_SingleOrderBook"></a>

## Enum `SingleOrderBook`



<pre><code>enum <a href="single_order_book.md#0x7_single_order_book_SingleOrderBook">SingleOrderBook</a>&lt;M: <b>copy</b>, drop, store&gt; <b>has</b> store
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>V1</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>orders: <a href="../../aptos-framework/doc/big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">big_ordered_map::BigOrderedMap</a>&lt;<a href="_OrderId">order_book_types::OrderId</a>, <a href="_OrderWithState">single_order_types::OrderWithState</a>&lt;M&gt;&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>client_order_ids: <a href="../../aptos-framework/doc/big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">big_ordered_map::BigOrderedMap</a>&lt;<a href="_AccountClientOrderId">order_book_types::AccountClientOrderId</a>, <a href="_OrderId">order_book_types::OrderId</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>pending_orders: <a href="pending_order_book_index.md#0x7_pending_order_book_index_PendingOrderBookIndex">pending_order_book_index::PendingOrderBookIndex</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x7_single_order_book_E_REINSERT_ORDER_MISMATCH"></a>



<pre><code><b>const</b> <a href="single_order_book.md#0x7_single_order_book_E_REINSERT_ORDER_MISMATCH">E_REINSERT_ORDER_MISMATCH</a>: u64 = 8;
</code></pre>



<a id="0x7_single_order_book_EORDER_ALREADY_EXISTS"></a>



<pre><code><b>const</b> <a href="single_order_book.md#0x7_single_order_book_EORDER_ALREADY_EXISTS">EORDER_ALREADY_EXISTS</a>: u64 = 1;
</code></pre>



<a id="0x7_single_order_book_EINVALID_ADD_SIZE_TO_ORDER"></a>



<pre><code><b>const</b> <a href="single_order_book.md#0x7_single_order_book_EINVALID_ADD_SIZE_TO_ORDER">EINVALID_ADD_SIZE_TO_ORDER</a>: u64 = 6;
</code></pre>



<a id="0x7_single_order_book_EINVALID_INACTIVE_ORDER_STATE"></a>



<pre><code><b>const</b> <a href="single_order_book.md#0x7_single_order_book_EINVALID_INACTIVE_ORDER_STATE">EINVALID_INACTIVE_ORDER_STATE</a>: u64 = 5;
</code></pre>



<a id="0x7_single_order_book_EORDER_CREATOR_MISMATCH"></a>



<pre><code><b>const</b> <a href="single_order_book.md#0x7_single_order_book_EORDER_CREATOR_MISMATCH">EORDER_CREATOR_MISMATCH</a>: u64 = 9;
</code></pre>



<a id="0x7_single_order_book_EORDER_NOT_FOUND"></a>



<pre><code><b>const</b> <a href="single_order_book.md#0x7_single_order_book_EORDER_NOT_FOUND">EORDER_NOT_FOUND</a>: u64 = 4;
</code></pre>



<a id="0x7_single_order_book_EPOST_ONLY_FILLED"></a>



<pre><code><b>const</b> <a href="single_order_book.md#0x7_single_order_book_EPOST_ONLY_FILLED">EPOST_ONLY_FILLED</a>: u64 = 2;
</code></pre>



<a id="0x7_single_order_book_E_NOT_ACTIVE_ORDER"></a>



<pre><code><b>const</b> <a href="single_order_book.md#0x7_single_order_book_E_NOT_ACTIVE_ORDER">E_NOT_ACTIVE_ORDER</a>: u64 = 7;
</code></pre>



<a id="0x7_single_order_book_ENOT_SINGLE_ORDER_BOOK"></a>



<pre><code><b>const</b> <a href="single_order_book.md#0x7_single_order_book_ENOT_SINGLE_ORDER_BOOK">ENOT_SINGLE_ORDER_BOOK</a>: u64 = 10;
</code></pre>



<a id="0x7_single_order_book_ETRIGGER_COND_NOT_FOUND"></a>



<pre><code><b>const</b> <a href="single_order_book.md#0x7_single_order_book_ETRIGGER_COND_NOT_FOUND">ETRIGGER_COND_NOT_FOUND</a>: u64 = 11;
</code></pre>



<a id="0x7_single_order_book___lambda__1__get_single_match_for_taker"></a>

## Function `__lambda__1__get_single_match_for_taker`



<pre><code><b>fun</b> <a href="single_order_book.md#0x7_single_order_book___lambda__1__get_single_match_for_taker">__lambda__1__get_single_match_for_taker</a>&lt;M: <b>copy</b>, drop, store&gt;(remaining_size: u64, v: &<b>mut</b> <a href="_OrderWithState">single_order_types::OrderWithState</a>&lt;M&gt;): <a href="_OrderWithState">single_order_types::OrderWithState</a>&lt;M&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>|v| f(v)
</code></pre>



</details>

<a id="0x7_single_order_book___lambda__1__reinsert_order"></a>

## Function `__lambda__1__reinsert_order`



<pre><code><b>fun</b> <a href="single_order_book.md#0x7_single_order_book___lambda__1__reinsert_order">__lambda__1__reinsert_order</a>&lt;M: <b>copy</b>, drop, store&gt;(reinsert_remaining_size: u64, v: &<b>mut</b> <a href="_OrderWithState">single_order_types::OrderWithState</a>&lt;M&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>|v| modify_f(v)
</code></pre>



</details>

<a id="0x7_single_order_book___lambda__1__set_order_metadata"></a>

## Function `__lambda__1__set_order_metadata`



<pre><code><b>fun</b> <a href="single_order_book.md#0x7_single_order_book___lambda__1__set_order_metadata">__lambda__1__set_order_metadata</a>&lt;M: <b>copy</b>, drop, store&gt;(metadata: M, v: &<b>mut</b> <a href="_OrderWithState">single_order_types::OrderWithState</a>&lt;M&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>|v| modify_f(v)
</code></pre>



</details>

<a id="0x7_single_order_book___lambda__1__decrease_order_size"></a>

## Function `__lambda__1__decrease_order_size`



<pre><code><b>fun</b> <a href="single_order_book.md#0x7_single_order_book___lambda__1__decrease_order_size">__lambda__1__decrease_order_size</a>&lt;M: <b>copy</b>, drop, store&gt;(order_creator: <b>address</b>, size_delta: u64, v: &<b>mut</b> <a href="_OrderWithState">single_order_types::OrderWithState</a>&lt;M&gt;): <a href="_OrderWithState">single_order_types::OrderWithState</a>&lt;M&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>|v| modify_f(v)
</code></pre>



</details>

<a id="0x7_single_order_book_new_single_order_book"></a>

## Function `new_single_order_book`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="single_order_book.md#0x7_single_order_book_new_single_order_book">new_single_order_book</a>&lt;M: <b>copy</b>, drop, store&gt;(): <a href="single_order_book.md#0x7_single_order_book_SingleOrderBook">single_order_book::SingleOrderBook</a>&lt;M&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="single_order_book.md#0x7_single_order_book_new_single_order_book">new_single_order_book</a>&lt;M: store + <b>copy</b> + drop&gt;(): <a href="single_order_book.md#0x7_single_order_book_SingleOrderBook">SingleOrderBook</a>&lt;M&gt; {
    SingleOrderBook::V1 {
        orders: <a href="order_book_utils.md#0x7_order_book_utils_new_default_big_ordered_map">order_book_utils::new_default_big_ordered_map</a>(),
        client_order_ids: <a href="order_book_utils.md#0x7_order_book_utils_new_default_big_ordered_map">order_book_utils::new_default_big_ordered_map</a>(),
        pending_orders: new_pending_order_book_index()
    }
}
</code></pre>



</details>

<a id="0x7_single_order_book_cancel_order"></a>

## Function `cancel_order`

Cancels an order from the order book. If the order is active, it is removed from the active order book else
it is removed from the pending order book.
If order doesn't exist, it aborts with EORDER_NOT_FOUND.

<code>order_creator</code> is passed to only verify order cancellation is authorized correctly


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="single_order_book.md#0x7_single_order_book_cancel_order">cancel_order</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="single_order_book.md#0x7_single_order_book_SingleOrderBook">single_order_book::SingleOrderBook</a>&lt;M&gt;, price_time_idx: &<b>mut</b> <a href="price_time_index.md#0x7_price_time_index_PriceTimeIndex">price_time_index::PriceTimeIndex</a>, order_creator: <b>address</b>, order_id: <a href="_OrderId">order_book_types::OrderId</a>): <a href="_SingleOrder">single_order_types::SingleOrder</a>&lt;M&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="single_order_book.md#0x7_single_order_book_cancel_order">cancel_order</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="single_order_book.md#0x7_single_order_book_SingleOrderBook">SingleOrderBook</a>&lt;M&gt;,
    price_time_idx: &<b>mut</b> PriceTimeIndex,
    order_creator: <b>address</b>,
    order_id: OrderId
): SingleOrder&lt;M&gt; {
    <b>let</b> order_with_state_option = self.orders.remove_or_none(&order_id);
    <b>assert</b>!(order_with_state_option.is_some(), <a href="single_order_book.md#0x7_single_order_book_EORDER_NOT_FOUND">EORDER_NOT_FOUND</a>);
    <b>let</b> order_with_state = order_with_state_option.destroy_some();
    <b>let</b> (order, is_active) = order_with_state.destroy_order_from_state();
    <b>let</b> order_request = order.get_order_request();
    <b>assert</b>!(order_creator == order_request.get_account(), <a href="single_order_book.md#0x7_single_order_book_EORDER_CREATOR_MISMATCH">EORDER_CREATOR_MISMATCH</a>);
    <b>if</b> (is_active) {
        price_time_idx.cancel_active_order(
            order_request.get_price(),
            order.get_unique_priority_idx(),
            order_request.is_bid()
        );
        <b>if</b> (order_request.get_client_order_id().is_some()) {
            self.client_order_ids.remove(
                &new_account_client_order_id(
                    order_request.get_account(),
                    order_request.get_client_order_id().destroy_some()
                )
            );
        };
    } <b>else</b> {
        self.pending_orders.cancel_pending_order(
            order_request.get_trigger_condition().destroy_some(),
            order.get_unique_priority_idx()
        );
        <b>if</b> (order_request.get_client_order_id().is_some()) {
            self.client_order_ids.remove(
                &new_account_client_order_id(
                    order_request.get_account(),
                    order_request.get_client_order_id().destroy_some()
                )
            );
        };
    };
    <b>return</b> order
}
</code></pre>



</details>

<a id="0x7_single_order_book_try_cancel_order_with_client_order_id"></a>

## Function `try_cancel_order_with_client_order_id`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="single_order_book.md#0x7_single_order_book_try_cancel_order_with_client_order_id">try_cancel_order_with_client_order_id</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="single_order_book.md#0x7_single_order_book_SingleOrderBook">single_order_book::SingleOrderBook</a>&lt;M&gt;, price_time_idx: &<b>mut</b> <a href="price_time_index.md#0x7_price_time_index_PriceTimeIndex">price_time_index::PriceTimeIndex</a>, order_creator: <b>address</b>, client_order_id: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="_SingleOrder">single_order_types::SingleOrder</a>&lt;M&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="single_order_book.md#0x7_single_order_book_try_cancel_order_with_client_order_id">try_cancel_order_with_client_order_id</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="single_order_book.md#0x7_single_order_book_SingleOrderBook">SingleOrderBook</a>&lt;M&gt;,
    price_time_idx: &<b>mut</b> PriceTimeIndex,
    order_creator: <b>address</b>,
    client_order_id: String
): Option&lt;SingleOrder&lt;M&gt;&gt; {
    <b>let</b> account_client_order_id =
        new_account_client_order_id(order_creator, client_order_id);
    <b>let</b> order_id = self.client_order_ids.get(&account_client_order_id);
    <b>if</b> (order_id.is_none()) {
        <b>return</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>();
    };
    <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(
        self.<a href="single_order_book.md#0x7_single_order_book_cancel_order">cancel_order</a>(price_time_idx, order_creator, order_id.destroy_some())
    )
}
</code></pre>



</details>

<a id="0x7_single_order_book_try_cancel_order"></a>

## Function `try_cancel_order`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="single_order_book.md#0x7_single_order_book_try_cancel_order">try_cancel_order</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="single_order_book.md#0x7_single_order_book_SingleOrderBook">single_order_book::SingleOrderBook</a>&lt;M&gt;, price_time_idx: &<b>mut</b> <a href="price_time_index.md#0x7_price_time_index_PriceTimeIndex">price_time_index::PriceTimeIndex</a>, order_creator: <b>address</b>, order_id: <a href="_OrderId">order_book_types::OrderId</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="_SingleOrder">single_order_types::SingleOrder</a>&lt;M&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="single_order_book.md#0x7_single_order_book_try_cancel_order">try_cancel_order</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="single_order_book.md#0x7_single_order_book_SingleOrderBook">SingleOrderBook</a>&lt;M&gt;,
    price_time_idx: &<b>mut</b> PriceTimeIndex,
    order_creator: <b>address</b>,
    order_id: OrderId
): Option&lt;SingleOrder&lt;M&gt;&gt; {
    <b>let</b> is_creator =
        self.orders.get_and_map(
            &order_id,
            |order| order.get_order_from_state().get_order_request().get_account()
                == order_creator
        );

    <b>if</b> (is_creator.is_none() || !is_creator.destroy_some()) {
        <b>return</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>();
    };

    <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(self.<a href="single_order_book.md#0x7_single_order_book_cancel_order">cancel_order</a>(price_time_idx, order_creator, order_id))
}
</code></pre>



</details>

<a id="0x7_single_order_book_client_order_id_exists"></a>

## Function `client_order_id_exists`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="single_order_book.md#0x7_single_order_book_client_order_id_exists">client_order_id_exists</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="single_order_book.md#0x7_single_order_book_SingleOrderBook">single_order_book::SingleOrderBook</a>&lt;M&gt;, order_creator: <b>address</b>, client_order_id: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="single_order_book.md#0x7_single_order_book_client_order_id_exists">client_order_id_exists</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="single_order_book.md#0x7_single_order_book_SingleOrderBook">SingleOrderBook</a>&lt;M&gt;, order_creator: <b>address</b>, client_order_id: String
): bool {
    <b>let</b> account_client_order_id =
        new_account_client_order_id(order_creator, client_order_id);
    self.client_order_ids.contains(&account_client_order_id)
}
</code></pre>



</details>

<a id="0x7_single_order_book_place_maker_or_pending_order"></a>

## Function `place_maker_or_pending_order`

Places a maker order to the order book. If the order is a pending order, it is added to the pending order book
else it is added to the active order book. The API aborts if it's not a maker order or if the order already exists


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="single_order_book.md#0x7_single_order_book_place_maker_or_pending_order">place_maker_or_pending_order</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="single_order_book.md#0x7_single_order_book_SingleOrderBook">single_order_book::SingleOrderBook</a>&lt;M&gt;, price_time_idx: &<b>mut</b> <a href="price_time_index.md#0x7_price_time_index_PriceTimeIndex">price_time_index::PriceTimeIndex</a>, order_req: <a href="_SingleOrderRequest">single_order_types::SingleOrderRequest</a>&lt;M&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="single_order_book.md#0x7_single_order_book_place_maker_or_pending_order">place_maker_or_pending_order</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="single_order_book.md#0x7_single_order_book_SingleOrderBook">SingleOrderBook</a>&lt;M&gt;,
    price_time_idx: &<b>mut</b> PriceTimeIndex,
    order_req: SingleOrderRequest&lt;M&gt;
) {
    <b>let</b> ascending_idx = next_increasing_idx_type();
    <b>if</b> (order_req.get_trigger_condition().is_some()) {
        <b>return</b> self.<a href="single_order_book.md#0x7_single_order_book_place_pending_order_internal">place_pending_order_internal</a>(order_req, ascending_idx);
    };
    self.<a href="single_order_book.md#0x7_single_order_book_place_ready_maker_order_with_unique_idx">place_ready_maker_order_with_unique_idx</a>(
        price_time_idx, order_req, ascending_idx
    );
}
</code></pre>



</details>

<a id="0x7_single_order_book_place_ready_maker_order_with_unique_idx"></a>

## Function `place_ready_maker_order_with_unique_idx`



<pre><code><b>fun</b> <a href="single_order_book.md#0x7_single_order_book_place_ready_maker_order_with_unique_idx">place_ready_maker_order_with_unique_idx</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="single_order_book.md#0x7_single_order_book_SingleOrderBook">single_order_book::SingleOrderBook</a>&lt;M&gt;, price_time_idx: &<b>mut</b> <a href="price_time_index.md#0x7_price_time_index_PriceTimeIndex">price_time_index::PriceTimeIndex</a>, order_req: <a href="_SingleOrderRequest">single_order_types::SingleOrderRequest</a>&lt;M&gt;, ascending_idx: <a href="_IncreasingIdx">order_book_types::IncreasingIdx</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="single_order_book.md#0x7_single_order_book_place_ready_maker_order_with_unique_idx">place_ready_maker_order_with_unique_idx</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="single_order_book.md#0x7_single_order_book_SingleOrderBook">SingleOrderBook</a>&lt;M&gt;,
    price_time_idx: &<b>mut</b> PriceTimeIndex,
    order_req: SingleOrderRequest&lt;M&gt;,
    ascending_idx: IncreasingIdx
) {
    <b>let</b> order = new_single_order(order_req, ascending_idx);
    <b>assert</b>!(
        self.orders.upsert(
            order_req.get_order_id(), new_order_with_state(order, <b>true</b>)
        ).is_none(),
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="single_order_book.md#0x7_single_order_book_EORDER_ALREADY_EXISTS">EORDER_ALREADY_EXISTS</a>)
    );
    <b>if</b> (order_req.get_client_order_id().is_some()) {
        self.client_order_ids.add(
            new_account_client_order_id(
                order_req.get_account(),
                order_req.get_client_order_id().destroy_some()
            ),
            order_req.get_order_id()
        );
    };
    price_time_idx.place_maker_order(
        order_req.get_order_id(),
        single_order_type(),
        order_req.get_price(),
        ascending_idx,
        order_req.<a href="single_order_book.md#0x7_single_order_book_get_remaining_size">get_remaining_size</a>(),
        order_req.is_bid()
    );
}
</code></pre>



</details>

<a id="0x7_single_order_book_reinsert_order"></a>

## Function `reinsert_order`

Reinserts a maker order to the order book. This is used when the order is removed from the order book
but the clearinghouse fails to settle all or part of the order. If the order doesn't exist in the order book,
it is added to the order book, if it exists, its size is updated.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="single_order_book.md#0x7_single_order_book_reinsert_order">reinsert_order</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="single_order_book.md#0x7_single_order_book_SingleOrderBook">single_order_book::SingleOrderBook</a>&lt;M&gt;, price_time_idx: &<b>mut</b> <a href="price_time_index.md#0x7_price_time_index_PriceTimeIndex">price_time_index::PriceTimeIndex</a>, reinsert_order: <a href="_OrderMatchDetails">order_match_types::OrderMatchDetails</a>&lt;M&gt;, original_order: &<a href="_OrderMatchDetails">order_match_types::OrderMatchDetails</a>&lt;M&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="single_order_book.md#0x7_single_order_book_reinsert_order">reinsert_order</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="single_order_book.md#0x7_single_order_book_SingleOrderBook">SingleOrderBook</a>&lt;M&gt;,
    price_time_idx: &<b>mut</b> PriceTimeIndex,
    reinsert_order: OrderMatchDetails&lt;M&gt;,
    original_order: &OrderMatchDetails&lt;M&gt;
) {
    <b>assert</b>!(
        reinsert_order.validate_single_order_reinsertion_request(original_order),
        <a href="single_order_book.md#0x7_single_order_book_E_REINSERT_ORDER_MISMATCH">E_REINSERT_ORDER_MISMATCH</a>
    );
    <b>let</b> order_id = reinsert_order.get_order_id_from_match_details();
    <b>let</b> unique_idx = reinsert_order.get_unique_priority_idx_from_match_details();

    <b>let</b> reinsert_remaining_size =
        reinsert_order.get_remaining_size_from_match_details();
    <b>let</b> present =
        self.orders.modify_if_present(
            &order_id,
            |order_with_state| {
                order_with_state.increase_remaining_size_from_state(
                    reinsert_remaining_size
                );
            }
        );
    <b>if</b> (!present) {
        <b>return</b> self.<a href="single_order_book.md#0x7_single_order_book_place_ready_maker_order_with_unique_idx">place_ready_maker_order_with_unique_idx</a>(
            price_time_idx,
            new_order_request_from_match_details(reinsert_order),
            unique_idx
        );
    };

    price_time_idx.increase_order_size(
        reinsert_order.get_price_from_match_details(),
        unique_idx,
        reinsert_order.get_remaining_size_from_match_details(),
        reinsert_order.is_bid_from_match_details()
    );
}
</code></pre>



</details>

<a id="0x7_single_order_book_place_pending_order_internal"></a>

## Function `place_pending_order_internal`



<pre><code><b>fun</b> <a href="single_order_book.md#0x7_single_order_book_place_pending_order_internal">place_pending_order_internal</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="single_order_book.md#0x7_single_order_book_SingleOrderBook">single_order_book::SingleOrderBook</a>&lt;M&gt;, order_req: <a href="_SingleOrderRequest">single_order_types::SingleOrderRequest</a>&lt;M&gt;, ascending_idx: <a href="_IncreasingIdx">order_book_types::IncreasingIdx</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="single_order_book.md#0x7_single_order_book_place_pending_order_internal">place_pending_order_internal</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="single_order_book.md#0x7_single_order_book_SingleOrderBook">SingleOrderBook</a>&lt;M&gt;,
    order_req: SingleOrderRequest&lt;M&gt;,
    ascending_idx: IncreasingIdx
) {
    <b>let</b> order_id = order_req.get_order_id();
    <b>let</b> order = new_single_order(order_req, ascending_idx);
    self.orders.add(order_id, new_order_with_state(order, <b>false</b>));

    <b>if</b> (order_req.get_client_order_id().is_some()) {
        self.client_order_ids.add(
            new_account_client_order_id(
                order_req.get_account(),
                order_req.get_client_order_id().destroy_some()
            ),
            order_req.get_order_id()
        );
    };

    self.pending_orders.place_pending_order(
        order_id,
        order_req.get_trigger_condition().destroy_some(),
        ascending_idx
    );
}
</code></pre>



</details>

<a id="0x7_single_order_book_get_single_match_for_taker"></a>

## Function `get_single_match_for_taker`

Returns a single match for a taker order. It is responsibility of the caller to first call the <code>is_taker_order</code>
API to ensure that the order is a taker order before calling this API, otherwise it will abort.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="single_order_book.md#0x7_single_order_book_get_single_match_for_taker">get_single_match_for_taker</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="single_order_book.md#0x7_single_order_book_SingleOrderBook">single_order_book::SingleOrderBook</a>&lt;M&gt;, active_matched_order: <a href="_ActiveMatchedOrder">order_match_types::ActiveMatchedOrder</a>): <a href="_OrderMatch">order_match_types::OrderMatch</a>&lt;M&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="single_order_book.md#0x7_single_order_book_get_single_match_for_taker">get_single_match_for_taker</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="single_order_book.md#0x7_single_order_book_SingleOrderBook">SingleOrderBook</a>&lt;M&gt;, active_matched_order: ActiveMatchedOrder
): OrderMatch&lt;M&gt; {
    <b>let</b> (order_id, matched_size, remaining_size, order_book_type) =
        active_matched_order.destroy_active_matched_order();
    <b>assert</b>!(order_book_type == single_order_type(), <a href="single_order_book.md#0x7_single_order_book_ENOT_SINGLE_ORDER_BOOK">ENOT_SINGLE_ORDER_BOOK</a>);

    <b>let</b> order_with_state =
        <b>if</b> (remaining_size == 0) {
            <b>let</b> order = self.orders.remove(&order_id);
            order.set_remaining_size_from_state(0);
            order
        } <b>else</b> {
            self.orders.modify_and_return(
                &order_id,
                |order_with_state| {
                    aptos_trading::single_order_types::set_remaining_size_from_state(
                        order_with_state, remaining_size
                    );
                    // order_with_state.set_remaining_size_from_state(remaining_size);
                    *order_with_state
                }
            )
        };

    <b>let</b> (order, is_active) = order_with_state.destroy_order_from_state();
    <b>assert</b>!(is_active, <a href="single_order_book.md#0x7_single_order_book_EINVALID_INACTIVE_ORDER_STATE">EINVALID_INACTIVE_ORDER_STATE</a>);

    <b>let</b> (order_request, unique_priority_idx) = order.destroy_single_order();
    <b>let</b> (
        <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>,
        order_id,
        client_order_id,
        price,
        orig_size,
        size,
        is_bid,
        _trigger_condition,
        time_in_force,
        creation_time_micros,
        metadata
    ) = order_request.destroy_single_order_request();

    <b>if</b> (remaining_size == 0 && client_order_id.is_some()) {
        self.client_order_ids.remove(
            &new_account_client_order_id(
                order.get_order_request().get_account(),
                client_order_id.destroy_some()
            )
        );
    };
    new_order_match(
        new_single_order_match_details(
            order_id,
            <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>,
            client_order_id,
            unique_priority_idx,
            price,
            orig_size,
            size,
            is_bid,
            time_in_force,
            creation_time_micros,
            metadata
        ),
        matched_size
    )
}
</code></pre>



</details>

<a id="0x7_single_order_book_decrease_order_size"></a>

## Function `decrease_order_size`

Decrease the size of the order by the given size delta. The API aborts if the order is not found in the order book or
if the size delta is greater than or equal to the remaining size of the order. Please note that the API will abort and
not cancel the order if the size delta is equal to the remaining size of the order, to avoid unintended
cancellation of the order. Please use the <code>cancel_order</code> API to cancel the order.

<code>order_creator</code> is passed to only verify order cancellation is authorized correctly


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="single_order_book.md#0x7_single_order_book_decrease_order_size">decrease_order_size</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="single_order_book.md#0x7_single_order_book_SingleOrderBook">single_order_book::SingleOrderBook</a>&lt;M&gt;, price_time_idx: &<b>mut</b> <a href="price_time_index.md#0x7_price_time_index_PriceTimeIndex">price_time_index::PriceTimeIndex</a>, order_creator: <b>address</b>, order_id: <a href="_OrderId">order_book_types::OrderId</a>, size_delta: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="single_order_book.md#0x7_single_order_book_decrease_order_size">decrease_order_size</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="single_order_book.md#0x7_single_order_book_SingleOrderBook">SingleOrderBook</a>&lt;M&gt;,
    price_time_idx: &<b>mut</b> PriceTimeIndex,
    order_creator: <b>address</b>,
    order_id: OrderId,
    size_delta: u64
) {
    <b>let</b> order_opt =
        self.orders.modify_if_present_and_return(
            &order_id,
            |order_with_state| {
                <b>assert</b>!(
                    order_creator
                        == order_with_state.get_order_from_state().get_order_request()
                        .get_account(),
                    <a href="single_order_book.md#0x7_single_order_book_EORDER_CREATOR_MISMATCH">EORDER_CREATOR_MISMATCH</a>
                );
                // TODO should we be asserting that remaining size is greater than 0?
                aptos_trading::single_order_types::decrease_remaining_size_from_state(
                    order_with_state, size_delta
                );
                // order_with_state.decrease_remaining_size(size_delta);
                *order_with_state
            }
        );

    <b>assert</b>!(order_opt.is_some(), <a href="single_order_book.md#0x7_single_order_book_EORDER_NOT_FOUND">EORDER_NOT_FOUND</a>);
    <b>let</b> order_with_state = order_opt.destroy_some();

    <b>if</b> (order_with_state.<a href="single_order_book.md#0x7_single_order_book_is_active_order">is_active_order</a>()) {
        <b>let</b> order = order_with_state.get_order_from_state();
        price_time_idx.<a href="single_order_book.md#0x7_single_order_book_decrease_order_size">decrease_order_size</a>(
            order.get_order_request().get_price(),
            order_with_state.get_unique_priority_idx_from_state(),
            size_delta,
            order.get_order_request().is_bid()
        );
    };
}
</code></pre>



</details>

<a id="0x7_single_order_book_get_order_id_by_client_id"></a>

## Function `get_order_id_by_client_id`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="single_order_book.md#0x7_single_order_book_get_order_id_by_client_id">get_order_id_by_client_id</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="single_order_book.md#0x7_single_order_book_SingleOrderBook">single_order_book::SingleOrderBook</a>&lt;M&gt;, order_creator: <b>address</b>, client_order_id: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="_OrderId">order_book_types::OrderId</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="single_order_book.md#0x7_single_order_book_get_order_id_by_client_id">get_order_id_by_client_id</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="single_order_book.md#0x7_single_order_book_SingleOrderBook">SingleOrderBook</a>&lt;M&gt;, order_creator: <b>address</b>, client_order_id: String
): Option&lt;OrderId&gt; {
    <b>let</b> account_client_order_id =
        new_account_client_order_id(order_creator, client_order_id);
    self.client_order_ids.get(&account_client_order_id)
}
</code></pre>



</details>

<a id="0x7_single_order_book_get_order_metadata"></a>

## Function `get_order_metadata`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="single_order_book.md#0x7_single_order_book_get_order_metadata">get_order_metadata</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="single_order_book.md#0x7_single_order_book_SingleOrderBook">single_order_book::SingleOrderBook</a>&lt;M&gt;, order_id: <a href="_OrderId">order_book_types::OrderId</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;M&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="single_order_book.md#0x7_single_order_book_get_order_metadata">get_order_metadata</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="single_order_book.md#0x7_single_order_book_SingleOrderBook">SingleOrderBook</a>&lt;M&gt;, order_id: OrderId
): Option&lt;M&gt; {
    self.orders.get_and_map(&order_id, |order| order.get_metadata_from_state())
}
</code></pre>



</details>

<a id="0x7_single_order_book_set_order_metadata"></a>

## Function `set_order_metadata`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="single_order_book.md#0x7_single_order_book_set_order_metadata">set_order_metadata</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="single_order_book.md#0x7_single_order_book_SingleOrderBook">single_order_book::SingleOrderBook</a>&lt;M&gt;, order_id: <a href="_OrderId">order_book_types::OrderId</a>, metadata: M)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="single_order_book.md#0x7_single_order_book_set_order_metadata">set_order_metadata</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="single_order_book.md#0x7_single_order_book_SingleOrderBook">SingleOrderBook</a>&lt;M&gt;, order_id: OrderId, metadata: M
) {
    <b>let</b> present =
        self.orders.modify_if_present(
            &order_id,
            |order_with_state| {
                order_with_state.set_metadata_in_state(metadata);
            }
        );
    <b>assert</b>!(present, <a href="single_order_book.md#0x7_single_order_book_EORDER_NOT_FOUND">EORDER_NOT_FOUND</a>);
}
</code></pre>



</details>

<a id="0x7_single_order_book_is_active_order"></a>

## Function `is_active_order`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="single_order_book.md#0x7_single_order_book_is_active_order">is_active_order</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="single_order_book.md#0x7_single_order_book_SingleOrderBook">single_order_book::SingleOrderBook</a>&lt;M&gt;, order_id: <a href="_OrderId">order_book_types::OrderId</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="single_order_book.md#0x7_single_order_book_is_active_order">is_active_order</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="single_order_book.md#0x7_single_order_book_SingleOrderBook">SingleOrderBook</a>&lt;M&gt;, order_id: OrderId
): bool {
    self.orders.get_and_map(&order_id, |order| order.<a href="single_order_book.md#0x7_single_order_book_is_active_order">is_active_order</a>()).destroy_with_default(
        <b>false</b>
    )
}
</code></pre>



</details>

<a id="0x7_single_order_book_get_order"></a>

## Function `get_order`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="single_order_book.md#0x7_single_order_book_get_order">get_order</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="single_order_book.md#0x7_single_order_book_SingleOrderBook">single_order_book::SingleOrderBook</a>&lt;M&gt;, order_id: <a href="_OrderId">order_book_types::OrderId</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="_OrderWithState">single_order_types::OrderWithState</a>&lt;M&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="single_order_book.md#0x7_single_order_book_get_order">get_order</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="single_order_book.md#0x7_single_order_book_SingleOrderBook">SingleOrderBook</a>&lt;M&gt;, order_id: OrderId
): Option&lt;OrderWithState&lt;M&gt;&gt; {
    self.orders.get(&order_id)
}
</code></pre>



</details>

<a id="0x7_single_order_book_get_remaining_size"></a>

## Function `get_remaining_size`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="single_order_book.md#0x7_single_order_book_get_remaining_size">get_remaining_size</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="single_order_book.md#0x7_single_order_book_SingleOrderBook">single_order_book::SingleOrderBook</a>&lt;M&gt;, order_id: <a href="_OrderId">order_book_types::OrderId</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="single_order_book.md#0x7_single_order_book_get_remaining_size">get_remaining_size</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="single_order_book.md#0x7_single_order_book_SingleOrderBook">SingleOrderBook</a>&lt;M&gt;, order_id: OrderId
): u64 {
    self.orders.get_and_map(
        &order_id, |order| order.get_remaining_size_from_state()
    ).destroy_with_default(0)
}
</code></pre>



</details>

<a id="0x7_single_order_book_take_ready_price_based_orders"></a>

## Function `take_ready_price_based_orders`

Removes and returns the orders that are ready to be executed based on the current price.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="single_order_book.md#0x7_single_order_book_take_ready_price_based_orders">take_ready_price_based_orders</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="single_order_book.md#0x7_single_order_book_SingleOrderBook">single_order_book::SingleOrderBook</a>&lt;M&gt;, current_price: u64, order_limit: u64): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="_SingleOrder">single_order_types::SingleOrder</a>&lt;M&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="single_order_book.md#0x7_single_order_book_take_ready_price_based_orders">take_ready_price_based_orders</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="single_order_book.md#0x7_single_order_book_SingleOrderBook">SingleOrderBook</a>&lt;M&gt;, current_price: u64, order_limit: u64
): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;SingleOrder&lt;M&gt;&gt; {
    <b>let</b> self_orders = &<b>mut</b> self.orders;
    <b>let</b> self_client_order_ids = &<b>mut</b> self.client_order_ids;
    <b>let</b> order_ids =
        self.pending_orders.<a href="single_order_book.md#0x7_single_order_book_take_ready_price_based_orders">take_ready_price_based_orders</a>(
            current_price, order_limit
        );
    <b>let</b> orders = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>();

    order_ids.for_each(
        |order_id| {
            <b>let</b> order_with_state = self_orders.remove(&order_id);
            <b>let</b> (order, _) = order_with_state.destroy_order_from_state();
            <b>let</b> client_order_id = order.get_order_request().get_client_order_id();
            <b>if</b> (client_order_id.is_some()) {
                self_client_order_ids.remove(
                    &new_account_client_order_id(
                        order.get_order_request().get_account(),
                        client_order_id.destroy_some()
                    )
                );
            };
            orders.push_back(order);
        }
    );
    orders
}
</code></pre>



</details>

<a id="0x7_single_order_book_take_ready_time_based_orders"></a>

## Function `take_ready_time_based_orders`

Removes and returns the orders that are ready to be executed based on the time condition.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="single_order_book.md#0x7_single_order_book_take_ready_time_based_orders">take_ready_time_based_orders</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="single_order_book.md#0x7_single_order_book_SingleOrderBook">single_order_book::SingleOrderBook</a>&lt;M&gt;, order_limit: u64): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="_SingleOrder">single_order_types::SingleOrder</a>&lt;M&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="single_order_book.md#0x7_single_order_book_take_ready_time_based_orders">take_ready_time_based_orders</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="single_order_book.md#0x7_single_order_book_SingleOrderBook">SingleOrderBook</a>&lt;M&gt;, order_limit: u64
): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;SingleOrder&lt;M&gt;&gt; {
    <b>let</b> self_orders = &<b>mut</b> self.orders;
    <b>let</b> self_client_order_ids = &<b>mut</b> self.client_order_ids;
    <b>let</b> order_ids = self.pending_orders.<a href="single_order_book.md#0x7_single_order_book_take_ready_time_based_orders">take_ready_time_based_orders</a>(order_limit);
    <b>let</b> orders = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>();

    order_ids.for_each(
        |order_id| {
            <b>let</b> order_with_state = self_orders.remove(&order_id);
            <b>let</b> (order, _) = order_with_state.destroy_order_from_state();
            <b>let</b> client_order_id = order.get_order_request().get_client_order_id();
            <b>if</b> (client_order_id.is_some()) {
                self_client_order_ids.remove(
                    &new_account_client_order_id(
                        order.get_order_request().get_account(),
                        client_order_id.destroy_some()
                    )
                );
            };
            orders.push_back(order);
        }
    );
    orders
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
