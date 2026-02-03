
<a id="0x7_bulk_order_book"></a>

# Module `0x7::bulk_order_book`


<a id="@Bulk_Order_Book_Module_0"></a>

## Bulk Order Book Module


This module implements a bulk order book system that allows traders to place orders with multiple
price levels simultaneously. The bulk order book supports both maker and taker orders, with
sophisticated order matching, cancellation, and reinsertion capabilities.


<a id="@Key_Features:_1"></a>

### Key Features:



<a id="@1._Multi-Level_Orders_2"></a>

#### 1. Multi-Level Orders

- Traders can place orders with multiple price levels in a single transaction
- Bid orders: Prices must be in descending order (best price first)
- Ask orders: Prices must be in ascending order (best price first)
- Each price level has an associated size


<a id="@2._Order_Matching_3"></a>

#### 2. Order Matching

- Price-time priority: Orders are matched based on price first, then time
- Partial fills: Orders can be partially filled across multiple levels
- Automatic level progression: When a price level is fully consumed, the next level becomes active


<a id="@3._Order_Management_4"></a>

#### 3. Order Management

- **Cancellation**: Orders can be cancelled, clearing all active levels
- **Reinsertion**: Matched portions of orders can be reinserted back into the order book
- **Order ID Reuse**: Cancelled orders allow the same account to place new orders with the same ID


<a id="@Data_Structures:_5"></a>

### Data Structures:


- <code><a href="bulk_order_book.md#0x7_bulk_order_book_BulkOrderBook">BulkOrderBook</a></code>: Main order book container
- <code>BulkOrderRequest</code>: Request structure for placing new orders
- <code>BulkOrder</code>: Internal representation of a multi-level order
- <code>BulkOrderResult</code>: Result of order matching operations
- <code>SingleBulkOrderMatch</code>: Single match result between orders


<a id="@Error_Codes:_6"></a>

### Error Codes:

- <code><a href="bulk_order_book.md#0x7_bulk_order_book_EORDER_ALREADY_EXISTS">EORDER_ALREADY_EXISTS</a></code>: Order already exists for the account
- <code><a href="bulk_order_book.md#0x7_bulk_order_book_EPOST_ONLY_FILLED">EPOST_ONLY_FILLED</a></code>: Post-only order was filled (crossed the spread)
- <code><a href="bulk_order_book.md#0x7_bulk_order_book_EORDER_NOT_FOUND">EORDER_NOT_FOUND</a></code>: Order not found for cancellation or reinsertion
- <code><a href="bulk_order_book.md#0x7_bulk_order_book_EINVALID_INACTIVE_ORDER_STATE">EINVALID_INACTIVE_ORDER_STATE</a></code>: Order is in an invalid inactive state
- <code><a href="bulk_order_book.md#0x7_bulk_order_book_EINVALID_ADD_SIZE_TO_ORDER">EINVALID_ADD_SIZE_TO_ORDER</a></code>: Invalid size addition to order
- <code><a href="bulk_order_book.md#0x7_bulk_order_book_E_NOT_ACTIVE_ORDER">E_NOT_ACTIVE_ORDER</a></code>: Order is not active
- <code><a href="bulk_order_book.md#0x7_bulk_order_book_E_REINSERT_ORDER_MISMATCH">E_REINSERT_ORDER_MISMATCH</a></code>: Reinsertion order validation failed
- <code><a href="bulk_order_book.md#0x7_bulk_order_book_EORDER_CREATOR_MISMATCH">EORDER_CREATOR_MISMATCH</a></code>: Order creator mismatch
- <code><a href="bulk_order_book.md#0x7_bulk_order_book_EINVLID_MM_ORDER_REQUEST">EINVLID_MM_ORDER_REQUEST</a></code>: Invalid bulk order request (price ordering, sizes, etc.)
- <code><a href="bulk_order_book.md#0x7_bulk_order_book_EPRICE_CROSSING">EPRICE_CROSSING</a></code>: Price crossing is not allowed in bulk orders


