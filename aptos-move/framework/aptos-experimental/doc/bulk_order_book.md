
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
- <code><a href="bulk_order_book.md#0x7_bulk_order_book_EPOST_ONLY_FILLED">EPOST_ONLY_FILLED</a></code>: Post-only order was filled (not implemented in bulk orders)
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
-  [Function `get_remaining_size`](#0x7_bulk_order_book_get_remaining_size)
-  [Function `get_prices`](#0x7_bulk_order_book_get_prices)
-  [Function `get_sizes`](#0x7_bulk_order_book_get_sizes)
-  [Function `place_bulk_order`](#0x7_bulk_order_book_place_bulk_order)
    -  [Arguments:](#@Arguments:_21)
    -  [Aborts:](#@Aborts:_22)


<pre><code><b>use</b> <a href="../../aptos-framework/doc/big_ordered_map.md#0x1_big_ordered_map">0x1::big_ordered_map</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types">0x7::bulk_order_book_types</a>;
<b>use</b> <a href="order_book_types.md#0x7_order_book_types">0x7::order_book_types</a>;
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
<code>orders: <a href="../../aptos-framework/doc/big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">big_ordered_map::BigOrderedMap</a>&lt;<b>address</b>, <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrder">bulk_order_book_types::BulkOrder</a>&lt;M&gt;&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>order_id_to_address: <a href="../../aptos-framework/doc/big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">big_ordered_map::BigOrderedMap</a>&lt;<a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>, <b>address</b>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

</details>

<a id="@Constants_8"></a>

## Constants


<a id="0x7_bulk_order_book_EORDER_ALREADY_EXISTS"></a>



<pre><code><b>const</b> <a href="bulk_order_book.md#0x7_bulk_order_book_EORDER_ALREADY_EXISTS">EORDER_ALREADY_EXISTS</a>: u64 = 1;
</code></pre>



<a id="0x7_bulk_order_book_EINVLID_MM_ORDER_REQUEST"></a>



<pre><code><b>const</b> <a href="bulk_order_book.md#0x7_bulk_order_book_EINVLID_MM_ORDER_REQUEST">EINVLID_MM_ORDER_REQUEST</a>: u64 = 10;
</code></pre>



<a id="0x7_bulk_order_book_EPRICE_CROSSING"></a>



<pre><code><b>const</b> <a href="bulk_order_book.md#0x7_bulk_order_book_EPRICE_CROSSING">EPRICE_CROSSING</a>: u64 = 11;
</code></pre>



<a id="0x7_bulk_order_book_E_REINSERT_ORDER_MISMATCH"></a>



<pre><code><b>const</b> <a href="bulk_order_book.md#0x7_bulk_order_book_E_REINSERT_ORDER_MISMATCH">E_REINSERT_ORDER_MISMATCH</a>: u64 = 8;
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



<a id="0x7_bulk_order_book_E_INVALID_SEQUENCE_NUMBER"></a>



<pre><code><b>const</b> <a href="bulk_order_book.md#0x7_bulk_order_book_E_INVALID_SEQUENCE_NUMBER">E_INVALID_SEQUENCE_NUMBER</a>: u64 = 13;
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


<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_book.md#0x7_bulk_order_book_new_bulk_order_book">new_bulk_order_book</a>&lt;M: <b>copy</b>, drop, store&gt;(): <a href="bulk_order_book.md#0x7_bulk_order_book_BulkOrderBook">bulk_order_book::BulkOrderBook</a>&lt;M&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_book.md#0x7_bulk_order_book_new_bulk_order_book">new_bulk_order_book</a>&lt;M: store + <b>copy</b> + drop&gt;(): <a href="bulk_order_book.md#0x7_bulk_order_book_BulkOrderBook">BulkOrderBook</a>&lt;M&gt; {
    BulkOrderBook::V1 {
        orders:  <a href="order_book_types.md#0x7_order_book_types_new_default_big_ordered_map">order_book_types::new_default_big_ordered_map</a>(),
        order_id_to_address:  <a href="order_book_types.md#0x7_order_book_types_new_default_big_ordered_map">order_book_types::new_default_big_ordered_map</a>()
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


<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_book.md#0x7_bulk_order_book_get_single_match_for_taker">get_single_match_for_taker</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="bulk_order_book.md#0x7_bulk_order_book_BulkOrderBook">bulk_order_book::BulkOrderBook</a>&lt;M&gt;, price_time_idx: &<b>mut</b> <a href="price_time_index.md#0x7_price_time_index_PriceTimeIndex">price_time_index::PriceTimeIndex</a>, active_matched_order: <a href="order_book_types.md#0x7_order_book_types_ActiveMatchedOrder">order_book_types::ActiveMatchedOrder</a>, is_bid: bool): <a href="order_book_types.md#0x7_order_book_types_OrderMatch">order_book_types::OrderMatch</a>&lt;M&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_book.md#0x7_bulk_order_book_get_single_match_for_taker">get_single_match_for_taker</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="bulk_order_book.md#0x7_bulk_order_book_BulkOrderBook">BulkOrderBook</a>&lt;M&gt;,
    price_time_idx: &<b>mut</b> aptos_experimental::price_time_index::PriceTimeIndex,
    active_matched_order: ActiveMatchedOrder,
    is_bid: bool
): OrderMatch&lt;M&gt; {
    <b>let</b> (order_id, matched_size, remaining_size, order_book_type) =
        active_matched_order.destroy_active_matched_order();
    <b>assert</b>!(order_book_type == bulk_order_book_type(), <a href="bulk_order_book.md#0x7_bulk_order_book_ENOT_BULK_ORDER">ENOT_BULK_ORDER</a>);
    <b>let</b> order_address = self.order_id_to_address.get(&order_id).destroy_some();
    <b>let</b> order = self.orders.remove(&order_address);
    <b>let</b> order_match = new_bulk_order_match&lt;M&gt;(
        &<b>mut</b> order,
        !is_bid,
        matched_size,
    );
    <b>let</b> (next_price, next_size) = order.match_order_and_get_next(!is_bid, matched_size);
    <b>if</b> (remaining_size == 0 && next_price.is_some()) {
        <b>let</b> price = next_price.destroy_some();
        <b>let</b> size = next_size.destroy_some();
        price_time_idx.place_maker_order(
            order_id,
            bulk_order_book_type(),
            price,
            order.get_unique_priority_idx(),
            size,
            !is_bid,
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


<pre><code><b>fun</b> <a href="bulk_order_book.md#0x7_bulk_order_book_cancel_active_order_for_side">cancel_active_order_for_side</a>&lt;M: <b>copy</b>, drop, store&gt;(price_time_idx: &<b>mut</b> <a href="price_time_index.md#0x7_price_time_index_PriceTimeIndex">price_time_index::PriceTimeIndex</a>, order: &<a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrder">bulk_order_book_types::BulkOrder</a>&lt;M&gt;, is_bid: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="bulk_order_book.md#0x7_bulk_order_book_cancel_active_order_for_side">cancel_active_order_for_side</a>&lt;M: store + <b>copy</b> + drop&gt;(
    price_time_idx: &<b>mut</b> aptos_experimental::price_time_index::PriceTimeIndex,
    order: &BulkOrder&lt;M&gt;,
    is_bid: bool
) {
    <b>let</b> active_price = order.get_active_price(is_bid);
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


<pre><code><b>fun</b> <a href="bulk_order_book.md#0x7_bulk_order_book_cancel_active_orders">cancel_active_orders</a>&lt;M: <b>copy</b>, drop, store&gt;(price_time_idx: &<b>mut</b> <a href="price_time_index.md#0x7_price_time_index_PriceTimeIndex">price_time_index::PriceTimeIndex</a>, order: &<a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrder">bulk_order_book_types::BulkOrder</a>&lt;M&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="bulk_order_book.md#0x7_bulk_order_book_cancel_active_orders">cancel_active_orders</a>&lt;M: store + <b>copy</b> + drop&gt;(
    price_time_idx: &<b>mut</b> aptos_experimental::price_time_index::PriceTimeIndex, order: &BulkOrder&lt;M&gt;
) {
    <a href="bulk_order_book.md#0x7_bulk_order_book_cancel_active_order_for_side">cancel_active_order_for_side</a>(price_time_idx, order, <b>true</b>);  // cancel bid
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


<pre><code><b>fun</b> <a href="bulk_order_book.md#0x7_bulk_order_book_activate_first_price_level_for_side">activate_first_price_level_for_side</a>&lt;M: <b>copy</b>, drop, store&gt;(price_time_idx: &<b>mut</b> <a href="price_time_index.md#0x7_price_time_index_PriceTimeIndex">price_time_index::PriceTimeIndex</a>, order: &<a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrder">bulk_order_book_types::BulkOrder</a>&lt;M&gt;, order_id: <a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>, is_bid: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="bulk_order_book.md#0x7_bulk_order_book_activate_first_price_level_for_side">activate_first_price_level_for_side</a>&lt;M: store + <b>copy</b> + drop&gt;(
    price_time_idx: &<b>mut</b> aptos_experimental::price_time_index::PriceTimeIndex,
    order: &BulkOrder&lt;M&gt;,
    order_id: OrderIdType,
    is_bid: bool
) {
    <b>let</b> active_price = order.get_active_price(is_bid);
    <b>let</b> active_size = order.get_active_size(is_bid);
    <b>if</b> (active_price.is_some()) {
        price_time_idx.place_maker_order(
            order_id,
            bulk_order_book_type(),
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


<pre><code><b>fun</b> <a href="bulk_order_book.md#0x7_bulk_order_book_activate_first_price_levels">activate_first_price_levels</a>&lt;M: <b>copy</b>, drop, store&gt;(price_time_idx: &<b>mut</b> <a href="price_time_index.md#0x7_price_time_index_PriceTimeIndex">price_time_index::PriceTimeIndex</a>, order: &<a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrder">bulk_order_book_types::BulkOrder</a>&lt;M&gt;, order_id: <a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="bulk_order_book.md#0x7_bulk_order_book_activate_first_price_levels">activate_first_price_levels</a>&lt;M: store + <b>copy</b> + drop&gt;(
    price_time_idx: &<b>mut</b> aptos_experimental::price_time_index::PriceTimeIndex, order: &BulkOrder&lt;M&gt;, order_id: OrderIdType
) {
    <a href="bulk_order_book.md#0x7_bulk_order_book_activate_first_price_level_for_side">activate_first_price_level_for_side</a>(price_time_idx, order, order_id, <b>true</b>);  // activate bid
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


<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_book.md#0x7_bulk_order_book_reinsert_order">reinsert_order</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="bulk_order_book.md#0x7_bulk_order_book_BulkOrderBook">bulk_order_book::BulkOrderBook</a>&lt;M&gt;, price_time_idx: &<b>mut</b> <a href="price_time_index.md#0x7_price_time_index_PriceTimeIndex">price_time_index::PriceTimeIndex</a>, reinsert_order: <a href="order_book_types.md#0x7_order_book_types_OrderMatchDetails">order_book_types::OrderMatchDetails</a>&lt;M&gt;, original_order: &<a href="order_book_types.md#0x7_order_book_types_OrderMatchDetails">order_book_types::OrderMatchDetails</a>&lt;M&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_book.md#0x7_bulk_order_book_reinsert_order">reinsert_order</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="bulk_order_book.md#0x7_bulk_order_book_BulkOrderBook">BulkOrderBook</a>&lt;M&gt;,
    price_time_idx: &<b>mut</b> aptos_experimental::price_time_index::PriceTimeIndex,
    reinsert_order: OrderMatchDetails&lt;M&gt;,
    original_order: &OrderMatchDetails&lt;M&gt;
) {
    <b>assert</b>!(reinsert_order.validate_reinsertion_request(original_order), <a href="bulk_order_book.md#0x7_bulk_order_book_E_REINSERT_ORDER_MISMATCH">E_REINSERT_ORDER_MISMATCH</a>);
    <b>let</b> <a href="../../aptos-framework/doc/account.md#0x1_account">account</a> = reinsert_order.get_account_from_match_details();
    <b>assert</b>!(self.orders.contains(&<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>), <a href="bulk_order_book.md#0x7_bulk_order_book_EORDER_NOT_FOUND">EORDER_NOT_FOUND</a>);
    <b>let</b> order = self.orders.remove(&<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>);
    <a href="bulk_order_book.md#0x7_bulk_order_book_cancel_active_orders">cancel_active_orders</a>(price_time_idx, &order);
    order.<a href="bulk_order_book.md#0x7_bulk_order_book_reinsert_order">reinsert_order</a>(&reinsert_order);
    <a href="bulk_order_book.md#0x7_bulk_order_book_activate_first_price_levels">activate_first_price_levels</a>(price_time_idx, &order, reinsert_order.get_order_id_from_match_details());
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


<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_book.md#0x7_bulk_order_book_cancel_bulk_order">cancel_bulk_order</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="bulk_order_book.md#0x7_bulk_order_book_BulkOrderBook">bulk_order_book::BulkOrderBook</a>&lt;M&gt;, price_time_idx: &<b>mut</b> <a href="price_time_index.md#0x7_price_time_index_PriceTimeIndex">price_time_index::PriceTimeIndex</a>, <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>): (<a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>, u64, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_book.md#0x7_bulk_order_book_cancel_bulk_order">cancel_bulk_order</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="bulk_order_book.md#0x7_bulk_order_book_BulkOrderBook">BulkOrderBook</a>&lt;M&gt;,
    price_time_idx: &<b>mut</b> aptos_experimental::price_time_index::PriceTimeIndex,
    <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>
): (OrderIdType, u64, u64) {
    <b>if</b> (!self.orders.contains(&<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>)) {
        <b>abort</b> <a href="bulk_order_book.md#0x7_bulk_order_book_EORDER_NOT_FOUND">EORDER_NOT_FOUND</a>;
    };
    // For cancellation, instead of removing the order, we will just cancel the active orders and set the sizes <b>to</b> 0.
    // This allows us <b>to</b> reuse the order id for the same <a href="../../aptos-framework/doc/account.md#0x1_account">account</a> in the future without creating a new order.
    <b>let</b> order = self.orders.remove(&<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>);
    <b>let</b> order_id = order.get_order_id();
    <b>let</b> remaining_bid_size = order.get_total_remaining_size(<b>true</b>);
    <b>let</b> remaining_ask_size = order.get_total_remaining_size(<b>false</b>);
    <a href="bulk_order_book.md#0x7_bulk_order_book_cancel_active_orders">cancel_active_orders</a>(price_time_idx, &order);
    order.set_empty();
    self.orders.add(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>, order);
    (order_id, remaining_bid_size, remaining_ask_size)
}
</code></pre>



</details>

<a id="0x7_bulk_order_book_get_remaining_size"></a>

## Function `get_remaining_size`



<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_book.md#0x7_bulk_order_book_get_remaining_size">get_remaining_size</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="bulk_order_book.md#0x7_bulk_order_book_BulkOrderBook">bulk_order_book::BulkOrderBook</a>&lt;M&gt;, <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>, is_bid: bool): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_book.md#0x7_bulk_order_book_get_remaining_size">get_remaining_size</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="bulk_order_book.md#0x7_bulk_order_book_BulkOrderBook">BulkOrderBook</a>&lt;M&gt;,
    <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>,
    is_bid: bool
): u64 {
    <b>if</b> (!self.orders.contains(&<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>)) {
        <b>abort</b> <a href="bulk_order_book.md#0x7_bulk_order_book_EORDER_NOT_FOUND">EORDER_NOT_FOUND</a>;
    };

    self.orders.get(&<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>).destroy_some().get_total_remaining_size(is_bid)
}
</code></pre>



</details>

<a id="0x7_bulk_order_book_get_prices"></a>

## Function `get_prices`



<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_book.md#0x7_bulk_order_book_get_prices">get_prices</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="bulk_order_book.md#0x7_bulk_order_book_BulkOrderBook">bulk_order_book::BulkOrderBook</a>&lt;M&gt;, <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>, is_bid: bool): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_book.md#0x7_bulk_order_book_get_prices">get_prices</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="bulk_order_book.md#0x7_bulk_order_book_BulkOrderBook">BulkOrderBook</a>&lt;M&gt;,
    <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>,
    is_bid: bool
): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt; {
    <b>if</b> (!self.orders.contains(&<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>)) {
        <b>abort</b> <a href="bulk_order_book.md#0x7_bulk_order_book_EORDER_NOT_FOUND">EORDER_NOT_FOUND</a>;
    };

    self.orders.get(&<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>).destroy_some().get_all_prices(is_bid)
}
</code></pre>



</details>

<a id="0x7_bulk_order_book_get_sizes"></a>

## Function `get_sizes`



<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_book.md#0x7_bulk_order_book_get_sizes">get_sizes</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="bulk_order_book.md#0x7_bulk_order_book_BulkOrderBook">bulk_order_book::BulkOrderBook</a>&lt;M&gt;, <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>, is_bid: bool): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_book.md#0x7_bulk_order_book_get_sizes">get_sizes</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="bulk_order_book.md#0x7_bulk_order_book_BulkOrderBook">BulkOrderBook</a>&lt;M&gt;,
    <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>,
    is_bid: bool
): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt; {
    <b>if</b> (!self.orders.contains(&<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>)) {
        <b>abort</b> <a href="bulk_order_book.md#0x7_bulk_order_book_EORDER_NOT_FOUND">EORDER_NOT_FOUND</a>;
    };

    self.orders.get(&<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>).destroy_some().get_all_sizes(is_bid)
}
</code></pre>



</details>

<a id="0x7_bulk_order_book_place_bulk_order"></a>

## Function `place_bulk_order`

Places a new maker order in the bulk order book.

If an order already exists for the account, it will be replaced with the new order.
The first price levels of both bid and ask sides will be activated in the active order book.


<a id="@Arguments:_21"></a>

### Arguments:

- <code>self</code>: Mutable reference to the bulk order book
- <code>price_time_idx</code>: Mutable reference to the price time index
- <code>ascending_id_generator</code>: Mutable reference to the ascending id generator
- <code>order_req</code>: The bulk order request to place


<a id="@Aborts:_22"></a>

### Aborts:

- If the order request validation fails


<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_book.md#0x7_bulk_order_book_place_bulk_order">place_bulk_order</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="bulk_order_book.md#0x7_bulk_order_book_BulkOrderBook">bulk_order_book::BulkOrderBook</a>&lt;M&gt;, price_time_idx: &<b>mut</b> <a href="price_time_index.md#0x7_price_time_index_PriceTimeIndex">price_time_index::PriceTimeIndex</a>, ascending_id_generator: &<b>mut</b> <a href="order_book_types.md#0x7_order_book_types_AscendingIdGenerator">order_book_types::AscendingIdGenerator</a>, order_req: <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrderRequest">bulk_order_book_types::BulkOrderRequest</a>&lt;M&gt;): <a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_book.md#0x7_bulk_order_book_place_bulk_order">place_bulk_order</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="bulk_order_book.md#0x7_bulk_order_book_BulkOrderBook">BulkOrderBook</a>&lt;M&gt;,
    price_time_idx: &<b>mut</b> aptos_experimental::price_time_index::PriceTimeIndex,
    ascending_id_generator: &<b>mut</b> AscendingIdGenerator,
    order_req: BulkOrderRequest&lt;M&gt;
) : OrderIdType {
    <b>let</b> <a href="../../aptos-framework/doc/account.md#0x1_account">account</a> = get_account_from_order_request(&order_req);
    <b>let</b> new_sequence_number = aptos_experimental::bulk_order_book_types::get_sequence_number_from_order_request(&order_req);
    <b>let</b> existing_order = self.orders.contains(&<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>);
    <b>let</b> order_id = <b>if</b> (existing_order) {
        <b>let</b> old_order = self.orders.remove(&<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>);
        <b>let</b> existing_sequence_number = aptos_experimental::bulk_order_book_types::get_sequence_number_from_bulk_order(&old_order);
        <b>assert</b>!(new_sequence_number &gt; existing_sequence_number, <a href="bulk_order_book.md#0x7_bulk_order_book_E_INVALID_SEQUENCE_NUMBER">E_INVALID_SEQUENCE_NUMBER</a>);
        <a href="bulk_order_book.md#0x7_bulk_order_book_cancel_active_orders">cancel_active_orders</a>(price_time_idx, &old_order);
        old_order.get_order_id()
    } <b>else</b> {
        <b>let</b> order_id = new_order_id_type(ascending_id_generator.next_ascending_id());
        self.order_id_to_address.add(order_id, <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>);
        order_id
    };
    <b>let</b> new_order = new_bulk_order(
        order_id,
        new_unique_idx_type(ascending_id_generator.next_ascending_id()),
        order_req,
        price_time_idx.best_bid_price(),
        price_time_idx.best_ask_price(),
    );
    self.orders.add(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>, new_order);
    // Activate the first price levels in the active order book
    <a href="bulk_order_book.md#0x7_bulk_order_book_activate_first_price_levels">activate_first_price_levels</a>(price_time_idx, &new_order, order_id);
    order_id
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
