
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
- <code><a href="bulk_order_book.md#0x7_bulk_order_book_BulkOrderRequest">BulkOrderRequest</a></code>: Request structure for placing new orders
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


<a id="@Usage_Example:_7"></a>

### Usage Example:

```move
// Create a new bulk order book
let order_book = bulk_order_book::new_bulk_order_book();

// Create a bulk order request with multiple price levels
let bid_prices = vector[100, 99, 98];
let bid_sizes = vector[10, 20, 30];
let ask_prices = vector[101, 102, 103];
let ask_sizes = vector[15, 25, 35];

let order_request = bulk_order_book::new_bulk_order_request(
@trader,
bid_prices,
bid_sizes,
ask_prices,
ask_sizes
);

// Place the maker order
order_book.place_maker_order(order_request);

// Check if a taker order would match
if (order_book.is_taker_order(101, true)) {
// Get the match
let match_result = order_book.get_single_match_for_taker(101, 10, true);
// Process the match...
}

// Cancel the order
order_book.cancel_order(@trader);
```


-  [Bulk Order Book Module](#@Bulk_Order_Book_Module_0)
    -  [Key Features:](#@Key_Features:_1)
        -  [1. Multi-Level Orders](#@1._Multi-Level_Orders_2)
        -  [2. Order Matching](#@2._Order_Matching_3)
        -  [3. Order Management](#@3._Order_Management_4)
    -  [Data Structures:](#@Data_Structures:_5)
    -  [Error Codes:](#@Error_Codes:_6)
    -  [Usage Example:](#@Usage_Example:_7)
-  [Enum `BulkOrderRequest`](#0x7_bulk_order_book_BulkOrderRequest)
    -  [Fields:](#@Fields:_8)
    -  [Validation:](#@Validation:_9)
-  [Enum `BulkOrderBook`](#0x7_bulk_order_book_BulkOrderBook)
    -  [Fields:](#@Fields:_10)
-  [Constants](#@Constants_11)
-  [Function `new_bulk_order_book`](#0x7_bulk_order_book_new_bulk_order_book)
    -  [Returns:](#@Returns:_12)
-  [Function `new_bulk_order_request`](#0x7_bulk_order_book_new_bulk_order_request)
    -  [Arguments:](#@Arguments:_13)
    -  [Returns:](#@Returns:_14)
    -  [Aborts:](#@Aborts:_15)
-  [Function `get_single_match_for_taker`](#0x7_bulk_order_book_get_single_match_for_taker)
    -  [Arguments:](#@Arguments:_16)
    -  [Returns:](#@Returns:_17)
    -  [Side Effects:](#@Side_Effects:_18)
-  [Function `validate_price_ordering`](#0x7_bulk_order_book_validate_price_ordering)
    -  [Arguments:](#@Arguments:_19)
    -  [Aborts:](#@Aborts:_20)
-  [Function `validate_not_zero_sizes`](#0x7_bulk_order_book_validate_not_zero_sizes)
    -  [Arguments:](#@Arguments:_21)
    -  [Aborts:](#@Aborts:_22)
-  [Function `validate_no_price_crossing`](#0x7_bulk_order_book_validate_no_price_crossing)
    -  [Arguments:](#@Arguments:_23)
    -  [Aborts:](#@Aborts:_24)
-  [Function `validate_mm_order_request`](#0x7_bulk_order_book_validate_mm_order_request)
    -  [Arguments:](#@Arguments:_25)
    -  [Aborts:](#@Aborts:_26)
-  [Function `cancel_active_order_for_side`](#0x7_bulk_order_book_cancel_active_order_for_side)
    -  [Arguments:](#@Arguments:_27)
-  [Function `cancel_active_orders`](#0x7_bulk_order_book_cancel_active_orders)
    -  [Arguments:](#@Arguments:_28)
-  [Function `activate_first_price_level_for_side`](#0x7_bulk_order_book_activate_first_price_level_for_side)
    -  [Arguments:](#@Arguments:_29)
-  [Function `activate_first_price_levels`](#0x7_bulk_order_book_activate_first_price_levels)
    -  [Arguments:](#@Arguments:_30)
-  [Function `reinsert_order`](#0x7_bulk_order_book_reinsert_order)
    -  [Arguments:](#@Arguments:_31)
    -  [Aborts:](#@Aborts:_32)
-  [Function `cancel_bulk_order`](#0x7_bulk_order_book_cancel_bulk_order)
    -  [Arguments:](#@Arguments:_33)
    -  [Aborts:](#@Aborts:_34)
-  [Function `get_remaining_size`](#0x7_bulk_order_book_get_remaining_size)
-  [Function `place_bulk_order`](#0x7_bulk_order_book_place_bulk_order)
    -  [Arguments:](#@Arguments:_35)
    -  [Aborts:](#@Aborts:_36)


<pre><code><b>use</b> <a href="../../aptos-framework/doc/big_ordered_map.md#0x1_big_ordered_map">0x1::big_ordered_map</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types">0x7::bulk_order_book_types</a>;
<b>use</b> <a href="order_book_types.md#0x7_order_book_types">0x7::order_book_types</a>;
<b>use</b> <a href="price_time_index.md#0x7_price_time_index">0x7::price_time_index</a>;
</code></pre>



<a id="0x7_bulk_order_book_BulkOrderRequest"></a>

## Enum `BulkOrderRequest`

Request structure for placing a new bulk order with multiple price levels.


<a id="@Fields:_8"></a>

### Fields:

- <code><a href="../../aptos-framework/doc/account.md#0x1_account">account</a></code>: The account placing the order
- <code>bid_prices</code>: Vector of bid prices in descending order (best price first)
- <code>bid_sizes</code>: Vector of bid sizes corresponding to each price level
- <code>ask_prices</code>: Vector of ask prices in ascending order (best price first)
- <code>ask_sizes</code>: Vector of ask sizes corresponding to each price level


<a id="@Validation:_9"></a>

### Validation:

- Bid prices must be in descending order
- Ask prices must be in ascending order
- All sizes must be greater than 0
- Price and size vectors must have matching lengths


<pre><code>enum <a href="bulk_order_book.md#0x7_bulk_order_book_BulkOrderRequest">BulkOrderRequest</a> <b>has</b> <b>copy</b>, drop
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
<code>bid_prices: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>bid_sizes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>ask_prices: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>ask_sizes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

</details>

<a id="0x7_bulk_order_book_BulkOrderBook"></a>

## Enum `BulkOrderBook`

Main bulk order book container that manages all orders and their matching.


<a id="@Fields:_10"></a>

### Fields:

- <code>orders</code>: Map of account addresses to their bulk orders
- <code>order_id_to_address</code>: Map of order IDs to account addresses for lookup


<pre><code>enum <a href="bulk_order_book.md#0x7_bulk_order_book_BulkOrderBook">BulkOrderBook</a> <b>has</b> store
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>V1</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>orders: <a href="../../aptos-framework/doc/big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">big_ordered_map::BigOrderedMap</a>&lt;<b>address</b>, <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrder">bulk_order_book_types::BulkOrder</a>&gt;</code>
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

<a id="@Constants_11"></a>

## Constants


<a id="0x7_bulk_order_book_EORDER_ALREADY_EXISTS"></a>



<pre><code><b>const</b> <a href="bulk_order_book.md#0x7_bulk_order_book_EORDER_ALREADY_EXISTS">EORDER_ALREADY_EXISTS</a>: u64 = 1;
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



<a id="0x7_bulk_order_book_EINVLID_MM_ORDER_REQUEST"></a>



<pre><code><b>const</b> <a href="bulk_order_book.md#0x7_bulk_order_book_EINVLID_MM_ORDER_REQUEST">EINVLID_MM_ORDER_REQUEST</a>: u64 = 10;
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



<a id="0x7_bulk_order_book_EPRICE_CROSSING"></a>



<pre><code><b>const</b> <a href="bulk_order_book.md#0x7_bulk_order_book_EPRICE_CROSSING">EPRICE_CROSSING</a>: u64 = 11;
</code></pre>



<a id="0x7_bulk_order_book_E_NOT_ACTIVE_ORDER"></a>



<pre><code><b>const</b> <a href="bulk_order_book.md#0x7_bulk_order_book_E_NOT_ACTIVE_ORDER">E_NOT_ACTIVE_ORDER</a>: u64 = 7;
</code></pre>



<a id="0x7_bulk_order_book_new_bulk_order_book"></a>

## Function `new_bulk_order_book`

Creates a new empty bulk order book.


<a id="@Returns:_12"></a>

### Returns:

A new <code><a href="bulk_order_book.md#0x7_bulk_order_book_BulkOrderBook">BulkOrderBook</a></code> instance with empty order collections.


<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_book.md#0x7_bulk_order_book_new_bulk_order_book">new_bulk_order_book</a>(): <a href="bulk_order_book.md#0x7_bulk_order_book_BulkOrderBook">bulk_order_book::BulkOrderBook</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_book.md#0x7_bulk_order_book_new_bulk_order_book">new_bulk_order_book</a>(): <a href="bulk_order_book.md#0x7_bulk_order_book_BulkOrderBook">BulkOrderBook</a> {
    BulkOrderBook::V1 {
        orders:  <a href="order_book_types.md#0x7_order_book_types_new_default_big_ordered_map">order_book_types::new_default_big_ordered_map</a>(),
        order_id_to_address:  <a href="order_book_types.md#0x7_order_book_types_new_default_big_ordered_map">order_book_types::new_default_big_ordered_map</a>()
    }
}
</code></pre>



</details>

<a id="0x7_bulk_order_book_new_bulk_order_request"></a>

## Function `new_bulk_order_request`

Creates a new bulk order request with the specified price levels and sizes.


<a id="@Arguments:_13"></a>

### Arguments:

- <code><a href="../../aptos-framework/doc/account.md#0x1_account">account</a></code>: The account placing the order
- <code>bid_prices</code>: Vector of bid prices in descending order
- <code>bid_sizes</code>: Vector of bid sizes corresponding to each price level
- <code>ask_prices</code>: Vector of ask prices in ascending order
- <code>ask_sizes</code>: Vector of ask sizes corresponding to each price level
- <code>metadata</code>: Additional metadata for the order


<a id="@Returns:_14"></a>

### Returns:

A <code><a href="bulk_order_book.md#0x7_bulk_order_book_BulkOrderRequest">BulkOrderRequest</a></code> instance.


<a id="@Aborts:_15"></a>

### Aborts:

- If bid_prices and bid_sizes have different lengths
- If ask_prices and ask_sizes have different lengths


<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_book.md#0x7_bulk_order_book_new_bulk_order_request">new_bulk_order_request</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>, bid_prices: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, bid_sizes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, ask_prices: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, ask_sizes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;): <a href="bulk_order_book.md#0x7_bulk_order_book_BulkOrderRequest">bulk_order_book::BulkOrderRequest</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_book.md#0x7_bulk_order_book_new_bulk_order_request">new_bulk_order_request</a>(
    <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>,
    bid_prices: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    bid_sizes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    ask_prices: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    ask_sizes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;
): <a href="bulk_order_book.md#0x7_bulk_order_book_BulkOrderRequest">BulkOrderRequest</a> {
    <b>assert</b>!(bid_prices.length() == bid_sizes.length(), <a href="bulk_order_book.md#0x7_bulk_order_book_EINVLID_MM_ORDER_REQUEST">EINVLID_MM_ORDER_REQUEST</a>);
    <b>assert</b>!(ask_prices.length() == ask_sizes.length(), <a href="bulk_order_book.md#0x7_bulk_order_book_EINVLID_MM_ORDER_REQUEST">EINVLID_MM_ORDER_REQUEST</a>);
    <b>return</b> BulkOrderRequest::V1 {
        <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>,
        bid_prices,
        bid_sizes,
        ask_prices,
        ask_sizes
    }
}
</code></pre>



</details>

<a id="0x7_bulk_order_book_get_single_match_for_taker"></a>

## Function `get_single_match_for_taker`

Returns a single match for a taker order.

This function should only be called after verifying that the order is a taker order
using <code>is_taker_order()</code>. If called on a non-taker order, it will abort.


<a id="@Arguments:_16"></a>

### Arguments:

- <code>self</code>: Mutable reference to the bulk order book
- <code>price_time_idx</code>: Mutable reference to the price time index
- <code>price</code>: The price of the taker order
- <code>size</code>: The size of the taker order
- <code>is_bid</code>: True if the taker order is a bid, false if ask


<a id="@Returns:_17"></a>

### Returns:

A <code>SingleBulkOrderMatch</code> containing the match details.


<a id="@Side_Effects:_18"></a>

### Side Effects:

- Updates the matched order's remaining sizes
- Activates the next price level if the current level is fully consumed
- Updates the active order book


<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_book.md#0x7_bulk_order_book_get_single_match_for_taker">get_single_match_for_taker</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="bulk_order_book.md#0x7_bulk_order_book_BulkOrderBook">bulk_order_book::BulkOrderBook</a>, price_time_idx: &<b>mut</b> <a href="price_time_index.md#0x7_price_time_index_PriceTimeIndex">price_time_index::PriceTimeIndex</a>, active_matched_order: <a href="order_book_types.md#0x7_order_book_types_ActiveMatchedOrder">order_book_types::ActiveMatchedOrder</a>, is_bid: bool): <a href="order_book_types.md#0x7_order_book_types_OrderMatch">order_book_types::OrderMatch</a>&lt;M&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_book.md#0x7_bulk_order_book_get_single_match_for_taker">get_single_match_for_taker</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="bulk_order_book.md#0x7_bulk_order_book_BulkOrderBook">BulkOrderBook</a>,
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

<a id="0x7_bulk_order_book_validate_price_ordering"></a>

## Function `validate_price_ordering`

Validates that prices are in the correct order (descending for bids, ascending for asks).


<a id="@Arguments:_19"></a>

### Arguments:

- <code>prices</code>: Vector of prices to validate
- <code>is_descending</code>: True if prices should be in descending order, false for ascending


<a id="@Aborts:_20"></a>

### Aborts:

- If prices are not in the correct order


<pre><code><b>fun</b> <a href="bulk_order_book.md#0x7_bulk_order_book_validate_price_ordering">validate_price_ordering</a>(prices: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, is_descending: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="bulk_order_book.md#0x7_bulk_order_book_validate_price_ordering">validate_price_ordering</a>(
    prices: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    is_descending: bool
) {
    <b>let</b> i = 0;
    <b>if</b> (prices.length() == 0) {
        <b>return</b> ; // No prices <b>to</b> validate
    };
    <b>while</b> (i &lt; prices.length() - 1) {
        <b>if</b> (is_descending) {
            <b>assert</b>!(prices[i] &gt; prices[i + 1], <a href="bulk_order_book.md#0x7_bulk_order_book_EINVLID_MM_ORDER_REQUEST">EINVLID_MM_ORDER_REQUEST</a>);
        } <b>else</b> {
            <b>assert</b>!(prices[i] &lt; prices[i + 1], <a href="bulk_order_book.md#0x7_bulk_order_book_EINVLID_MM_ORDER_REQUEST">EINVLID_MM_ORDER_REQUEST</a>);
        };
        i += 1;
    };
}
</code></pre>



</details>

<a id="0x7_bulk_order_book_validate_not_zero_sizes"></a>

## Function `validate_not_zero_sizes`

Validates that all sizes in the vector are greater than 0.


<a id="@Arguments:_21"></a>

### Arguments:

- <code>sizes</code>: Vector of sizes to validate


<a id="@Aborts:_22"></a>

### Aborts:

- If the vector is empty
- If any size is 0


<pre><code><b>fun</b> <a href="bulk_order_book.md#0x7_bulk_order_book_validate_not_zero_sizes">validate_not_zero_sizes</a>(sizes: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="bulk_order_book.md#0x7_bulk_order_book_validate_not_zero_sizes">validate_not_zero_sizes</a>(
    sizes: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;
) {
    <b>let</b> i = 0;
    <b>while</b> (i &lt; sizes.length()) {
        <b>assert</b>!(sizes[i] &gt; 0, <a href="bulk_order_book.md#0x7_bulk_order_book_EINVLID_MM_ORDER_REQUEST">EINVLID_MM_ORDER_REQUEST</a>);
        i += 1;
    };
}
</code></pre>



</details>

<a id="0x7_bulk_order_book_validate_no_price_crossing"></a>

## Function `validate_no_price_crossing`

Validates that bid and ask prices don't cross.

This ensures that the highest bid price is lower than the lowest ask price,
preventing self-matching within a single order.


<a id="@Arguments:_23"></a>

### Arguments:

- <code>bid_prices</code>: Vector of bid prices (should be in descending order)
- <code>ask_prices</code>: Vector of ask prices (should be in ascending order)


<a id="@Aborts:_24"></a>

### Aborts:

- If the highest bid price is greater than or equal to the lowest ask price


<pre><code><b>fun</b> <a href="bulk_order_book.md#0x7_bulk_order_book_validate_no_price_crossing">validate_no_price_crossing</a>(bid_prices: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, ask_prices: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="bulk_order_book.md#0x7_bulk_order_book_validate_no_price_crossing">validate_no_price_crossing</a>(
    bid_prices: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    ask_prices: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;
) {
    <b>if</b> (bid_prices.length() &gt; 0 && ask_prices.length() &gt; 0) {
        <b>let</b> highest_bid = bid_prices[0]; // First element is highest (descending order)
        <b>let</b> lowest_ask = ask_prices[0];  // First element is lowest (ascending order)
        <b>assert</b>!(highest_bid &lt; lowest_ask, <a href="bulk_order_book.md#0x7_bulk_order_book_EPRICE_CROSSING">EPRICE_CROSSING</a>);
    };
}
</code></pre>



</details>

<a id="0x7_bulk_order_book_validate_mm_order_request"></a>

## Function `validate_mm_order_request`

Validates a bulk order request for correctness.


<a id="@Arguments:_25"></a>

### Arguments:

- <code>order_req</code>: The bulk order request to validate


<a id="@Aborts:_26"></a>

### Aborts:

- If any validation fails (price ordering, sizes, vector lengths, price crossing)


<pre><code><b>fun</b> <a href="bulk_order_book.md#0x7_bulk_order_book_validate_mm_order_request">validate_mm_order_request</a>(order_req: &<a href="bulk_order_book.md#0x7_bulk_order_book_BulkOrderRequest">bulk_order_book::BulkOrderRequest</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="bulk_order_book.md#0x7_bulk_order_book_validate_mm_order_request">validate_mm_order_request</a>(
    order_req: &<a href="bulk_order_book.md#0x7_bulk_order_book_BulkOrderRequest">BulkOrderRequest</a>,
) {
    // Ensure bid prices are in descending order and ask prices are in ascending order
    <b>assert</b>!(order_req.bid_sizes.length() &gt; 0 || order_req.ask_sizes.length() &gt; 0, <a href="bulk_order_book.md#0x7_bulk_order_book_EINVLID_MM_ORDER_REQUEST">EINVLID_MM_ORDER_REQUEST</a>);
    <a href="bulk_order_book.md#0x7_bulk_order_book_validate_not_zero_sizes">validate_not_zero_sizes</a>(&order_req.bid_sizes);
    <a href="bulk_order_book.md#0x7_bulk_order_book_validate_not_zero_sizes">validate_not_zero_sizes</a>(&order_req.ask_sizes);
    <b>assert</b>!(order_req.bid_prices.length() == order_req.bid_sizes.length(), <a href="bulk_order_book.md#0x7_bulk_order_book_EINVLID_MM_ORDER_REQUEST">EINVLID_MM_ORDER_REQUEST</a>);
    <b>assert</b>!(order_req.ask_prices.length() == order_req.ask_sizes.length(), <a href="bulk_order_book.md#0x7_bulk_order_book_EINVLID_MM_ORDER_REQUEST">EINVLID_MM_ORDER_REQUEST</a>);
    <a href="bulk_order_book.md#0x7_bulk_order_book_validate_price_ordering">validate_price_ordering</a>(&order_req.bid_prices, <b>true</b>);  // descending
    <a href="bulk_order_book.md#0x7_bulk_order_book_validate_price_ordering">validate_price_ordering</a>(&order_req.ask_prices, <b>false</b>); // ascending
    <a href="bulk_order_book.md#0x7_bulk_order_book_validate_no_price_crossing">validate_no_price_crossing</a>(&order_req.bid_prices, &order_req.ask_prices);
}
</code></pre>



</details>

<a id="0x7_bulk_order_book_cancel_active_order_for_side"></a>

## Function `cancel_active_order_for_side`

Cancels active orders for a specific side (bid or ask) of a bulk order.


<a id="@Arguments:_27"></a>

### Arguments:

- <code>active_orders</code>: Reference to the active order book
- <code>order</code>: The bulk order to cancel active orders for
- <code>is_bid</code>: True to cancel bid orders, false for ask orders


<pre><code><b>fun</b> <a href="bulk_order_book.md#0x7_bulk_order_book_cancel_active_order_for_side">cancel_active_order_for_side</a>(price_time_idx: &<b>mut</b> <a href="price_time_index.md#0x7_price_time_index_PriceTimeIndex">price_time_index::PriceTimeIndex</a>, order: &<a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrder">bulk_order_book_types::BulkOrder</a>, is_bid: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="bulk_order_book.md#0x7_bulk_order_book_cancel_active_order_for_side">cancel_active_order_for_side</a>(
    price_time_idx: &<b>mut</b> aptos_experimental::price_time_index::PriceTimeIndex,
    order: &BulkOrder,
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


<a id="@Arguments:_28"></a>

### Arguments:

- <code>active_orders</code>: Reference to the active order book
- <code>order</code>: The bulk order to cancel active orders for


<pre><code><b>fun</b> <a href="bulk_order_book.md#0x7_bulk_order_book_cancel_active_orders">cancel_active_orders</a>(price_time_idx: &<b>mut</b> <a href="price_time_index.md#0x7_price_time_index_PriceTimeIndex">price_time_index::PriceTimeIndex</a>, order: &<a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrder">bulk_order_book_types::BulkOrder</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="bulk_order_book.md#0x7_bulk_order_book_cancel_active_orders">cancel_active_orders</a>(
    price_time_idx: &<b>mut</b> aptos_experimental::price_time_index::PriceTimeIndex, order: &BulkOrder
) {
    <a href="bulk_order_book.md#0x7_bulk_order_book_cancel_active_order_for_side">cancel_active_order_for_side</a>(price_time_idx, order, <b>true</b>);  // cancel bid
    <a href="bulk_order_book.md#0x7_bulk_order_book_cancel_active_order_for_side">cancel_active_order_for_side</a>(price_time_idx, order, <b>false</b>); // cancel ask
}
</code></pre>



</details>

<a id="0x7_bulk_order_book_activate_first_price_level_for_side"></a>

## Function `activate_first_price_level_for_side`

Activates the first price level for a specific side of a bulk order.


<a id="@Arguments:_29"></a>

### Arguments:

- <code>active_orders</code>: Reference to the active order book
- <code>order</code>: The bulk order to activate levels for
- <code>order_id</code>: The order ID for the bulk order
- <code>is_bid</code>: True to activate bid levels, false for ask levels


<pre><code><b>fun</b> <a href="bulk_order_book.md#0x7_bulk_order_book_activate_first_price_level_for_side">activate_first_price_level_for_side</a>(price_time_idx: &<b>mut</b> <a href="price_time_index.md#0x7_price_time_index_PriceTimeIndex">price_time_index::PriceTimeIndex</a>, order: &<a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrder">bulk_order_book_types::BulkOrder</a>, order_id: <a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>, is_bid: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="bulk_order_book.md#0x7_bulk_order_book_activate_first_price_level_for_side">activate_first_price_level_for_side</a>(
    price_time_idx: &<b>mut</b> aptos_experimental::price_time_index::PriceTimeIndex,
    order: &BulkOrder,
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


<a id="@Arguments:_30"></a>

### Arguments:

- <code>active_orders</code>: Reference to the active order book
- <code>order</code>: The bulk order to activate levels for
- <code>order_id</code>: The order ID for the bulk order


<pre><code><b>fun</b> <a href="bulk_order_book.md#0x7_bulk_order_book_activate_first_price_levels">activate_first_price_levels</a>(price_time_idx: &<b>mut</b> <a href="price_time_index.md#0x7_price_time_index_PriceTimeIndex">price_time_index::PriceTimeIndex</a>, order: &<a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrder">bulk_order_book_types::BulkOrder</a>, order_id: <a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="bulk_order_book.md#0x7_bulk_order_book_activate_first_price_levels">activate_first_price_levels</a>(
    price_time_idx: &<b>mut</b> aptos_experimental::price_time_index::PriceTimeIndex, order: &BulkOrder, order_id: OrderIdType
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


<a id="@Arguments:_31"></a>

### Arguments:

- <code>self</code>: Mutable reference to the bulk order book
- <code>price_time_idx</code>: Mutable reference to the price time index
- <code>reinsert_order</code>: The order result to reinsert
- <code>original_order</code>: The original order result for validation


<a id="@Aborts:_32"></a>

### Aborts:

- If the order account doesn't exist in the order book
- If the reinsertion validation fails


<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_book.md#0x7_bulk_order_book_reinsert_order">reinsert_order</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="bulk_order_book.md#0x7_bulk_order_book_BulkOrderBook">bulk_order_book::BulkOrderBook</a>, price_time_idx: &<b>mut</b> <a href="price_time_index.md#0x7_price_time_index_PriceTimeIndex">price_time_index::PriceTimeIndex</a>, reinsert_order: <a href="order_book_types.md#0x7_order_book_types_OrderMatchDetails">order_book_types::OrderMatchDetails</a>&lt;M&gt;, original_order: &<a href="order_book_types.md#0x7_order_book_types_OrderMatchDetails">order_book_types::OrderMatchDetails</a>&lt;M&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_book.md#0x7_bulk_order_book_reinsert_order">reinsert_order</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="bulk_order_book.md#0x7_bulk_order_book_BulkOrderBook">BulkOrderBook</a>,
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


<a id="@Arguments:_33"></a>

### Arguments:

- <code>self</code>: Mutable reference to the bulk order book
- <code>price_time_idx</code>: Mutable reference to the price time index
- <code><a href="../../aptos-framework/doc/account.md#0x1_account">account</a></code>: The account whose order should be cancelled


<a id="@Aborts:_34"></a>

### Aborts:

- If no order exists for the specified account


<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_book.md#0x7_bulk_order_book_cancel_bulk_order">cancel_bulk_order</a>(self: &<b>mut</b> <a href="bulk_order_book.md#0x7_bulk_order_book_BulkOrderBook">bulk_order_book::BulkOrderBook</a>, price_time_idx: &<b>mut</b> <a href="price_time_index.md#0x7_price_time_index_PriceTimeIndex">price_time_index::PriceTimeIndex</a>, <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>): (<a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>, u64, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_book.md#0x7_bulk_order_book_cancel_bulk_order">cancel_bulk_order</a>(
    self: &<b>mut</b> <a href="bulk_order_book.md#0x7_bulk_order_book_BulkOrderBook">BulkOrderBook</a>,
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



<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_book.md#0x7_bulk_order_book_get_remaining_size">get_remaining_size</a>(self: &<a href="bulk_order_book.md#0x7_bulk_order_book_BulkOrderBook">bulk_order_book::BulkOrderBook</a>, <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>, is_bid: bool): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_book.md#0x7_bulk_order_book_get_remaining_size">get_remaining_size</a>(
    self: &<a href="bulk_order_book.md#0x7_bulk_order_book_BulkOrderBook">BulkOrderBook</a>,
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

<a id="0x7_bulk_order_book_place_bulk_order"></a>

## Function `place_bulk_order`

Places a new maker order in the bulk order book.

If an order already exists for the account, it will be replaced with the new order.
The first price levels of both bid and ask sides will be activated in the active order book.


<a id="@Arguments:_35"></a>

### Arguments:

- <code>self</code>: Mutable reference to the bulk order book
- <code>price_time_idx</code>: Mutable reference to the price time index
- <code>ascending_id_generator</code>: Mutable reference to the ascending id generator
- <code>order_req</code>: The bulk order request to place


<a id="@Aborts:_36"></a>

### Aborts:

- If the order request validation fails


<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_book.md#0x7_bulk_order_book_place_bulk_order">place_bulk_order</a>(self: &<b>mut</b> <a href="bulk_order_book.md#0x7_bulk_order_book_BulkOrderBook">bulk_order_book::BulkOrderBook</a>, price_time_idx: &<b>mut</b> <a href="price_time_index.md#0x7_price_time_index_PriceTimeIndex">price_time_index::PriceTimeIndex</a>, ascending_id_generator: &<b>mut</b> <a href="order_book_types.md#0x7_order_book_types_AscendingIdGenerator">order_book_types::AscendingIdGenerator</a>, order_req: <a href="bulk_order_book.md#0x7_bulk_order_book_BulkOrderRequest">bulk_order_book::BulkOrderRequest</a>): <a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_book.md#0x7_bulk_order_book_place_bulk_order">place_bulk_order</a>(
    self: &<b>mut</b> <a href="bulk_order_book.md#0x7_bulk_order_book_BulkOrderBook">BulkOrderBook</a>,
    price_time_idx: &<b>mut</b> aptos_experimental::price_time_index::PriceTimeIndex,
    ascending_id_generator: &<b>mut</b> AscendingIdGenerator,
    order_req: <a href="bulk_order_book.md#0x7_bulk_order_book_BulkOrderRequest">BulkOrderRequest</a>
) : OrderIdType {
    <a href="bulk_order_book.md#0x7_bulk_order_book_validate_mm_order_request">validate_mm_order_request</a>(&order_req);
    <b>let</b> existing_order = self.orders.contains(&order_req.<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>);
    <b>let</b> order_id = <b>if</b> (existing_order) {
        <b>let</b> old_order = self.orders.remove(&order_req.<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>);
        <a href="bulk_order_book.md#0x7_bulk_order_book_cancel_active_orders">cancel_active_orders</a>(price_time_idx, &old_order);
        old_order.get_order_id()
    } <b>else</b> {
        <b>let</b> order_id = new_order_id_type(ascending_id_generator.next_ascending_id());
        self.order_id_to_address.add(order_id, order_req.<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>);
        order_id
    };
    <b>let</b> BulkOrderRequest::V1 { <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>, bid_prices, bid_sizes, ask_prices, ask_sizes } = order_req;
    <b>let</b> new_order = new_bulk_order(
        order_id,
        <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>,
        new_unique_idx_type(ascending_id_generator.next_ascending_id()),
        bid_prices,
        bid_sizes,
        ask_prices,
        ask_sizes
    );
    self.orders.add(order_req.<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>, new_order);
    // Activate the first price levels in the active order book
    <a href="bulk_order_book.md#0x7_bulk_order_book_activate_first_price_levels">activate_first_price_levels</a>(price_time_idx, &new_order, order_id);
    order_id
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