-  [Bulk Order Book Module](#@Bulk_Order_Book_Module_0)
    -  [Key Features:](#@Key_Features:_1)
        -  [1. Multi-Level Orders](#@1._Multi-Level_Orders_2)
        -  [2. Order Matching](#@2._Order_Matching_3)
        -  [3. Order Management](#@3._Order_Management_4)
    -  [Data Structures:](#@Data_Structures:_5)
    -  [Error Codes:](#@Error_Codes:_6)
-  [Enum `BulkOrderBook`](#0x7_bulk_order_book_BulkOrderBook)
    -  [Fields:](#@Fields:_7)
-  [Constants](#@Constants_8)
-  [Function `new_bulk_order_book`](#0x7_bulk_order_book_new_bulk_order_book)
    -  [Returns:](#@Returns:_9)
-  [Function `get_single_match_for_taker`](#0x7_bulk_order_book_get_single_match_for_taker)
    -  [Arguments:](#@Arguments:_10)
    -  [Returns:](#@Returns:_11)
    -  [Side Effects:](#@Side_Effects:_12)
-  [Function `cancel_active_order_for_side`](#0x7_bulk_order_book_cancel_active_order_for_side)
    -  [Arguments:](#@Arguments:_13)
-  [Function `cancel_active_orders`](#0x7_bulk_order_book_cancel_active_orders)
    -  [Arguments:](#@Arguments:_14)
-  [Function `activate_first_price_level_for_side`](#0x7_bulk_order_book_activate_first_price_level_for_side)
    -  [Arguments:](#@Arguments:_15)
-  [Function `activate_first_price_levels`](#0x7_bulk_order_book_activate_first_price_levels)
    -  [Arguments:](#@Arguments:_16)
-  [Function `reinsert_order`](#0x7_bulk_order_book_reinsert_order)
    -  [Arguments:](#@Arguments:_17)
    -  [Aborts:](#@Aborts:_18)
-  [Function `cancel_bulk_order`](#0x7_bulk_order_book_cancel_bulk_order)
    -  [Arguments:](#@Arguments:_19)
    -  [Aborts:](#@Aborts:_20)
-  [Function `cancel_bulk_order_at_price`](#0x7_bulk_order_book_cancel_bulk_order_at_price)
    -  [Arguments:](#@Arguments:_21)
    -  [Returns:](#@Returns:_22)
    -  [Aborts:](#@Aborts:_23)
-  [Function `get_bulk_order`](#0x7_bulk_order_book_get_bulk_order)
-  [Function `get_remaining_size`](#0x7_bulk_order_book_get_remaining_size)
-  [Function `get_prices`](#0x7_bulk_order_book_get_prices)
-  [Function `get_sizes`](#0x7_bulk_order_book_get_sizes)
-  [Function `place_bulk_order`](#0x7_bulk_order_book_place_bulk_order)
    -  [Arguments:](#@Arguments:_24)
    -  [Aborts:](#@Aborts:_25)


<pre><code><b>use</b> <a href="../../aptos-framework/doc/big_ordered_map.md#0x1_big_ordered_map">0x1::big_ordered_map</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
<b>use</b> <a href="">0x5::bulk_order_types</a>;
<b>use</b> <a href="">0x5::order_book_types</a>;
<b>use</b> <a href="">0x5::order_match_types</a>;
<b>use</b> <a href="bulk_order_utils.md#0x7_bulk_order_utils">0x7::bulk_order_utils</a>;
<b>use</b> <a href="price_time_index.md#0x7_price_time_index">0x7::price_time_index</a>;
</code></pre>



<a id="0x7_bulk_order_book_BulkOrderBook"></a>

## Enum `BulkOrderBook`

Main bulk order book container that manages all orders and their matching.


<a id="@Fields:_7"></a>

### Fields:

- <code>orders</code>: Map of account addresses to their bulk orders
- <code>order_id_to_address</code>: Map of order IDs to account addresses for lookup


<pre><code>enum <a href="bulk_order_book.md#0x7_bulk_order_book_BulkOrderBook">BulkOrderBook</a>&lt;M: <b>copy</b>, drop, store&gt; <b>has</b> store
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>V1</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>orders: <a href="../../aptos-framework/doc/big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">big_ordered_map::BigOrderedMap</a>&lt;<b>address</b>, <a href="_BulkOrder">bulk_order_types::BulkOrder</a>&lt;M&gt;&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>order_id_to_address: <a href="../../aptos-framework/doc/big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">big_ordered_map::BigOrderedMap</a>&lt;<a href="_OrderId">order_book_types::OrderId</a>, <b>address</b>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

</details>

<a id="@Constants_8"></a>

## Constants


<a id="0x7_bulk_order_book_E_REINSERT_ORDER_MISMATCH"></a>



<pre><code><b>const</b> <a href="bulk_order_book.md#0x7_bulk_order_book_E_REINSERT_ORDER_MISMATCH">E_REINSERT_ORDER_MISMATCH</a>: u64 = 8;
</code></pre>



<a id="0x7_bulk_order_book_EINVLID_MM_ORDER_REQUEST"></a>



<pre><code><b>const</b> <a href="bulk_order_book.md#0x7_bulk_order_book_EINVLID_MM_ORDER_REQUEST">EINVLID_MM_ORDER_REQUEST</a>: u64 = 10;
</code></pre>



<a id="0x7_bulk_order_book_EPRICE_CROSSING"></a>



<pre><code><b>const</b> <a href="bulk_order_book.md#0x7_bulk_order_book_EPRICE_CROSSING">EPRICE_CROSSING</a>: u64 = 11;
</code></pre>



<a id="0x7_bulk_order_book_E_INVALID_SEQUENCE_NUMBER"></a>



<pre><code><b>const</b> <a href="bulk_order_book.md#0x7_bulk_order_book_E_INVALID_SEQUENCE_NUMBER">E_INVALID_SEQUENCE_NUMBER</a>: u64 = 13;
</code></pre>



<a id="0x7_bulk_order_book_EORDER_ALREADY_EXISTS"></a>



<pre><code><b>const</b> <a href="bulk_order_book.md#0x7_bulk_order_book_EORDER_ALREADY_EXISTS">EORDER_ALREADY_EXISTS</a>: u64 = 1;
</code></pre>



<a id="0x7_bulk_order_book_EINVALID_ADD_SIZE_TO_ORDER"></a>



<pre><code><b>const</b> <a href="bulk_order_book.md#0x7_bulk_order_book_EINVALID_ADD_SIZE_TO_ORDER">EINVALID_ADD_SIZE_TO_ORDER</a>: u64 = 6;
</code></pre>



<a id="0x7_bulk_order_book_EINVALID_INACTIVE_ORDER_STATE"></a>



<pre><code><b>const</b> <a href="bulk_order_book.md#0x7_bulk_order_book_EINVALID_INACTIVE_ORDER_STATE">EINVALID_INACTIVE_ORDER_STATE</a>: u64 = 5;
</code></pre>



<a id="0x7_bulk_order_book_ENOT_BULK_ORDER"></a>



<pre><code><b>const</b> <a href="bulk_order_book.md#0x7_bulk_order_book_ENOT_BULK_ORDER">ENOT_BULK_ORDER</a>: u64 = 12;
</code></pre>



<a id="0x7_bulk_order_book_EORDER_CREATOR_MISMATCH"></a>



<pre><code><b>const</b> <a href="bulk_order_book.md#0x7_bulk_order_book_EORDER_CREATOR_MISMATCH">EORDER_CREATOR_MISMATCH</a>: u64 = 9;
</code></pre>



<a id="0x7_bulk_order_book_EORDER_NOT_FOUND"></a>



<pre><code><b>const</b> <a href="bulk_order_book.md#0x7_bulk_order_book_EORDER_NOT_FOUND">EORDER_NOT_FOUND</a>: u64 = 4;
</code></pre>



<a id="0x7_bulk_order_book_EPOST_ONLY_FILLED"></a>



<pre><code><b>const</b> <a href="bulk_order_book.md#0x7_bulk_order_book_EPOST_ONLY_FILLED">EPOST_ONLY_FILLED</a>: u64 = 2;
</code></pre>



<a id="0x7_bulk_order_book_E_NOT_ACTIVE_ORDER"></a>



<pre><code><b>const</b> <a href="bulk_order_book.md#0x7_bulk_order_book_E_NOT_ACTIVE_ORDER">E_NOT_ACTIVE_ORDER</a>: u64 = 7;
</code></pre>



<a id="0x7_bulk_order_book_new_bulk_order_book"></a>

## Function `new_bulk_order_book`

Creates a new empty bulk order book.


<a id="@Returns:_9"></a>

### Returns:

A new <code><a href="bulk_order_book.md#0x7_bulk_order_book_BulkOrderBook">BulkOrderBook</a></code> instance with empty order collections.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="bulk_order_book.md#0x7_bulk_order_book_new_bulk_order_book">new_bulk_order_book</a>&lt;M: <b>copy</b>, drop, store&gt;(): <a href="bulk_order_book.md#0x7_bulk_order_book_BulkOrderBook">bulk_order_book::BulkOrderBook</a>&lt;M&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="bulk_order_book.md#0x7_bulk_order_book_new_bulk_order_book">new_bulk_order_book</a>&lt;M: store + <b>copy</b> + drop&gt;(): <a href="bulk_order_book.md#0x7_bulk_order_book_BulkOrderBook">BulkOrderBook</a>&lt;M&gt; {
    BulkOrderBook::V1 {
        orders: <a href="order_book_utils.md#0x7_order_book_utils_new_default_big_ordered_map">order_book_utils::new_default_big_ordered_map</a>(),
        order_id_to_address: <a href="order_book_utils.md#0x7_order_book_utils_new_default_big_ordered_map">order_book_utils::new_default_big_ordered_map</a>()
    }
}
</code></pre>



</details>

<a id="0x7_bulk_order_book_get_single_match_for_taker"></a>

## Function `get_single_match_for_taker`

Returns a single match for a taker order.

This function should only be called after verifying that the order is a taker order
using <code>is_taker_order()</code>. If called on a non-taker order, it will abort.


<a id="@Arguments:_10"></a>

### Arguments:

- <code>self</code>: Mutable reference to the bulk order book
- <code>price_time_idx</code>: Mutable reference to the price time index
- <code>price</code>: The price of the taker order
- <code>size</code>: The size of the taker order
- <code>is_bid</code>: True if the taker order is a bid, false if ask


<a id="@Returns:_11"></a>

### Returns:

A <code>SingleBulkOrderMatch</code> containing the match details.


<a id="@Side_Effects:_12"></a>

### Side Effects:

- Updates the matched order's remaining sizes
- Activates the next price level if the current level is fully consumed
- Updates the active order book


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="bulk_order_book.md#0x7_bulk_order_book_get_single_match_for_taker">get_single_match_for_taker</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="bulk_order_book.md#0x7_bulk_order_book_BulkOrderBook">bulk_order_book::BulkOrderBook</a>&lt;M&gt;, price_time_idx: &<b>mut</b> <a href="price_time_index.md#0x7_price_time_index_PriceTimeIndex">price_time_index::PriceTimeIndex</a>, active_matched_order: <a href="_ActiveMatchedOrder">order_match_types::ActiveMatchedOrder</a>, is_bid: bool): <a href="_OrderMatch">order_match_types::OrderMatch</a>&lt;M&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="bulk_order_book.md#0x7_bulk_order_book_get_single_match_for_taker">get_single_match_for_taker</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="bulk_order_book.md#0x7_bulk_order_book_BulkOrderBook">BulkOrderBook</a>&lt;M&gt;,
    price_time_idx: &<b>mut</b> aptos_experimental::price_time_index::PriceTimeIndex,
    active_matched_order: ActiveMatchedOrder,
    is_bid: bool
): OrderMatch&lt;M&gt; {
    <b>let</b> (order_id, matched_size, remaining_size, order_book_type) =
        active_matched_order.destroy_active_matched_order();
    <b>assert</b>!(order_book_type == bulk_order_type(), <a href="bulk_order_book.md#0x7_bulk_order_book_ENOT_BULK_ORDER">ENOT_BULK_ORDER</a>);
    <b>let</b> order_address = self.order_id_to_address.get(&order_id).destroy_some();
    <b>let</b> order = self.orders.remove(&order_address);
    <b>let</b> order_match = new_bulk_order_match&lt;M&gt;(&order, !is_bid, matched_size);
    <b>let</b> (next_price, next_size) =
        <a href="bulk_order_utils.md#0x7_bulk_order_utils_match_order_and_get_next_from_bulk_order">bulk_order_utils::match_order_and_get_next_from_bulk_order</a>(
            &<b>mut</b> order, !is_bid, matched_size
        );
    <b>if</b> (remaining_size == 0 && next_price.is_some()) {
        <b>let</b> price = next_price.destroy_some();
        <b>let</b> size = next_size.destroy_some();
        price_time_idx.place_maker_order(
            order_id,
            bulk_order_type(),
            price,
            order.get_unique_priority_idx(),
            size,
            !is_bid
        );
    };
    self.orders.add(order_address, order);
    <b>return</b> order_match
}
</code></pre>



</details>

<a id="0x7_bulk_order_book_cancel_active_order_for_side"></a>

## Function `cancel_active_order_for_side`

Cancels active orders for a specific side (bid or ask) of a bulk order.


<a id="@Arguments:_13"></a>

### Arguments:

- <code>active_orders</code>: Reference to the active order book
- <code>order</code>: The bulk order to cancel active orders for
- <code>is_bid</code>: True to cancel bid orders, false for ask orders


<pre><code><b>fun</b> <a href="bulk_order_book.md#0x7_bulk_order_book_cancel_active_order_for_side">cancel_active_order_for_side</a>&lt;M: <b>copy</b>, drop, store&gt;(price_time_idx: &<b>mut</b> <a href="price_time_index.md#0x7_price_time_index_PriceTimeIndex">price_time_index::PriceTimeIndex</a>, order: &<a href="_BulkOrder">bulk_order_types::BulkOrder</a>&lt;M&gt;, is_bid: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="bulk_order_book.md#0x7_bulk_order_book_cancel_active_order_for_side">cancel_active_order_for_side</a>&lt;M: store + <b>copy</b> + drop&gt;(
    price_time_idx: &<b>mut</b> aptos_experimental::price_time_index::PriceTimeIndex,
    order: &BulkOrder&lt;M&gt;,
    is_bid: bool
) {
    <b>let</b> active_price = order.get_order_request().get_active_price(is_bid);
    <b>if</b> (active_price.is_some()) {
        price_time_idx.cancel_active_order(
            active_price.destroy_some(),
            order.get_unique_priority_idx(),
            is_bid
        );
    };
}
</code></pre>



</details>

<a id="0x7_bulk_order_book_cancel_active_orders"></a>

## Function `cancel_active_orders`

Cancels all active orders (both bid and ask) for a bulk order.


<a id="@Arguments:_14"></a>

### Arguments:

- <code>active_orders</code>: Reference to the active order book
- <code>order</code>: The bulk order to cancel active orders for


<pre><code><b>fun</b> <a href="bulk_order_book.md#0x7_bulk_order_book_cancel_active_orders">cancel_active_orders</a>&lt;M: <b>copy</b>, drop, store&gt;(price_time_idx: &<b>mut</b> <a href="price_time_index.md#0x7_price_time_index_PriceTimeIndex">price_time_index::PriceTimeIndex</a>, order: &<a href="_BulkOrder">bulk_order_types::BulkOrder</a>&lt;M&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="bulk_order_book.md#0x7_bulk_order_book_cancel_active_orders">cancel_active_orders</a>&lt;M: store + <b>copy</b> + drop&gt;(
    price_time_idx: &<b>mut</b> aptos_experimental::price_time_index::PriceTimeIndex,
    order: &BulkOrder&lt;M&gt;
) {
    <a href="bulk_order_book.md#0x7_bulk_order_book_cancel_active_order_for_side">cancel_active_order_for_side</a>(price_time_idx, order, <b>true</b>); // cancel bid
    <a href="bulk_order_book.md#0x7_bulk_order_book_cancel_active_order_for_side">cancel_active_order_for_side</a>(price_time_idx, order, <b>false</b>); // cancel ask
}
</code></pre>



</details>

<a id="0x7_bulk_order_book_activate_first_price_level_for_side"></a>

## Function `activate_first_price_level_for_side`

Activates the first price level for a specific side of a bulk order.


<a id="@Arguments:_15"></a>

### Arguments:

- <code>active_orders</code>: Reference to the active order book
- <code>order</code>: The bulk order to activate levels for
- <code>order_id</code>: The order ID for the bulk order
- <code>is_bid</code>: True to activate bid levels, false for ask levels


<pre><code><b>fun</b> <a href="bulk_order_book.md#0x7_bulk_order_book_activate_first_price_level_for_side">activate_first_price_level_for_side</a>&lt;M: <b>copy</b>, drop, store&gt;(price_time_idx: &<b>mut</b> <a href="price_time_index.md#0x7_price_time_index_PriceTimeIndex">price_time_index::PriceTimeIndex</a>, order: &<a href="_BulkOrder">bulk_order_types::BulkOrder</a>&lt;M&gt;, order_id: <a href="_OrderId">order_book_types::OrderId</a>, is_bid: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="bulk_order_book.md#0x7_bulk_order_book_activate_first_price_level_for_side">activate_first_price_level_for_side</a>&lt;M: store + <b>copy</b> + drop&gt;(
    price_time_idx: &<b>mut</b> aptos_experimental::price_time_index::PriceTimeIndex,
    order: &BulkOrder&lt;M&gt;,
    order_id: OrderId,
    is_bid: bool
) {
    <b>let</b> order_request = order.get_order_request();
    <b>let</b> active_price = order_request.get_active_price(is_bid);
    <b>let</b> active_size = order_request.get_active_size(is_bid);
    <b>if</b> (active_price.is_some()) {
        price_time_idx.place_maker_order(
            order_id,
            bulk_order_type(),
            active_price.destroy_some(),
            order.get_unique_priority_idx(),
            active_size.destroy_some(),
            is_bid
        );
    }
}
</code></pre>



</details>

<a id="0x7_bulk_order_book_activate_first_price_levels"></a>

## Function `activate_first_price_levels`

Activates the first price levels for both bid and ask sides of a bulk order.


<a id="@Arguments:_16"></a>

### Arguments:

- <code>active_orders</code>: Reference to the active order book
- <code>order</code>: The bulk order to activate levels for
- <code>order_id</code>: The order ID for the bulk order


<pre><code><b>fun</b> <a href="bulk_order_book.md#0x7_bulk_order_book_activate_first_price_levels">activate_first_price_levels</a>&lt;M: <b>copy</b>, drop, store&gt;(price_time_idx: &<b>mut</b> <a href="price_time_index.md#0x7_price_time_index_PriceTimeIndex">price_time_index::PriceTimeIndex</a>, order: &<a href="_BulkOrder">bulk_order_types::BulkOrder</a>&lt;M&gt;, order_id: <a href="_OrderId">order_book_types::OrderId</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="bulk_order_book.md#0x7_bulk_order_book_activate_first_price_levels">activate_first_price_levels</a>&lt;M: store + <b>copy</b> + drop&gt;(
    price_time_idx: &<b>mut</b> aptos_experimental::price_time_index::PriceTimeIndex,
    order: &BulkOrder&lt;M&gt;,
    order_id: OrderId
) {
    <a href="bulk_order_book.md#0x7_bulk_order_book_activate_first_price_level_for_side">activate_first_price_level_for_side</a>(price_time_idx, order, order_id, <b>true</b>); // activate bid
    <a href="bulk_order_book.md#0x7_bulk_order_book_activate_first_price_level_for_side">activate_first_price_level_for_side</a>(price_time_idx, order, order_id, <b>false</b>); // activate ask
}
</code></pre>



</details>

<a id="0x7_bulk_order_book_reinsert_order"></a>

## Function `reinsert_order`

Reinserts a bulk order back into the order book after it has been matched.

This function allows traders to reinsert portions of their orders that were matched,
effectively allowing them to "reuse" matched liquidity.


<a id="@Arguments:_17"></a>

### Arguments:

- <code>self</code>: Mutable reference to the bulk order book
- <code>price_time_idx</code>: Mutable reference to the price time index
- <code>reinsert_order</code>: The order result to reinsert
- <code>original_order</code>: The original order result for validation


<a id="@Aborts:_18"></a>

### Aborts:

- If the order account doesn't exist in the order book
- If the reinsertion validation fails


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="bulk_order_book.md#0x7_bulk_order_book_reinsert_order">reinsert_order</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="bulk_order_book.md#0x7_bulk_order_book_BulkOrderBook">bulk_order_book::BulkOrderBook</a>&lt;M&gt;, price_time_idx: &<b>mut</b> <a href="price_time_index.md#0x7_price_time_index_PriceTimeIndex">price_time_index::PriceTimeIndex</a>, reinsert_order: <a href="_OrderMatchDetails">order_match_types::OrderMatchDetails</a>&lt;M&gt;, original_order: &<a href="_OrderMatchDetails">order_match_types::OrderMatchDetails</a>&lt;M&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="bulk_order_book.md#0x7_bulk_order_book_reinsert_order">reinsert_order</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="bulk_order_book.md#0x7_bulk_order_book_BulkOrderBook">BulkOrderBook</a>&lt;M&gt;,
    price_time_idx: &<b>mut</b> aptos_experimental::price_time_index::PriceTimeIndex,
    reinsert_order: OrderMatchDetails&lt;M&gt;,
    original_order: &OrderMatchDetails&lt;M&gt;
) {
    <b>assert</b>!(
        reinsert_order.validate_bulk_order_reinsertion_request(original_order),
        <a href="bulk_order_book.md#0x7_bulk_order_book_E_REINSERT_ORDER_MISMATCH">E_REINSERT_ORDER_MISMATCH</a>
    );
    <b>let</b> <a href="../../aptos-framework/doc/account.md#0x1_account">account</a> = reinsert_order.get_account_from_match_details();
    <b>let</b> order_option = self.orders.remove_or_none(&<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>);
    <b>assert</b>!(order_option.is_some(), <a href="bulk_order_book.md#0x7_bulk_order_book_EORDER_NOT_FOUND">EORDER_NOT_FOUND</a>);
    <b>let</b> order = order_option.destroy_some();
    <a href="bulk_order_book.md#0x7_bulk_order_book_cancel_active_orders">cancel_active_orders</a>(price_time_idx, &order);
    <a href="bulk_order_utils.md#0x7_bulk_order_utils_reinsert_order_into_bulk_order">bulk_order_utils::reinsert_order_into_bulk_order</a>(&<b>mut</b> order, &reinsert_order);
    <a href="bulk_order_book.md#0x7_bulk_order_book_activate_first_price_levels">activate_first_price_levels</a>(
        price_time_idx, &order, reinsert_order.get_order_id_from_match_details()
    );
    self.orders.add(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>, order);
}
</code></pre>



</details>

<a id="0x7_bulk_order_book_cancel_bulk_order"></a>

## Function `cancel_bulk_order`

Cancels a bulk order for the specified account.

Instead of removing the order entirely, this function clears all active levels
and sets the order to empty state, allowing the same account to place new orders
with the same order ID in the future.


<a id="@Arguments:_19"></a>

### Arguments:

- <code>self</code>: Mutable reference to the bulk order book
- <code>price_time_idx</code>: Mutable reference to the price time index
- <code><a href="../../aptos-framework/doc/account.md#0x1_account">account</a></code>: The account whose order should be cancelled


<a id="@Aborts:_20"></a>

### Aborts:

- If no order exists for the specified account


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="bulk_order_book.md#0x7_bulk_order_book_cancel_bulk_order">cancel_bulk_order</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="bulk_order_book.md#0x7_bulk_order_book_BulkOrderBook">bulk_order_book::BulkOrderBook</a>&lt;M&gt;, price_time_idx: &<b>mut</b> <a href="price_time_index.md#0x7_price_time_index_PriceTimeIndex">price_time_index::PriceTimeIndex</a>, <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>): <a href="_BulkOrder">bulk_order_types::BulkOrder</a>&lt;M&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="bulk_order_book.md#0x7_bulk_order_book_cancel_bulk_order">cancel_bulk_order</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="bulk_order_book.md#0x7_bulk_order_book_BulkOrderBook">BulkOrderBook</a>&lt;M&gt;,
    price_time_idx: &<b>mut</b> aptos_experimental::price_time_index::PriceTimeIndex,
    <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>
): BulkOrder&lt;M&gt; {
    // For cancellation, instead of removing the order, we will just cancel the active orders and set the sizes <b>to</b> 0.
    // This allows us <b>to</b> reuse the order id for the same <a href="../../aptos-framework/doc/account.md#0x1_account">account</a> in the future without creating a new order.
    <b>let</b> order_opt = self.orders.remove_or_none(&<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>);
    <b>assert</b>!(order_opt.is_some(), <a href="bulk_order_book.md#0x7_bulk_order_book_EORDER_NOT_FOUND">EORDER_NOT_FOUND</a>);
    <b>let</b> order = order_opt.destroy_some();
    <b>let</b> order_copy = order;
    <a href="bulk_order_book.md#0x7_bulk_order_book_cancel_active_orders">cancel_active_orders</a>(price_time_idx, &order);
    order.set_empty();
    self.orders.add(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>, order);
    order_copy
}
</code></pre>



</details>

<a id="0x7_bulk_order_book_cancel_bulk_order_at_price"></a>

## Function `cancel_bulk_order_at_price`

Cancels a specific price level in a bulk order.

This function removes only the specified price level from the bulk order,
keeping all other price levels intact. If the cancelled price level was active,
it will be removed from the active order book and the next price level (if any)
will be activated.


<a id="@Arguments:_21"></a>

### Arguments:

- <code>self</code>: Mutable reference to the bulk order book
- <code>price_time_idx</code>: Mutable reference to the price time index
- <code><a href="../../aptos-framework/doc/account.md#0x1_account">account</a></code>: The account whose order contains the price level to cancel
- <code>price</code>: The price level to cancel
- <code>is_bid</code>: True to cancel from bid side, false for ask side


<a id="@Returns:_22"></a>

### Returns:

A tuple containing:
- The cancelled size at that price level
- The updated bulk order (copy for event emission)


<a id="@Aborts:_23"></a>

### Aborts:

- If no order exists for the specified account


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="bulk_order_book.md#0x7_bulk_order_book_cancel_bulk_order_at_price">cancel_bulk_order_at_price</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="bulk_order_book.md#0x7_bulk_order_book_BulkOrderBook">bulk_order_book::BulkOrderBook</a>&lt;M&gt;, price_time_idx: &<b>mut</b> <a href="price_time_index.md#0x7_price_time_index_PriceTimeIndex">price_time_index::PriceTimeIndex</a>, <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>, price: u64, is_bid: bool): (u64, <a href="_BulkOrder">bulk_order_types::BulkOrder</a>&lt;M&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="bulk_order_book.md#0x7_bulk_order_book_cancel_bulk_order_at_price">cancel_bulk_order_at_price</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="bulk_order_book.md#0x7_bulk_order_book_BulkOrderBook">BulkOrderBook</a>&lt;M&gt;,
    price_time_idx: &<b>mut</b> aptos_experimental::price_time_index::PriceTimeIndex,
    <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>,
    price: u64,
    is_bid: bool
): (u64, BulkOrder&lt;M&gt;) {
    <b>let</b> order_opt = self.orders.remove_or_none(&<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>);
    <b>assert</b>!(order_opt.is_some(), <a href="bulk_order_book.md#0x7_bulk_order_book_EORDER_NOT_FOUND">EORDER_NOT_FOUND</a>);
    <b>let</b> order = order_opt.destroy_some();

    // Check <b>if</b> the price <b>to</b> cancel is the currently active price
    <b>let</b> active_price = order.get_order_request().get_active_price(is_bid);
    <b>let</b> was_active = active_price.is_some() && active_price.destroy_some() == price;

    // If this was the active price level, we need <b>to</b> cancel it from the active order book first
    <b>if</b> (was_active) {
        <a href="bulk_order_book.md#0x7_bulk_order_book_cancel_active_order_for_side">cancel_active_order_for_side</a>(price_time_idx, &order, is_bid);
    };

    // Cancel the specific price level
    <b>let</b> cancelled_size =
        <a href="bulk_order_utils.md#0x7_bulk_order_utils_cancel_at_price_level">bulk_order_utils::cancel_at_price_level</a>(&<b>mut</b> order, price, is_bid);

    // If this was the active price level, activate the next price level <b>if</b> available
    <b>if</b> (was_active) {
        <b>let</b> order_id = order.get_order_id();
        <a href="bulk_order_book.md#0x7_bulk_order_book_activate_first_price_level_for_side">activate_first_price_level_for_side</a>(price_time_idx, &order, order_id, is_bid);
    };

    <b>let</b> order_copy = order;
    self.orders.add(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>, order);
    (cancelled_size, order_copy)
}
</code></pre>



</details>

<a id="0x7_bulk_order_book_get_bulk_order"></a>

## Function `get_bulk_order`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="bulk_order_book.md#0x7_bulk_order_book_get_bulk_order">get_bulk_order</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="bulk_order_book.md#0x7_bulk_order_book_BulkOrderBook">bulk_order_book::BulkOrderBook</a>&lt;M&gt;, <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>): <a href="_BulkOrder">bulk_order_types::BulkOrder</a>&lt;M&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="bulk_order_book.md#0x7_bulk_order_book_get_bulk_order">get_bulk_order</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="bulk_order_book.md#0x7_bulk_order_book_BulkOrderBook">BulkOrderBook</a>&lt;M&gt;, <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>
): BulkOrder&lt;M&gt; {
    <b>let</b> result = self.orders.get(&<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>);
    <b>assert</b>!(result.is_some(), <a href="bulk_order_book.md#0x7_bulk_order_book_EORDER_NOT_FOUND">EORDER_NOT_FOUND</a>);
    result.destroy_some()
}
</code></pre>



</details>

<a id="0x7_bulk_order_book_get_remaining_size"></a>

## Function `get_remaining_size`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="bulk_order_book.md#0x7_bulk_order_book_get_remaining_size">get_remaining_size</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="bulk_order_book.md#0x7_bulk_order_book_BulkOrderBook">bulk_order_book::BulkOrderBook</a>&lt;M&gt;, <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>, is_bid: bool): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="bulk_order_book.md#0x7_bulk_order_book_get_remaining_size">get_remaining_size</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="bulk_order_book.md#0x7_bulk_order_book_BulkOrderBook">BulkOrderBook</a>&lt;M&gt;, <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>, is_bid: bool
): u64 {
    <b>let</b> result_option =
        self.orders.get_and_map(
            &<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>,
            |order| order.get_order_request().get_total_remaining_size(is_bid)
        );
    <b>assert</b>!(result_option.is_some(), <a href="bulk_order_book.md#0x7_bulk_order_book_EORDER_NOT_FOUND">EORDER_NOT_FOUND</a>);
    result_option.destroy_some()
}
</code></pre>



</details>

<a id="0x7_bulk_order_book_get_prices"></a>

## Function `get_prices`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="bulk_order_book.md#0x7_bulk_order_book_get_prices">get_prices</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="bulk_order_book.md#0x7_bulk_order_book_BulkOrderBook">bulk_order_book::BulkOrderBook</a>&lt;M&gt;, <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>, is_bid: bool): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="bulk_order_book.md#0x7_bulk_order_book_get_prices">get_prices</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="bulk_order_book.md#0x7_bulk_order_book_BulkOrderBook">BulkOrderBook</a>&lt;M&gt;, <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>, is_bid: bool
): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt; {
    <b>let</b> result_option =
        self.orders.get_and_map(
            &<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>,
            |order| order.get_order_request().get_all_prices(is_bid)
        );
    <b>assert</b>!(result_option.is_some(), <a href="bulk_order_book.md#0x7_bulk_order_book_EORDER_NOT_FOUND">EORDER_NOT_FOUND</a>);
    result_option.destroy_some()
}
</code></pre>



</details>

<a id="0x7_bulk_order_book_get_sizes"></a>

## Function `get_sizes`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="bulk_order_book.md#0x7_bulk_order_book_get_sizes">get_sizes</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="bulk_order_book.md#0x7_bulk_order_book_BulkOrderBook">bulk_order_book::BulkOrderBook</a>&lt;M&gt;, <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>, is_bid: bool): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="bulk_order_book.md#0x7_bulk_order_book_get_sizes">get_sizes</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="bulk_order_book.md#0x7_bulk_order_book_BulkOrderBook">BulkOrderBook</a>&lt;M&gt;, <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>, is_bid: bool
): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt; {
    <b>let</b> result_option =
        self.orders.get_and_map(
            &<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>,
            |order| order.get_order_request().get_all_sizes(is_bid)
        );
    <b>assert</b>!(result_option.is_some(), <a href="bulk_order_book.md#0x7_bulk_order_book_EORDER_NOT_FOUND">EORDER_NOT_FOUND</a>);
    result_option.destroy_some()
}
</code></pre>



</details>

<a id="0x7_bulk_order_book_place_bulk_order"></a>

## Function `place_bulk_order`

Places a new maker order in the bulk order book.

If an order already exists for the account, it will be replaced with the new order.
The first price levels of both bid and ask sides will be activated in the active order book.


<a id="@Arguments:_24"></a>

### Arguments:

- <code>self</code>: Mutable reference to the bulk order book
- <code>price_time_idx</code>: Mutable reference to the price time index
- <code>order_req</code>: The bulk order request to place


<a id="@Aborts:_25"></a>

### Aborts:

- If the order request validation fails


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="bulk_order_book.md#0x7_bulk_order_book_place_bulk_order">place_bulk_order</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="bulk_order_book.md#0x7_bulk_order_book_BulkOrderBook">bulk_order_book::BulkOrderBook</a>&lt;M&gt;, price_time_idx: &<b>mut</b> <a href="price_time_index.md#0x7_price_time_index_PriceTimeIndex">price_time_index::PriceTimeIndex</a>, order_req: <a href="_BulkOrderRequest">bulk_order_types::BulkOrderRequest</a>&lt;M&gt;): <a href="_BulkOrderPlaceResponse">bulk_order_types::BulkOrderPlaceResponse</a>&lt;M&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="bulk_order_book.md#0x7_bulk_order_book_place_bulk_order">place_bulk_order</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="bulk_order_book.md#0x7_bulk_order_book_BulkOrderBook">BulkOrderBook</a>&lt;M&gt;,
    price_time_idx: &<b>mut</b> aptos_experimental::price_time_index::PriceTimeIndex,
    order_req: BulkOrderRequest&lt;M&gt;
): BulkOrderPlaceResponse&lt;M&gt; {
    <b>let</b> <a href="../../aptos-framework/doc/account.md#0x1_account">account</a> = order_req.get_account();
    <b>let</b> new_sequence_number = order_req.get_sequence_number();
    <b>let</b> order_option = self.orders.remove_or_none(&<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>);
    <b>let</b> (order_id, previous_seq_num) =
        <b>if</b> (order_option.is_some()) {
            <b>let</b> old_order = order_option.destroy_some();
            <b>let</b> existing_sequence_number =
                old_order.get_order_request().get_sequence_number();
            // Return rejection response instead of aborting
            <b>if</b> (new_sequence_number &lt;= existing_sequence_number) {
                // Put the <b>old</b> order back
                self.orders.add(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>, old_order);
                <b>return</b> new_bulk_order_place_response_rejection(
                    <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>,
                    new_sequence_number,
                    existing_sequence_number
                )
            };
            <a href="bulk_order_book.md#0x7_bulk_order_book_cancel_active_orders">cancel_active_orders</a>(price_time_idx, &old_order);
            (old_order.get_order_id(), std::option::some(existing_sequence_number))
        } <b>else</b> {
            <b>let</b> order_id = next_order_id();
            self.order_id_to_address.add(order_id, <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>);
            (order_id, std::option::none())
        };
    <b>let</b> (
        bulk_order,
        cancelled_bid_prices,
        cancelled_bid_sizes,
        cancelled_ask_prices,
        cancelled_ask_sizes
    ) =
        <a href="bulk_order_utils.md#0x7_bulk_order_utils_new_bulk_order_with_sanitization">bulk_order_utils::new_bulk_order_with_sanitization</a>(
            order_id,
            next_increasing_idx_type(),
            order_req,
            price_time_idx.best_bid_price(),
            price_time_idx.best_ask_price()
        );
    self.orders.add(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>, bulk_order);
    // Activate the first price levels in the active order book
    <a href="bulk_order_book.md#0x7_bulk_order_book_activate_first_price_levels">activate_first_price_levels</a>(price_time_idx, &bulk_order, order_id);
    new_bulk_order_place_response_success(
        bulk_order,
        cancelled_bid_prices,
        cancelled_bid_sizes,
        cancelled_ask_prices,
        cancelled_ask_sizes,
        previous_seq_num
    )
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
