
<a id="0x7_single_order_book"></a>

# Module `0x7::single_order_book`

This module provides a core order book functionality for a trading system. On a high level, it has three major
components
1. ActiveOrderBook: This is the main order book that keeps track of active orders and their states. The active order
book is backed by a BigOrderedMap, which is a data structure that allows for efficient insertion, deletion, and matching of the order
The orders are matched based on time-price priority.
2. PendingOrderBookIndex: This keeps track of pending orders. The pending orders are those that are not active yet. Three
types of pending orders are supported.
- Price move up - Trigggered when the price moves above a certain price level
- Price move down - Triggered when the price moves below a certain price level
- Time based - Triggered when a certain time has passed
3. Orders: This is a BigOrderMap of order id to order details.


-  [Enum `OrderRequest`](#0x7_single_order_book_OrderRequest)
-  [Enum `RetailOrderBook`](#0x7_single_order_book_RetailOrderBook)
-  [Enum `OrderType`](#0x7_single_order_book_OrderType)
-  [Struct `TestMetadata`](#0x7_single_order_book_TestMetadata)
-  [Constants](#@Constants_0)
-  [Function `new_order_request`](#0x7_single_order_book_new_order_request)
-  [Function `new_single_order_book`](#0x7_single_order_book_new_single_order_book)
-  [Function `new_price_time_index`](#0x7_single_order_book_new_price_time_index)
-  [Function `cancel_order`](#0x7_single_order_book_cancel_order)
-  [Function `try_cancel_order_with_client_order_id`](#0x7_single_order_book_try_cancel_order_with_client_order_id)
-  [Function `client_order_id_exists`](#0x7_single_order_book_client_order_id_exists)
-  [Function `place_maker_order`](#0x7_single_order_book_place_maker_order)
-  [Function `reinsert_maker_order`](#0x7_single_order_book_reinsert_maker_order)
-  [Function `modify_order`](#0x7_single_order_book_modify_order)
-  [Function `modify_and_copy_order`](#0x7_single_order_book_modify_and_copy_order)
-  [Function `modify_or_remove_order`](#0x7_single_order_book_modify_or_remove_order)
-  [Function `place_pending_maker_order`](#0x7_single_order_book_place_pending_maker_order)
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
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
<b>use</b> <a href="order_book_types.md#0x7_order_book_types">0x7::order_book_types</a>;
<b>use</b> <a href="pending_order_book_index.md#0x7_pending_order_book_index">0x7::pending_order_book_index</a>;
<b>use</b> <a href="price_time_index.md#0x7_price_time_index">0x7::price_time_index</a>;
<b>use</b> <a href="single_order_types.md#0x7_single_order_types">0x7::single_order_types</a>;
</code></pre>



<a id="0x7_single_order_book_OrderRequest"></a>

## Enum `OrderRequest`



<pre><code>enum <a href="single_order_book.md#0x7_single_order_book_OrderRequest">OrderRequest</a>&lt;M: <b>copy</b>, drop, store&gt; <b>has</b> <b>copy</b>, drop
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>V1</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code><a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>order_id: <a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a></code>
</dt>
<dd>

</dd>
<dt>
<code>client_order_id: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>price: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>orig_size: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>remaining_size: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>is_bid: bool</code>
</dt>
<dd>

</dd>
<dt>
<code>trigger_condition: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="order_book_types.md#0x7_order_book_types_TriggerCondition">order_book_types::TriggerCondition</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>time_in_force: <a href="order_book_types.md#0x7_order_book_types_TimeInForce">order_book_types::TimeInForce</a></code>
</dt>
<dd>

</dd>
<dt>
<code>metadata: M</code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

</details>

<a id="0x7_single_order_book_RetailOrderBook"></a>

## Enum `RetailOrderBook`



<pre><code>enum <a href="single_order_book.md#0x7_single_order_book_RetailOrderBook">RetailOrderBook</a>&lt;M: <b>copy</b>, drop, store&gt; <b>has</b> store
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>V1</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>orders: <a href="../../aptos-framework/doc/big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">big_ordered_map::BigOrderedMap</a>&lt;<a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>, <a href="single_order_types.md#0x7_single_order_types_OrderWithState">single_order_types::OrderWithState</a>&lt;M&gt;&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>client_order_ids: <a href="../../aptos-framework/doc/big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">big_ordered_map::BigOrderedMap</a>&lt;<a href="order_book_types.md#0x7_order_book_types_AccountClientOrderId">order_book_types::AccountClientOrderId</a>, <a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>&gt;</code>
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

<a id="0x7_single_order_book_OrderType"></a>

## Enum `OrderType`



<pre><code>enum <a href="single_order_book.md#0x7_single_order_book_OrderType">OrderType</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>GoodTilCancelled</summary>


<details>
<summary>Fields</summary>


<dl>
</dl>


</details>

</details>

<details>
<summary>PostOnly</summary>


<details>
<summary>Fields</summary>


<dl>
</dl>


</details>

</details>

<details>
<summary>FillOrKill</summary>


<details>
<summary>Fields</summary>


<dl>
</dl>


</details>

</details>

</details>

<a id="0x7_single_order_book_TestMetadata"></a>

## Struct `TestMetadata`



<pre><code><b>struct</b> <a href="single_order_book.md#0x7_single_order_book_TestMetadata">TestMetadata</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>dummy_field: bool</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


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



<a id="0x7_single_order_book_E_REINSERT_ORDER_MISMATCH"></a>



<pre><code><b>const</b> <a href="single_order_book.md#0x7_single_order_book_E_REINSERT_ORDER_MISMATCH">E_REINSERT_ORDER_MISMATCH</a>: u64 = 8;
</code></pre>



<a id="0x7_single_order_book_new_order_request"></a>

## Function `new_order_request`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="single_order_book.md#0x7_single_order_book_new_order_request">new_order_request</a>&lt;M: <b>copy</b>, drop, store&gt;(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>, order_id: <a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>, client_order_id: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;, price: u64, orig_size: u64, remaining_size: u64, is_bid: bool, trigger_condition: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="order_book_types.md#0x7_order_book_types_TriggerCondition">order_book_types::TriggerCondition</a>&gt;, time_in_force: <a href="order_book_types.md#0x7_order_book_types_TimeInForce">order_book_types::TimeInForce</a>, metadata: M): <a href="single_order_book.md#0x7_single_order_book_OrderRequest">single_order_book::OrderRequest</a>&lt;M&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="single_order_book.md#0x7_single_order_book_new_order_request">new_order_request</a>&lt;M: store + <b>copy</b> + drop&gt;(
    <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>,
    order_id: OrderIdType,
    client_order_id: Option&lt;u64&gt;,
    price: u64,
    orig_size: u64,
    remaining_size: u64,
    is_bid: bool,
    trigger_condition: Option&lt;TriggerCondition&gt;,
    time_in_force: TimeInForce,
    metadata: M
): <a href="single_order_book.md#0x7_single_order_book_OrderRequest">OrderRequest</a>&lt;M&gt; {
    OrderRequest::V1 {
        <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>,
        order_id,
        client_order_id,
        price,
        orig_size,
        remaining_size,
        is_bid,
        trigger_condition,
        time_in_force,
        metadata
    }
}
</code></pre>



</details>

<a id="0x7_single_order_book_new_single_order_book"></a>

## Function `new_single_order_book`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="single_order_book.md#0x7_single_order_book_new_single_order_book">new_single_order_book</a>&lt;M: <b>copy</b>, drop, store&gt;(): <a href="single_order_book.md#0x7_single_order_book_RetailOrderBook">single_order_book::RetailOrderBook</a>&lt;M&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="single_order_book.md#0x7_single_order_book_new_single_order_book">new_single_order_book</a>&lt;M: store + <b>copy</b> + drop&gt;(): <a href="single_order_book.md#0x7_single_order_book_RetailOrderBook">RetailOrderBook</a>&lt;M&gt; {
    RetailOrderBook::V1 {
        orders: new_default_big_ordered_map(),
        client_order_ids: new_default_big_ordered_map(),
        pending_orders: new_pending_order_book_index()
    }
}
</code></pre>



</details>

<a id="0x7_single_order_book_new_price_time_index"></a>

## Function `new_price_time_index`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="single_order_book.md#0x7_single_order_book_new_price_time_index">new_price_time_index</a>(): <a href="price_time_index.md#0x7_price_time_index_PriceTimeIndex">price_time_index::PriceTimeIndex</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="single_order_book.md#0x7_single_order_book_new_price_time_index">new_price_time_index</a>(): PriceTimeIndex {
    new_price_time_idx()
}
</code></pre>



</details>

<a id="0x7_single_order_book_cancel_order"></a>

## Function `cancel_order`

Cancels an order from the order book. If the order is active, it is removed from the active order book else
it is removed from the pending order book.
If order doesn't exist, it aborts with EORDER_NOT_FOUND.

<code>order_creator</code> is passed to only verify order cancellation is authorized correctly


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="single_order_book.md#0x7_single_order_book_cancel_order">cancel_order</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="single_order_book.md#0x7_single_order_book_RetailOrderBook">single_order_book::RetailOrderBook</a>&lt;M&gt;, price_time_idx: &<b>mut</b> <a href="price_time_index.md#0x7_price_time_index_PriceTimeIndex">price_time_index::PriceTimeIndex</a>, order_creator: <b>address</b>, order_id: <a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>): <a href="single_order_types.md#0x7_single_order_types_Order">single_order_types::Order</a>&lt;M&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="single_order_book.md#0x7_single_order_book_cancel_order">cancel_order</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="single_order_book.md#0x7_single_order_book_RetailOrderBook">RetailOrderBook</a>&lt;M&gt;, price_time_idx: &<b>mut</b> PriceTimeIndex, order_creator: <b>address</b>, order_id: OrderIdType
): Order&lt;M&gt; {
    <b>assert</b>!(self.orders.contains(&order_id), <a href="single_order_book.md#0x7_single_order_book_EORDER_NOT_FOUND">EORDER_NOT_FOUND</a>);
    <b>let</b> order_with_state = self.orders.remove(&order_id);
    <b>let</b> (order, is_active) = order_with_state.destroy_order_from_state();
    <b>assert</b>!(order_creator == order.get_account(), <a href="single_order_book.md#0x7_single_order_book_EORDER_CREATOR_MISMATCH">EORDER_CREATOR_MISMATCH</a>);
    <b>if</b> (is_active) {
        <b>let</b> unique_priority_idx = order.get_unique_priority_idx();
        <b>let</b> (
            <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>,
            _order_id,
            client_order_id,
            bid_price,
            _orig_size,
            _size,
            is_bid,
            _,
            _,
            _
        ) = order.destroy_order();
        price_time_idx.cancel_active_order(bid_price, unique_priority_idx, is_bid);
        <b>if</b> (client_order_id.is_some()) {
            self.client_order_ids.remove(
                &new_account_client_order_id(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>, client_order_id.destroy_some())
            );
        };
    } <b>else</b> {
        <b>let</b> unique_priority_idx = order.get_unique_priority_idx();
        <b>let</b> (
            _account,
            _order_id,
            client_order_id,
            _bid_price,
            _orig_size,
            _size,
            _is_bid,
            trigger_condition,
            _,
            _
        ) = order.destroy_order();
        self.pending_orders.cancel_pending_order(
            trigger_condition.destroy_some(), unique_priority_idx
        );
        <b>if</b> (client_order_id.is_some()) {
            self.client_order_ids.remove(
                &new_account_client_order_id(
                    order.get_account(), client_order_id.destroy_some()
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



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="single_order_book.md#0x7_single_order_book_try_cancel_order_with_client_order_id">try_cancel_order_with_client_order_id</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="single_order_book.md#0x7_single_order_book_RetailOrderBook">single_order_book::RetailOrderBook</a>&lt;M&gt;, price_time_idx: &<b>mut</b> <a href="price_time_index.md#0x7_price_time_index_PriceTimeIndex">price_time_index::PriceTimeIndex</a>, order_creator: <b>address</b>, client_order_id: u64): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="single_order_types.md#0x7_single_order_types_Order">single_order_types::Order</a>&lt;M&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="single_order_book.md#0x7_single_order_book_try_cancel_order_with_client_order_id">try_cancel_order_with_client_order_id</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="single_order_book.md#0x7_single_order_book_RetailOrderBook">RetailOrderBook</a>&lt;M&gt;, price_time_idx: &<b>mut</b> PriceTimeIndex, order_creator: <b>address</b>, client_order_id: u64
): Option&lt;Order&lt;M&gt;&gt; {
    <b>let</b> account_client_order_id =
        new_account_client_order_id(order_creator, client_order_id);
    <b>if</b> (!self.client_order_ids.contains(&account_client_order_id)) {
        <b>return</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>();
    };
    <b>let</b> order_id = self.client_order_ids.borrow(&account_client_order_id);
    <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(self.<a href="single_order_book.md#0x7_single_order_book_cancel_order">cancel_order</a>(price_time_idx, order_creator, *order_id))
}
</code></pre>



</details>

<a id="0x7_single_order_book_client_order_id_exists"></a>

## Function `client_order_id_exists`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="single_order_book.md#0x7_single_order_book_client_order_id_exists">client_order_id_exists</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="single_order_book.md#0x7_single_order_book_RetailOrderBook">single_order_book::RetailOrderBook</a>&lt;M&gt;, order_creator: <b>address</b>, client_order_id: u64): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="single_order_book.md#0x7_single_order_book_client_order_id_exists">client_order_id_exists</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="single_order_book.md#0x7_single_order_book_RetailOrderBook">RetailOrderBook</a>&lt;M&gt;, order_creator: <b>address</b>, client_order_id: u64
): bool {
    <b>let</b> account_client_order_id =
        new_account_client_order_id(order_creator, client_order_id);
    self.client_order_ids.contains(&account_client_order_id)
}
</code></pre>



</details>

<a id="0x7_single_order_book_place_maker_order"></a>

## Function `place_maker_order`

Places a maker order to the order book. If the order is a pending order, it is added to the pending order book
else it is added to the active order book. The API aborts if its not a maker order or if the order already exists


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="single_order_book.md#0x7_single_order_book_place_maker_order">place_maker_order</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="single_order_book.md#0x7_single_order_book_RetailOrderBook">single_order_book::RetailOrderBook</a>&lt;M&gt;, price_time_idx: &<b>mut</b> <a href="price_time_index.md#0x7_price_time_index_PriceTimeIndex">price_time_index::PriceTimeIndex</a>, ascending_id_generator: &<b>mut</b> <a href="order_book_types.md#0x7_order_book_types_AscendingIdGenerator">order_book_types::AscendingIdGenerator</a>, order_req: <a href="single_order_book.md#0x7_single_order_book_OrderRequest">single_order_book::OrderRequest</a>&lt;M&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="single_order_book.md#0x7_single_order_book_place_maker_order">place_maker_order</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="single_order_book.md#0x7_single_order_book_RetailOrderBook">RetailOrderBook</a>&lt;M&gt;, price_time_idx: &<b>mut</b> PriceTimeIndex, ascending_id_generator: &<b>mut</b> AscendingIdGenerator, order_req: <a href="single_order_book.md#0x7_single_order_book_OrderRequest">OrderRequest</a>&lt;M&gt;
) {
    <b>if</b> (order_req.trigger_condition.is_some()) {
        <b>return</b> self.<a href="single_order_book.md#0x7_single_order_book_place_pending_maker_order">place_pending_maker_order</a>(ascending_id_generator, order_req);
    };

    <b>let</b> ascending_idx =
        new_unique_idx_type(ascending_id_generator.next_ascending_id());

    <b>assert</b>!(
        !self.orders.contains(&order_req.order_id),
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="single_order_book.md#0x7_single_order_book_EORDER_ALREADY_EXISTS">EORDER_ALREADY_EXISTS</a>)
    );

    <b>let</b> order =
        new_order(
            order_req.order_id,
            order_req.<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>,
            ascending_idx,
            order_req.client_order_id,
            order_req.price,
            order_req.orig_size,
            order_req.remaining_size,
            order_req.is_bid,
            order_req.trigger_condition,
            order_req.time_in_force,
            order_req.metadata
        );
    self.orders.add(order_req.order_id, new_order_with_state(order, <b>true</b>));
    <b>if</b> (order_req.client_order_id.is_some()) {
        self.client_order_ids.add(
            new_account_client_order_id(
                order_req.<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>, order_req.client_order_id.destroy_some()
            ),
            order_req.order_id
        );
    };
    price_time_idx.<a href="single_order_book.md#0x7_single_order_book_place_maker_order">place_maker_order</a>(
        order_req.order_id,
        order_req.price,
        ascending_idx,
        order_req.remaining_size,
        order_req.is_bid
    );
}
</code></pre>



</details>

<a id="0x7_single_order_book_reinsert_maker_order"></a>

## Function `reinsert_maker_order`

Reinserts a maker order to the order book. This is used when the order is removed from the order book
but the clearinghouse fails to settle all or part of the order. If the order doesn't exist in the order book,
it is added to the order book, if it exists, it's size is updated.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="single_order_book.md#0x7_single_order_book_reinsert_maker_order">reinsert_maker_order</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="single_order_book.md#0x7_single_order_book_RetailOrderBook">single_order_book::RetailOrderBook</a>&lt;M&gt;, price_time_idx: &<b>mut</b> <a href="price_time_index.md#0x7_price_time_index_PriceTimeIndex">price_time_index::PriceTimeIndex</a>, ascending_id_generator: &<b>mut</b> <a href="order_book_types.md#0x7_order_book_types_AscendingIdGenerator">order_book_types::AscendingIdGenerator</a>, order_req: <a href="single_order_book.md#0x7_single_order_book_OrderRequest">single_order_book::OrderRequest</a>&lt;M&gt;, original_order: <a href="single_order_types.md#0x7_single_order_types_Order">single_order_types::Order</a>&lt;M&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="single_order_book.md#0x7_single_order_book_reinsert_maker_order">reinsert_maker_order</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="single_order_book.md#0x7_single_order_book_RetailOrderBook">RetailOrderBook</a>&lt;M&gt;, price_time_idx: &<b>mut</b> PriceTimeIndex, ascending_id_generator: &<b>mut</b> AscendingIdGenerator, order_req: <a href="single_order_book.md#0x7_single_order_book_OrderRequest">OrderRequest</a>&lt;M&gt;, original_order: Order&lt;M&gt;
) {
    <b>assert</b>!(
        &original_order.get_order_id() == &order_req.order_id,
        <a href="single_order_book.md#0x7_single_order_book_E_REINSERT_ORDER_MISMATCH">E_REINSERT_ORDER_MISMATCH</a>
    );
    <b>assert</b>!(
        &original_order.get_account() == &order_req.<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>,
        <a href="single_order_book.md#0x7_single_order_book_E_REINSERT_ORDER_MISMATCH">E_REINSERT_ORDER_MISMATCH</a>
    );
    <b>assert</b>!(
        original_order.get_orig_size() == order_req.orig_size,
        <a href="single_order_book.md#0x7_single_order_book_E_REINSERT_ORDER_MISMATCH">E_REINSERT_ORDER_MISMATCH</a>
    );
    <b>assert</b>!(original_order.get_client_order_id() == order_req.client_order_id,
        <a href="single_order_book.md#0x7_single_order_book_E_REINSERT_ORDER_MISMATCH">E_REINSERT_ORDER_MISMATCH</a>);
    // TODO check what should the rule be for remaining_size. check test_maker_order_reinsert_not_exists unit test.
    // <b>assert</b>!(
    //     original_order.<a href="single_order_book.md#0x7_single_order_book_get_remaining_size">get_remaining_size</a>() &gt;= order_req.remaining_size,
    //     <a href="single_order_book.md#0x7_single_order_book_E_REINSERT_ORDER_MISMATCH">E_REINSERT_ORDER_MISMATCH</a>
    // );
    <b>assert</b>!(original_order.get_price() == order_req.price, <a href="single_order_book.md#0x7_single_order_book_E_REINSERT_ORDER_MISMATCH">E_REINSERT_ORDER_MISMATCH</a>);
    <b>assert</b>!(original_order.is_bid() == order_req.is_bid, <a href="single_order_book.md#0x7_single_order_book_E_REINSERT_ORDER_MISMATCH">E_REINSERT_ORDER_MISMATCH</a>);

    <b>assert</b>!(order_req.trigger_condition.is_none(), <a href="single_order_book.md#0x7_single_order_book_E_NOT_ACTIVE_ORDER">E_NOT_ACTIVE_ORDER</a>);
    <b>if</b> (!self.orders.contains(&order_req.order_id)) {
        <b>return</b> self.<a href="single_order_book.md#0x7_single_order_book_place_maker_order">place_maker_order</a>(price_time_idx, ascending_id_generator, order_req);
    };

    <a href="single_order_book.md#0x7_single_order_book_modify_order">modify_order</a>(&<b>mut</b> self.orders, &order_req.order_id, |order_with_state| {
        order_with_state.increase_remaining_size(order_req.remaining_size);
    });
    price_time_idx.increase_order_size(
        order_req.price,
        original_order.get_unique_priority_idx(),
        order_req.remaining_size,
        order_req.is_bid
    );
}
</code></pre>



</details>

<a id="0x7_single_order_book_modify_order"></a>

## Function `modify_order`



<pre><code><b>fun</b> <a href="single_order_book.md#0x7_single_order_book_modify_order">modify_order</a>&lt;M: <b>copy</b>, drop, store&gt;(orders: &<b>mut</b> <a href="../../aptos-framework/doc/big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">big_ordered_map::BigOrderedMap</a>&lt;<a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>, <a href="single_order_types.md#0x7_single_order_types_OrderWithState">single_order_types::OrderWithState</a>&lt;M&gt;&gt;, order_id: &<a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>, modify_fn: |&<b>mut</b> <a href="single_order_types.md#0x7_single_order_types_OrderWithState">single_order_types::OrderWithState</a>&lt;M&gt;|)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="single_order_book.md#0x7_single_order_book_modify_order">modify_order</a>&lt;M: store + <b>copy</b> + drop&gt;(
    orders: &<b>mut</b> BigOrderedMap&lt;OrderIdType, OrderWithState&lt;M&gt;&gt;, order_id: &OrderIdType, modify_fn: |&<b>mut</b>  OrderWithState&lt;M&gt;|
) {
    <b>let</b> order = *orders.borrow(order_id);
    modify_fn(&<b>mut</b> order);
    orders.upsert(*order_id, order);
}
</code></pre>



</details>

<a id="0x7_single_order_book_modify_and_copy_order"></a>

## Function `modify_and_copy_order`



<pre><code><b>fun</b> <a href="single_order_book.md#0x7_single_order_book_modify_and_copy_order">modify_and_copy_order</a>&lt;M: <b>copy</b>, drop, store&gt;(orders: &<b>mut</b> <a href="../../aptos-framework/doc/big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">big_ordered_map::BigOrderedMap</a>&lt;<a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>, <a href="single_order_types.md#0x7_single_order_types_OrderWithState">single_order_types::OrderWithState</a>&lt;M&gt;&gt;, order_id: &<a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>, modify_fn: |&<b>mut</b> <a href="single_order_types.md#0x7_single_order_types_OrderWithState">single_order_types::OrderWithState</a>&lt;M&gt;|): <a href="single_order_types.md#0x7_single_order_types_OrderWithState">single_order_types::OrderWithState</a>&lt;M&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="single_order_book.md#0x7_single_order_book_modify_and_copy_order">modify_and_copy_order</a>&lt;M: store + <b>copy</b> + drop&gt;(
    orders: &<b>mut</b> BigOrderedMap&lt;OrderIdType, OrderWithState&lt;M&gt;&gt;, order_id: &OrderIdType, modify_fn: |&<b>mut</b>  OrderWithState&lt;M&gt;|
): OrderWithState&lt;M&gt; {
    <b>let</b> order = *orders.borrow(order_id);
    modify_fn(&<b>mut</b> order);
    orders.upsert(*order_id, order);
    order
}
</code></pre>



</details>

<a id="0x7_single_order_book_modify_or_remove_order"></a>

## Function `modify_or_remove_order`



<pre><code><b>fun</b> <a href="single_order_book.md#0x7_single_order_book_modify_or_remove_order">modify_or_remove_order</a>&lt;M: <b>copy</b>, drop, store&gt;(orders: &<b>mut</b> <a href="../../aptos-framework/doc/big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">big_ordered_map::BigOrderedMap</a>&lt;<a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>, <a href="single_order_types.md#0x7_single_order_types_OrderWithState">single_order_types::OrderWithState</a>&lt;M&gt;&gt;, order_id: &<a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>, modify_fn: |&<b>mut</b> <a href="single_order_types.md#0x7_single_order_types_OrderWithState">single_order_types::OrderWithState</a>&lt;M&gt;|bool): <a href="single_order_types.md#0x7_single_order_types_OrderWithState">single_order_types::OrderWithState</a>&lt;M&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="single_order_book.md#0x7_single_order_book_modify_or_remove_order">modify_or_remove_order</a>&lt;M: store + <b>copy</b> + drop&gt;(
    orders: &<b>mut</b> BigOrderedMap&lt;OrderIdType, OrderWithState&lt;M&gt;&gt;, order_id: &OrderIdType, modify_fn: |&<b>mut</b>  OrderWithState&lt;M&gt;| bool
): OrderWithState&lt;M&gt; {
    <b>let</b> order = orders.remove(order_id);
    <b>let</b> keep = modify_fn(&<b>mut</b> order);
    <b>if</b> (keep) {
        orders.add(*order_id, order);
    };
    order
}
</code></pre>



</details>

<a id="0x7_single_order_book_place_pending_maker_order"></a>

## Function `place_pending_maker_order`



<pre><code><b>fun</b> <a href="single_order_book.md#0x7_single_order_book_place_pending_maker_order">place_pending_maker_order</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="single_order_book.md#0x7_single_order_book_RetailOrderBook">single_order_book::RetailOrderBook</a>&lt;M&gt;, ascending_id_generator: &<b>mut</b> <a href="order_book_types.md#0x7_order_book_types_AscendingIdGenerator">order_book_types::AscendingIdGenerator</a>, order_req: <a href="single_order_book.md#0x7_single_order_book_OrderRequest">single_order_book::OrderRequest</a>&lt;M&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="single_order_book.md#0x7_single_order_book_place_pending_maker_order">place_pending_maker_order</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="single_order_book.md#0x7_single_order_book_RetailOrderBook">RetailOrderBook</a>&lt;M&gt;, ascending_id_generator: &<b>mut</b> AscendingIdGenerator, order_req: <a href="single_order_book.md#0x7_single_order_book_OrderRequest">OrderRequest</a>&lt;M&gt;
) {
    <b>let</b> order_id = order_req.order_id;
    <b>let</b> ascending_idx =
        new_unique_idx_type(ascending_id_generator.next_ascending_id());
    <b>let</b> order =
        new_order(
            order_id,
            order_req.<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>,
            ascending_idx,
            order_req.client_order_id,
            order_req.price,
            order_req.orig_size,
            order_req.remaining_size,
            order_req.is_bid,
            order_req.trigger_condition,
            order_req.time_in_force,
            order_req.metadata
        );

    self.orders.add(order_id, new_order_with_state(order, <b>false</b>));

    self.pending_orders.<a href="single_order_book.md#0x7_single_order_book_place_pending_maker_order">place_pending_maker_order</a>(
        order_id,
        order_req.trigger_condition.destroy_some(),
        ascending_idx,
    );
}
</code></pre>



</details>

<a id="0x7_single_order_book_get_single_match_for_taker"></a>

## Function `get_single_match_for_taker`

Returns a single match for a taker order. It is responsibility of the caller to first call the <code>is_taker_order</code>
API to ensure that the order is a taker order before calling this API, otherwise it will abort.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="single_order_book.md#0x7_single_order_book_get_single_match_for_taker">get_single_match_for_taker</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="single_order_book.md#0x7_single_order_book_RetailOrderBook">single_order_book::RetailOrderBook</a>&lt;M&gt;, price_time_idx: &<b>mut</b> <a href="price_time_index.md#0x7_price_time_index_PriceTimeIndex">price_time_index::PriceTimeIndex</a>, price: u64, size: u64, is_bid: bool): <a href="single_order_types.md#0x7_single_order_types_SingleOrderMatch">single_order_types::SingleOrderMatch</a>&lt;M&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="single_order_book.md#0x7_single_order_book_get_single_match_for_taker">get_single_match_for_taker</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="single_order_book.md#0x7_single_order_book_RetailOrderBook">RetailOrderBook</a>&lt;M&gt;,
    price_time_idx: &<b>mut</b> PriceTimeIndex,
    price: u64,
    size: u64,
    is_bid: bool
): SingleOrderMatch&lt;M&gt; {
    <b>let</b> result = price_time_idx.get_single_match_result(price, size, is_bid);
    <b>let</b> (order_id, matched_size, remaining_size) =
        result.destroy_active_matched_order();

    <b>let</b> order_with_state = <a href="single_order_book.md#0x7_single_order_book_modify_or_remove_order">modify_or_remove_order</a>(&<b>mut</b> self.orders, &order_id, |order_with_state| {
        order_with_state.set_remaining_size(remaining_size);
        remaining_size &gt; 0
    });

    <b>let</b> (order, is_active) = order_with_state.destroy_order_from_state();
    <b>if</b> (remaining_size == 0 && order.get_client_order_id().is_some()) {
        self.client_order_ids.remove(
            &new_account_client_order_id(
                order.get_account(), order.get_client_order_id().destroy_some()
            )
        );
    };
    <b>assert</b>!(is_active, <a href="single_order_book.md#0x7_single_order_book_EINVALID_INACTIVE_ORDER_STATE">EINVALID_INACTIVE_ORDER_STATE</a>);
    new_single_order_match(order, matched_size)
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


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="single_order_book.md#0x7_single_order_book_decrease_order_size">decrease_order_size</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="single_order_book.md#0x7_single_order_book_RetailOrderBook">single_order_book::RetailOrderBook</a>&lt;M&gt;, price_time_idx: &<b>mut</b> <a href="price_time_index.md#0x7_price_time_index_PriceTimeIndex">price_time_index::PriceTimeIndex</a>, order_creator: <b>address</b>, order_id: <a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>, size_delta: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="single_order_book.md#0x7_single_order_book_decrease_order_size">decrease_order_size</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="single_order_book.md#0x7_single_order_book_RetailOrderBook">RetailOrderBook</a>&lt;M&gt;,
    price_time_idx: &<b>mut</b> PriceTimeIndex,
    order_creator: <b>address</b>,
    order_id: OrderIdType,
    size_delta: u64
) {
    <b>assert</b>!(self.orders.contains(&order_id), <a href="single_order_book.md#0x7_single_order_book_EORDER_NOT_FOUND">EORDER_NOT_FOUND</a>);

    <b>let</b> order_with_state = <a href="single_order_book.md#0x7_single_order_book_modify_and_copy_order">modify_and_copy_order</a>(&<b>mut</b> self.orders, &order_id, |order_with_state| {
        <b>assert</b>!(
            order_creator == order_with_state.get_order_from_state().get_account(),
            <a href="single_order_book.md#0x7_single_order_book_EORDER_CREATOR_MISMATCH">EORDER_CREATOR_MISMATCH</a>
        );
        order_with_state.decrease_remaining_size(size_delta);

        // TODO should we be asserting that remaining size is greater than 0?
    });

    <b>if</b> (order_with_state.<a href="single_order_book.md#0x7_single_order_book_is_active_order">is_active_order</a>()) {
        <b>let</b> order = order_with_state.get_order_from_state();
        price_time_idx
            .<a href="single_order_book.md#0x7_single_order_book_decrease_order_size">decrease_order_size</a>(
            order.get_price(),
            order_with_state.get_unique_priority_idx_from_state(),
            size_delta,
            order.is_bid()
        );
    };
}
</code></pre>



</details>

<a id="0x7_single_order_book_get_order_id_by_client_id"></a>

## Function `get_order_id_by_client_id`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="single_order_book.md#0x7_single_order_book_get_order_id_by_client_id">get_order_id_by_client_id</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="single_order_book.md#0x7_single_order_book_RetailOrderBook">single_order_book::RetailOrderBook</a>&lt;M&gt;, order_creator: <b>address</b>, client_order_id: u64): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="single_order_book.md#0x7_single_order_book_get_order_id_by_client_id">get_order_id_by_client_id</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="single_order_book.md#0x7_single_order_book_RetailOrderBook">RetailOrderBook</a>&lt;M&gt;, order_creator: <b>address</b>, client_order_id: u64
): Option&lt;OrderIdType&gt; {
    <b>let</b> account_client_order_id =
        new_account_client_order_id(order_creator, client_order_id);
    <b>if</b> (!self.client_order_ids.contains(&account_client_order_id)) {
        <b>return</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>();
    };
    <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(*self.client_order_ids.borrow(&account_client_order_id))
}
</code></pre>



</details>

<a id="0x7_single_order_book_get_order_metadata"></a>

## Function `get_order_metadata`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="single_order_book.md#0x7_single_order_book_get_order_metadata">get_order_metadata</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="single_order_book.md#0x7_single_order_book_RetailOrderBook">single_order_book::RetailOrderBook</a>&lt;M&gt;, order_id: <a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;M&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="single_order_book.md#0x7_single_order_book_get_order_metadata">get_order_metadata</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="single_order_book.md#0x7_single_order_book_RetailOrderBook">RetailOrderBook</a>&lt;M&gt;, order_id: OrderIdType
): Option&lt;M&gt; {
    <b>if</b> (!self.orders.contains(&order_id)) {
        <b>return</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>();
    };
    <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(self.orders.borrow(&order_id).get_metadata_from_state())
}
</code></pre>



</details>

<a id="0x7_single_order_book_set_order_metadata"></a>

## Function `set_order_metadata`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="single_order_book.md#0x7_single_order_book_set_order_metadata">set_order_metadata</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="single_order_book.md#0x7_single_order_book_RetailOrderBook">single_order_book::RetailOrderBook</a>&lt;M&gt;, order_id: <a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>, metadata: M)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="single_order_book.md#0x7_single_order_book_set_order_metadata">set_order_metadata</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="single_order_book.md#0x7_single_order_book_RetailOrderBook">RetailOrderBook</a>&lt;M&gt;, order_id: OrderIdType, metadata: M
) {
    <b>assert</b>!(self.orders.contains(&order_id), <a href="single_order_book.md#0x7_single_order_book_EORDER_NOT_FOUND">EORDER_NOT_FOUND</a>);

    <a href="single_order_book.md#0x7_single_order_book_modify_order">modify_order</a>(&<b>mut</b> self.orders, &order_id, |order_with_state| {
        order_with_state.set_metadata_in_state(metadata);
    });
}
</code></pre>



</details>

<a id="0x7_single_order_book_is_active_order"></a>

## Function `is_active_order`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="single_order_book.md#0x7_single_order_book_is_active_order">is_active_order</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="single_order_book.md#0x7_single_order_book_RetailOrderBook">single_order_book::RetailOrderBook</a>&lt;M&gt;, order_id: <a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="single_order_book.md#0x7_single_order_book_is_active_order">is_active_order</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="single_order_book.md#0x7_single_order_book_RetailOrderBook">RetailOrderBook</a>&lt;M&gt;, order_id: OrderIdType
): bool {
    <b>if</b> (!self.orders.contains(&order_id)) {
        <b>return</b> <b>false</b>;
    };
    self.orders.borrow(&order_id).<a href="single_order_book.md#0x7_single_order_book_is_active_order">is_active_order</a>()
}
</code></pre>



</details>

<a id="0x7_single_order_book_get_order"></a>

## Function `get_order`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="single_order_book.md#0x7_single_order_book_get_order">get_order</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="single_order_book.md#0x7_single_order_book_RetailOrderBook">single_order_book::RetailOrderBook</a>&lt;M&gt;, order_id: <a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="single_order_types.md#0x7_single_order_types_OrderWithState">single_order_types::OrderWithState</a>&lt;M&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="single_order_book.md#0x7_single_order_book_get_order">get_order</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="single_order_book.md#0x7_single_order_book_RetailOrderBook">RetailOrderBook</a>&lt;M&gt;, order_id: OrderIdType
): Option&lt;OrderWithState&lt;M&gt;&gt; {
    <b>if</b> (!self.orders.contains(&order_id)) {
        <b>return</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>();
    };
    <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(*self.orders.borrow(&order_id))
}
</code></pre>



</details>

<a id="0x7_single_order_book_get_remaining_size"></a>

## Function `get_remaining_size`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="single_order_book.md#0x7_single_order_book_get_remaining_size">get_remaining_size</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="single_order_book.md#0x7_single_order_book_RetailOrderBook">single_order_book::RetailOrderBook</a>&lt;M&gt;, order_id: <a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="single_order_book.md#0x7_single_order_book_get_remaining_size">get_remaining_size</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="single_order_book.md#0x7_single_order_book_RetailOrderBook">RetailOrderBook</a>&lt;M&gt;, order_id: OrderIdType
): u64 {
    <b>if</b> (!self.orders.contains(&order_id)) {
        <b>return</b> 0;
    };
    self.orders.borrow(&order_id).get_remaining_size_from_state()
}
</code></pre>



</details>

<a id="0x7_single_order_book_take_ready_price_based_orders"></a>

## Function `take_ready_price_based_orders`

Removes and returns the orders that are ready to be executed based on the current price.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="single_order_book.md#0x7_single_order_book_take_ready_price_based_orders">take_ready_price_based_orders</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="single_order_book.md#0x7_single_order_book_RetailOrderBook">single_order_book::RetailOrderBook</a>&lt;M&gt;, current_price: u64, order_limit: u64): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="single_order_types.md#0x7_single_order_types_Order">single_order_types::Order</a>&lt;M&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="single_order_book.md#0x7_single_order_book_take_ready_price_based_orders">take_ready_price_based_orders</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="single_order_book.md#0x7_single_order_book_RetailOrderBook">RetailOrderBook</a>&lt;M&gt;, current_price: u64, order_limit: u64
): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Order&lt;M&gt;&gt; {
    <b>let</b> self_orders = &<b>mut</b> self.orders;
    <b>let</b> order_ids = self.pending_orders.<a href="single_order_book.md#0x7_single_order_book_take_ready_price_based_orders">take_ready_price_based_orders</a>(current_price, order_limit);
    <b>let</b> orders = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>();

    order_ids.for_each(|order_id| {
        <b>let</b> order_with_state = self_orders.remove(&order_id);
        <b>let</b> (order, _) = order_with_state.destroy_order_from_state();
        orders.push_back(order);
    });
    orders
}
</code></pre>



