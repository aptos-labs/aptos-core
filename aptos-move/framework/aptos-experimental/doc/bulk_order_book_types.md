
<a id="0x7_bulk_order_book_types"></a>

# Module `0x7::bulk_order_book_types`


<a id="@Bulk_Order_Book_Types_Module_0"></a>

## Bulk Order Book Types Module


This module defines the core data structures and types used by the bulk order book system.
It provides the foundational types for representing multi-level orders and their management.


<a id="@Key_Data_Structures:_1"></a>

### Key Data Structures:



<a id="@1._BulkOrder_2"></a>

#### 1. BulkOrder

Represents a multi-level order with both bid and ask sides. Each side can have multiple
price levels with associated sizes.


<a id="@Core_Functionality:_3"></a>

### Core Functionality:


- **Order Creation**: Functions to create new bulk orders
- **Order Matching**: Logic for matching orders and updating remaining quantities
- **Order Reinsertion**: Support for reinserting matched portions back into the order book
- **Order Management**: Helper functions for order state management and cleanup


<a id="@Error_Codes:_4"></a>

### Error Codes:

- <code><a href="bulk_order_book_types.md#0x7_bulk_order_book_types_EUNEXPECTED_MATCH_PRICE">EUNEXPECTED_MATCH_PRICE</a></code>: Unexpected price during order matching
- <code><a href="bulk_order_book_types.md#0x7_bulk_order_book_types_EUNEXPECTED_MATCH_SIZE">EUNEXPECTED_MATCH_SIZE</a></code>: Unexpected size during order matching
- <code><a href="bulk_order_book_types.md#0x7_bulk_order_book_types_E_REINSERT_ORDER_MISMATCH">E_REINSERT_ORDER_MISMATCH</a></code>: Order mismatch during reinsertion validation


<a id="@Usage_Example:_5"></a>

### Usage Example:

```move
// Create a new bulk order
let order = bulk_order_book_types::new_bulk_order(
order_id,
@trader,
unique_priority_idx,
bid_prices,
bid_sizes,
ask_prices,
ask_sizes
);
```
(work in progress)


