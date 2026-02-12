
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


<a id="@Usage_Example:_4"></a>

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
    -  [Usage Example:](#@Usage_Example:_4)
-  [Enum `BulkOrderRequest`](#0x5_bulk_order_types_BulkOrderRequest)
    -  [Fields:](#@Fields:_5)
    -  [Validation:](#@Validation:_6)
-  [Enum `BulkOrder`](#0x5_bulk_order_types_BulkOrder)
    -  [Fields:](#@Fields:_7)
-  [Enum `BulkOrderPlaceResponse`](#0x5_bulk_order_types_BulkOrderPlaceResponse)
-  [Function `new_bulk_order`](#0x5_bulk_order_types_new_bulk_order)
    -  [Arguments:](#@Arguments:_8)
-  [Function `new_bulk_order_request`](#0x5_bulk_order_types_new_bulk_order_request)
    -  [Arguments:](#@Arguments:_9)
    -  [Returns:](#@Returns:_10)
-  [Function `new_bulk_order_place_response_success`](#0x5_bulk_order_types_new_bulk_order_place_response_success)
-  [Function `new_bulk_order_place_response_rejection`](#0x5_bulk_order_types_new_bulk_order_place_response_rejection)
-  [Function `get_unique_priority_idx`](#0x5_bulk_order_types_get_unique_priority_idx)
    -  [Arguments:](#@Arguments:_11)
    -  [Returns:](#@Returns:_12)
-  [Function `get_order_id`](#0x5_bulk_order_types_get_order_id)
    -  [Arguments:](#@Arguments:_13)
    -  [Returns:](#@Returns:_14)
-  [Function `get_creation_time_micros`](#0x5_bulk_order_types_get_creation_time_micros)
-  [Function `get_order_request`](#0x5_bulk_order_types_get_order_request)
-  [Function `get_order_request_mut`](#0x5_bulk_order_types_get_order_request_mut)
-  [Function `get_account`](#0x5_bulk_order_types_get_account)
-  [Function `get_sequence_number`](#0x5_bulk_order_types_get_sequence_number)
-  [Function `get_total_remaining_size`](#0x5_bulk_order_types_get_total_remaining_size)
-  [Function `get_active_price`](#0x5_bulk_order_types_get_active_price)
    -  [Arguments:](#@Arguments:_15)
    -  [Returns:](#@Returns:_16)
-  [Function `get_all_prices`](#0x5_bulk_order_types_get_all_prices)
-  [Function `get_all_prices_mut`](#0x5_bulk_order_types_get_all_prices_mut)
-  [Function `get_all_sizes`](#0x5_bulk_order_types_get_all_sizes)
-  [Function `get_all_sizes_mut`](#0x5_bulk_order_types_get_all_sizes_mut)
-  [Function `get_active_size`](#0x5_bulk_order_types_get_active_size)
    -  [Arguments:](#@Arguments:_17)
    -  [Returns:](#@Returns:_18)
-  [Function `get_prices_and_sizes_mut`](#0x5_bulk_order_types_get_prices_and_sizes_mut)
-  [Function `is_success_response`](#0x5_bulk_order_types_is_success_response)
-  [Function `is_rejection_response`](#0x5_bulk_order_types_is_rejection_response)
-  [Function `destroy_bulk_order_place_response_success`](#0x5_bulk_order_types_destroy_bulk_order_place_response_success)
-  [Function `destroy_bulk_order_place_response_rejection`](#0x5_bulk_order_types_destroy_bulk_order_place_response_rejection)
-  [Function `new_bulk_order_match`](#0x5_bulk_order_types_new_bulk_order_match)
-  [Function `set_empty`](#0x5_bulk_order_types_set_empty)
    -  [Arguments:](#@Arguments:_19)
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


<a id="@Fields:_5"></a>

### Fields:

- <code><a href="../../aptos-framework/doc/account.md#0x1_account">account</a></code>: The account placing the order
- <code>bid_prices</code>: Vector of bid prices in descending order (best price first)
- <code>bid_sizes</code>: Vector of bid sizes corresponding to each price level
- <code>ask_prices</code>: Vector of ask prices in ascending order (best price first)
- <code>ask_sizes</code>: Vector of ask sizes corresponding to each price level
- <code>metadata</code>: Additional metadata for the order


<a id="@Validation:_6"></a>

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


<a id="@Fields:_7"></a>

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

<a id="0x5_bulk_order_types_new_bulk_order"></a>

## Function `new_bulk_order`

Creates a new bulk order with the specified parameters.


<a id="@Arguments:_8"></a>

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


<a id="@Arguments:_9"></a>

### Arguments:

- <code><a href="../../aptos-framework/doc/account.md#0x1_account">account</a></code>: The account placing the order
- <code>bid_prices</code>: Vector of bid prices in descending order
- <code>bid_sizes</code>: Vector of bid sizes corresponding to each price level
- <code>ask_prices</code>: Vector of ask prices in ascending order
- <code>ask_sizes</code>: Vector of ask sizes corresponding to each price level
- <code>metadata</code>: Additional metadata for the order


<a id="@Returns:_10"></a>

### Returns:

A <code><a href="bulk_order_types.md#0x5_bulk_order_types_BulkOrderRequest">BulkOrderRequest</a></code> instance.

Does no validation itself.


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
    BulkOrderRequest::V1 {
        <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>,
        order_sequence_number: sequence_number,
        bid_prices,
        bid_sizes,
        ask_prices,
        ask_sizes,
        metadata
    }
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


<a id="@Arguments:_11"></a>

### Arguments:

- <code>self</code>: Reference to the bulk order


<a id="@Returns:_12"></a>

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


<a id="@Arguments:_13"></a>

### Arguments:

- <code>self</code>: Reference to the bulk order


<a id="@Returns:_14"></a>

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


<a id="@Arguments:_15"></a>

### Arguments:

- <code>self</code>: Reference to the bulk order
- <code>is_bid</code>: True to get bid price, false for ask price


<a id="@Returns:_16"></a>

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


<a id="@Arguments:_17"></a>

### Arguments:

- <code>self</code>: Reference to the bulk order
- <code>is_bid</code>: True to get bid size, false for ask size


<a id="@Returns:_18"></a>

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


<a id="@Arguments:_19"></a>

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