</details>

<a id="0x7_single_order_book_take_ready_time_based_orders"></a>

## Function `take_ready_time_based_orders`

Removes and returns the orders that are ready to be executed based on the time condition.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="single_order_book.md#0x7_single_order_book_take_ready_time_based_orders">take_ready_time_based_orders</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="single_order_book.md#0x7_single_order_book_RetailOrderBook">single_order_book::RetailOrderBook</a>&lt;M&gt;, order_limit: u64): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="single_order_types.md#0x7_single_order_types_Order">single_order_types::Order</a>&lt;M&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="single_order_book.md#0x7_single_order_book_take_ready_time_based_orders">take_ready_time_based_orders</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="single_order_book.md#0x7_single_order_book_RetailOrderBook">RetailOrderBook</a>&lt;M&gt;,  order_limit: u64
): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Order&lt;M&gt;&gt; {
    <b>let</b> self_orders = &<b>mut</b> self.orders;
    <b>let</b> order_ids = self.pending_orders.take_time_time_based_orders(order_limit);
    <b>let</b> orders = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>();

    order_ids.for_each(|order_id| {
        <b>let</b> order_with_state = self_orders.remove(&order_id);
        <b>let</b> (order, _) = order_with_state.destroy_order_from_state();
        orders.push_back(order);
    });
    orders
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
