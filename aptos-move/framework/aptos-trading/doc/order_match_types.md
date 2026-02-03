
<a id="0x5_order_match_types"></a>

# Module `0x5::order_match_types`



-  [Enum `OrderMatchDetails`](#0x5_order_match_types_OrderMatchDetails)
    -  [Fields:](#@Fields:_0)
-  [Enum `OrderMatch`](#0x5_order_match_types_OrderMatch)
    -  [Fields:](#@Fields:_1)
-  [Enum `ActiveMatchedOrder`](#0x5_order_match_types_ActiveMatchedOrder)
-  [Constants](#@Constants_2)
-  [Function `new_single_order_match_details`](#0x5_order_match_types_new_single_order_match_details)
-  [Function `new_bulk_order_match_details`](#0x5_order_match_types_new_bulk_order_match_details)
-  [Function `new_order_match`](#0x5_order_match_types_new_order_match)
-  [Function `new_order_match_details_with_modified_size`](#0x5_order_match_types_new_order_match_details_with_modified_size)
-  [Function `new_active_matched_order`](#0x5_order_match_types_new_active_matched_order)
-  [Function `get_matched_size`](#0x5_order_match_types_get_matched_size)
-  [Function `get_account_from_match_details`](#0x5_order_match_types_get_account_from_match_details)
-  [Function `get_order_id_from_match_details`](#0x5_order_match_types_get_order_id_from_match_details)
-  [Function `get_unique_priority_idx_from_match_details`](#0x5_order_match_types_get_unique_priority_idx_from_match_details)
-  [Function `get_price_from_match_details`](#0x5_order_match_types_get_price_from_match_details)
-  [Function `get_orig_size_from_match_details`](#0x5_order_match_types_get_orig_size_from_match_details)
-  [Function `get_remaining_size_from_match_details`](#0x5_order_match_types_get_remaining_size_from_match_details)
-  [Function `get_time_in_force_from_match_details`](#0x5_order_match_types_get_time_in_force_from_match_details)
-  [Function `get_metadata_from_match_details`](#0x5_order_match_types_get_metadata_from_match_details)
-  [Function `get_client_order_id_from_match_details`](#0x5_order_match_types_get_client_order_id_from_match_details)
-  [Function `is_bid_from_match_details`](#0x5_order_match_types_is_bid_from_match_details)
-  [Function `get_book_type_from_match_details`](#0x5_order_match_types_get_book_type_from_match_details)
-  [Function `is_bulk_order_from_match_details`](#0x5_order_match_types_is_bulk_order_from_match_details)
-  [Function `is_single_order_from_match_details`](#0x5_order_match_types_is_single_order_from_match_details)
-  [Function `get_sequence_number_from_match_details`](#0x5_order_match_types_get_sequence_number_from_match_details)
-  [Function `get_creation_time_micros_from_match_details`](#0x5_order_match_types_get_creation_time_micros_from_match_details)
-  [Function `destroy_order_match`](#0x5_order_match_types_destroy_order_match)
-  [Function `destroy_single_order_match_details`](#0x5_order_match_types_destroy_single_order_match_details)
-  [Function `validate_single_order_reinsertion_request`](#0x5_order_match_types_validate_single_order_reinsertion_request)
-  [Function `validate_bulk_order_reinsertion_request`](#0x5_order_match_types_validate_bulk_order_reinsertion_request)
-  [Function `destroy_active_matched_order`](#0x5_order_match_types_destroy_active_matched_order)
-  [Function `is_active_matched_book_type_single_order`](#0x5_order_match_types_is_active_matched_book_type_single_order)


<pre><code><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;
<b>use</b> <a href="order_book_types.md#0x5_order_book_types">0x5::order_book_types</a>;
</code></pre>



<a id="0x5_order_match_types_OrderMatchDetails"></a>

## Enum `OrderMatchDetails`

Represents the details of a matched order.

Contains information about an order that was matched, including its
identifier, account, priority index, price, sizes, and side.


<a id="@Fields:_0"></a>

### Fields:

- <code>order_id</code>: Unique identifier for the order
- <code><a href="../../aptos-framework/doc/account.md#0x1_account">account</a></code>: Account that placed the order
- <code>unique_priority_idx</code>: Priority index for time-based ordering
- <code>price</code>: Price at which the order was matched
- <code>orig_size</code>: Original size of the order
- <code>remaining_size</code>: Remaining size after the match
- <code>is_bid</code>: True if this was a bid order, false if ask order


<pre><code>enum <a href="order_match_types.md#0x5_order_match_types_OrderMatchDetails">OrderMatchDetails</a>&lt;M: <b>copy</b>, drop, store&gt; <b>has</b> <b>copy</b>, drop
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>SingleOrder</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>order_id: <a href="order_book_types.md#0x5_order_book_types_OrderId">order_book_types::OrderId</a></code>
</dt>
<dd>

</dd>
<dt>
<code><a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>client_order_id: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>unique_priority_idx: <a href="order_book_types.md#0x5_order_book_types_IncreasingIdx">order_book_types::IncreasingIdx</a></code>
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
<code>time_in_force: <a href="order_book_types.md#0x5_order_book_types_TimeInForce">order_book_types::TimeInForce</a></code>
</dt>
<dd>

</dd>
<dt>
<code>creation_time_micros: u64</code>
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

<details>
<summary>BulkOrder</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>order_id: <a href="order_book_types.md#0x5_order_book_types_OrderId">order_book_types::OrderId</a></code>
</dt>
<dd>

</dd>
<dt>
<code><a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>unique_priority_idx: <a href="order_book_types.md#0x5_order_book_types_IncreasingIdx">order_book_types::IncreasingIdx</a></code>
</dt>
<dd>

</dd>
<dt>
<code>price: u64</code>
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
<code>sequence_number: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>creation_time_micros: u64</code>
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

<a id="0x5_order_match_types_OrderMatch"></a>

## Enum `OrderMatch`

Represents a single match between a taker order and a maker order.

Contains the matched order details and the size that was matched in this
particular match operation.


<a id="@Fields:_1"></a>

### Fields:

- <code>order</code>: The matched order result
- <code>matched_size</code>: The size that was matched in this operation


<pre><code>enum <a href="order_match_types.md#0x5_order_match_types_OrderMatch">OrderMatch</a>&lt;M: <b>copy</b>, drop, store&gt; <b>has</b> <b>copy</b>, drop
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>V1</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>order: <a href="order_match_types.md#0x5_order_match_types_OrderMatchDetails">order_match_types::OrderMatchDetails</a>&lt;M&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>matched_size: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

</details>

<a id="0x5_order_match_types_ActiveMatchedOrder"></a>

## Enum `ActiveMatchedOrder`



<pre><code>enum <a href="order_match_types.md#0x5_order_match_types_ActiveMatchedOrder">ActiveMatchedOrder</a> <b>has</b> <b>copy</b>, drop
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>V1</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>order_id: <a href="order_book_types.md#0x5_order_book_types_OrderId">order_book_types::OrderId</a></code>
</dt>
<dd>

</dd>
<dt>
<code>matched_size: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>remaining_size: u64</code>
</dt>
<dd>
 Remaining size of the maker order
</dd>
<dt>
<code>order_book_type: <a href="order_book_types.md#0x5_order_book_types_OrderType">order_book_types::OrderType</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

</details>

<a id="@Constants_2"></a>

## Constants


<a id="0x5_order_match_types_E_REINSERT_ORDER_MISMATCH"></a>



<pre><code><b>const</b> <a href="order_match_types.md#0x5_order_match_types_E_REINSERT_ORDER_MISMATCH">E_REINSERT_ORDER_MISMATCH</a>: u64 = 8;
</code></pre>



<a id="0x5_order_match_types_new_single_order_match_details"></a>

## Function `new_single_order_match_details`



<pre><code><b>public</b> <b>fun</b> <a href="order_match_types.md#0x5_order_match_types_new_single_order_match_details">new_single_order_match_details</a>&lt;M: <b>copy</b>, drop, store&gt;(order_id: <a href="order_book_types.md#0x5_order_book_types_OrderId">order_book_types::OrderId</a>, <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>, client_order_id: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, unique_priority_idx: <a href="order_book_types.md#0x5_order_book_types_IncreasingIdx">order_book_types::IncreasingIdx</a>, price: u64, orig_size: u64, remaining_size: u64, is_bid: bool, time_in_force: <a href="order_book_types.md#0x5_order_book_types_TimeInForce">order_book_types::TimeInForce</a>, creation_time_micros: u64, metadata: M): <a href="order_match_types.md#0x5_order_match_types_OrderMatchDetails">order_match_types::OrderMatchDetails</a>&lt;M&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_match_types.md#0x5_order_match_types_new_single_order_match_details">new_single_order_match_details</a>&lt;M: store + <b>copy</b> + drop&gt;(
    order_id: OrderId,
    <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>,
    client_order_id: Option&lt;String&gt;,
    unique_priority_idx: IncreasingIdx,
    price: u64,
    orig_size: u64,
    remaining_size: u64,
    is_bid: bool,
    time_in_force: TimeInForce,
    creation_time_micros: u64,
    metadata: M
): <a href="order_match_types.md#0x5_order_match_types_OrderMatchDetails">OrderMatchDetails</a>&lt;M&gt; {
    OrderMatchDetails::SingleOrder {
        order_id,
        <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>,
        client_order_id,
        unique_priority_idx,
        price,
        orig_size,
        remaining_size,
        is_bid,
        time_in_force,
        creation_time_micros,
        metadata
    }
}
</code></pre>



</details>

<a id="0x5_order_match_types_new_bulk_order_match_details"></a>

## Function `new_bulk_order_match_details`



<pre><code><b>public</b> <b>fun</b> <a href="order_match_types.md#0x5_order_match_types_new_bulk_order_match_details">new_bulk_order_match_details</a>&lt;M: <b>copy</b>, drop, store&gt;(order_id: <a href="order_book_types.md#0x5_order_book_types_OrderId">order_book_types::OrderId</a>, <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>, unique_priority_idx: <a href="order_book_types.md#0x5_order_book_types_IncreasingIdx">order_book_types::IncreasingIdx</a>, price: u64, remaining_size: u64, is_bid: bool, sequence_number: u64, creation_time_micros: u64, metadata: M): <a href="order_match_types.md#0x5_order_match_types_OrderMatchDetails">order_match_types::OrderMatchDetails</a>&lt;M&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_match_types.md#0x5_order_match_types_new_bulk_order_match_details">new_bulk_order_match_details</a>&lt;M: store + <b>copy</b> + drop&gt;(
    order_id: OrderId,
    <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>,
    unique_priority_idx: IncreasingIdx,
    price: u64,
    remaining_size: u64,
    is_bid: bool,
    sequence_number: u64,
    creation_time_micros: u64,
    metadata: M
): <a href="order_match_types.md#0x5_order_match_types_OrderMatchDetails">OrderMatchDetails</a>&lt;M&gt; {
    OrderMatchDetails::BulkOrder {
        order_id,
        <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>,
        unique_priority_idx,
        price,
        remaining_size,
        is_bid,
        sequence_number,
        creation_time_micros,
        metadata
    }
}
</code></pre>



</details>

<a id="0x5_order_match_types_new_order_match"></a>

## Function `new_order_match`



<pre><code><b>public</b> <b>fun</b> <a href="order_match_types.md#0x5_order_match_types_new_order_match">new_order_match</a>&lt;M: <b>copy</b>, drop, store&gt;(order: <a href="order_match_types.md#0x5_order_match_types_OrderMatchDetails">order_match_types::OrderMatchDetails</a>&lt;M&gt;, matched_size: u64): <a href="order_match_types.md#0x5_order_match_types_OrderMatch">order_match_types::OrderMatch</a>&lt;M&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_match_types.md#0x5_order_match_types_new_order_match">new_order_match</a>&lt;M: store + <b>copy</b> + drop&gt;(
    order: <a href="order_match_types.md#0x5_order_match_types_OrderMatchDetails">OrderMatchDetails</a>&lt;M&gt;, matched_size: u64
): <a href="order_match_types.md#0x5_order_match_types_OrderMatch">OrderMatch</a>&lt;M&gt; {
    OrderMatch::V1 { order, matched_size }
}
</code></pre>



</details>

<a id="0x5_order_match_types_new_order_match_details_with_modified_size"></a>

## Function `new_order_match_details_with_modified_size`



<pre><code><b>public</b> <b>fun</b> <a href="order_match_types.md#0x5_order_match_types_new_order_match_details_with_modified_size">new_order_match_details_with_modified_size</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="order_match_types.md#0x5_order_match_types_OrderMatchDetails">order_match_types::OrderMatchDetails</a>&lt;M&gt;, remaining_size: u64): <a href="order_match_types.md#0x5_order_match_types_OrderMatchDetails">order_match_types::OrderMatchDetails</a>&lt;M&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_match_types.md#0x5_order_match_types_new_order_match_details_with_modified_size">new_order_match_details_with_modified_size</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="order_match_types.md#0x5_order_match_types_OrderMatchDetails">OrderMatchDetails</a>&lt;M&gt;, remaining_size: u64
): <a href="order_match_types.md#0x5_order_match_types_OrderMatchDetails">OrderMatchDetails</a>&lt;M&gt; {
    <b>let</b> res = *self;
    res.remaining_size = remaining_size;
    res
}
</code></pre>



</details>

<a id="0x5_order_match_types_new_active_matched_order"></a>

## Function `new_active_matched_order`



<pre><code><b>public</b> <b>fun</b> <a href="order_match_types.md#0x5_order_match_types_new_active_matched_order">new_active_matched_order</a>(order_id: <a href="order_book_types.md#0x5_order_book_types_OrderId">order_book_types::OrderId</a>, matched_size: u64, remaining_size: u64, order_book_type: <a href="order_book_types.md#0x5_order_book_types_OrderType">order_book_types::OrderType</a>): <a href="order_match_types.md#0x5_order_match_types_ActiveMatchedOrder">order_match_types::ActiveMatchedOrder</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_match_types.md#0x5_order_match_types_new_active_matched_order">new_active_matched_order</a>(
    order_id: OrderId,
    matched_size: u64,
    remaining_size: u64,
    order_book_type: OrderType
): <a href="order_match_types.md#0x5_order_match_types_ActiveMatchedOrder">ActiveMatchedOrder</a> {
    ActiveMatchedOrder::V1 { order_id, matched_size, remaining_size, order_book_type }
}
</code></pre>



</details>

<a id="0x5_order_match_types_get_matched_size"></a>

## Function `get_matched_size`



<pre><code><b>public</b> <b>fun</b> <a href="order_match_types.md#0x5_order_match_types_get_matched_size">get_matched_size</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="order_match_types.md#0x5_order_match_types_OrderMatch">order_match_types::OrderMatch</a>&lt;M&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_match_types.md#0x5_order_match_types_get_matched_size">get_matched_size</a>&lt;M: store + <b>copy</b> + drop&gt;(self: &<a href="order_match_types.md#0x5_order_match_types_OrderMatch">OrderMatch</a>&lt;M&gt;): u64 {
    self.matched_size
}
</code></pre>



</details>

<a id="0x5_order_match_types_get_account_from_match_details"></a>

## Function `get_account_from_match_details`

Validates that a reinsertion request is valid for the given original order.


<pre><code><b>public</b> <b>fun</b> <a href="order_match_types.md#0x5_order_match_types_get_account_from_match_details">get_account_from_match_details</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="order_match_types.md#0x5_order_match_types_OrderMatchDetails">order_match_types::OrderMatchDetails</a>&lt;M&gt;): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_match_types.md#0x5_order_match_types_get_account_from_match_details">get_account_from_match_details</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="order_match_types.md#0x5_order_match_types_OrderMatchDetails">OrderMatchDetails</a>&lt;M&gt;
): <b>address</b> {
    self.<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>
}
</code></pre>



</details>

<a id="0x5_order_match_types_get_order_id_from_match_details"></a>

## Function `get_order_id_from_match_details`



<pre><code><b>public</b> <b>fun</b> <a href="order_match_types.md#0x5_order_match_types_get_order_id_from_match_details">get_order_id_from_match_details</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="order_match_types.md#0x5_order_match_types_OrderMatchDetails">order_match_types::OrderMatchDetails</a>&lt;M&gt;): <a href="order_book_types.md#0x5_order_book_types_OrderId">order_book_types::OrderId</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_match_types.md#0x5_order_match_types_get_order_id_from_match_details">get_order_id_from_match_details</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="order_match_types.md#0x5_order_match_types_OrderMatchDetails">OrderMatchDetails</a>&lt;M&gt;
): OrderId {
    self.order_id
}
</code></pre>



</details>

<a id="0x5_order_match_types_get_unique_priority_idx_from_match_details"></a>

## Function `get_unique_priority_idx_from_match_details`



<pre><code><b>public</b> <b>fun</b> <a href="order_match_types.md#0x5_order_match_types_get_unique_priority_idx_from_match_details">get_unique_priority_idx_from_match_details</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="order_match_types.md#0x5_order_match_types_OrderMatchDetails">order_match_types::OrderMatchDetails</a>&lt;M&gt;): <a href="order_book_types.md#0x5_order_book_types_IncreasingIdx">order_book_types::IncreasingIdx</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_match_types.md#0x5_order_match_types_get_unique_priority_idx_from_match_details">get_unique_priority_idx_from_match_details</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="order_match_types.md#0x5_order_match_types_OrderMatchDetails">OrderMatchDetails</a>&lt;M&gt;
): IncreasingIdx {
    self.unique_priority_idx
}
</code></pre>



</details>

<a id="0x5_order_match_types_get_price_from_match_details"></a>

## Function `get_price_from_match_details`



<pre><code><b>public</b> <b>fun</b> <a href="order_match_types.md#0x5_order_match_types_get_price_from_match_details">get_price_from_match_details</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="order_match_types.md#0x5_order_match_types_OrderMatchDetails">order_match_types::OrderMatchDetails</a>&lt;M&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_match_types.md#0x5_order_match_types_get_price_from_match_details">get_price_from_match_details</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="order_match_types.md#0x5_order_match_types_OrderMatchDetails">OrderMatchDetails</a>&lt;M&gt;
): u64 {
    self.price
}
</code></pre>



</details>

<a id="0x5_order_match_types_get_orig_size_from_match_details"></a>

## Function `get_orig_size_from_match_details`



<pre><code><b>public</b> <b>fun</b> <a href="order_match_types.md#0x5_order_match_types_get_orig_size_from_match_details">get_orig_size_from_match_details</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="order_match_types.md#0x5_order_match_types_OrderMatchDetails">order_match_types::OrderMatchDetails</a>&lt;M&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_match_types.md#0x5_order_match_types_get_orig_size_from_match_details">get_orig_size_from_match_details</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="order_match_types.md#0x5_order_match_types_OrderMatchDetails">OrderMatchDetails</a>&lt;M&gt;
): u64 {
    self.orig_size
}
</code></pre>



</details>

<a id="0x5_order_match_types_get_remaining_size_from_match_details"></a>

## Function `get_remaining_size_from_match_details`



<pre><code><b>public</b> <b>fun</b> <a href="order_match_types.md#0x5_order_match_types_get_remaining_size_from_match_details">get_remaining_size_from_match_details</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="order_match_types.md#0x5_order_match_types_OrderMatchDetails">order_match_types::OrderMatchDetails</a>&lt;M&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_match_types.md#0x5_order_match_types_get_remaining_size_from_match_details">get_remaining_size_from_match_details</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="order_match_types.md#0x5_order_match_types_OrderMatchDetails">OrderMatchDetails</a>&lt;M&gt;
): u64 {
    self.remaining_size
}
</code></pre>



</details>

<a id="0x5_order_match_types_get_time_in_force_from_match_details"></a>

## Function `get_time_in_force_from_match_details`



<pre><code><b>public</b> <b>fun</b> <a href="order_match_types.md#0x5_order_match_types_get_time_in_force_from_match_details">get_time_in_force_from_match_details</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="order_match_types.md#0x5_order_match_types_OrderMatchDetails">order_match_types::OrderMatchDetails</a>&lt;M&gt;): <a href="order_book_types.md#0x5_order_book_types_TimeInForce">order_book_types::TimeInForce</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_match_types.md#0x5_order_match_types_get_time_in_force_from_match_details">get_time_in_force_from_match_details</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="order_match_types.md#0x5_order_match_types_OrderMatchDetails">OrderMatchDetails</a>&lt;M&gt;
): TimeInForce {
    <b>if</b> (self is OrderMatchDetails::SingleOrder) {
        self.time_in_force
    } <b>else</b> {
        good_till_cancelled()
    }
}
</code></pre>



</details>

<a id="0x5_order_match_types_get_metadata_from_match_details"></a>

## Function `get_metadata_from_match_details`



<pre><code><b>public</b> <b>fun</b> <a href="order_match_types.md#0x5_order_match_types_get_metadata_from_match_details">get_metadata_from_match_details</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="order_match_types.md#0x5_order_match_types_OrderMatchDetails">order_match_types::OrderMatchDetails</a>&lt;M&gt;): M
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_match_types.md#0x5_order_match_types_get_metadata_from_match_details">get_metadata_from_match_details</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="order_match_types.md#0x5_order_match_types_OrderMatchDetails">OrderMatchDetails</a>&lt;M&gt;
): M {
    self.metadata
}
</code></pre>



</details>

<a id="0x5_order_match_types_get_client_order_id_from_match_details"></a>

## Function `get_client_order_id_from_match_details`



<pre><code><b>public</b> <b>fun</b> <a href="order_match_types.md#0x5_order_match_types_get_client_order_id_from_match_details">get_client_order_id_from_match_details</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="order_match_types.md#0x5_order_match_types_OrderMatchDetails">order_match_types::OrderMatchDetails</a>&lt;M&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_match_types.md#0x5_order_match_types_get_client_order_id_from_match_details">get_client_order_id_from_match_details</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="order_match_types.md#0x5_order_match_types_OrderMatchDetails">OrderMatchDetails</a>&lt;M&gt;
): Option&lt;String&gt; {
    <b>if</b> (self is OrderMatchDetails::SingleOrder) {
        self.client_order_id
    } <b>else</b> {
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
    }
}
</code></pre>



</details>

<a id="0x5_order_match_types_is_bid_from_match_details"></a>

## Function `is_bid_from_match_details`



<pre><code><b>public</b> <b>fun</b> <a href="order_match_types.md#0x5_order_match_types_is_bid_from_match_details">is_bid_from_match_details</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="order_match_types.md#0x5_order_match_types_OrderMatchDetails">order_match_types::OrderMatchDetails</a>&lt;M&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_match_types.md#0x5_order_match_types_is_bid_from_match_details">is_bid_from_match_details</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="order_match_types.md#0x5_order_match_types_OrderMatchDetails">OrderMatchDetails</a>&lt;M&gt;
): bool {
    self.is_bid
}
</code></pre>



</details>

<a id="0x5_order_match_types_get_book_type_from_match_details"></a>

## Function `get_book_type_from_match_details`



<pre><code><b>public</b> <b>fun</b> <a href="order_match_types.md#0x5_order_match_types_get_book_type_from_match_details">get_book_type_from_match_details</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="order_match_types.md#0x5_order_match_types_OrderMatchDetails">order_match_types::OrderMatchDetails</a>&lt;M&gt;): <a href="order_book_types.md#0x5_order_book_types_OrderType">order_book_types::OrderType</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_match_types.md#0x5_order_match_types_get_book_type_from_match_details">get_book_type_from_match_details</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="order_match_types.md#0x5_order_match_types_OrderMatchDetails">OrderMatchDetails</a>&lt;M&gt;
): OrderType {
    <b>if</b> (self is OrderMatchDetails::SingleOrder) {
        single_order_type()
    } <b>else</b> {
        bulk_order_type()
    }
}
</code></pre>



</details>

<a id="0x5_order_match_types_is_bulk_order_from_match_details"></a>

## Function `is_bulk_order_from_match_details`



<pre><code><b>public</b> <b>fun</b> <a href="order_match_types.md#0x5_order_match_types_is_bulk_order_from_match_details">is_bulk_order_from_match_details</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="order_match_types.md#0x5_order_match_types_OrderMatchDetails">order_match_types::OrderMatchDetails</a>&lt;M&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_match_types.md#0x5_order_match_types_is_bulk_order_from_match_details">is_bulk_order_from_match_details</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="order_match_types.md#0x5_order_match_types_OrderMatchDetails">OrderMatchDetails</a>&lt;M&gt;
): bool {
    self is OrderMatchDetails::BulkOrder
}
</code></pre>



</details>

<a id="0x5_order_match_types_is_single_order_from_match_details"></a>

## Function `is_single_order_from_match_details`



<pre><code><b>public</b> <b>fun</b> <a href="order_match_types.md#0x5_order_match_types_is_single_order_from_match_details">is_single_order_from_match_details</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="order_match_types.md#0x5_order_match_types_OrderMatchDetails">order_match_types::OrderMatchDetails</a>&lt;M&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_match_types.md#0x5_order_match_types_is_single_order_from_match_details">is_single_order_from_match_details</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="order_match_types.md#0x5_order_match_types_OrderMatchDetails">OrderMatchDetails</a>&lt;M&gt;
): bool {
    self is OrderMatchDetails::SingleOrder
}
</code></pre>



</details>

<a id="0x5_order_match_types_get_sequence_number_from_match_details"></a>

## Function `get_sequence_number_from_match_details`

This should only be called on bulk orders, aborts if called for non-bulk order.


<pre><code><b>public</b> <b>fun</b> <a href="order_match_types.md#0x5_order_match_types_get_sequence_number_from_match_details">get_sequence_number_from_match_details</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="order_match_types.md#0x5_order_match_types_OrderMatchDetails">order_match_types::OrderMatchDetails</a>&lt;M&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_match_types.md#0x5_order_match_types_get_sequence_number_from_match_details">get_sequence_number_from_match_details</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="order_match_types.md#0x5_order_match_types_OrderMatchDetails">OrderMatchDetails</a>&lt;M&gt;
): u64 {
    self.sequence_number
}
</code></pre>



</details>

<a id="0x5_order_match_types_get_creation_time_micros_from_match_details"></a>

## Function `get_creation_time_micros_from_match_details`



<pre><code><b>public</b> <b>fun</b> <a href="order_match_types.md#0x5_order_match_types_get_creation_time_micros_from_match_details">get_creation_time_micros_from_match_details</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="order_match_types.md#0x5_order_match_types_OrderMatchDetails">order_match_types::OrderMatchDetails</a>&lt;M&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_match_types.md#0x5_order_match_types_get_creation_time_micros_from_match_details">get_creation_time_micros_from_match_details</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="order_match_types.md#0x5_order_match_types_OrderMatchDetails">OrderMatchDetails</a>&lt;M&gt;
): u64 {
    self.creation_time_micros
}
</code></pre>



</details>

<a id="0x5_order_match_types_destroy_order_match"></a>

## Function `destroy_order_match`



<pre><code><b>public</b> <b>fun</b> <a href="order_match_types.md#0x5_order_match_types_destroy_order_match">destroy_order_match</a>&lt;M: <b>copy</b>, drop, store&gt;(self: <a href="order_match_types.md#0x5_order_match_types_OrderMatch">order_match_types::OrderMatch</a>&lt;M&gt;): (<a href="order_match_types.md#0x5_order_match_types_OrderMatchDetails">order_match_types::OrderMatchDetails</a>&lt;M&gt;, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_match_types.md#0x5_order_match_types_destroy_order_match">destroy_order_match</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: <a href="order_match_types.md#0x5_order_match_types_OrderMatch">OrderMatch</a>&lt;M&gt;
): (<a href="order_match_types.md#0x5_order_match_types_OrderMatchDetails">OrderMatchDetails</a>&lt;M&gt;, u64) {
    <b>let</b> OrderMatch::V1 { order, matched_size } = self;
    (order, matched_size)
}
</code></pre>



</details>

<a id="0x5_order_match_types_destroy_single_order_match_details"></a>

## Function `destroy_single_order_match_details`



<pre><code><b>public</b> <b>fun</b> <a href="order_match_types.md#0x5_order_match_types_destroy_single_order_match_details">destroy_single_order_match_details</a>&lt;M: <b>copy</b>, drop, store&gt;(self: <a href="order_match_types.md#0x5_order_match_types_OrderMatchDetails">order_match_types::OrderMatchDetails</a>&lt;M&gt;): (<a href="order_book_types.md#0x5_order_book_types_OrderId">order_book_types::OrderId</a>, <b>address</b>, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, <a href="order_book_types.md#0x5_order_book_types_IncreasingIdx">order_book_types::IncreasingIdx</a>, u64, u64, u64, bool, <a href="order_book_types.md#0x5_order_book_types_TimeInForce">order_book_types::TimeInForce</a>, u64, M)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_match_types.md#0x5_order_match_types_destroy_single_order_match_details">destroy_single_order_match_details</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: <a href="order_match_types.md#0x5_order_match_types_OrderMatchDetails">OrderMatchDetails</a>&lt;M&gt;
): (
    OrderId,
    <b>address</b>,
    Option&lt;String&gt;,
    IncreasingIdx,
    u64,
    u64,
    u64,
    bool,
    TimeInForce,
    u64,
    M
) {
    <b>let</b> OrderMatchDetails::SingleOrder {
        order_id,
        <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>,
        client_order_id,
        unique_priority_idx,
        price,
        orig_size,
        remaining_size,
        is_bid,
        time_in_force,
        creation_time_micros,
        metadata
    } = self;
    (
        order_id,
        <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>,
        client_order_id,
        unique_priority_idx,
        price,
        orig_size,
        remaining_size,
        is_bid,
        time_in_force,
        creation_time_micros,
        metadata
    )
}
</code></pre>



</details>

<a id="0x5_order_match_types_validate_single_order_reinsertion_request"></a>

## Function `validate_single_order_reinsertion_request`



<pre><code><b>public</b> <b>fun</b> <a href="order_match_types.md#0x5_order_match_types_validate_single_order_reinsertion_request">validate_single_order_reinsertion_request</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="order_match_types.md#0x5_order_match_types_OrderMatchDetails">order_match_types::OrderMatchDetails</a>&lt;M&gt;, other: &<a href="order_match_types.md#0x5_order_match_types_OrderMatchDetails">order_match_types::OrderMatchDetails</a>&lt;M&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_match_types.md#0x5_order_match_types_validate_single_order_reinsertion_request">validate_single_order_reinsertion_request</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="order_match_types.md#0x5_order_match_types_OrderMatchDetails">OrderMatchDetails</a>&lt;M&gt;, other: &<a href="order_match_types.md#0x5_order_match_types_OrderMatchDetails">OrderMatchDetails</a>&lt;M&gt;
): bool {
    <b>assert</b>!(self is OrderMatchDetails::SingleOrder, <a href="order_match_types.md#0x5_order_match_types_E_REINSERT_ORDER_MISMATCH">E_REINSERT_ORDER_MISMATCH</a>);
    <b>assert</b>!(other is OrderMatchDetails::SingleOrder, <a href="order_match_types.md#0x5_order_match_types_E_REINSERT_ORDER_MISMATCH">E_REINSERT_ORDER_MISMATCH</a>);

    self.order_id == other.order_id
        && self.<a href="../../aptos-framework/doc/account.md#0x1_account">account</a> == other.<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>
        && self.unique_priority_idx == other.unique_priority_idx
        && self.price == other.price
        && self.orig_size == other.orig_size
        && self.is_bid == other.is_bid
}
</code></pre>



</details>

<a id="0x5_order_match_types_validate_bulk_order_reinsertion_request"></a>

## Function `validate_bulk_order_reinsertion_request`



<pre><code><b>public</b> <b>fun</b> <a href="order_match_types.md#0x5_order_match_types_validate_bulk_order_reinsertion_request">validate_bulk_order_reinsertion_request</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="order_match_types.md#0x5_order_match_types_OrderMatchDetails">order_match_types::OrderMatchDetails</a>&lt;M&gt;, other: &<a href="order_match_types.md#0x5_order_match_types_OrderMatchDetails">order_match_types::OrderMatchDetails</a>&lt;M&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_match_types.md#0x5_order_match_types_validate_bulk_order_reinsertion_request">validate_bulk_order_reinsertion_request</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="order_match_types.md#0x5_order_match_types_OrderMatchDetails">OrderMatchDetails</a>&lt;M&gt;, other: &<a href="order_match_types.md#0x5_order_match_types_OrderMatchDetails">OrderMatchDetails</a>&lt;M&gt;
): bool {
    <b>assert</b>!(self is OrderMatchDetails::BulkOrder, <a href="order_match_types.md#0x5_order_match_types_E_REINSERT_ORDER_MISMATCH">E_REINSERT_ORDER_MISMATCH</a>);
    <b>assert</b>!(other is OrderMatchDetails::BulkOrder, <a href="order_match_types.md#0x5_order_match_types_E_REINSERT_ORDER_MISMATCH">E_REINSERT_ORDER_MISMATCH</a>);

    self.order_id == other.order_id
        && self.<a href="../../aptos-framework/doc/account.md#0x1_account">account</a> == other.<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>
        && self.unique_priority_idx == other.unique_priority_idx
        && self.price == other.price
        && self.is_bid == other.is_bid
        && self.sequence_number == other.sequence_number
}
</code></pre>



</details>

<a id="0x5_order_match_types_destroy_active_matched_order"></a>

## Function `destroy_active_matched_order`



<pre><code><b>public</b> <b>fun</b> <a href="order_match_types.md#0x5_order_match_types_destroy_active_matched_order">destroy_active_matched_order</a>(self: <a href="order_match_types.md#0x5_order_match_types_ActiveMatchedOrder">order_match_types::ActiveMatchedOrder</a>): (<a href="order_book_types.md#0x5_order_book_types_OrderId">order_book_types::OrderId</a>, u64, u64, <a href="order_book_types.md#0x5_order_book_types_OrderType">order_book_types::OrderType</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_match_types.md#0x5_order_match_types_destroy_active_matched_order">destroy_active_matched_order</a>(self: <a href="order_match_types.md#0x5_order_match_types_ActiveMatchedOrder">ActiveMatchedOrder</a>)
    : (OrderId, u64, u64, OrderType) {
    <b>let</b> ActiveMatchedOrder::V1 {
        order_id,
        matched_size,
        remaining_size,
        order_book_type
    } = self;
    (order_id, matched_size, remaining_size, order_book_type)
}
</code></pre>



</details>

<a id="0x5_order_match_types_is_active_matched_book_type_single_order"></a>

## Function `is_active_matched_book_type_single_order`



<pre><code><b>public</b> <b>fun</b> <a href="order_match_types.md#0x5_order_match_types_is_active_matched_book_type_single_order">is_active_matched_book_type_single_order</a>(self: &<a href="order_match_types.md#0x5_order_match_types_ActiveMatchedOrder">order_match_types::ActiveMatchedOrder</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_match_types.md#0x5_order_match_types_is_active_matched_book_type_single_order">is_active_matched_book_type_single_order</a>(
    self: &<a href="order_match_types.md#0x5_order_match_types_ActiveMatchedOrder">ActiveMatchedOrder</a>
): bool {
    is_single_order_type(&self.order_book_type)
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
