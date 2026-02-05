
<a id="0x5_bulk_order_types"></a>

# Module `0x5::bulk_order_types`


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

- <code><a href="bulk_order_types.md#0x5_bulk_order_types_EUNEXPECTED_MATCH_PRICE">EUNEXPECTED_MATCH_PRICE</a></code>: Unexpected price during order matching
- <code>EUNEXPECTED_MATCH_SIZE</code>: Unexpected size during order matching
- <code><a href="bulk_order_types.md#0x5_bulk_order_types_E_REINSERT_ORDER_MISMATCH">E_REINSERT_ORDER_MISMATCH</a></code>: Order mismatch during reinsertion validation


<a id="@Usage_Example:_5"></a>

### Usage Example:

```move
// Create a new bulk order
let order = bulk_order_types::new_bulk_order(
order_request,
order_id,
unique_priority_idx,
creation_time_micros
);
```
(work in progress)


-  [Bulk Order Book Types Module](#@Bulk_Order_Book_Types_Module_0)
    -  [Key Data Structures:](#@Key_Data_Structures:_1)
        -  [1. BulkOrder](#@1._BulkOrder_2)
    -  [Core Functionality:](#@Core_Functionality:_3)
    -  [Error Codes:](#@Error_Codes:_4)
    -  [Usage Example:](#@Usage_Example:_5)
-  [Enum `BulkOrderRequest`](#0x5_bulk_order_types_BulkOrderRequest)
    -  [Fields:](#@Fields:_6)
    -  [Validation:](#@Validation:_7)
-  [Enum `BulkOrder`](#0x5_bulk_order_types_BulkOrder)
    -  [Fields:](#@Fields:_8)
-  [Enum `BulkOrderPlaceResponse`](#0x5_bulk_order_types_BulkOrderPlaceResponse)
-  [Constants](#@Constants_9)
-  [Function `new_bulk_order`](#0x5_bulk_order_types_new_bulk_order)
    -  [Arguments:](#@Arguments:_10)
-  [Function `new_bulk_order_request`](#0x5_bulk_order_types_new_bulk_order_request)
    -  [Arguments:](#@Arguments:_11)
    -  [Returns:](#@Returns:_12)
    -  [Aborts:](#@Aborts:_13)
-  [Function `new_bulk_order_place_response_success`](#0x5_bulk_order_types_new_bulk_order_place_response_success)
-  [Function `new_bulk_order_place_response_rejection`](#0x5_bulk_order_types_new_bulk_order_place_response_rejection)
-  [Function `get_unique_priority_idx`](#0x5_bulk_order_types_get_unique_priority_idx)
    -  [Arguments:](#@Arguments:_14)
    -  [Returns:](#@Returns:_15)
-  [Function `get_order_id`](#0x5_bulk_order_types_get_order_id)
    -  [Arguments:](#@Arguments:_16)
    -  [Returns:](#@Returns:_17)
-  [Function `get_creation_time_micros`](#0x5_bulk_order_types_get_creation_time_micros)
-  [Function `get_order_request`](#0x5_bulk_order_types_get_order_request)
-  [Function `get_order_request_mut`](#0x5_bulk_order_types_get_order_request_mut)
-  [Function `get_account`](#0x5_bulk_order_types_get_account)
-  [Function `get_sequence_number`](#0x5_bulk_order_types_get_sequence_number)
-  [Function `get_total_remaining_size`](#0x5_bulk_order_types_get_total_remaining_size)
-  [Function `get_active_price`](#0x5_bulk_order_types_get_active_price)
    -  [Arguments:](#@Arguments:_18)
    -  [Returns:](#@Returns:_19)
-  [Function `get_all_prices`](#0x5_bulk_order_types_get_all_prices)
-  [Function `get_all_prices_mut`](#0x5_bulk_order_types_get_all_prices_mut)
-  [Function `get_all_sizes`](#0x5_bulk_order_types_get_all_sizes)
-  [Function `get_all_sizes_mut`](#0x5_bulk_order_types_get_all_sizes_mut)
-  [Function `get_active_size`](#0x5_bulk_order_types_get_active_size)
    -  [Arguments:](#@Arguments:_20)
    -  [Returns:](#@Returns:_21)
-  [Function `get_prices_and_sizes_mut`](#0x5_bulk_order_types_get_prices_and_sizes_mut)
-  [Function `is_success_response`](#0x5_bulk_order_types_is_success_response)
-  [Function `is_rejection_response`](#0x5_bulk_order_types_is_rejection_response)
-  [Function `destroy_bulk_order_place_response_success`](#0x5_bulk_order_types_destroy_bulk_order_place_response_success)
-  [Function `destroy_bulk_order_place_response_rejection`](#0x5_bulk_order_types_destroy_bulk_order_place_response_rejection)
-  [Function `validate_not_zero_sizes`](#0x5_bulk_order_types_validate_not_zero_sizes)
    -  [Arguments:](#@Arguments:_22)
-  [Function `validate_price_ordering`](#0x5_bulk_order_types_validate_price_ordering)
    -  [Arguments:](#@Arguments:_23)
-  [Function `new_bulk_order_match`](#0x5_bulk_order_types_new_bulk_order_match)
-  [Function `set_empty`](#0x5_bulk_order_types_set_empty)
    -  [Arguments:](#@Arguments:_24)
-  [Function `destroy_bulk_order`](#0x5_bulk_order_types_destroy_bulk_order)
-  [Function `destroy_bulk_order_request`](#0x5_bulk_order_types_destroy_bulk_order_request)


<pre><code><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
<b>use</b> <a href="order_book_types.md#0x5_order_book_types">0x5::order_book_types</a>;
<b>use</b> <a href="order_match_types.md#0x5_order_match_types">0x5::order_match_types</a>;
</code></pre>



<a id="0x5_bulk_order_types_BulkOrderRequest"></a>

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
Bulk orders do not support TimeInForce options and behave as maker orders only


<pre><code>enum <a href="bulk_order_types.md#0x5_bulk_order_types_BulkOrderRequest">BulkOrderRequest</a>&lt;M: <b>copy</b>, drop, store&gt; <b>has</b> <b>copy</b>, drop, store
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

<a id="0x5_bulk_order_types_BulkOrder"></a>

## Enum `BulkOrder`

Represents a multi-level order with both bid and ask sides.

Each side can have multiple price levels with associated sizes. The order maintains
both original and remaining sizes for tracking purposes.


<a id="@Fields:_8"></a>

### Fields:

- <code>order_id</code>: Unique identifier for the order
- <code>unique_priority_idx</code>: Priority index for time-based ordering


<pre><code>enum <a href="bulk_order_types.md#0x5_bulk_order_types_BulkOrder">BulkOrder</a>&lt;M: <b>copy</b>, drop, store&gt; <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>V1</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>order_request: <a href="bulk_order_types.md#0x5_bulk_order_types_BulkOrderRequest">bulk_order_types::BulkOrderRequest</a>&lt;M&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>order_id: <a href="order_book_types.md#0x5_order_book_types_OrderId">order_book_types::OrderId</a></code>
</dt>
<dd>

</dd>
<dt>
<code>unique_priority_idx: <a href="order_book_types.md#0x5_order_book_types_IncreasingIdx">order_book_types::IncreasingIdx</a></code>
</dt>
<dd>

</dd>
<dt>
<code>creation_time_micros: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

</details>

<a id="0x5_bulk_order_types_BulkOrderPlaceResponse"></a>

## Enum `BulkOrderPlaceResponse`



<pre><code>enum <a href="bulk_order_types.md#0x5_bulk_order_types_BulkOrderPlaceResponse">BulkOrderPlaceResponse</a>&lt;M: <b>copy</b>, drop, store&gt; <b>has</b> <b>copy</b>, drop
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>Success_V1</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>order: <a href="bulk_order_types.md#0x5_bulk_order_types_BulkOrder">bulk_order_types::BulkOrder</a>&lt;M&gt;</code>
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
<summary>Rejection_V1</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code><a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>sequence_number: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>existing_sequence_number: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

</details>

<a id="@Constants_9"></a>

## Constants


<a id="0x5_bulk_order_types_E_REINSERT_ORDER_MISMATCH"></a>



<pre><code><b>const</b> <a href="bulk_order_types.md#0x5_bulk_order_types_E_REINSERT_ORDER_MISMATCH">E_REINSERT_ORDER_MISMATCH</a>: u64 = 3;
</code></pre>



<a id="0x5_bulk_order_types_EINVLID_MM_ORDER_REQUEST"></a>



<pre><code><b>const</b> <a href="bulk_order_types.md#0x5_bulk_order_types_EINVLID_MM_ORDER_REQUEST">EINVLID_MM_ORDER_REQUEST</a>: u64 = 4;
</code></pre>



<a id="0x5_bulk_order_types_EPRICE_CROSSING"></a>



<pre><code><b>const</b> <a href="bulk_order_types.md#0x5_bulk_order_types_EPRICE_CROSSING">EPRICE_CROSSING</a>: u64 = 5;
</code></pre>



<a id="0x5_bulk_order_types_EUNEXPECTED_MATCH_PRICE"></a>



<pre><code><b>const</b> <a href="bulk_order_types.md#0x5_bulk_order_types_EUNEXPECTED_MATCH_PRICE">EUNEXPECTED_MATCH_PRICE</a>: u64 = 1;
</code></pre>



<a id="0x5_bulk_order_types_E_ASK_LENGTH_MISMATCH"></a>



<pre><code><b>const</b> <a href="bulk_order_types.md#0x5_bulk_order_types_E_ASK_LENGTH_MISMATCH">E_ASK_LENGTH_MISMATCH</a>: u64 = 7;
</code></pre>



<a id="0x5_bulk_order_types_E_ASK_ORDER_INVALID"></a>



<pre><code><b>const</b> <a href="bulk_order_types.md#0x5_bulk_order_types_E_ASK_ORDER_INVALID">E_ASK_ORDER_INVALID</a>: u64 = 13;
</code></pre>



<a id="0x5_bulk_order_types_E_ASK_SIZE_ZERO"></a>



<pre><code><b>const</b> <a href="bulk_order_types.md#0x5_bulk_order_types_E_ASK_SIZE_ZERO">E_ASK_SIZE_ZERO</a>: u64 = 11;
</code></pre>



<a id="0x5_bulk_order_types_E_BID_LENGTH_MISMATCH"></a>



<pre><code><b>const</b> <a href="bulk_order_types.md#0x5_bulk_order_types_E_BID_LENGTH_MISMATCH">E_BID_LENGTH_MISMATCH</a>: u64 = 6;
</code></pre>



<a id="0x5_bulk_order_types_E_BID_ORDER_INVALID"></a>



<pre><code><b>const</b> <a href="bulk_order_types.md#0x5_bulk_order_types_E_BID_ORDER_INVALID">E_BID_ORDER_INVALID</a>: u64 = 12;
</code></pre>



<a id="0x5_bulk_order_types_E_BID_SIZE_ZERO"></a>



<pre><code><b>const</b> <a href="bulk_order_types.md#0x5_bulk_order_types_E_BID_SIZE_ZERO">E_BID_SIZE_ZERO</a>: u64 = 10;
</code></pre>



<a id="0x5_bulk_order_types_E_BULK_ORDER_DEPTH_EXCEEDED"></a>



<pre><code><b>const</b> <a href="bulk_order_types.md#0x5_bulk_order_types_E_BULK_ORDER_DEPTH_EXCEEDED">E_BULK_ORDER_DEPTH_EXCEEDED</a>: u64 = 14;
</code></pre>



<a id="0x5_bulk_order_types_E_EMPTY_ORDER"></a>



<pre><code><b>const</b> <a href="bulk_order_types.md#0x5_bulk_order_types_E_EMPTY_ORDER">E_EMPTY_ORDER</a>: u64 = 9;
</code></pre>



<a id="0x5_bulk_order_types_E_INVALID_SEQUENCE_NUMBER"></a>



<pre><code><b>const</b> <a href="bulk_order_types.md#0x5_bulk_order_types_E_INVALID_SEQUENCE_NUMBER">E_INVALID_SEQUENCE_NUMBER</a>: u64 = 15;
</code></pre>



<a id="0x5_bulk_order_types_MAX_BULK_ORDER_DEPTH_PER_SIDE"></a>

Maximum number of price levels per side (bid or ask) in a bulk order.
This limit prevents gas DoS scenarios when cancelling bulk orders.


<pre><code><b>const</b> <a href="bulk_order_types.md#0x5_bulk_order_types_MAX_BULK_ORDER_DEPTH_PER_SIDE">MAX_BULK_ORDER_DEPTH_PER_SIDE</a>: u64 = 30;
</code></pre>



<a id="0x5_bulk_order_types_new_bulk_order"></a>

## Function `new_bulk_order`

Creates a new bulk order with the specified parameters.


<a id="@Arguments:_10"></a>

### Arguments:

- <code>order_request</code>: The bulk order request containing all order details
- <code>order_id</code>: Unique identifier for the order
- <code>unique_priority_idx</code>: Priority index for time-based ordering
- <code>creation_time_micros</code>: Creation time of the order

Does no validation itself.


<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_types.md#0x5_bulk_order_types_new_bulk_order">new_bulk_order</a>&lt;M: <b>copy</b>, drop, store&gt;(order_request: <a href="bulk_order_types.md#0x5_bulk_order_types_BulkOrderRequest">bulk_order_types::BulkOrderRequest</a>&lt;M&gt;, order_id: <a href="order_book_types.md#0x5_order_book_types_OrderId">order_book_types::OrderId</a>, unique_priority_idx: <a href="order_book_types.md#0x5_order_book_types_IncreasingIdx">order_book_types::IncreasingIdx</a>, creation_time_micros: u64): <a href="bulk_order_types.md#0x5_bulk_order_types_BulkOrder">bulk_order_types::BulkOrder</a>&lt;M&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_types.md#0x5_bulk_order_types_new_bulk_order">new_bulk_order</a>&lt;M: store + <b>copy</b> + drop&gt;(
    order_request: <a href="bulk_order_types.md#0x5_bulk_order_types_BulkOrderRequest">BulkOrderRequest</a>&lt;M&gt;,
    order_id: OrderId,
    unique_priority_idx: IncreasingIdx,
    creation_time_micros: u64
): <a href="bulk_order_types.md#0x5_bulk_order_types_BulkOrder">BulkOrder</a>&lt;M&gt; {
    BulkOrder::V1 {
        order_request,
        order_id,
        unique_priority_idx,
        creation_time_micros
    }
}
</code></pre>



</details>

<a id="0x5_bulk_order_types_new_bulk_order_request"></a>

## Function `new_bulk_order_request`

Creates a new bulk order request with the specified price levels and sizes.


<a id="@Arguments:_11"></a>

### Arguments:

- <code><a href="../../aptos-framework/doc/account.md#0x1_account">account</a></code>: The account placing the order
- <code>bid_prices</code>: Vector of bid prices in descending order
- <code>bid_sizes</code>: Vector of bid sizes corresponding to each price level
- <code>ask_prices</code>: Vector of ask prices in ascending order
- <code>ask_sizes</code>: Vector of ask sizes corresponding to each price level
- <code>metadata</code>: Additional metadata for the order


<a id="@Returns:_12"></a>

### Returns:

A <code><a href="bulk_order_types.md#0x5_bulk_order_types_BulkOrderRequest">BulkOrderRequest</a></code> instance.


<a id="@Aborts:_13"></a>

### Aborts:

- If sequence_number is 0 (reserved to avoid ambiguity in events)
- If bid_prices and bid_sizes have different lengths
- If ask_prices and ask_sizes have different lengths
- If bid_prices or ask_prices exceeds MAX_BULK_ORDER_DEPTH_PER_SIDE (30) levels


<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_types.md#0x5_bulk_order_types_new_bulk_order_request">new_bulk_order_request</a>&lt;M: <b>copy</b>, drop, store&gt;(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>, sequence_number: u64, bid_prices: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, bid_sizes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, ask_prices: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, ask_sizes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, metadata: M): <a href="bulk_order_types.md#0x5_bulk_order_types_BulkOrderRequest">bulk_order_types::BulkOrderRequest</a>&lt;M&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_types.md#0x5_bulk_order_types_new_bulk_order_request">new_bulk_order_request</a>&lt;M: store + <b>copy</b> + drop&gt;(
    <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>,
    sequence_number: u64,
    bid_prices: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    bid_sizes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    ask_prices: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    ask_sizes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    metadata: M
): <a href="bulk_order_types.md#0x5_bulk_order_types_BulkOrderRequest">BulkOrderRequest</a>&lt;M&gt; {
    // Sequence number 0 is reserved <b>to</b> avoid ambiguity in events
    <b>assert</b>!(sequence_number &gt; 0, <a href="bulk_order_types.md#0x5_bulk_order_types_E_INVALID_SEQUENCE_NUMBER">E_INVALID_SEQUENCE_NUMBER</a>);

    <b>let</b> num_bids = bid_prices.length();
    <b>let</b> num_asks = ask_prices.length();

    // Basic length validation
    <b>assert</b>!(num_bids == bid_sizes.length(), <a href="bulk_order_types.md#0x5_bulk_order_types_E_BID_LENGTH_MISMATCH">E_BID_LENGTH_MISMATCH</a>);
    <b>assert</b>!(num_asks == ask_sizes.length(), <a href="bulk_order_types.md#0x5_bulk_order_types_E_ASK_LENGTH_MISMATCH">E_ASK_LENGTH_MISMATCH</a>);
    <b>assert</b>!(num_bids &gt; 0 || num_asks &gt; 0, <a href="bulk_order_types.md#0x5_bulk_order_types_E_EMPTY_ORDER">E_EMPTY_ORDER</a>);
    // Depth validation <b>to</b> prevent gas DoS when cancelling
    <b>assert</b>!(num_bids &lt;= <a href="bulk_order_types.md#0x5_bulk_order_types_MAX_BULK_ORDER_DEPTH_PER_SIDE">MAX_BULK_ORDER_DEPTH_PER_SIDE</a>, <a href="bulk_order_types.md#0x5_bulk_order_types_E_BULK_ORDER_DEPTH_EXCEEDED">E_BULK_ORDER_DEPTH_EXCEEDED</a>);
    <b>assert</b>!(num_asks &lt;= <a href="bulk_order_types.md#0x5_bulk_order_types_MAX_BULK_ORDER_DEPTH_PER_SIDE">MAX_BULK_ORDER_DEPTH_PER_SIDE</a>, <a href="bulk_order_types.md#0x5_bulk_order_types_E_BULK_ORDER_DEPTH_EXCEEDED">E_BULK_ORDER_DEPTH_EXCEEDED</a>);
    <b>assert</b>!(<a href="bulk_order_types.md#0x5_bulk_order_types_validate_not_zero_sizes">validate_not_zero_sizes</a>(&bid_sizes), <a href="bulk_order_types.md#0x5_bulk_order_types_E_BID_SIZE_ZERO">E_BID_SIZE_ZERO</a>);
    <b>assert</b>!(<a href="bulk_order_types.md#0x5_bulk_order_types_validate_not_zero_sizes">validate_not_zero_sizes</a>(&ask_sizes), <a href="bulk_order_types.md#0x5_bulk_order_types_E_ASK_SIZE_ZERO">E_ASK_SIZE_ZERO</a>);
    <b>assert</b>!(<a href="bulk_order_types.md#0x5_bulk_order_types_validate_price_ordering">validate_price_ordering</a>(&bid_prices, <b>true</b>), <a href="bulk_order_types.md#0x5_bulk_order_types_E_BID_ORDER_INVALID">E_BID_ORDER_INVALID</a>);
    <b>assert</b>!(<a href="bulk_order_types.md#0x5_bulk_order_types_validate_price_ordering">validate_price_ordering</a>(&ask_prices, <b>false</b>), <a href="bulk_order_types.md#0x5_bulk_order_types_E_ASK_ORDER_INVALID">E_ASK_ORDER_INVALID</a>);

    <b>if</b> (num_bids &gt; 0 && num_asks &gt; 0) {
        // First element in bids is the highest (descending order), first element in asks is the lowest (ascending
        // order).
        <b>assert</b>!(bid_prices[0] &lt; ask_prices[0], <a href="bulk_order_types.md#0x5_bulk_order_types_EPRICE_CROSSING">EPRICE_CROSSING</a>);
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
    req
}
</code></pre>



</details>

<a id="0x5_bulk_order_types_new_bulk_order_place_response_success"></a>

## Function `new_bulk_order_place_response_success`



<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_types.md#0x5_bulk_order_types_new_bulk_order_place_response_success">new_bulk_order_place_response_success</a>&lt;M: <b>copy</b>, drop, store&gt;(order: <a href="bulk_order_types.md#0x5_bulk_order_types_BulkOrder">bulk_order_types::BulkOrder</a>&lt;M&gt;, cancelled_bid_prices: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, cancelled_bid_sizes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, cancelled_ask_prices: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, cancelled_ask_sizes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, previous_seq_num: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;): <a href="bulk_order_types.md#0x5_bulk_order_types_BulkOrderPlaceResponse">bulk_order_types::BulkOrderPlaceResponse</a>&lt;M&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_types.md#0x5_bulk_order_types_new_bulk_order_place_response_success">new_bulk_order_place_response_success</a>&lt;M: store + <b>copy</b> + drop&gt;(
    order: <a href="bulk_order_types.md#0x5_bulk_order_types_BulkOrder">BulkOrder</a>&lt;M&gt;,
    cancelled_bid_prices: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    cancelled_bid_sizes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    cancelled_ask_prices: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    cancelled_ask_sizes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    previous_seq_num: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;
): <a href="bulk_order_types.md#0x5_bulk_order_types_BulkOrderPlaceResponse">BulkOrderPlaceResponse</a>&lt;M&gt; {
    BulkOrderPlaceResponse::Success_V1 {
        order,
        cancelled_bid_prices,
        cancelled_bid_sizes,
        cancelled_ask_prices,
        cancelled_ask_sizes,
        previous_seq_num
    }
}
</code></pre>



</details>

<a id="0x5_bulk_order_types_new_bulk_order_place_response_rejection"></a>

## Function `new_bulk_order_place_response_rejection`



<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_types.md#0x5_bulk_order_types_new_bulk_order_place_response_rejection">new_bulk_order_place_response_rejection</a>&lt;M: <b>copy</b>, drop, store&gt;(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>, sequence_number: u64, existing_sequence_number: u64): <a href="bulk_order_types.md#0x5_bulk_order_types_BulkOrderPlaceResponse">bulk_order_types::BulkOrderPlaceResponse</a>&lt;M&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_types.md#0x5_bulk_order_types_new_bulk_order_place_response_rejection">new_bulk_order_place_response_rejection</a>&lt;M: store + <b>copy</b> + drop&gt;(
    <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>, sequence_number: u64, existing_sequence_number: u64
): <a href="bulk_order_types.md#0x5_bulk_order_types_BulkOrderPlaceResponse">BulkOrderPlaceResponse</a>&lt;M&gt; {
    BulkOrderPlaceResponse::Rejection_V1 {
        <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>,
        sequence_number,
        existing_sequence_number
    }
}
</code></pre>



</details>

<a id="0x5_bulk_order_types_get_unique_priority_idx"></a>

## Function `get_unique_priority_idx`

Gets the unique priority index of a bulk order.


<a id="@Arguments:_14"></a>

### Arguments:

- <code>self</code>: Reference to the bulk order


<a id="@Returns:_15"></a>

### Returns:

The unique priority index for time-based ordering.


<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_types.md#0x5_bulk_order_types_get_unique_priority_idx">get_unique_priority_idx</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="bulk_order_types.md#0x5_bulk_order_types_BulkOrder">bulk_order_types::BulkOrder</a>&lt;M&gt;): <a href="order_book_types.md#0x5_order_book_types_IncreasingIdx">order_book_types::IncreasingIdx</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_types.md#0x5_bulk_order_types_get_unique_priority_idx">get_unique_priority_idx</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="bulk_order_types.md#0x5_bulk_order_types_BulkOrder">BulkOrder</a>&lt;M&gt;
): IncreasingIdx {
    self.unique_priority_idx
}
</code></pre>



</details>

<a id="0x5_bulk_order_types_get_order_id"></a>

## Function `get_order_id`

Gets the order ID of a bulk order.


<a id="@Arguments:_16"></a>

### Arguments:

- <code>self</code>: Reference to the bulk order


<a id="@Returns:_17"></a>

### Returns:

The unique order identifier.


<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_types.md#0x5_bulk_order_types_get_order_id">get_order_id</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="bulk_order_types.md#0x5_bulk_order_types_BulkOrder">bulk_order_types::BulkOrder</a>&lt;M&gt;): <a href="order_book_types.md#0x5_order_book_types_OrderId">order_book_types::OrderId</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_types.md#0x5_bulk_order_types_get_order_id">get_order_id</a>&lt;M: store + <b>copy</b> + drop&gt;(self: &<a href="bulk_order_types.md#0x5_bulk_order_types_BulkOrder">BulkOrder</a>&lt;M&gt;): OrderId {
    self.order_id
}
</code></pre>



</details>

<a id="0x5_bulk_order_types_get_creation_time_micros"></a>

## Function `get_creation_time_micros`



<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_types.md#0x5_bulk_order_types_get_creation_time_micros">get_creation_time_micros</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="bulk_order_types.md#0x5_bulk_order_types_BulkOrder">bulk_order_types::BulkOrder</a>&lt;M&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_types.md#0x5_bulk_order_types_get_creation_time_micros">get_creation_time_micros</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="bulk_order_types.md#0x5_bulk_order_types_BulkOrder">BulkOrder</a>&lt;M&gt;
): u64 {
    self.creation_time_micros
}
</code></pre>



</details>

<a id="0x5_bulk_order_types_get_order_request"></a>

## Function `get_order_request`



<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_types.md#0x5_bulk_order_types_get_order_request">get_order_request</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="bulk_order_types.md#0x5_bulk_order_types_BulkOrder">bulk_order_types::BulkOrder</a>&lt;M&gt;): &<a href="bulk_order_types.md#0x5_bulk_order_types_BulkOrderRequest">bulk_order_types::BulkOrderRequest</a>&lt;M&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_types.md#0x5_bulk_order_types_get_order_request">get_order_request</a>&lt;M: store + <b>copy</b> + drop&gt;(self: &<a href="bulk_order_types.md#0x5_bulk_order_types_BulkOrder">BulkOrder</a>&lt;M&gt;)
    : &<a href="bulk_order_types.md#0x5_bulk_order_types_BulkOrderRequest">BulkOrderRequest</a>&lt;M&gt; {
    &self.order_request
}
</code></pre>



</details>

<a id="0x5_bulk_order_types_get_order_request_mut"></a>

## Function `get_order_request_mut`



<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_types.md#0x5_bulk_order_types_get_order_request_mut">get_order_request_mut</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="bulk_order_types.md#0x5_bulk_order_types_BulkOrder">bulk_order_types::BulkOrder</a>&lt;M&gt;): &<b>mut</b> <a href="bulk_order_types.md#0x5_bulk_order_types_BulkOrderRequest">bulk_order_types::BulkOrderRequest</a>&lt;M&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_types.md#0x5_bulk_order_types_get_order_request_mut">get_order_request_mut</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="bulk_order_types.md#0x5_bulk_order_types_BulkOrder">BulkOrder</a>&lt;M&gt;
): &<b>mut</b> <a href="bulk_order_types.md#0x5_bulk_order_types_BulkOrderRequest">BulkOrderRequest</a>&lt;M&gt; {
    &<b>mut</b> self.order_request
}
</code></pre>



</details>

<a id="0x5_bulk_order_types_get_account"></a>

## Function `get_account`



<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_types.md#0x5_bulk_order_types_get_account">get_account</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="bulk_order_types.md#0x5_bulk_order_types_BulkOrderRequest">bulk_order_types::BulkOrderRequest</a>&lt;M&gt;): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_types.md#0x5_bulk_order_types_get_account">get_account</a>&lt;M: store + <b>copy</b> + drop&gt;(self: &<a href="bulk_order_types.md#0x5_bulk_order_types_BulkOrderRequest">BulkOrderRequest</a>&lt;M&gt;): <b>address</b> {
    self.<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>
}
</code></pre>



</details>

<a id="0x5_bulk_order_types_get_sequence_number"></a>

## Function `get_sequence_number`



<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_types.md#0x5_bulk_order_types_get_sequence_number">get_sequence_number</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="bulk_order_types.md#0x5_bulk_order_types_BulkOrderRequest">bulk_order_types::BulkOrderRequest</a>&lt;M&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_types.md#0x5_bulk_order_types_get_sequence_number">get_sequence_number</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="bulk_order_types.md#0x5_bulk_order_types_BulkOrderRequest">BulkOrderRequest</a>&lt;M&gt;
): u64 {
    self.order_sequence_number
}
</code></pre>



</details>

<a id="0x5_bulk_order_types_get_total_remaining_size"></a>

## Function `get_total_remaining_size`



<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_types.md#0x5_bulk_order_types_get_total_remaining_size">get_total_remaining_size</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="bulk_order_types.md#0x5_bulk_order_types_BulkOrderRequest">bulk_order_types::BulkOrderRequest</a>&lt;M&gt;, is_bid: bool): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_types.md#0x5_bulk_order_types_get_total_remaining_size">get_total_remaining_size</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="bulk_order_types.md#0x5_bulk_order_types_BulkOrderRequest">BulkOrderRequest</a>&lt;M&gt;, is_bid: bool
): u64 {
    <b>if</b> (is_bid) {
        self.bid_sizes.fold(0, |acc, size| acc + size)
    } <b>else</b> {
        self.ask_sizes.fold(0, |acc, size| acc + size)
    }
}
</code></pre>



</details>

<a id="0x5_bulk_order_types_get_active_price"></a>

## Function `get_active_price`

Gets the active price for a specific side of a bulk order.


<a id="@Arguments:_18"></a>

### Arguments:

- <code>self</code>: Reference to the bulk order
- <code>is_bid</code>: True to get bid price, false for ask price


<a id="@Returns:_19"></a>

### Returns:

An option containing the active price if available, none otherwise.


<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_types.md#0x5_bulk_order_types_get_active_price">get_active_price</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="bulk_order_types.md#0x5_bulk_order_types_BulkOrderRequest">bulk_order_types::BulkOrderRequest</a>&lt;M&gt;, is_bid: bool): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_types.md#0x5_bulk_order_types_get_active_price">get_active_price</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="bulk_order_types.md#0x5_bulk_order_types_BulkOrderRequest">BulkOrderRequest</a>&lt;M&gt;, is_bid: bool
): Option&lt;u64&gt; {
    <b>let</b> prices =
        <b>if</b> (is_bid) {
            &self.bid_prices
        } <b>else</b> {
            &self.ask_prices
        };
    <b>if</b> (prices.length() == 0) {
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>() // No active price level
    } <b>else</b> {
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(prices[0]) // Return the first price level
    }
}
</code></pre>



</details>

<a id="0x5_bulk_order_types_get_all_prices"></a>

## Function `get_all_prices`



<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_types.md#0x5_bulk_order_types_get_all_prices">get_all_prices</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="bulk_order_types.md#0x5_bulk_order_types_BulkOrderRequest">bulk_order_types::BulkOrderRequest</a>&lt;M&gt;, is_bid: bool): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_types.md#0x5_bulk_order_types_get_all_prices">get_all_prices</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="bulk_order_types.md#0x5_bulk_order_types_BulkOrderRequest">BulkOrderRequest</a>&lt;M&gt;, is_bid: bool
): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt; {
    <b>if</b> (is_bid) {
        self.bid_prices
    } <b>else</b> {
        self.ask_prices
    }
}
</code></pre>



</details>

<a id="0x5_bulk_order_types_get_all_prices_mut"></a>

## Function `get_all_prices_mut`



<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_types.md#0x5_bulk_order_types_get_all_prices_mut">get_all_prices_mut</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="bulk_order_types.md#0x5_bulk_order_types_BulkOrderRequest">bulk_order_types::BulkOrderRequest</a>&lt;M&gt;, is_bid: bool): &<b>mut</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_types.md#0x5_bulk_order_types_get_all_prices_mut">get_all_prices_mut</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="bulk_order_types.md#0x5_bulk_order_types_BulkOrderRequest">BulkOrderRequest</a>&lt;M&gt;, is_bid: bool
): &<b>mut</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt; {
    <b>if</b> (is_bid) {
        &<b>mut</b> self.bid_prices
    } <b>else</b> {
        &<b>mut</b> self.ask_prices
    }
}
</code></pre>



</details>

<a id="0x5_bulk_order_types_get_all_sizes"></a>

## Function `get_all_sizes`



<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_types.md#0x5_bulk_order_types_get_all_sizes">get_all_sizes</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="bulk_order_types.md#0x5_bulk_order_types_BulkOrderRequest">bulk_order_types::BulkOrderRequest</a>&lt;M&gt;, is_bid: bool): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_types.md#0x5_bulk_order_types_get_all_sizes">get_all_sizes</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="bulk_order_types.md#0x5_bulk_order_types_BulkOrderRequest">BulkOrderRequest</a>&lt;M&gt;, is_bid: bool
): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt; {
    <b>if</b> (is_bid) {
        self.bid_sizes
    } <b>else</b> {
        self.ask_sizes
    }
}
</code></pre>



</details>

<a id="0x5_bulk_order_types_get_all_sizes_mut"></a>

## Function `get_all_sizes_mut`



<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_types.md#0x5_bulk_order_types_get_all_sizes_mut">get_all_sizes_mut</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="bulk_order_types.md#0x5_bulk_order_types_BulkOrderRequest">bulk_order_types::BulkOrderRequest</a>&lt;M&gt;, is_bid: bool): &<b>mut</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_types.md#0x5_bulk_order_types_get_all_sizes_mut">get_all_sizes_mut</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="bulk_order_types.md#0x5_bulk_order_types_BulkOrderRequest">BulkOrderRequest</a>&lt;M&gt;, is_bid: bool
): &<b>mut</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt; {
    <b>if</b> (is_bid) {
        &<b>mut</b> self.bid_sizes
    } <b>else</b> {
        &<b>mut</b> self.ask_sizes
    }
}
</code></pre>



</details>

<a id="0x5_bulk_order_types_get_active_size"></a>

## Function `get_active_size`

Gets the active size for a specific side of a bulk order.


<a id="@Arguments:_20"></a>

### Arguments:

- <code>self</code>: Reference to the bulk order
- <code>is_bid</code>: True to get bid size, false for ask size


<a id="@Returns:_21"></a>

### Returns:

An option containing the active size if available, none otherwise.


<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_types.md#0x5_bulk_order_types_get_active_size">get_active_size</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="bulk_order_types.md#0x5_bulk_order_types_BulkOrderRequest">bulk_order_types::BulkOrderRequest</a>&lt;M&gt;, is_bid: bool): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_types.md#0x5_bulk_order_types_get_active_size">get_active_size</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="bulk_order_types.md#0x5_bulk_order_types_BulkOrderRequest">BulkOrderRequest</a>&lt;M&gt;, is_bid: bool
): Option&lt;u64&gt; {
    <b>let</b> sizes =
        <b>if</b> (is_bid) {
            &self.bid_sizes
        } <b>else</b> {
            &self.ask_sizes
        };
    <b>if</b> (sizes.length() == 0) {
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>() // No active size level
    } <b>else</b> {
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(sizes[0]) // Return the first size level
    }
}
</code></pre>



</details>

<a id="0x5_bulk_order_types_get_prices_and_sizes_mut"></a>

## Function `get_prices_and_sizes_mut`



<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_types.md#0x5_bulk_order_types_get_prices_and_sizes_mut">get_prices_and_sizes_mut</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="bulk_order_types.md#0x5_bulk_order_types_BulkOrderRequest">bulk_order_types::BulkOrderRequest</a>&lt;M&gt;, is_bid: bool): (&<b>mut</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, &<b>mut</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_types.md#0x5_bulk_order_types_get_prices_and_sizes_mut">get_prices_and_sizes_mut</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="bulk_order_types.md#0x5_bulk_order_types_BulkOrderRequest">BulkOrderRequest</a>&lt;M&gt;, is_bid: bool
): (&<b>mut</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, &<b>mut</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;) {
    <b>if</b> (is_bid) {
        (&<b>mut</b> self.bid_prices, &<b>mut</b> self.bid_sizes)
    } <b>else</b> {
        (&<b>mut</b> self.ask_prices, &<b>mut</b> self.ask_sizes)
    }
}
</code></pre>



</details>

<a id="0x5_bulk_order_types_is_success_response"></a>

## Function `is_success_response`



<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_types.md#0x5_bulk_order_types_is_success_response">is_success_response</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="bulk_order_types.md#0x5_bulk_order_types_BulkOrderPlaceResponse">bulk_order_types::BulkOrderPlaceResponse</a>&lt;M&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_types.md#0x5_bulk_order_types_is_success_response">is_success_response</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="bulk_order_types.md#0x5_bulk_order_types_BulkOrderPlaceResponse">BulkOrderPlaceResponse</a>&lt;M&gt;
): bool {
    self is BulkOrderPlaceResponse::Success_V1
}
</code></pre>



</details>

<a id="0x5_bulk_order_types_is_rejection_response"></a>

## Function `is_rejection_response`



<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_types.md#0x5_bulk_order_types_is_rejection_response">is_rejection_response</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="bulk_order_types.md#0x5_bulk_order_types_BulkOrderPlaceResponse">bulk_order_types::BulkOrderPlaceResponse</a>&lt;M&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_types.md#0x5_bulk_order_types_is_rejection_response">is_rejection_response</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="bulk_order_types.md#0x5_bulk_order_types_BulkOrderPlaceResponse">BulkOrderPlaceResponse</a>&lt;M&gt;
): bool {
    self is BulkOrderPlaceResponse::Rejection_V1
}
</code></pre>



</details>

<a id="0x5_bulk_order_types_destroy_bulk_order_place_response_success"></a>

## Function `destroy_bulk_order_place_response_success`



<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_types.md#0x5_bulk_order_types_destroy_bulk_order_place_response_success">destroy_bulk_order_place_response_success</a>&lt;M: <b>copy</b>, drop, store&gt;(self: <a href="bulk_order_types.md#0x5_bulk_order_types_BulkOrderPlaceResponse">bulk_order_types::BulkOrderPlaceResponse</a>&lt;M&gt;): (<a href="bulk_order_types.md#0x5_bulk_order_types_BulkOrder">bulk_order_types::BulkOrder</a>&lt;M&gt;, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_types.md#0x5_bulk_order_types_destroy_bulk_order_place_response_success">destroy_bulk_order_place_response_success</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: <a href="bulk_order_types.md#0x5_bulk_order_types_BulkOrderPlaceResponse">BulkOrderPlaceResponse</a>&lt;M&gt;
): (
    <a href="bulk_order_types.md#0x5_bulk_order_types_BulkOrder">BulkOrder</a>&lt;M&gt;,
    <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;
) {
    <b>let</b> BulkOrderPlaceResponse::Success_V1 {
        order,
        cancelled_bid_prices,
        cancelled_bid_sizes,
        cancelled_ask_prices,
        cancelled_ask_sizes,
        previous_seq_num
    } = self;
    (
        order,
        cancelled_bid_prices,
        cancelled_bid_sizes,
        cancelled_ask_prices,
        cancelled_ask_sizes,
        previous_seq_num
    )
}
</code></pre>



</details>

<a id="0x5_bulk_order_types_destroy_bulk_order_place_response_rejection"></a>

## Function `destroy_bulk_order_place_response_rejection`



<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_types.md#0x5_bulk_order_types_destroy_bulk_order_place_response_rejection">destroy_bulk_order_place_response_rejection</a>&lt;M: <b>copy</b>, drop, store&gt;(self: <a href="bulk_order_types.md#0x5_bulk_order_types_BulkOrderPlaceResponse">bulk_order_types::BulkOrderPlaceResponse</a>&lt;M&gt;): (<b>address</b>, u64, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_types.md#0x5_bulk_order_types_destroy_bulk_order_place_response_rejection">destroy_bulk_order_place_response_rejection</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: <a href="bulk_order_types.md#0x5_bulk_order_types_BulkOrderPlaceResponse">BulkOrderPlaceResponse</a>&lt;M&gt;
): (<b>address</b>, u64, u64) {
    <b>let</b> BulkOrderPlaceResponse::Rejection_V1 {
        <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>,
        sequence_number,
        existing_sequence_number
    } = self;
    (<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>, sequence_number, existing_sequence_number)
}
</code></pre>



</details>

<a id="0x5_bulk_order_types_validate_not_zero_sizes"></a>

## Function `validate_not_zero_sizes`

Validates that all sizes in the vector are greater than 0.


<a id="@Arguments:_22"></a>

### Arguments:

- <code>sizes</code>: Vector of sizes to validate


<pre><code><b>fun</b> <a href="bulk_order_types.md#0x5_bulk_order_types_validate_not_zero_sizes">validate_not_zero_sizes</a>(sizes: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="bulk_order_types.md#0x5_bulk_order_types_validate_not_zero_sizes">validate_not_zero_sizes</a>(sizes: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;): bool {
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

<a id="0x5_bulk_order_types_validate_price_ordering"></a>

## Function `validate_price_ordering`

Validates that prices are in the correct order (descending for bids, ascending for asks).


<a id="@Arguments:_23"></a>

### Arguments:

- <code>prices</code>: Vector of prices to validate
- <code>is_descending</code>: True if prices should be in descending order, false for ascending


<pre><code><b>fun</b> <a href="bulk_order_types.md#0x5_bulk_order_types_validate_price_ordering">validate_price_ordering</a>(prices: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, is_descending: bool): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="bulk_order_types.md#0x5_bulk_order_types_validate_price_ordering">validate_price_ordering</a>(
    prices: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, is_descending: bool
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

<a id="0x5_bulk_order_types_new_bulk_order_match"></a>

## Function `new_bulk_order_match`



<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_types.md#0x5_bulk_order_types_new_bulk_order_match">new_bulk_order_match</a>&lt;M: <b>copy</b>, drop, store&gt;(order: &<a href="bulk_order_types.md#0x5_bulk_order_types_BulkOrder">bulk_order_types::BulkOrder</a>&lt;M&gt;, is_bid: bool, matched_size: u64): <a href="order_match_types.md#0x5_order_match_types_OrderMatch">order_match_types::OrderMatch</a>&lt;M&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_types.md#0x5_bulk_order_types_new_bulk_order_match">new_bulk_order_match</a>&lt;M: store + <b>copy</b> + drop&gt;(
    order: &<a href="bulk_order_types.md#0x5_bulk_order_types_BulkOrder">BulkOrder</a>&lt;M&gt;, is_bid: bool, matched_size: u64
): OrderMatch&lt;M&gt; {
    <b>let</b> order_request = &order.order_request;
    <b>let</b> (price, remaining_size) =
        <b>if</b> (is_bid) {
            (order_request.bid_prices[0], order_request.bid_sizes[0] - matched_size)
        } <b>else</b> {
            (order_request.ask_prices[0], order_request.ask_sizes[0] - matched_size)
        };
    new_order_match&lt;M&gt;(
        new_bulk_order_match_details&lt;M&gt;(
            order.order_id,
            order_request.<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>,
            order.unique_priority_idx,
            price,
            remaining_size,
            is_bid,
            order_request.order_sequence_number,
            order.creation_time_micros,
            order_request.metadata
        ),
        matched_size
    )
}
</code></pre>



</details>

<a id="0x5_bulk_order_types_set_empty"></a>

## Function `set_empty`

Sets the bulk order to empty state by clearing all sizes.

This function is used during order cancellation to clear the order state
while preserving the order ID for potential reuse.


<a id="@Arguments:_24"></a>

### Arguments:

- <code>self</code>: Mutable reference to the bulk order


<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_types.md#0x5_bulk_order_types_set_empty">set_empty</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="bulk_order_types.md#0x5_bulk_order_types_BulkOrder">bulk_order_types::BulkOrder</a>&lt;M&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_types.md#0x5_bulk_order_types_set_empty">set_empty</a>&lt;M: store + <b>copy</b> + drop&gt;(self: &<b>mut</b> <a href="bulk_order_types.md#0x5_bulk_order_types_BulkOrder">BulkOrder</a>&lt;M&gt;) {
    self.order_request.bid_sizes = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>();
    self.order_request.ask_sizes = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>();
    self.order_request.bid_prices = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>();
    self.order_request.ask_prices = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>();
}
</code></pre>



</details>

<a id="0x5_bulk_order_types_destroy_bulk_order"></a>

## Function `destroy_bulk_order`



<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_types.md#0x5_bulk_order_types_destroy_bulk_order">destroy_bulk_order</a>&lt;M: <b>copy</b>, drop, store&gt;(self: <a href="bulk_order_types.md#0x5_bulk_order_types_BulkOrder">bulk_order_types::BulkOrder</a>&lt;M&gt;): (<a href="bulk_order_types.md#0x5_bulk_order_types_BulkOrderRequest">bulk_order_types::BulkOrderRequest</a>&lt;M&gt;, <a href="order_book_types.md#0x5_order_book_types_OrderId">order_book_types::OrderId</a>, <a href="order_book_types.md#0x5_order_book_types_IncreasingIdx">order_book_types::IncreasingIdx</a>, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_types.md#0x5_bulk_order_types_destroy_bulk_order">destroy_bulk_order</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: <a href="bulk_order_types.md#0x5_bulk_order_types_BulkOrder">BulkOrder</a>&lt;M&gt;
): (<a href="bulk_order_types.md#0x5_bulk_order_types_BulkOrderRequest">BulkOrderRequest</a>&lt;M&gt;, OrderId, IncreasingIdx, u64) {
    <b>let</b> BulkOrder::V1 {
        order_request,
        order_id,
        unique_priority_idx,
        creation_time_micros
    } = self;
    (order_request, order_id, unique_priority_idx, creation_time_micros)
}
</code></pre>



</details>

<a id="0x5_bulk_order_types_destroy_bulk_order_request"></a>

## Function `destroy_bulk_order_request`



<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_types.md#0x5_bulk_order_types_destroy_bulk_order_request">destroy_bulk_order_request</a>&lt;M: <b>copy</b>, drop, store&gt;(self: <a href="bulk_order_types.md#0x5_bulk_order_types_BulkOrderRequest">bulk_order_types::BulkOrderRequest</a>&lt;M&gt;): (<b>address</b>, u64, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, M)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bulk_order_types.md#0x5_bulk_order_types_destroy_bulk_order_request">destroy_bulk_order_request</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: <a href="bulk_order_types.md#0x5_bulk_order_types_BulkOrderRequest">BulkOrderRequest</a>&lt;M&gt;
): (<b>address</b>, u64, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, M) {
    <b>let</b> BulkOrderRequest::V1 {
        <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>,
        order_sequence_number,
        bid_prices,
        bid_sizes,
        ask_prices,
        ask_sizes,
        metadata
    } = self;
    (
        <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>,
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
