
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
-  [Enum `BulkOrderRequest`](#0x7_bulk_order_book_types_BulkOrderRequest)
    -  [Fields:](#@Fields:_6)
    -  [Validation:](#@Validation:_7)
-  [Enum `BulkOrder`](#0x7_bulk_order_book_types_BulkOrder)
    -  [Fields:](#@Fields:_8)
-  [Enum `BulkOrderPlaceResponse`](#0x7_bulk_order_book_types_BulkOrderPlaceResponse)
-  [Struct `BulkOrderRequestResponse`](#0x7_bulk_order_book_types_BulkOrderRequestResponse)
-  [Constants](#@Constants_9)
-  [Function `new_bulk_order`](#0x7_bulk_order_book_types_new_bulk_order)
    -  [Arguments:](#@Arguments:_10)
    -  [Returns:](#@Returns:_11)
-  [Function `new_bulk_order_request`](#0x7_bulk_order_book_types_new_bulk_order_request)
    -  [Arguments:](#@Arguments:_12)
    -  [Returns:](#@Returns:_13)
    -  [Aborts:](#@Aborts:_14)
-  [Function `get_account_from_order_request`](#0x7_bulk_order_book_types_get_account_from_order_request)
-  [Function `get_sequence_number_from_order_request`](#0x7_bulk_order_book_types_get_sequence_number_from_order_request)
-  [Function `get_sequence_number_from_bulk_order`](#0x7_bulk_order_book_types_get_sequence_number_from_bulk_order)
-  [Function `new_bulk_order_place_response_success`](#0x7_bulk_order_book_types_new_bulk_order_place_response_success)
-  [Function `new_bulk_order_place_response_rejection`](#0x7_bulk_order_book_types_new_bulk_order_place_response_rejection)
-  [Function `is_success`](#0x7_bulk_order_book_types_is_success)
-  [Function `is_rejection`](#0x7_bulk_order_book_types_is_rejection)
-  [Function `is_bulk_order_success_response`](#0x7_bulk_order_book_types_is_bulk_order_success_response)
-  [Function `destroy_bulk_order_place_success_response`](#0x7_bulk_order_book_types_destroy_bulk_order_place_success_response)
-  [Function `destroy_bulk_order_place_reject_response`](#0x7_bulk_order_book_types_destroy_bulk_order_place_reject_response)
-  [Function `new_bulk_order_request_response_success`](#0x7_bulk_order_book_types_new_bulk_order_request_response_success)
-  [Function `new_bulk_order_request_response_rejection`](#0x7_bulk_order_book_types_new_bulk_order_request_response_rejection)
-  [Function `destroy_bulk_order_request_response`](#0x7_bulk_order_book_types_destroy_bulk_order_request_response)
-  [Function `validate_not_zero_sizes`](#0x7_bulk_order_book_types_validate_not_zero_sizes)
    -  [Arguments:](#@Arguments:_15)
-  [Function `validate_price_ordering`](#0x7_bulk_order_book_types_validate_price_ordering)
    -  [Arguments:](#@Arguments:_16)
-  [Function `validate_no_price_crossing`](#0x7_bulk_order_book_types_validate_no_price_crossing)
    -  [Arguments:](#@Arguments:_17)
-  [Function `discard_price_crossing_levels`](#0x7_bulk_order_book_types_discard_price_crossing_levels)
-  [Function `new_bulk_order_match`](#0x7_bulk_order_book_types_new_bulk_order_match)
-  [Function `get_total_remaining_size`](#0x7_bulk_order_book_types_get_total_remaining_size)
-  [Function `get_unique_priority_idx`](#0x7_bulk_order_book_types_get_unique_priority_idx)
    -  [Arguments:](#@Arguments:_18)
    -  [Returns:](#@Returns:_19)
-  [Function `get_order_id`](#0x7_bulk_order_book_types_get_order_id)
    -  [Arguments:](#@Arguments:_20)
    -  [Returns:](#@Returns:_21)
-  [Function `get_account`](#0x7_bulk_order_book_types_get_account)
    -  [Arguments:](#@Arguments:_22)
    -  [Returns:](#@Returns:_23)
-  [Function `get_active_price`](#0x7_bulk_order_book_types_get_active_price)
    -  [Arguments:](#@Arguments:_24)
    -  [Returns:](#@Returns:_25)
-  [Function `get_all_prices`](#0x7_bulk_order_book_types_get_all_prices)
-  [Function `get_all_sizes`](#0x7_bulk_order_book_types_get_all_sizes)
-  [Function `get_active_size`](#0x7_bulk_order_book_types_get_active_size)
    -  [Arguments:](#@Arguments:_26)
    -  [Returns:](#@Returns:_27)
-  [Function `reinsert_order`](#0x7_bulk_order_book_types_reinsert_order)
    -  [Arguments:](#@Arguments:_28)
-  [Function `match_order_and_get_next`](#0x7_bulk_order_book_types_match_order_and_get_next)
    -  [Arguments:](#@Arguments:_29)
    -  [Returns:](#@Returns:_30)
    -  [Aborts:](#@Aborts:_31)
-  [Function `set_empty`](#0x7_bulk_order_book_types_set_empty)
    -  [Arguments:](#@Arguments:_32)
-  [Function `destroy_bulk_order`](#0x7_bulk_order_book_types_destroy_bulk_order)


<pre><code><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
<b>use</b> <a href="order_book_types.md#0x7_order_book_types">0x7::order_book_types</a>;
</code></pre>



<a id="0x7_bulk_order_book_types_BulkOrderRequest"></a>

## Enum `BulkOrderRequest`

Request structure for placing a new bulk order with multiple price levels.


<a id="@Fields:_6"></a>

### Fields:

- <code><a href="../../aptos-framework/doc/account.md#0x1_account">account</a></code>: The account placing the order
- <code>bid_prices</code>: Vector of bid prices in descending order (best price first)
- <code>bid_sizes</code>: Vector of bid sizes corresponding to each price level
- <code>ask_prices</code>: Vector of ask prices in ascending order (best price first)
- <code>ask_sizes</code>: Vector of ask sizes corresponding to each price level
- <code>metadata</code>: Additional metadata for the order


<a id="@Validation:_7"></a>

### Validation:

- Bid prices must be in descending order
- Ask prices must be in ascending order
- All sizes must be greater than 0
- Price and size vectors must have matching lengths.
All bulk orders by default are post-only and will not cross the spread -
GTC and non-reduce-only orders


<pre><code>enum <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrderRequest">BulkOrderRequest</a>&lt;M: <b>copy</b>, drop, store&gt; <b>has</b> <b>copy</b>, drop
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
<code>order_sequence_number: u64</code>
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
<dt>
<code>metadata: M</code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

</details>

<a id="0x7_bulk_order_book_types_BulkOrder"></a>

## Enum `BulkOrder`

Represents a multi-level order with both bid and ask sides.

Each side can have multiple price levels with associated sizes. The order maintains
both original and remaining sizes for tracking purposes.


<a id="@Fields:_8"></a>

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
- <code>metadata</code>: Additional metadata for the order


<pre><code>enum <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrder">BulkOrder</a>&lt;M: <b>copy</b>, drop, store&gt; <b>has</b> <b>copy</b>, drop, store
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
<code>order_sequence_number: u64</code>
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
<dt>
<code>metadata: M</code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

</details>

<a id="0x7_bulk_order_book_types_BulkOrderPlaceResponse"></a>

## Enum `BulkOrderPlaceResponse`



<pre><code>enum <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrderPlaceResponse">BulkOrderPlaceResponse</a>&lt;M: <b>copy</b>, drop, store&gt; <b>has</b> <b>copy</b>, drop
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>Success</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>order: <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrder">bulk_order_book_types::BulkOrder</a>&lt;M&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>cancelled_bid_prices: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>cancelled_bid_sizes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>cancelled_ask_prices: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>cancelled_ask_sizes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>previous_seq_num: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

<details>
<summary>Rejection</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>reason: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

</details>

<a id="0x7_bulk_order_book_types_BulkOrderRequestResponse"></a>

## Struct `BulkOrderRequestResponse`



<pre><code><b>struct</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrderRequestResponse">BulkOrderRequestResponse</a>&lt;M: <b>copy</b>, drop, store&gt; <b>has</b> <b>copy</b>, drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>request: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrderRequest">bulk_order_book_types::BulkOrderRequest</a>&lt;M&gt;&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>rejection_reason: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_9"></a>

## Constants


<a id="0x7_bulk_order_book_types_E_REINSERT_ORDER_MISMATCH"></a>



<pre><code><b>const</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_E_REINSERT_ORDER_MISMATCH">E_REINSERT_ORDER_MISMATCH</a>: u64 = 3;
</code></pre>



<a id="0x7_bulk_order_book_types_EINVLID_MM_ORDER_REQUEST"></a>



<pre><code><b>const</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_EINVLID_MM_ORDER_REQUEST">EINVLID_MM_ORDER_REQUEST</a>: u64 = 4;
</code></pre>



<a id="0x7_bulk_order_book_types_EPRICE_CROSSING"></a>



<pre><code><b>const</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_EPRICE_CROSSING">EPRICE_CROSSING</a>: u64 = 5;
</code></pre>



<a id="0x7_bulk_order_book_types_EUNEXPECTED_MATCH_PRICE"></a>



<pre><code><b>const</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_EUNEXPECTED_MATCH_PRICE">EUNEXPECTED_MATCH_PRICE</a>: u64 = 1;
</code></pre>



<a id="0x7_bulk_order_book_types_EUNEXPECTED_MATCH_SIZE"></a>



<pre><code><b>const</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_EUNEXPECTED_MATCH_SIZE">EUNEXPECTED_MATCH_SIZE</a>: u64 = 2;
</code></pre>



<a id="0x7_bulk_order_book_types_new_bulk_order"></a>

## Function `new_bulk_order`

Creates a new bulk order with the specified parameters.


<a id="@Arguments:_10"></a>

### Arguments:

- <code>order_id</code>: Unique identifier for the order
- <code>unique_priority_idx</code>: Priority index for time-based ordering
- <code>order_req</code>: The bulk order request containing all order details
- <code>best_bid_price</code>: Current best bid price in the market
- <code>best_ask_price</code>: Current best ask price in the market


<a id="@Returns:_11"></a>

### Returns:

A tuple containing:
- <code><a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrder">BulkOrder</a>&lt;M&gt;</code>: The created bulk order with non-crossing levels
- <code><a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;</code>: Cancelled bid prices (levels that crossed the spread)
- <code><a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;</code>: Cancelled bid sizes corresponding to cancelled prices
- <code><a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;</code>: Cancelled ask prices (levels that crossed the spread)
- <code><a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;</code>: Cancelled ask sizes corresponding to cancelled prices


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_new_bulk_order">new_bulk_order</a>&lt;M: <b>copy</b>, drop, store&gt;(order_id: <a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>, unique_priority_idx: <a href="order_book_types.md#0x7_order_book_types_UniqueIdxType">order_book_types::UniqueIdxType</a>, order_req: <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrderRequest">bulk_order_book_types::BulkOrderRequest</a>&lt;M&gt;, best_bid_price: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;, best_ask_price: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;): (<a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrder">bulk_order_book_types::BulkOrder</a>&lt;M&gt;, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_new_bulk_order">new_bulk_order</a>&lt;M: store + <b>copy</b> + drop&gt;(
    order_id: OrderIdType,
    unique_priority_idx: UniqueIdxType,
    order_req: <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrderRequest">BulkOrderRequest</a>&lt;M&gt;,
    best_bid_price: Option&lt;u64&gt;,
    best_ask_price: Option&lt;u64&gt;,
): (<a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrder">BulkOrder</a>&lt;M&gt;, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;) {
    <b>let</b> BulkOrderRequest::V1 { <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>, order_sequence_number, bid_prices, bid_sizes, ask_prices, ask_sizes, metadata } = order_req;
    <b>let</b> bid_price_crossing_idx = <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_discard_price_crossing_levels">discard_price_crossing_levels</a>(&bid_prices, best_ask_price, <b>true</b>);
    <b>let</b> ask_price_crossing_idx = <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_discard_price_crossing_levels">discard_price_crossing_levels</a>(&ask_prices, best_bid_price, <b>false</b>);

    // Extract cancelled levels (the ones that were discarded)
    <b>let</b> (cancelled_bid_prices, cancelled_bid_sizes, post_only_bid_prices, post_only_bid_sizes) = <b>if</b> (bid_price_crossing_idx &gt; 0) {
        <b>let</b> post_only_bid_prices = bid_prices.trim(bid_price_crossing_idx);
        <b>let</b> post_only_bid_sizes = bid_sizes.trim(bid_price_crossing_idx);
        (bid_prices.slice(0, bid_price_crossing_idx), bid_sizes.slice(0, bid_price_crossing_idx), post_only_bid_prices, post_only_bid_sizes)
    } <b>else</b> {
        (<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>&lt;u64&gt;(), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>&lt;u64&gt;(), bid_prices, bid_sizes)
    };
    <b>let</b> (cancelled_ask_prices, cancelled_ask_sizes, post_only_ask_prices, post_only_ask_sizes) = <b>if</b> (ask_price_crossing_idx &gt; 0) {
        <b>let</b> post_only_ask_prices = ask_prices.trim(ask_price_crossing_idx);
        <b>let</b> post_only_ask_sizes = ask_sizes.trim(ask_price_crossing_idx);
        (ask_prices.slice(0, ask_price_crossing_idx), ask_sizes.slice(0, ask_price_crossing_idx), post_only_ask_prices, post_only_ask_sizes)
    } <b>else</b> {
        (<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>&lt;u64&gt;(), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>&lt;u64&gt;(), ask_prices, ask_sizes)
    };
    <b>let</b> bulk_order = BulkOrder::V1 {
        order_id,
        <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>,
        unique_priority_idx,
        order_sequence_number,
        bid_prices: post_only_bid_prices,
        bid_sizes: post_only_bid_sizes,
        ask_prices: post_only_ask_prices,
        ask_sizes: post_only_ask_sizes,
        metadata
    };
    (bulk_order, cancelled_bid_prices, cancelled_bid_sizes, cancelled_ask_prices, cancelled_ask_sizes)
}
</code></pre>



</details>

<a id="0x7_bulk_order_book_types_new_bulk_order_request"></a>

## Function `new_bulk_order_request`

Creates a new bulk order request with the specified price levels and sizes.


<a id="@Arguments:_12"></a>

### Arguments:

- <code><a href="../../aptos-framework/doc/account.md#0x1_account">account</a></code>: The account placing the order
- <code>bid_prices</code>: Vector of bid prices in descending order
- <code>bid_sizes</code>: Vector of bid sizes corresponding to each price level
- <code>ask_prices</code>: Vector of ask prices in ascending order
- <code>ask_sizes</code>: Vector of ask sizes corresponding to each price level
- <code>metadata</code>: Additional metadata for the order


<a id="@Returns:_13"></a>

### Returns:

A <code><a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrderRequest">BulkOrderRequest</a></code> instance.


<a id="@Aborts:_14"></a>

### Aborts:

- If bid_prices and bid_sizes have different lengths
- If ask_prices and ask_sizes have different lengths


<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_new_bulk_order_request">new_bulk_order_request</a>&lt;M: <b>copy</b>, drop, store&gt;(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>, sequence_number: u64, bid_prices: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, bid_sizes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, ask_prices: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, ask_sizes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, metadata: M): <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrderRequestResponse">bulk_order_book_types::BulkOrderRequestResponse</a>&lt;M&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_new_bulk_order_request">new_bulk_order_request</a>&lt;M: store + <b>copy</b> + drop&gt;(
    <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>,
    sequence_number: u64,
    bid_prices: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    bid_sizes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    ask_prices: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    ask_sizes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    metadata: M
): <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrderRequestResponse">BulkOrderRequestResponse</a>&lt;M&gt; {
    // Basic length validation
    <b>if</b> (bid_prices.length() != bid_sizes.length()) {
        <b>return</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_new_bulk_order_request_response_rejection">new_bulk_order_request_response_rejection</a>(
            std::string::utf8(b"Bid length mismatch")
        );
    };
    <b>if</b> (ask_prices.length() != ask_sizes.length()) {
        <b>return</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_new_bulk_order_request_response_rejection">new_bulk_order_request_response_rejection</a>(
            std::string::utf8(b"Ask length mismatch")
        );
    };

    <b>let</b> req = BulkOrderRequest::V1 {
        <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>,
        order_sequence_number: sequence_number,
        bid_prices,
        bid_sizes,
        ask_prices,
        ask_sizes,
        metadata
    };

    // Ensure bid prices are in descending order and ask prices are in ascending order
    // Check <b>if</b> at least one side <b>has</b> orders
    <b>if</b> (req.bid_sizes.length() == 0 && req.ask_sizes.length() == 0) {
        <b>return</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_new_bulk_order_request_response_rejection">new_bulk_order_request_response_rejection</a>(
            std::string::utf8(b"No orders")
        );
    };

    // Check for zero sizes
    <b>if</b> (!<a href="bulk_order_book_types.md#0x7_bulk_order_book_types_validate_not_zero_sizes">validate_not_zero_sizes</a>(&req.bid_sizes)) {
        <b>return</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_new_bulk_order_request_response_rejection">new_bulk_order_request_response_rejection</a>(
            std::string::utf8(b"Zero bid size")
        );
    };
    <b>if</b> (!<a href="bulk_order_book_types.md#0x7_bulk_order_book_types_validate_not_zero_sizes">validate_not_zero_sizes</a>(&req.ask_sizes)) {
        <b>return</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_new_bulk_order_request_response_rejection">new_bulk_order_request_response_rejection</a>(
            std::string::utf8(b"Zero ask size")
        );
    };

    // Check price ordering
    <b>if</b> (!<a href="bulk_order_book_types.md#0x7_bulk_order_book_types_validate_price_ordering">validate_price_ordering</a>(&req.bid_prices, <b>true</b>)) {
        <b>return</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_new_bulk_order_request_response_rejection">new_bulk_order_request_response_rejection</a>(
            std::string::utf8(b"Bid order invalid")
        );
    };
    <b>if</b> (!<a href="bulk_order_book_types.md#0x7_bulk_order_book_types_validate_price_ordering">validate_price_ordering</a>(&req.ask_prices, <b>false</b>)) {
        <b>return</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_new_bulk_order_request_response_rejection">new_bulk_order_request_response_rejection</a>(
            std::string::utf8(b"Ask order invalid")
        );
    };

    // Check for price crossing
    <b>if</b> (!<a href="bulk_order_book_types.md#0x7_bulk_order_book_types_validate_no_price_crossing">validate_no_price_crossing</a>(&req.bid_prices, &req.ask_prices)) {
        <b>return</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_new_bulk_order_request_response_rejection">new_bulk_order_request_response_rejection</a>(
            std::string::utf8(b"Price crossing")
        );
    };

    <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_new_bulk_order_request_response_success">new_bulk_order_request_response_success</a>(req)
}
</code></pre>



</details>

<a id="0x7_bulk_order_book_types_get_account_from_order_request"></a>

## Function `get_account_from_order_request`



<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_get_account_from_order_request">get_account_from_order_request</a>&lt;M: <b>copy</b>, drop, store&gt;(order_req: &<a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrderRequest">bulk_order_book_types::BulkOrderRequest</a>&lt;M&gt;): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_get_account_from_order_request">get_account_from_order_request</a>&lt;M: store + <b>copy</b> + drop&gt;(
    order_req: &<a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrderRequest">BulkOrderRequest</a>&lt;M&gt;
): <b>address</b> {
    <b>let</b> BulkOrderRequest::V1 { <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>, .. } = order_req;
    *<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>
}
</code></pre>



</details>

<a id="0x7_bulk_order_book_types_get_sequence_number_from_order_request"></a>

## Function `get_sequence_number_from_order_request`



<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_get_sequence_number_from_order_request">get_sequence_number_from_order_request</a>&lt;M: <b>copy</b>, drop, store&gt;(order_req: &<a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrderRequest">bulk_order_book_types::BulkOrderRequest</a>&lt;M&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_get_sequence_number_from_order_request">get_sequence_number_from_order_request</a>&lt;M: store + <b>copy</b> + drop&gt;(
    order_req: &<a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrderRequest">BulkOrderRequest</a>&lt;M&gt;
): u64 {
    <b>let</b> BulkOrderRequest::V1 { order_sequence_number: sequence_number, .. } = order_req;
    *sequence_number
}
</code></pre>



</details>

<a id="0x7_bulk_order_book_types_get_sequence_number_from_bulk_order"></a>

## Function `get_sequence_number_from_bulk_order`



<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_get_sequence_number_from_bulk_order">get_sequence_number_from_bulk_order</a>&lt;M: <b>copy</b>, drop, store&gt;(order: &<a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrder">bulk_order_book_types::BulkOrder</a>&lt;M&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_get_sequence_number_from_bulk_order">get_sequence_number_from_bulk_order</a>&lt;M: store + <b>copy</b> + drop&gt;(
    order: &<a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrder">BulkOrder</a>&lt;M&gt;
): u64 {
    <b>let</b> BulkOrder::V1 { order_sequence_number: sequence_number, .. } = order;
    *sequence_number
}
</code></pre>



</details>

<a id="0x7_bulk_order_book_types_new_bulk_order_place_response_success"></a>

## Function `new_bulk_order_place_response_success`



<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_new_bulk_order_place_response_success">new_bulk_order_place_response_success</a>&lt;M: <b>copy</b>, drop, store&gt;(order: <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrder">bulk_order_book_types::BulkOrder</a>&lt;M&gt;, cancelled_bid_prices: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, cancelled_bid_sizes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, cancelled_ask_prices: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, cancelled_ask_sizes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, previous_seq_num: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;): <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrderPlaceResponse">bulk_order_book_types::BulkOrderPlaceResponse</a>&lt;M&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_new_bulk_order_place_response_success">new_bulk_order_place_response_success</a>&lt;M: store + <b>copy</b> + drop&gt;(
    order: <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrder">BulkOrder</a>&lt;M&gt;,
    cancelled_bid_prices: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    cancelled_bid_sizes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    cancelled_ask_prices: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    cancelled_ask_sizes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    previous_seq_num: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;
): <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrderPlaceResponse">BulkOrderPlaceResponse</a>&lt;M&gt; {
    BulkOrderPlaceResponse::Success {
        order,
        cancelled_bid_prices,
        cancelled_bid_sizes,
        cancelled_ask_prices,
        cancelled_ask_sizes,
        previous_seq_num,
    }
}
</code></pre>



</details>

<a id="0x7_bulk_order_book_types_new_bulk_order_place_response_rejection"></a>

## Function `new_bulk_order_place_response_rejection`



<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_new_bulk_order_place_response_rejection">new_bulk_order_place_response_rejection</a>&lt;M: <b>copy</b>, drop, store&gt;(rejection_reason: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrderPlaceResponse">bulk_order_book_types::BulkOrderPlaceResponse</a>&lt;M&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_new_bulk_order_place_response_rejection">new_bulk_order_place_response_rejection</a>&lt;M: store + <b>copy</b> + drop&gt;(
    rejection_reason: std::string::String
): <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrderPlaceResponse">BulkOrderPlaceResponse</a>&lt;M&gt; {
    BulkOrderPlaceResponse::Rejection {
        reason: rejection_reason,
    }
}
</code></pre>



</details>

<a id="0x7_bulk_order_book_types_is_success"></a>

## Function `is_success`



<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_is_success">is_success</a>&lt;M: <b>copy</b>, drop, store&gt;(response: &<a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrderPlaceResponse">bulk_order_book_types::BulkOrderPlaceResponse</a>&lt;M&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_is_success">is_success</a>&lt;M: store + <b>copy</b> + drop&gt;(
    response: &<a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrderPlaceResponse">BulkOrderPlaceResponse</a>&lt;M&gt;
): bool {
    <b>if</b> (response is BulkOrderPlaceResponse::Success) {
        <b>true</b>
    } <b>else</b> {
        <b>false</b>
    }
}
</code></pre>



</details>

<a id="0x7_bulk_order_book_types_is_rejection"></a>

## Function `is_rejection`



<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_is_rejection">is_rejection</a>&lt;M: <b>copy</b>, drop, store&gt;(response: &<a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrderPlaceResponse">bulk_order_book_types::BulkOrderPlaceResponse</a>&lt;M&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_is_rejection">is_rejection</a>&lt;M: store + <b>copy</b> + drop&gt;(
    response: &<a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrderPlaceResponse">BulkOrderPlaceResponse</a>&lt;M&gt;
): bool {
    <b>if</b> (response is BulkOrderPlaceResponse::Rejection) {
        <b>true</b>
    } <b>else</b> {
        <b>false</b>
    }
}
</code></pre>



</details>

<a id="0x7_bulk_order_book_types_is_bulk_order_success_response"></a>

## Function `is_bulk_order_success_response`



<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_is_bulk_order_success_response">is_bulk_order_success_response</a>&lt;M: <b>copy</b>, drop, store&gt;(response: &<a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrderPlaceResponse">bulk_order_book_types::BulkOrderPlaceResponse</a>&lt;M&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_is_bulk_order_success_response">is_bulk_order_success_response</a>&lt;M: store + <b>copy</b> + drop&gt;(
    response: &<a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrderPlaceResponse">BulkOrderPlaceResponse</a>&lt;M&gt;
): bool {
    response is BulkOrderPlaceResponse::Success
}
</code></pre>



</details>

<a id="0x7_bulk_order_book_types_destroy_bulk_order_place_success_response"></a>

## Function `destroy_bulk_order_place_success_response`



<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_destroy_bulk_order_place_success_response">destroy_bulk_order_place_success_response</a>&lt;M: <b>copy</b>, drop, store&gt;(response: <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrderPlaceResponse">bulk_order_book_types::BulkOrderPlaceResponse</a>&lt;M&gt;): (<a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrder">bulk_order_book_types::BulkOrder</a>&lt;M&gt;, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_destroy_bulk_order_place_success_response">destroy_bulk_order_place_success_response</a>&lt;M: store + <b>copy</b> + drop&gt;(
    response: <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrderPlaceResponse">BulkOrderPlaceResponse</a>&lt;M&gt;
): (<a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrder">BulkOrder</a>&lt;M&gt;, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;) {
    <b>let</b> BulkOrderPlaceResponse::Success { order, cancelled_bid_prices, cancelled_bid_sizes, cancelled_ask_prices, cancelled_ask_sizes, previous_seq_num } = response;
    (order, cancelled_bid_prices, cancelled_bid_sizes, cancelled_ask_prices, cancelled_ask_sizes, previous_seq_num)
}
</code></pre>



</details>

<a id="0x7_bulk_order_book_types_destroy_bulk_order_place_reject_response"></a>

## Function `destroy_bulk_order_place_reject_response`



<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_destroy_bulk_order_place_reject_response">destroy_bulk_order_place_reject_response</a>&lt;M: <b>copy</b>, drop, store&gt;(response: <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrderPlaceResponse">bulk_order_book_types::BulkOrderPlaceResponse</a>&lt;M&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_destroy_bulk_order_place_reject_response">destroy_bulk_order_place_reject_response</a>&lt;M: store + <b>copy</b> + drop&gt;(
    response: <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrderPlaceResponse">BulkOrderPlaceResponse</a>&lt;M&gt;
): std::string::String {
    <b>let</b> BulkOrderPlaceResponse::Rejection { reason } = response;
    reason
}
</code></pre>



</details>

<a id="0x7_bulk_order_book_types_new_bulk_order_request_response_success"></a>

## Function `new_bulk_order_request_response_success`



<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_new_bulk_order_request_response_success">new_bulk_order_request_response_success</a>&lt;M: <b>copy</b>, drop, store&gt;(request: <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrderRequest">bulk_order_book_types::BulkOrderRequest</a>&lt;M&gt;): <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrderRequestResponse">bulk_order_book_types::BulkOrderRequestResponse</a>&lt;M&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_new_bulk_order_request_response_success">new_bulk_order_request_response_success</a>&lt;M: store + <b>copy</b> + drop&gt;(
    request: <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrderRequest">BulkOrderRequest</a>&lt;M&gt;
): <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrderRequestResponse">BulkOrderRequestResponse</a>&lt;M&gt; {
    <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrderRequestResponse">BulkOrderRequestResponse</a> {
        request: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(request),
        rejection_reason: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(),
    }
}
</code></pre>



</details>

<a id="0x7_bulk_order_book_types_new_bulk_order_request_response_rejection"></a>

## Function `new_bulk_order_request_response_rejection`



<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_new_bulk_order_request_response_rejection">new_bulk_order_request_response_rejection</a>&lt;M: <b>copy</b>, drop, store&gt;(rejection_reason: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrderRequestResponse">bulk_order_book_types::BulkOrderRequestResponse</a>&lt;M&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_new_bulk_order_request_response_rejection">new_bulk_order_request_response_rejection</a>&lt;M: store + <b>copy</b> + drop&gt;(
    rejection_reason: std::string::String
): <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrderRequestResponse">BulkOrderRequestResponse</a>&lt;M&gt; {
    <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrderRequestResponse">BulkOrderRequestResponse</a> {
        request: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(),
        rejection_reason: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(rejection_reason),
    }
}
</code></pre>



</details>

<a id="0x7_bulk_order_book_types_destroy_bulk_order_request_response"></a>

## Function `destroy_bulk_order_request_response`



<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_destroy_bulk_order_request_response">destroy_bulk_order_request_response</a>&lt;M: <b>copy</b>, drop, store&gt;(response: <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrderRequestResponse">bulk_order_book_types::BulkOrderRequestResponse</a>&lt;M&gt;): (<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrderRequest">bulk_order_book_types::BulkOrderRequest</a>&lt;M&gt;&gt;, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_destroy_bulk_order_request_response">destroy_bulk_order_request_response</a>&lt;M: store + <b>copy</b> + drop&gt;(
    response: <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrderRequestResponse">BulkOrderRequestResponse</a>&lt;M&gt;
): (<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrderRequest">BulkOrderRequest</a>&lt;M&gt;&gt;, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;std::string::String&gt;) {
    <b>let</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrderRequestResponse">BulkOrderRequestResponse</a> { request, rejection_reason } = response;
    (request, rejection_reason)
}
</code></pre>



</details>

<a id="0x7_bulk_order_book_types_validate_not_zero_sizes"></a>

## Function `validate_not_zero_sizes`

Validates that all sizes in the vector are greater than 0.


<a id="@Arguments:_15"></a>

### Arguments:

- <code>sizes</code>: Vector of sizes to validate


<pre><code><b>fun</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_validate_not_zero_sizes">validate_not_zero_sizes</a>(sizes: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_validate_not_zero_sizes">validate_not_zero_sizes</a>(sizes: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;): bool {
    <b>let</b> i = 0;
    <b>while</b> (i &lt; sizes.length()) {
        <b>if</b> (sizes[i] == 0) {
            <b>return</b> <b>false</b>;
        };
        i += 1;
    };
    <b>true</b>
}
</code></pre>



</details>

<a id="0x7_bulk_order_book_types_validate_price_ordering"></a>

## Function `validate_price_ordering`

Validates that prices are in the correct order (descending for bids, ascending for asks).


<a id="@Arguments:_16"></a>

### Arguments:

- <code>prices</code>: Vector of prices to validate
- <code>is_descending</code>: True if prices should be in descending order, false for ascending


<pre><code><b>fun</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_validate_price_ordering">validate_price_ordering</a>(prices: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, is_descending: bool): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_validate_price_ordering">validate_price_ordering</a>(
    prices: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    is_descending: bool
): bool {
    <b>let</b> i = 0;
    <b>if</b> (prices.length() == 0) {
        <b>return</b> <b>true</b>; // No prices <b>to</b> validate
    };
    <b>while</b> (i &lt; prices.length() - 1) {
        <b>if</b> (is_descending) {
            <b>if</b> (prices[i] &lt;= prices[i + 1]) {
                <b>return</b> <b>false</b>;
            };
        } <b>else</b> {
            <b>if</b> (prices[i] &gt;= prices[i + 1]) {
                <b>return</b> <b>false</b>;
            };
        };
        i += 1;
    };
    <b>true</b>
}
</code></pre>



</details>

<a id="0x7_bulk_order_book_types_validate_no_price_crossing"></a>

## Function `validate_no_price_crossing`

Validates that bid and ask prices don't cross.

This ensures that the highest bid price is lower than the lowest ask price,
preventing self-matching within a single order.


<a id="@Arguments:_17"></a>

### Arguments:

- <code>bid_prices</code>: Vector of bid prices (should be in descending order)
- <code>ask_prices</code>: Vector of ask prices (should be in ascending order)


<pre><code><b>fun</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_validate_no_price_crossing">validate_no_price_crossing</a>(bid_prices: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, ask_prices: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_validate_no_price_crossing">validate_no_price_crossing</a>(
    bid_prices: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    ask_prices: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;
): bool {
    <b>if</b> (bid_prices.length() &gt; 0 && ask_prices.length() &gt; 0) {
        <b>let</b> highest_bid = bid_prices[0]; // First element is highest (descending order)
        <b>let</b> lowest_ask = ask_prices[0];  // First element is lowest (ascending order)
        <b>return</b> highest_bid &lt; lowest_ask;
    };
    <b>true</b>
}
</code></pre>



</details>

<a id="0x7_bulk_order_book_types_discard_price_crossing_levels"></a>

## Function `discard_price_crossing_levels`



<pre><code><b>fun</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_discard_price_crossing_levels">discard_price_crossing_levels</a>(prices: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, best_price: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;, is_bid: bool): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_discard_price_crossing_levels">discard_price_crossing_levels</a>(
    prices: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    best_price: Option&lt;u64&gt;,
    is_bid: bool,
): u64 {
    // Discard bid levels that are &gt;= best ask price
    <b>let</b> i = 0;
    <b>if</b> (best_price != <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()) {
        <b>let</b> best_price = best_price.destroy_some();
        <b>while</b> (i &lt; prices.length()) {
            <b>if</b> (is_bid && prices[i] &lt; best_price) {
                <b>break</b>;
            } <b>else</b> <b>if</b> (!is_bid && prices[i] &gt; best_price) {
                <b>break</b>;
            };
            i += 1;
        };
    };
    i // Return the index of the first non-crossing level
}
</code></pre>



</details>

<a id="0x7_bulk_order_book_types_new_bulk_order_match"></a>

## Function `new_bulk_order_match`



<pre><code>#[lint::skip(#[needless_mutable_reference])]
<b>public</b>(<b>friend</b>) <b>fun</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_new_bulk_order_match">new_bulk_order_match</a>&lt;M: <b>copy</b>, drop, store&gt;(order: &<b>mut</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrder">bulk_order_book_types::BulkOrder</a>&lt;M&gt;, is_bid: bool, matched_size: u64): <a href="order_book_types.md#0x7_order_book_types_OrderMatch">order_book_types::OrderMatch</a>&lt;M&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_new_bulk_order_match">new_bulk_order_match</a>&lt;M: store + <b>copy</b> + drop&gt;(
    order: &<b>mut</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrder">BulkOrder</a>&lt;M&gt;,
    is_bid: bool,
    matched_size: u64
): OrderMatch&lt;M&gt; {
    <b>let</b> (price, remaining_size) = <b>if</b> (is_bid) {
        (order.bid_prices[0], order.bid_sizes[0]  - matched_size)
    } <b>else</b> {
        (order.ask_prices[0], order.ask_sizes[0] - matched_size)
    };
    new_order_match&lt;M&gt;(
        new_bulk_order_match_details&lt;M&gt;(
            order.<a href="bulk_order_book_types.md#0x7_bulk_order_book_types_get_order_id">get_order_id</a>(),
            order.<a href="bulk_order_book_types.md#0x7_bulk_order_book_types_get_account">get_account</a>(),
            order.<a href="bulk_order_book_types.md#0x7_bulk_order_book_types_get_unique_priority_idx">get_unique_priority_idx</a>(),
            price,
            remaining_size,
            is_bid,
            <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_get_sequence_number_from_bulk_order">get_sequence_number_from_bulk_order</a>(order),
            order.metadata,
        ),
        matched_size
    )
}
</code></pre>



</details>

<a id="0x7_bulk_order_book_types_get_total_remaining_size"></a>

## Function `get_total_remaining_size`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_get_total_remaining_size">get_total_remaining_size</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrder">bulk_order_book_types::BulkOrder</a>&lt;M&gt;, is_bid: bool): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>)  <b>fun</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_get_total_remaining_size">get_total_remaining_size</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrder">BulkOrder</a>&lt;M&gt;,
    is_bid: bool,
): u64 {
    <b>if</b> (is_bid) {
        self.bid_sizes.fold(0, |acc, size| acc + size)
    } <b>else</b> {
        self.ask_sizes.fold(0, |acc, size| acc + size)
    }
}
</code></pre>



</details>

<a id="0x7_bulk_order_book_types_get_unique_priority_idx"></a>

## Function `get_unique_priority_idx`

Gets the unique priority index of a bulk order.


<a id="@Arguments:_18"></a>

### Arguments:

- <code>self</code>: Reference to the bulk order


<a id="@Returns:_19"></a>

### Returns:

The unique priority index for time-based ordering.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_get_unique_priority_idx">get_unique_priority_idx</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrder">bulk_order_book_types::BulkOrder</a>&lt;M&gt;): <a href="order_book_types.md#0x7_order_book_types_UniqueIdxType">order_book_types::UniqueIdxType</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_get_unique_priority_idx">get_unique_priority_idx</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrder">BulkOrder</a>&lt;M&gt;,
): UniqueIdxType {
    self.unique_priority_idx
}
</code></pre>



</details>

<a id="0x7_bulk_order_book_types_get_order_id"></a>

## Function `get_order_id`

Gets the order ID of a bulk order.


<a id="@Arguments:_20"></a>

### Arguments:

- <code>self</code>: Reference to the bulk order


<a id="@Returns:_21"></a>

### Returns:

The unique order identifier.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_get_order_id">get_order_id</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrder">bulk_order_book_types::BulkOrder</a>&lt;M&gt;): <a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_get_order_id">get_order_id</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrder">BulkOrder</a>&lt;M&gt;,
): OrderIdType {
    self.order_id
}
</code></pre>



</details>

<a id="0x7_bulk_order_book_types_get_account"></a>

## Function `get_account`

Gets the account of a bulk order.


<a id="@Arguments:_22"></a>

### Arguments:

- <code>self</code>: Reference to the bulk order


<a id="@Returns:_23"></a>

### Returns:

The account that placed the order.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_get_account">get_account</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrder">bulk_order_book_types::BulkOrder</a>&lt;M&gt;): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_get_account">get_account</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrder">BulkOrder</a>&lt;M&gt;,
): <b>address</b> {
    self.<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>
}
</code></pre>



</details>

<a id="0x7_bulk_order_book_types_get_active_price"></a>

## Function `get_active_price`

Gets the active price for a specific side of a bulk order.


<a id="@Arguments:_24"></a>

### Arguments:

- <code>self</code>: Reference to the bulk order
- <code>is_bid</code>: True to get bid price, false for ask price


<a id="@Returns:_25"></a>

### Returns:

An option containing the active price if available, none otherwise.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_get_active_price">get_active_price</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrder">bulk_order_book_types::BulkOrder</a>&lt;M&gt;, is_bid: bool): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_get_active_price">get_active_price</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrder">BulkOrder</a>&lt;M&gt;,
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

<a id="0x7_bulk_order_book_types_get_all_prices"></a>

## Function `get_all_prices`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_get_all_prices">get_all_prices</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrder">bulk_order_book_types::BulkOrder</a>&lt;M&gt;, is_bid: bool): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_get_all_prices">get_all_prices</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrder">BulkOrder</a>&lt;M&gt;,
    is_bid: bool,
): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt; {
    <b>if</b> (is_bid) {
        self.bid_prices
    } <b>else</b> {
        self.ask_prices
    }
}
</code></pre>



</details>

<a id="0x7_bulk_order_book_types_get_all_sizes"></a>

## Function `get_all_sizes`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_get_all_sizes">get_all_sizes</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrder">bulk_order_book_types::BulkOrder</a>&lt;M&gt;, is_bid: bool): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_get_all_sizes">get_all_sizes</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrder">BulkOrder</a>&lt;M&gt;,
    is_bid: bool,
): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt; {
    <b>if</b> (is_bid) {
        self.bid_sizes
    } <b>else</b> {
        self.ask_sizes
    }
}
</code></pre>



</details>

<a id="0x7_bulk_order_book_types_get_active_size"></a>

## Function `get_active_size`

Gets the active size for a specific side of a bulk order.


<a id="@Arguments:_26"></a>

### Arguments:

- <code>self</code>: Reference to the bulk order
- <code>is_bid</code>: True to get bid size, false for ask size


<a id="@Returns:_27"></a>

### Returns:

An option containing the active size if available, none otherwise.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_get_active_size">get_active_size</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrder">bulk_order_book_types::BulkOrder</a>&lt;M&gt;, is_bid: bool): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_get_active_size">get_active_size</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrder">BulkOrder</a>&lt;M&gt;,
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


<a id="@Arguments:_28"></a>

### Arguments:

- <code>self</code>: Mutable reference to the bulk order
- <code>other</code>: Reference to the order result to reinsert


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_reinsert_order">reinsert_order</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrder">bulk_order_book_types::BulkOrder</a>&lt;M&gt;, other: &<a href="order_book_types.md#0x7_order_book_types_OrderMatchDetails">order_book_types::OrderMatchDetails</a>&lt;M&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_reinsert_order">reinsert_order</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrder">BulkOrder</a>&lt;M&gt;,
    other: &OrderMatchDetails&lt;M&gt;,
) {
    // Reinsert the order into the bulk order
    <b>let</b> (prices, sizes) = <b>if</b> (other.is_bid_from_match_details()) {
        (&<b>mut</b> self.bid_prices, &<b>mut</b> self.bid_sizes)
    } <b>else</b> {
        (&<b>mut</b> self.ask_prices, &<b>mut</b> self.ask_sizes)
    };
    // Reinsert the price and size at the front of the respective vectors - <b>if</b> the price already <b>exists</b>, we ensure that
    // it is same <b>as</b> the reinsertion price and we just increase the size
    // If the price does not exist, we insert it at the front.
    <b>if</b> (prices.length() &gt; 0 && prices[0] == other.get_price_from_match_details()) {
        sizes[0] += other.get_remaining_size_from_match_details(); // Increase the size at the first price level
    } <b>else</b> {
        prices.insert(0, other.get_price_from_match_details()); // Insert the new price at the front
        sizes.insert(0, other.get_remaining_size_from_match_details()); // Insert the new size at the front
    }
}
</code></pre>



</details>

<a id="0x7_bulk_order_book_types_match_order_and_get_next"></a>

## Function `match_order_and_get_next`

Matches an order and returns the next active price and size.

This function reduces the size at the first price level by the matched size.
If the first level becomes empty, it is removed and the next level becomes active.


<a id="@Arguments:_29"></a>

### Arguments:

- <code>self</code>: Mutable reference to the bulk order
- <code>is_bid</code>: True if matching against bid side, false for ask side
- <code>matched_size</code>: Size that was matched in this operation


<a id="@Returns:_30"></a>

### Returns:

A tuple containing the next active price and size as options.


<a id="@Aborts:_31"></a>

### Aborts:

- If the matched size exceeds the available size at the first level


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_match_order_and_get_next">match_order_and_get_next</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrder">bulk_order_book_types::BulkOrder</a>&lt;M&gt;, is_bid: bool, matched_size: u64): (<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_match_order_and_get_next">match_order_and_get_next</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrder">BulkOrder</a>&lt;M&gt;,
    is_bid: bool,
    matched_size: u64,
): (Option&lt;u64&gt;, Option&lt;u64&gt;) {
    <b>let</b> (prices, sizes) = <b>if</b> (is_bid) {
        (&<b>mut</b> self.bid_prices, &<b>mut</b> self.bid_sizes)
    } <b>else</b> {
        (&<b>mut</b> self.ask_prices, &<b>mut</b> self.ask_sizes)
    };
    <b>assert</b>!(matched_size &lt;= sizes[0], <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_EUNEXPECTED_MATCH_SIZE">EUNEXPECTED_MATCH_SIZE</a>); // Ensure the remaining size is not more than the size at the first price level
    sizes[0] -= matched_size; // Decrease the size at the first price level by the matched size
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


<a id="@Arguments:_32"></a>

### Arguments:

- <code>self</code>: Mutable reference to the bulk order


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_set_empty">set_empty</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrder">bulk_order_book_types::BulkOrder</a>&lt;M&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_set_empty">set_empty</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrder">BulkOrder</a>&lt;M&gt;
) {
    self.bid_sizes = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>();
    self.ask_sizes = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>();
    self.bid_prices = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>();
    self.ask_prices = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>();
}
</code></pre>



</details>

<a id="0x7_bulk_order_book_types_destroy_bulk_order"></a>

## Function `destroy_bulk_order`



<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_destroy_bulk_order">destroy_bulk_order</a>&lt;M: <b>copy</b>, drop, store&gt;(self: <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrder">bulk_order_book_types::BulkOrder</a>&lt;M&gt;): (<a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>, <b>address</b>, <a href="order_book_types.md#0x7_order_book_types_UniqueIdxType">order_book_types::UniqueIdxType</a>, u64, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, M)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_destroy_bulk_order">destroy_bulk_order</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrder">BulkOrder</a>&lt;M&gt;
): (
    OrderIdType,
    <b>address</b>,
    UniqueIdxType,
    u64,
    <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    M
) {
    <b>let</b> BulkOrder::V1 {
        order_id,
        <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>,
        unique_priority_idx,
        order_sequence_number,
        bid_prices,
        bid_sizes,
        ask_prices,
        ask_sizes,
        metadata
    } = self;
    (
        order_id,
        <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>,
        unique_priority_idx,
        order_sequence_number,
        bid_prices,
        bid_sizes,
        ask_prices,
        ask_sizes,
        metadata
    )
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