-  [Bulk Order Book Types Module](#@Bulk_Order_Book_Types_Module_0)
    -  [Key Data Structures:](#@Key_Data_Structures:_1)
        -  [1. BulkOrder](#@1._BulkOrder_2)
    -  [Core Functionality:](#@Core_Functionality:_3)
    -  [Error Codes:](#@Error_Codes:_4)
    -  [Usage Example:](#@Usage_Example:_5)
-  [Enum `BulkOrder`](#0x7_bulk_order_book_types_BulkOrder)
    -  [Fields:](#@Fields:_6)
-  [Constants](#@Constants_7)
-  [Function `new_bulk_order`](#0x7_bulk_order_book_types_new_bulk_order)
    -  [Arguments:](#@Arguments:_8)
    -  [Returns:](#@Returns:_9)
-  [Function `new_bulk_order_match`](#0x7_bulk_order_book_types_new_bulk_order_match)
    -  [Arguments:](#@Arguments:_10)
    -  [Returns:](#@Returns:_11)
-  [Function `is_remaining_order`](#0x7_bulk_order_book_types_is_remaining_order)
    -  [Arguments:](#@Arguments:_12)
    -  [Returns:](#@Returns:_13)
-  [Function `get_total_remaining_size`](#0x7_bulk_order_book_types_get_total_remaining_size)
-  [Function `get_unique_priority_idx`](#0x7_bulk_order_book_types_get_unique_priority_idx)
    -  [Arguments:](#@Arguments:_14)
    -  [Returns:](#@Returns:_15)
-  [Function `get_order_id`](#0x7_bulk_order_book_types_get_order_id)
    -  [Arguments:](#@Arguments:_16)
    -  [Returns:](#@Returns:_17)
-  [Function `get_account`](#0x7_bulk_order_book_types_get_account)
    -  [Arguments:](#@Arguments:_18)
    -  [Returns:](#@Returns:_19)
-  [Function `get_active_price`](#0x7_bulk_order_book_types_get_active_price)
    -  [Arguments:](#@Arguments:_20)
    -  [Returns:](#@Returns:_21)
-  [Function `get_active_size`](#0x7_bulk_order_book_types_get_active_size)
    -  [Arguments:](#@Arguments:_22)
    -  [Returns:](#@Returns:_23)
-  [Function `reinsert_order`](#0x7_bulk_order_book_types_reinsert_order)
    -  [Arguments:](#@Arguments:_24)
-  [Function `match_order_and_get_next`](#0x7_bulk_order_book_types_match_order_and_get_next)
    -  [Arguments:](#@Arguments:_25)
    -  [Returns:](#@Returns:_26)
    -  [Aborts:](#@Aborts:_27)
-  [Function `set_empty`](#0x7_bulk_order_book_types_set_empty)
    -  [Arguments:](#@Arguments:_28)


<pre><code><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
<b>use</b> <a href="order_book_types.md#0x7_order_book_types">0x7::order_book_types</a>;
</code></pre>



<a id="0x7_bulk_order_book_types_BulkOrder"></a>

## Enum `BulkOrder`

Represents a multi-level order with both bid and ask sides.

Each side can have multiple price levels with associated sizes. The order maintains
both original and remaining sizes for tracking purposes.


<a id="@Fields:_6"></a>

### Fields:

- <code>order_id</code>: Unique identifier for the order
- <code><a href="../../aptos-framework/doc/account.md#0x1_account">account</a></code>: Account that placed the order
- <code>unique_priority_idx</code>: Priority index for time-based ordering
- <code>orig_bid_size</code>: Original total size of all bid levels
- <code>orig_ask_size</code>: Original total size of all ask levels
- <code>total_remaining_bid_size</code>: Current remaining size of all bid levels
- <code>total_remaining_ask_size</code>: Current remaining size of all ask levels
- <code>bid_prices</code>: Vector of bid prices in descending order
- <code>bid_sizes</code>: Vector of bid sizes corresponding to each price level
- <code>ask_prices</code>: Vector of ask prices in ascending order
- <code>ask_sizes</code>: Vector of ask sizes corresponding to each price level


<pre><code>enum <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrder">BulkOrder</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>V1</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>order_id: <a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a></code>
</dt>
<dd>

</dd>
<dt>
<code><a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>unique_priority_idx: <a href="order_book_types.md#0x7_order_book_types_UniqueIdxType">order_book_types::UniqueIdxType</a></code>
</dt>
<dd>

</dd>
<dt>
<code>orig_bid_size: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>orig_ask_size: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>total_remaining_bid_size: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>total_remaining_ask_size: u64</code>
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

<a id="@Constants_7"></a>

## Constants


<a id="0x7_bulk_order_book_types_EUNEXPECTED_MATCH_PRICE"></a>



<pre><code><b>const</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_EUNEXPECTED_MATCH_PRICE">EUNEXPECTED_MATCH_PRICE</a>: u64 = 1;
</code></pre>



<a id="0x7_bulk_order_book_types_EUNEXPECTED_MATCH_SIZE"></a>



<pre><code><b>const</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_EUNEXPECTED_MATCH_SIZE">EUNEXPECTED_MATCH_SIZE</a>: u64 = 2;
</code></pre>



<a id="0x7_bulk_order_book_types_E_REINSERT_ORDER_MISMATCH"></a>



<pre><code><b>const</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_E_REINSERT_ORDER_MISMATCH">E_REINSERT_ORDER_MISMATCH</a>: u64 = 3;
</code></pre>



<a id="0x7_bulk_order_book_types_new_bulk_order"></a>

## Function `new_bulk_order`

Creates a new bulk order with the specified parameters.


<a id="@Arguments:_8"></a>

### Arguments:

- <code>order_id</code>: Unique identifier for the order
- <code><a href="../../aptos-framework/doc/account.md#0x1_account">account</a></code>: Account placing the order
- <code>unique_priority_idx</code>: Priority index for time-based ordering
- <code>bid_prices</code>: Vector of bid prices in descending order
- <code>bid_sizes</code>: Vector of bid sizes corresponding to each price level
- <code>ask_prices</code>: Vector of ask prices in ascending order
- <code>ask_sizes</code>: Vector of ask sizes corresponding to each price level


<a id="@Returns:_9"></a>

### Returns:

A new <code><a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrder">BulkOrder</a></code> instance with calculated original and remaining sizes.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_new_bulk_order">new_bulk_order</a>(order_id: <a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>, <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>, unique_priority_idx: <a href="order_book_types.md#0x7_order_book_types_UniqueIdxType">order_book_types::UniqueIdxType</a>, bid_prices: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, bid_sizes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, ask_prices: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, ask_sizes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;): <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrder">bulk_order_book_types::BulkOrder</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_new_bulk_order">new_bulk_order</a>(
    order_id: OrderIdType,
    <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>,
    unique_priority_idx: UniqueIdxType,
    bid_prices: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    bid_sizes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    ask_prices: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    ask_sizes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;
): <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrder">BulkOrder</a> {
    // Original bid and ask sizes are the sum of the sizes at each price level
    <b>let</b> orig_bid_size = bid_sizes.fold(0, |acc, size| acc + size);
    <b>let</b> orig_ask_size = ask_sizes.fold(0, |acc, size| acc + size);
    BulkOrder::V1 {
        order_id,
        <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>,
        unique_priority_idx,
        orig_bid_size,
        orig_ask_size,
        total_remaining_bid_size: orig_bid_size, // Initially, the remaining size is the original size
        total_remaining_ask_size: orig_ask_size, // Initially, the remaining size is the original size
        bid_prices,
        bid_sizes,
        ask_prices,
        ask_sizes
    }
}
</code></pre>



</details>

<a id="0x7_bulk_order_book_types_new_bulk_order_match"></a>

## Function `new_bulk_order_match`

Creates a new single bulk order match result.


<a id="@Arguments:_10"></a>

### Arguments:

- <code>order</code>: Reference to the bulk order being matched
- <code>is_bid</code>: True if matching against bid side, false for ask side
- <code>matched_size</code>: Size that was matched in this operation


<a id="@Returns:_11"></a>

### Returns:

A <code>SingleBulkOrderMatch</code> containing the match details.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_new_bulk_order_match">new_bulk_order_match</a>&lt;M: <b>copy</b>, drop, store&gt;(order: &<b>mut</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrder">bulk_order_book_types::BulkOrder</a>, is_bid: bool, matched_size: u64): <a href="order_book_types.md#0x7_order_book_types_OrderMatch">order_book_types::OrderMatch</a>&lt;M&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_new_bulk_order_match">new_bulk_order_match</a>&lt;M: store + <b>copy</b> + drop&gt;(
    order: &<b>mut</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrder">BulkOrder</a>,
    is_bid: bool,
    matched_size: u64
): OrderMatch&lt;M&gt; {
    // print( &order.total_remaining_bid_size);
    <b>let</b> (price, orig_size, remaining_size) = <b>if</b> (is_bid) {
        (order.bid_prices[0], order.orig_bid_size, order.total_remaining_bid_size - matched_size)
    } <b>else</b> {
        (order.ask_prices[0], order.orig_ask_size, order.total_remaining_ask_size - matched_size)
    };
    new_order_match&lt;M&gt;(
        new_order_match_details&lt;M&gt;(
            order.<a href="bulk_order_book_types.md#0x7_bulk_order_book_types_get_order_id">get_order_id</a>(),
            order.<a href="bulk_order_book_types.md#0x7_bulk_order_book_types_get_account">get_account</a>(),
            <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(),
            order.<a href="bulk_order_book_types.md#0x7_bulk_order_book_types_get_unique_priority_idx">get_unique_priority_idx</a>(),
            price,
            orig_size,
            remaining_size,
            is_bid,
            good_till_cancelled(),
            <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(),
            bulk_order_book_type(),
        ),
        matched_size
    )
}
</code></pre>



</details>

<a id="0x7_bulk_order_book_types_is_remaining_order"></a>

## Function `is_remaining_order`

Checks if a bulk order has remaining orders on the specified side.


<a id="@Arguments:_12"></a>

### Arguments:

- <code>self</code>: Reference to the bulk order
- <code>is_bid</code>: True to check bid side, false for ask side


<a id="@Returns:_13"></a>

### Returns:

True if there are remaining orders on the specified side, false otherwise.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_is_remaining_order">is_remaining_order</a>(self: &<a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrder">bulk_order_book_types::BulkOrder</a>, is_bid: bool): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_is_remaining_order">is_remaining_order</a>(
    self: &<a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrder">BulkOrder</a>,
    is_bid: bool,
): bool {
    <b>let</b> sizes = <b>if</b> (is_bid) { self.bid_sizes } <b>else</b> { self.ask_sizes };
    <b>return</b> sizes.length() &gt; 0 && sizes[0] &gt; 0 // Check <b>if</b> the first price level <b>has</b> a non-zero size
}
</code></pre>



</details>

<a id="0x7_bulk_order_book_types_get_total_remaining_size"></a>

## Function `get_total_remaining_size`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_get_total_remaining_size">get_total_remaining_size</a>(self: &<a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrder">bulk_order_book_types::BulkOrder</a>, is_bid: bool): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>)  <b>fun</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_get_total_remaining_size">get_total_remaining_size</a>(
    self: &<a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrder">BulkOrder</a>,
    is_bid: bool,
): u64 {
    <b>if</b> (is_bid) {
        self.total_remaining_bid_size
    } <b>else</b> {
        self.total_remaining_ask_size
    }
}
</code></pre>



</details>

<a id="0x7_bulk_order_book_types_get_unique_priority_idx"></a>

## Function `get_unique_priority_idx`

Gets the unique priority index of a bulk order.


<a id="@Arguments:_14"></a>

### Arguments:

- <code>self</code>: Reference to the bulk order


<a id="@Returns:_15"></a>

### Returns:

The unique priority index for time-based ordering.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_get_unique_priority_idx">get_unique_priority_idx</a>(self: &<a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrder">bulk_order_book_types::BulkOrder</a>): <a href="order_book_types.md#0x7_order_book_types_UniqueIdxType">order_book_types::UniqueIdxType</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_get_unique_priority_idx">get_unique_priority_idx</a>(
    self: &<a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrder">BulkOrder</a>,
): UniqueIdxType {
    self.unique_priority_idx
}
</code></pre>



</details>

<a id="0x7_bulk_order_book_types_get_order_id"></a>

## Function `get_order_id`

Gets the order ID of a bulk order.


<a id="@Arguments:_16"></a>

### Arguments:

- <code>self</code>: Reference to the bulk order


<a id="@Returns:_17"></a>

### Returns:

The unique order identifier.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_get_order_id">get_order_id</a>(self: &<a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrder">bulk_order_book_types::BulkOrder</a>): <a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_get_order_id">get_order_id</a>(
    self: &<a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrder">BulkOrder</a>,
): OrderIdType {
    self.order_id
}
</code></pre>



</details>

<a id="0x7_bulk_order_book_types_get_account"></a>

## Function `get_account`

Gets the account of a bulk order.


<a id="@Arguments:_18"></a>

### Arguments:

- <code>self</code>: Reference to the bulk order


<a id="@Returns:_19"></a>

### Returns:

The account that placed the order.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_get_account">get_account</a>(self: &<a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrder">bulk_order_book_types::BulkOrder</a>): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_get_account">get_account</a>(
    self: &<a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrder">BulkOrder</a>,
): <b>address</b> {
    self.<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>
}
</code></pre>



</details>

<a id="0x7_bulk_order_book_types_get_active_price"></a>

## Function `get_active_price`

Gets the active price for a specific side of a bulk order.


<a id="@Arguments:_20"></a>

### Arguments:

- <code>self</code>: Reference to the bulk order
- <code>is_bid</code>: True to get bid price, false for ask price


<a id="@Returns:_21"></a>

### Returns:

An option containing the active price if available, none otherwise.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_get_active_price">get_active_price</a>(self: &<a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrder">bulk_order_book_types::BulkOrder</a>, is_bid: bool): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_get_active_price">get_active_price</a>(
    self: &<a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrder">BulkOrder</a>,
    is_bid: bool,
): Option&lt;u64&gt; {
    <b>let</b> prices = <b>if</b> (is_bid) { self.bid_prices } <b>else</b> { self.ask_prices };
    <b>if</b> (prices.length() == 0) {
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>() // No active price level
    } <b>else</b> {
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(prices[0]) // Return the first price level
    }
}
</code></pre>



</details>

<a id="0x7_bulk_order_book_types_get_active_size"></a>

## Function `get_active_size`

Gets the active size for a specific side of a bulk order.


<a id="@Arguments:_22"></a>

### Arguments:

- <code>self</code>: Reference to the bulk order
- <code>is_bid</code>: True to get bid size, false for ask size


<a id="@Returns:_23"></a>

### Returns:

An option containing the active size if available, none otherwise.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_get_active_size">get_active_size</a>(self: &<a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrder">bulk_order_book_types::BulkOrder</a>, is_bid: bool): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_get_active_size">get_active_size</a>(
    self: &<a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrder">BulkOrder</a>,
    is_bid: bool,
): Option&lt;u64&gt; {
    <b>let</b> sizes = <b>if</b> (is_bid) { self.bid_sizes } <b>else</b> { self.ask_sizes };
    <b>if</b> (sizes.length() == 0) {
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>() // No active size level
    } <b>else</b> {
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(sizes[0]) // Return the first size level
    }
}
</code></pre>



</details>

<a id="0x7_bulk_order_book_types_reinsert_order"></a>

## Function `reinsert_order`

Sets a bulk order to empty state, clearing all price levels and sizes.

This function is used during order cancellation to clear the order state
while preserving the order ID for potential reuse.
Validates that a reinsertion request is consistent with the original order.

Reinserts an order into a bulk order.

This function adds the reinserted order's price and size to the appropriate side
of the bulk order. If the price already exists at the first level, it increases
the size; otherwise, it inserts the new price level at the front.


<a id="@Arguments:_24"></a>

### Arguments:

- <code>self</code>: Mutable reference to the bulk order
- <code>other</code>: Reference to the order result to reinsert


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_reinsert_order">reinsert_order</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrder">bulk_order_book_types::BulkOrder</a>, other: &<a href="order_book_types.md#0x7_order_book_types_OrderMatchDetails">order_book_types::OrderMatchDetails</a>&lt;M&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_reinsert_order">reinsert_order</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrder">BulkOrder</a>,
    other: &OrderMatchDetails&lt;M&gt;,
) {
    // Reinsert the order into the bulk order
    <b>let</b> (prices, sizes, total_remaining) = <b>if</b> (other.is_bid_from_match_details()) {
        (&<b>mut</b> self.bid_prices, &<b>mut</b> self.bid_sizes, &<b>mut</b> self.total_remaining_bid_size)
    } <b>else</b> {
        (&<b>mut</b> self.ask_prices, &<b>mut</b> self.ask_sizes, &<b>mut</b> self.total_remaining_ask_size)
    };
    // Reinsert the price and size at the front of the respective vectors - <b>if</b> the price already <b>exists</b>, we ensure that
    // it is same <b>as</b> the reinsertion price and we just increase the size
    // If the price does not exist, we insert it at the front.
    <b>if</b> (prices.length() &gt; 0 && prices[0] == other.get_price_from_match_details()) {
        sizes[0] += other.get_remaining_size_from_match_details(); // Increase the size at the first price level
        *total_remaining += other.get_remaining_size_from_match_details(); // Increase the total remaining size
    } <b>else</b> {
        prices.insert(0, other.get_price_from_match_details()); // Insert the new price at the front
        sizes.insert(0, other.get_remaining_size_from_match_details()); // Insert the new size at the front
        *total_remaining += other.get_remaining_size_from_match_details(); // Set the total remaining size <b>to</b> the new size
    }
}
</code></pre>



</details>

<a id="0x7_bulk_order_book_types_match_order_and_get_next"></a>

## Function `match_order_and_get_next`

Matches an order and returns the next active price and size.

This function reduces the size at the first price level by the matched size.
If the first level becomes empty, it is removed and the next level becomes active.


<a id="@Arguments:_25"></a>

### Arguments:

- <code>self</code>: Mutable reference to the bulk order
- <code>is_bid</code>: True if matching against bid side, false for ask side
- <code>matched_size</code>: Size that was matched in this operation


<a id="@Returns:_26"></a>

### Returns:

A tuple containing the next active price and size as options.


<a id="@Aborts:_27"></a>

### Aborts:

- If the matched size exceeds the available size at the first level


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_match_order_and_get_next">match_order_and_get_next</a>(self: &<b>mut</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrder">bulk_order_book_types::BulkOrder</a>, is_bid: bool, matched_size: u64): (<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_match_order_and_get_next">match_order_and_get_next</a>(
    self: &<b>mut</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrder">BulkOrder</a>,
    is_bid: bool,
    matched_size: u64,
): (Option&lt;u64&gt;, Option&lt;u64&gt;) {
    <b>let</b> (prices, sizes, total_remaining) = <b>if</b> (is_bid) {
        (&<b>mut</b> self.bid_prices, &<b>mut</b> self.bid_sizes, &<b>mut</b> self.total_remaining_bid_size)
    } <b>else</b> {
        (&<b>mut</b> self.ask_prices, &<b>mut</b> self.ask_sizes, &<b>mut</b> self.total_remaining_ask_size)
    };
    <b>assert</b>!(matched_size &lt;= sizes[0], <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_EUNEXPECTED_MATCH_SIZE">EUNEXPECTED_MATCH_SIZE</a>); // Ensure the remaining size is not more than the size at the first price level
    sizes[0] -= matched_size; // Decrease the size at the first price level by the matched size
    *total_remaining -= matched_size; // Decrease the total remaining size
    <b>if</b> (sizes[0] == 0) {
        // If the size at the first price level is now 0, remove this price level
        prices.remove(0);
        sizes.remove(0);
    };
    <b>if</b> (sizes.length() == 0) {
        (<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()) // No active price or size left
    } <b>else</b> {
        (<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(prices[0]), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(sizes[0])) // Return the next active price and size
    }
}
</code></pre>



</details>

<a id="0x7_bulk_order_book_types_set_empty"></a>

## Function `set_empty`

Sets the bulk order to empty state by clearing all sizes.


<a id="@Arguments:_28"></a>

### Arguments:

- <code>self</code>: Mutable reference to the bulk order


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_set_empty">set_empty</a>(self: &<b>mut</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrder">bulk_order_book_types::BulkOrder</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_set_empty">set_empty</a>(
    self: &<b>mut</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrder">BulkOrder</a>
) {
    self.total_remaining_bid_size = 0;
    self.total_remaining_ask_size = 0;
    self.bid_sizes = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>();
    self.ask_sizes = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>();
    self.bid_prices = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>();
    self.ask_prices = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>();
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
