
<a id="0x7_order_book_types"></a>

# Module `0x7::order_book_types`

Order book type definitions


-  [Struct `OrderIdType`](#0x7_order_book_types_OrderIdType)
-  [Struct `AccountClientOrderId`](#0x7_order_book_types_AccountClientOrderId)
-  [Struct `UniqueIdxType`](#0x7_order_book_types_UniqueIdxType)
-  [Struct `OrderType`](#0x7_order_book_types_OrderType)
-  [Enum `TimeInForce`](#0x7_order_book_types_TimeInForce)
-  [Enum `TriggerCondition`](#0x7_order_book_types_TriggerCondition)
-  [Enum `OrderMatchDetails`](#0x7_order_book_types_OrderMatchDetails)
    -  [Fields:](#@Fields:_0)
-  [Enum `OrderMatch`](#0x7_order_book_types_OrderMatch)
    -  [Fields:](#@Fields:_1)
-  [Struct `ActiveMatchedOrder`](#0x7_order_book_types_ActiveMatchedOrder)
-  [Constants](#@Constants_2)
-  [Function `single_order_type`](#0x7_order_book_types_single_order_type)
-  [Function `bulk_order_type`](#0x7_order_book_types_bulk_order_type)
-  [Function `is_bulk_order_type`](#0x7_order_book_types_is_bulk_order_type)
-  [Function `is_single_order_type`](#0x7_order_book_types_is_single_order_type)
-  [Function `new_default_big_ordered_map`](#0x7_order_book_types_new_default_big_ordered_map)
-  [Function `next_order_id`](#0x7_order_book_types_next_order_id)
-  [Function `new_order_id_type`](#0x7_order_book_types_new_order_id_type)
-  [Function `new_account_client_order_id`](#0x7_order_book_types_new_account_client_order_id)
-  [Function `new_unique_idx_type`](#0x7_order_book_types_new_unique_idx_type)
-  [Function `descending_idx`](#0x7_order_book_types_descending_idx)
-  [Function `get_order_id_value`](#0x7_order_book_types_get_order_id_value)
-  [Function `time_in_force_from_index`](#0x7_order_book_types_time_in_force_from_index)
-  [Function `good_till_cancelled`](#0x7_order_book_types_good_till_cancelled)
-  [Function `post_only`](#0x7_order_book_types_post_only)
-  [Function `immediate_or_cancel`](#0x7_order_book_types_immediate_or_cancel)
-  [Function `new_time_based_trigger_condition`](#0x7_order_book_types_new_time_based_trigger_condition)
-  [Function `price_move_up_condition`](#0x7_order_book_types_price_move_up_condition)
-  [Function `price_move_down_condition`](#0x7_order_book_types_price_move_down_condition)
-  [Function `index`](#0x7_order_book_types_index)
-  [Function `destroy_order_match`](#0x7_order_book_types_destroy_order_match)
-  [Function `destroy_single_order_match_details`](#0x7_order_book_types_destroy_single_order_match_details)
-  [Function `destroy_bulk_order_match_details`](#0x7_order_book_types_destroy_bulk_order_match_details)
-  [Function `get_matched_size`](#0x7_order_book_types_get_matched_size)
-  [Function `get_account_from_match_details`](#0x7_order_book_types_get_account_from_match_details)
-  [Function `get_order_id_from_match_details`](#0x7_order_book_types_get_order_id_from_match_details)
-  [Function `get_unique_priority_idx_from_match_details`](#0x7_order_book_types_get_unique_priority_idx_from_match_details)
-  [Function `get_price_from_match_details`](#0x7_order_book_types_get_price_from_match_details)
-  [Function `get_orig_size_from_match_details`](#0x7_order_book_types_get_orig_size_from_match_details)
-  [Function `get_remaining_size_from_match_details`](#0x7_order_book_types_get_remaining_size_from_match_details)
-  [Function `get_time_in_force_from_match_details`](#0x7_order_book_types_get_time_in_force_from_match_details)
-  [Function `get_metadata_from_match_details`](#0x7_order_book_types_get_metadata_from_match_details)
-  [Function `get_client_order_id_from_match_details`](#0x7_order_book_types_get_client_order_id_from_match_details)
-  [Function `is_bid_from_match_details`](#0x7_order_book_types_is_bid_from_match_details)
-  [Function `get_book_type_from_match_details`](#0x7_order_book_types_get_book_type_from_match_details)
-  [Function `is_bulk_order_from_match_details`](#0x7_order_book_types_is_bulk_order_from_match_details)
-  [Function `is_single_order_from_match_details`](#0x7_order_book_types_is_single_order_from_match_details)
-  [Function `get_sequence_number_from_match_details`](#0x7_order_book_types_get_sequence_number_from_match_details)
-  [Function `get_creation_time_micros_from_match_details`](#0x7_order_book_types_get_creation_time_micros_from_match_details)
-  [Function `new_single_order_match_details`](#0x7_order_book_types_new_single_order_match_details)
-  [Function `new_bulk_order_match_details`](#0x7_order_book_types_new_bulk_order_match_details)
-  [Function `new_order_match_details_with_modified_size`](#0x7_order_book_types_new_order_match_details_with_modified_size)
-  [Function `new_order_match`](#0x7_order_book_types_new_order_match)
-  [Function `validate_single_order_reinsertion_request`](#0x7_order_book_types_validate_single_order_reinsertion_request)
-  [Function `validate_bulk_order_reinsertion_request`](#0x7_order_book_types_validate_bulk_order_reinsertion_request)
-  [Function `new_active_matched_order`](#0x7_order_book_types_new_active_matched_order)
-  [Function `destroy_active_matched_order`](#0x7_order_book_types_destroy_active_matched_order)
-  [Function `get_active_matched_size`](#0x7_order_book_types_get_active_matched_size)
-  [Function `is_active_matched_book_type_single_order`](#0x7_order_book_types_is_active_matched_book_type_single_order)
-  [Function `reverse_bits`](#0x7_order_book_types_reverse_bits)


<pre><code><b>use</b> <a href="../../aptos-framework/doc/big_ordered_map.md#0x1_big_ordered_map">0x1::big_ordered_map</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;
<b>use</b> <a href="../../aptos-framework/doc/transaction_context.md#0x1_transaction_context">0x1::transaction_context</a>;
</code></pre>



<a id="0x7_order_book_types_OrderIdType"></a>

## Struct `OrderIdType`



<pre><code><b>struct</b> <a href="order_book_types.md#0x7_order_book_types_OrderIdType">OrderIdType</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>order_id: u128</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x7_order_book_types_AccountClientOrderId"></a>

## Struct `AccountClientOrderId`



<pre><code><b>struct</b> <a href="order_book_types.md#0x7_order_book_types_AccountClientOrderId">AccountClientOrderId</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code><a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>client_order_id: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x7_order_book_types_UniqueIdxType"></a>

## Struct `UniqueIdxType`



<pre><code><b>struct</b> <a href="order_book_types.md#0x7_order_book_types_UniqueIdxType">UniqueIdxType</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>idx: u128</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x7_order_book_types_OrderType"></a>

## Struct `OrderType`



<pre><code><b>struct</b> <a href="order_book_types.md#0x7_order_book_types_OrderType">OrderType</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>type: u16</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x7_order_book_types_TimeInForce"></a>

## Enum `TimeInForce`

Order time in force


<pre><code>enum <a href="order_book_types.md#0x7_order_book_types_TimeInForce">TimeInForce</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>GTC</summary>


<details>
<summary>Fields</summary>


<dl>
</dl>


</details>

</details>

<details>
<summary>POST_ONLY</summary>


<details>
<summary>Fields</summary>


<dl>
</dl>


</details>

</details>

<details>
<summary>IOC</summary>


<details>
<summary>Fields</summary>


<dl>
</dl>


</details>

</details>

</details>

<a id="0x7_order_book_types_TriggerCondition"></a>

## Enum `TriggerCondition`



<pre><code>enum <a href="order_book_types.md#0x7_order_book_types_TriggerCondition">TriggerCondition</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>PriceMoveAbove</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>0: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

<details>
<summary>PriceMoveBelow</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>0: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

<details>
<summary>TimeBased</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>0: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

</details>

<a id="0x7_order_book_types_OrderMatchDetails"></a>

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


<pre><code>enum <a href="order_book_types.md#0x7_order_book_types_OrderMatchDetails">OrderMatchDetails</a>&lt;M: <b>copy</b>, drop, store&gt; <b>has</b> <b>copy</b>, drop
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>SingleOrder</summary>


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
<code>client_order_id: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>unique_priority_idx: <a href="order_book_types.md#0x7_order_book_types_UniqueIdxType">order_book_types::UniqueIdxType</a></code>
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
<code>time_in_force: <a href="order_book_types.md#0x7_order_book_types_TimeInForce">order_book_types::TimeInForce</a></code>
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

<a id="0x7_order_book_types_OrderMatch"></a>

## Enum `OrderMatch`

Represents a single match between a taker order and a maker order.

Contains the matched order details and the size that was matched in this
particular match operation.


<a id="@Fields:_1"></a>

### Fields:

- <code>order</code>: The matched order result
- <code>matched_size</code>: The size that was matched in this operation


<pre><code>enum <a href="order_book_types.md#0x7_order_book_types_OrderMatch">OrderMatch</a>&lt;M: <b>copy</b>, drop, store&gt; <b>has</b> <b>copy</b>, drop
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>V1</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>order: <a href="order_book_types.md#0x7_order_book_types_OrderMatchDetails">order_book_types::OrderMatchDetails</a>&lt;M&gt;</code>
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

<a id="0x7_order_book_types_ActiveMatchedOrder"></a>

## Struct `ActiveMatchedOrder`



<pre><code><b>struct</b> <a href="order_book_types.md#0x7_order_book_types_ActiveMatchedOrder">ActiveMatchedOrder</a> <b>has</b> <b>copy</b>, drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>order_id: <a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a></code>
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
<code>order_book_type: <a href="order_book_types.md#0x7_order_book_types_OrderType">order_book_types::OrderType</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_2"></a>

## Constants


<a id="0x7_order_book_types_BIG_MAP_INNER_DEGREE"></a>



<pre><code><b>const</b> <a href="order_book_types.md#0x7_order_book_types_BIG_MAP_INNER_DEGREE">BIG_MAP_INNER_DEGREE</a>: u16 = 64;
</code></pre>



<a id="0x7_order_book_types_BIG_MAP_LEAF_DEGREE"></a>



<pre><code><b>const</b> <a href="order_book_types.md#0x7_order_book_types_BIG_MAP_LEAF_DEGREE">BIG_MAP_LEAF_DEGREE</a>: u16 = 32;
</code></pre>



<a id="0x7_order_book_types_BULK_ORDER_TYPE"></a>



<pre><code><b>const</b> <a href="order_book_types.md#0x7_order_book_types_BULK_ORDER_TYPE">BULK_ORDER_TYPE</a>: u16 = 1;
</code></pre>



<a id="0x7_order_book_types_EINVALID_TIME_IN_FORCE"></a>



<pre><code><b>const</b> <a href="order_book_types.md#0x7_order_book_types_EINVALID_TIME_IN_FORCE">EINVALID_TIME_IN_FORCE</a>: u64 = 5;
</code></pre>



<a id="0x7_order_book_types_E_REINSERT_ORDER_MISMATCH"></a>



<pre><code><b>const</b> <a href="order_book_types.md#0x7_order_book_types_E_REINSERT_ORDER_MISMATCH">E_REINSERT_ORDER_MISMATCH</a>: u64 = 8;
</code></pre>



<a id="0x7_order_book_types_SINGLE_ORDER_TYPE"></a>



<pre><code><b>const</b> <a href="order_book_types.md#0x7_order_book_types_SINGLE_ORDER_TYPE">SINGLE_ORDER_TYPE</a>: u16 = 0;
</code></pre>



<a id="0x7_order_book_types_U128_MAX"></a>



<pre><code><b>const</b> <a href="order_book_types.md#0x7_order_book_types_U128_MAX">U128_MAX</a>: u128 = 340282366920938463463374607431768211455;
</code></pre>



<a id="0x7_order_book_types_single_order_type"></a>

## Function `single_order_type`



<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_single_order_type">single_order_type</a>(): <a href="order_book_types.md#0x7_order_book_types_OrderType">order_book_types::OrderType</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_single_order_type">single_order_type</a>(): <a href="order_book_types.md#0x7_order_book_types_OrderType">OrderType</a> {
    <a href="order_book_types.md#0x7_order_book_types_OrderType">OrderType</a> { type: <a href="order_book_types.md#0x7_order_book_types_SINGLE_ORDER_TYPE">SINGLE_ORDER_TYPE</a> }
}
</code></pre>



</details>

<a id="0x7_order_book_types_bulk_order_type"></a>

## Function `bulk_order_type`



<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_bulk_order_type">bulk_order_type</a>(): <a href="order_book_types.md#0x7_order_book_types_OrderType">order_book_types::OrderType</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_bulk_order_type">bulk_order_type</a>(): <a href="order_book_types.md#0x7_order_book_types_OrderType">OrderType</a> {
    <a href="order_book_types.md#0x7_order_book_types_OrderType">OrderType</a> { type: <a href="order_book_types.md#0x7_order_book_types_BULK_ORDER_TYPE">BULK_ORDER_TYPE</a> }
}
</code></pre>



</details>

<a id="0x7_order_book_types_is_bulk_order_type"></a>

## Function `is_bulk_order_type`



<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_is_bulk_order_type">is_bulk_order_type</a>(order_type: &<a href="order_book_types.md#0x7_order_book_types_OrderType">order_book_types::OrderType</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_is_bulk_order_type">is_bulk_order_type</a>(order_type: &<a href="order_book_types.md#0x7_order_book_types_OrderType">OrderType</a>): bool {
    order_type.type == <a href="order_book_types.md#0x7_order_book_types_BULK_ORDER_TYPE">BULK_ORDER_TYPE</a>
}
</code></pre>



</details>

<a id="0x7_order_book_types_is_single_order_type"></a>

## Function `is_single_order_type`



<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_is_single_order_type">is_single_order_type</a>(order_type: &<a href="order_book_types.md#0x7_order_book_types_OrderType">order_book_types::OrderType</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_is_single_order_type">is_single_order_type</a>(order_type: &<a href="order_book_types.md#0x7_order_book_types_OrderType">OrderType</a>): bool {
    order_type.type == <a href="order_book_types.md#0x7_order_book_types_SINGLE_ORDER_TYPE">SINGLE_ORDER_TYPE</a>
}
</code></pre>



</details>

<a id="0x7_order_book_types_new_default_big_ordered_map"></a>

## Function `new_default_big_ordered_map`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_new_default_big_ordered_map">new_default_big_ordered_map</a>&lt;K: store, V: store&gt;(): <a href="../../aptos-framework/doc/big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">big_ordered_map::BigOrderedMap</a>&lt;K, V&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_new_default_big_ordered_map">new_default_big_ordered_map</a>&lt;K: store, V: store&gt;(): BigOrderedMap&lt;K, V&gt; {
    <a href="../../aptos-framework/doc/big_ordered_map.md#0x1_big_ordered_map_new_with_config">big_ordered_map::new_with_config</a>(
        <a href="order_book_types.md#0x7_order_book_types_BIG_MAP_INNER_DEGREE">BIG_MAP_INNER_DEGREE</a>,
        <a href="order_book_types.md#0x7_order_book_types_BIG_MAP_LEAF_DEGREE">BIG_MAP_LEAF_DEGREE</a>,
        <b>true</b>
    )
}
</code></pre>



</details>

<a id="0x7_order_book_types_next_order_id"></a>

## Function `next_order_id`



<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_next_order_id">next_order_id</a>(): <a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_next_order_id">next_order_id</a>(): <a href="order_book_types.md#0x7_order_book_types_OrderIdType">OrderIdType</a> {
    // reverse bits <b>to</b> make order ids random, so indices on top of them are shuffled.
    <a href="order_book_types.md#0x7_order_book_types_OrderIdType">OrderIdType</a> { order_id: <a href="order_book_types.md#0x7_order_book_types_reverse_bits">reverse_bits</a>(<a href="../../aptos-framework/doc/transaction_context.md#0x1_transaction_context_monotonically_increasing_counter">transaction_context::monotonically_increasing_counter</a>()) }
}
</code></pre>



</details>

<a id="0x7_order_book_types_new_order_id_type"></a>

## Function `new_order_id_type`



<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_new_order_id_type">new_order_id_type</a>(order_id: u128): <a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_new_order_id_type">new_order_id_type</a>(order_id: u128): <a href="order_book_types.md#0x7_order_book_types_OrderIdType">OrderIdType</a> {
    <a href="order_book_types.md#0x7_order_book_types_OrderIdType">OrderIdType</a> { order_id }
}
</code></pre>



</details>

<a id="0x7_order_book_types_new_account_client_order_id"></a>

## Function `new_account_client_order_id`



<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_new_account_client_order_id">new_account_client_order_id</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>, client_order_id: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <a href="order_book_types.md#0x7_order_book_types_AccountClientOrderId">order_book_types::AccountClientOrderId</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_new_account_client_order_id">new_account_client_order_id</a>(
    <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>, client_order_id: String
): <a href="order_book_types.md#0x7_order_book_types_AccountClientOrderId">AccountClientOrderId</a> {
    <a href="order_book_types.md#0x7_order_book_types_AccountClientOrderId">AccountClientOrderId</a> { <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>, client_order_id }
}
</code></pre>



</details>

<a id="0x7_order_book_types_new_unique_idx_type"></a>

## Function `new_unique_idx_type`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_new_unique_idx_type">new_unique_idx_type</a>(idx: u128): <a href="order_book_types.md#0x7_order_book_types_UniqueIdxType">order_book_types::UniqueIdxType</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_new_unique_idx_type">new_unique_idx_type</a>(idx: u128): <a href="order_book_types.md#0x7_order_book_types_UniqueIdxType">UniqueIdxType</a> {
    <a href="order_book_types.md#0x7_order_book_types_UniqueIdxType">UniqueIdxType</a> { idx }
}
</code></pre>



</details>

<a id="0x7_order_book_types_descending_idx"></a>

## Function `descending_idx`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_descending_idx">descending_idx</a>(self: &<a href="order_book_types.md#0x7_order_book_types_UniqueIdxType">order_book_types::UniqueIdxType</a>): <a href="order_book_types.md#0x7_order_book_types_UniqueIdxType">order_book_types::UniqueIdxType</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_descending_idx">descending_idx</a>(self: &<a href="order_book_types.md#0x7_order_book_types_UniqueIdxType">UniqueIdxType</a>): <a href="order_book_types.md#0x7_order_book_types_UniqueIdxType">UniqueIdxType</a> {
    <a href="order_book_types.md#0x7_order_book_types_UniqueIdxType">UniqueIdxType</a> { idx: <a href="order_book_types.md#0x7_order_book_types_U128_MAX">U128_MAX</a> - self.idx }
}
</code></pre>



</details>

<a id="0x7_order_book_types_get_order_id_value"></a>

## Function `get_order_id_value`



<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_get_order_id_value">get_order_id_value</a>(self: &<a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_get_order_id_value">get_order_id_value</a>(self: &<a href="order_book_types.md#0x7_order_book_types_OrderIdType">OrderIdType</a>): u128 {
    self.order_id
}
</code></pre>



</details>

<a id="0x7_order_book_types_time_in_force_from_index"></a>

## Function `time_in_force_from_index`



<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_time_in_force_from_index">time_in_force_from_index</a>(index: u8): <a href="order_book_types.md#0x7_order_book_types_TimeInForce">order_book_types::TimeInForce</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_time_in_force_from_index">time_in_force_from_index</a>(index: u8): <a href="order_book_types.md#0x7_order_book_types_TimeInForce">TimeInForce</a> {
    <b>if</b> (index == 0) {
        TimeInForce::GTC
    } <b>else</b> <b>if</b> (index == 1) {
        TimeInForce::POST_ONLY
    } <b>else</b> <b>if</b> (index == 2) {
        TimeInForce::IOC
    } <b>else</b> {
        <b>abort</b> <a href="order_book_types.md#0x7_order_book_types_EINVALID_TIME_IN_FORCE">EINVALID_TIME_IN_FORCE</a>
    }
}
</code></pre>



</details>

<a id="0x7_order_book_types_good_till_cancelled"></a>

## Function `good_till_cancelled`



<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_good_till_cancelled">good_till_cancelled</a>(): <a href="order_book_types.md#0x7_order_book_types_TimeInForce">order_book_types::TimeInForce</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_good_till_cancelled">good_till_cancelled</a>(): <a href="order_book_types.md#0x7_order_book_types_TimeInForce">TimeInForce</a> {
    TimeInForce::GTC
}
</code></pre>



</details>

<a id="0x7_order_book_types_post_only"></a>

## Function `post_only`



<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_post_only">post_only</a>(): <a href="order_book_types.md#0x7_order_book_types_TimeInForce">order_book_types::TimeInForce</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_post_only">post_only</a>(): <a href="order_book_types.md#0x7_order_book_types_TimeInForce">TimeInForce</a> {
    TimeInForce::POST_ONLY
}
</code></pre>



</details>

<a id="0x7_order_book_types_immediate_or_cancel"></a>

## Function `immediate_or_cancel`



<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_immediate_or_cancel">immediate_or_cancel</a>(): <a href="order_book_types.md#0x7_order_book_types_TimeInForce">order_book_types::TimeInForce</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_immediate_or_cancel">immediate_or_cancel</a>(): <a href="order_book_types.md#0x7_order_book_types_TimeInForce">TimeInForce</a> {
    TimeInForce::IOC
}
</code></pre>



</details>

<a id="0x7_order_book_types_new_time_based_trigger_condition"></a>

## Function `new_time_based_trigger_condition`



<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_new_time_based_trigger_condition">new_time_based_trigger_condition</a>(time_secs: u64): <a href="order_book_types.md#0x7_order_book_types_TriggerCondition">order_book_types::TriggerCondition</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_new_time_based_trigger_condition">new_time_based_trigger_condition</a>(time_secs: u64): <a href="order_book_types.md#0x7_order_book_types_TriggerCondition">TriggerCondition</a> {
    TriggerCondition::TimeBased(time_secs)
}
</code></pre>



</details>

<a id="0x7_order_book_types_price_move_up_condition"></a>

## Function `price_move_up_condition`



<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_price_move_up_condition">price_move_up_condition</a>(price: u64): <a href="order_book_types.md#0x7_order_book_types_TriggerCondition">order_book_types::TriggerCondition</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_price_move_up_condition">price_move_up_condition</a>(price: u64): <a href="order_book_types.md#0x7_order_book_types_TriggerCondition">TriggerCondition</a> {
    TriggerCondition::PriceMoveAbove(price)
}
</code></pre>



</details>

<a id="0x7_order_book_types_price_move_down_condition"></a>

## Function `price_move_down_condition`



<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_price_move_down_condition">price_move_down_condition</a>(price: u64): <a href="order_book_types.md#0x7_order_book_types_TriggerCondition">order_book_types::TriggerCondition</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_price_move_down_condition">price_move_down_condition</a>(price: u64): <a href="order_book_types.md#0x7_order_book_types_TriggerCondition">TriggerCondition</a> {
    TriggerCondition::PriceMoveBelow(price)
}
</code></pre>



</details>

<a id="0x7_order_book_types_index"></a>

## Function `index`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_index">index</a>(self: &<a href="order_book_types.md#0x7_order_book_types_TriggerCondition">order_book_types::TriggerCondition</a>): (<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_index">index</a>(self: &<a href="order_book_types.md#0x7_order_book_types_TriggerCondition">TriggerCondition</a>):
    (<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;) {
    match(self) {
        TriggerCondition::PriceMoveAbove(price) =&gt; {
            (<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(*price), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>())
        }
        TriggerCondition::PriceMoveBelow(price) =&gt; {
            (<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(*price), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>())
        }
        TriggerCondition::TimeBased(time) =&gt; {
            (<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(*time))
        }
    }
}
</code></pre>



</details>

<a id="0x7_order_book_types_destroy_order_match"></a>

## Function `destroy_order_match`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_destroy_order_match">destroy_order_match</a>&lt;M: <b>copy</b>, drop, store&gt;(self: <a href="order_book_types.md#0x7_order_book_types_OrderMatch">order_book_types::OrderMatch</a>&lt;M&gt;): (<a href="order_book_types.md#0x7_order_book_types_OrderMatchDetails">order_book_types::OrderMatchDetails</a>&lt;M&gt;, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_destroy_order_match">destroy_order_match</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: <a href="order_book_types.md#0x7_order_book_types_OrderMatch">OrderMatch</a>&lt;M&gt;,
): (<a href="order_book_types.md#0x7_order_book_types_OrderMatchDetails">OrderMatchDetails</a>&lt;M&gt;, u64) {
    <b>let</b> OrderMatch::V1 { order, matched_size } = self;
    (order, matched_size)
}
</code></pre>



</details>

<a id="0x7_order_book_types_destroy_single_order_match_details"></a>

## Function `destroy_single_order_match_details`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_destroy_single_order_match_details">destroy_single_order_match_details</a>&lt;M: <b>copy</b>, drop, store&gt;(self: <a href="order_book_types.md#0x7_order_book_types_OrderMatchDetails">order_book_types::OrderMatchDetails</a>&lt;M&gt;): (<a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>, <b>address</b>, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, <a href="order_book_types.md#0x7_order_book_types_UniqueIdxType">order_book_types::UniqueIdxType</a>, u64, u64, u64, bool, <a href="order_book_types.md#0x7_order_book_types_TimeInForce">order_book_types::TimeInForce</a>, u64, M)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_destroy_single_order_match_details">destroy_single_order_match_details</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: <a href="order_book_types.md#0x7_order_book_types_OrderMatchDetails">OrderMatchDetails</a>&lt;M&gt;,
): (<a href="order_book_types.md#0x7_order_book_types_OrderIdType">OrderIdType</a>, <b>address</b>, Option&lt;String&gt;, <a href="order_book_types.md#0x7_order_book_types_UniqueIdxType">UniqueIdxType</a>, u64, u64, u64, bool, <a href="order_book_types.md#0x7_order_book_types_TimeInForce">TimeInForce</a>, u64, M) {
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
        metadata,
    } = self;
    (order_id, <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>, client_order_id, unique_priority_idx, price, orig_size, remaining_size, is_bid, time_in_force, creation_time_micros, metadata)
}
</code></pre>



</details>

<a id="0x7_order_book_types_destroy_bulk_order_match_details"></a>

## Function `destroy_bulk_order_match_details`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_destroy_bulk_order_match_details">destroy_bulk_order_match_details</a>&lt;M: <b>copy</b>, drop, store&gt;(self: <a href="order_book_types.md#0x7_order_book_types_OrderMatchDetails">order_book_types::OrderMatchDetails</a>&lt;M&gt;): (<a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>, <b>address</b>, <a href="order_book_types.md#0x7_order_book_types_UniqueIdxType">order_book_types::UniqueIdxType</a>, u64, u64, bool, u64, u64, M)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_destroy_bulk_order_match_details">destroy_bulk_order_match_details</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: <a href="order_book_types.md#0x7_order_book_types_OrderMatchDetails">OrderMatchDetails</a>&lt;M&gt;,
): (<a href="order_book_types.md#0x7_order_book_types_OrderIdType">OrderIdType</a>, <b>address</b>, <a href="order_book_types.md#0x7_order_book_types_UniqueIdxType">UniqueIdxType</a>, u64, u64, bool, u64, u64, M) {
    <b>let</b> OrderMatchDetails::BulkOrder {
        order_id,
        <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>,
        unique_priority_idx,
        price,
        remaining_size,
        is_bid,
        sequence_number,
        creation_time_micros,
        metadata,
    } = self;
    (order_id, <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>, unique_priority_idx, price, remaining_size, is_bid, sequence_number, creation_time_micros, metadata)
}
</code></pre>



</details>

<a id="0x7_order_book_types_get_matched_size"></a>

## Function `get_matched_size`



<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_get_matched_size">get_matched_size</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="order_book_types.md#0x7_order_book_types_OrderMatch">order_book_types::OrderMatch</a>&lt;M&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_get_matched_size">get_matched_size</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="order_book_types.md#0x7_order_book_types_OrderMatch">OrderMatch</a>&lt;M&gt;,
): u64 {
    self.matched_size
}
</code></pre>



</details>

<a id="0x7_order_book_types_get_account_from_match_details"></a>

## Function `get_account_from_match_details`

Validates that a reinsertion request is valid for the given original order.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_get_account_from_match_details">get_account_from_match_details</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="order_book_types.md#0x7_order_book_types_OrderMatchDetails">order_book_types::OrderMatchDetails</a>&lt;M&gt;): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_get_account_from_match_details">get_account_from_match_details</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="order_book_types.md#0x7_order_book_types_OrderMatchDetails">OrderMatchDetails</a>&lt;M&gt;,
): <b>address</b> {
    self.<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>
}
</code></pre>



</details>

<a id="0x7_order_book_types_get_order_id_from_match_details"></a>

## Function `get_order_id_from_match_details`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_get_order_id_from_match_details">get_order_id_from_match_details</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="order_book_types.md#0x7_order_book_types_OrderMatchDetails">order_book_types::OrderMatchDetails</a>&lt;M&gt;): <a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_get_order_id_from_match_details">get_order_id_from_match_details</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="order_book_types.md#0x7_order_book_types_OrderMatchDetails">OrderMatchDetails</a>&lt;M&gt;,
): <a href="order_book_types.md#0x7_order_book_types_OrderIdType">OrderIdType</a> {
    self.order_id
}
</code></pre>



</details>

<a id="0x7_order_book_types_get_unique_priority_idx_from_match_details"></a>

## Function `get_unique_priority_idx_from_match_details`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_get_unique_priority_idx_from_match_details">get_unique_priority_idx_from_match_details</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="order_book_types.md#0x7_order_book_types_OrderMatchDetails">order_book_types::OrderMatchDetails</a>&lt;M&gt;): <a href="order_book_types.md#0x7_order_book_types_UniqueIdxType">order_book_types::UniqueIdxType</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_get_unique_priority_idx_from_match_details">get_unique_priority_idx_from_match_details</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="order_book_types.md#0x7_order_book_types_OrderMatchDetails">OrderMatchDetails</a>&lt;M&gt;,
): <a href="order_book_types.md#0x7_order_book_types_UniqueIdxType">UniqueIdxType</a> {
    self.unique_priority_idx
}
</code></pre>



</details>

<a id="0x7_order_book_types_get_price_from_match_details"></a>

## Function `get_price_from_match_details`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_get_price_from_match_details">get_price_from_match_details</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="order_book_types.md#0x7_order_book_types_OrderMatchDetails">order_book_types::OrderMatchDetails</a>&lt;M&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_get_price_from_match_details">get_price_from_match_details</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="order_book_types.md#0x7_order_book_types_OrderMatchDetails">OrderMatchDetails</a>&lt;M&gt;,
): u64 {
    self.price
}
</code></pre>



</details>

<a id="0x7_order_book_types_get_orig_size_from_match_details"></a>

## Function `get_orig_size_from_match_details`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_get_orig_size_from_match_details">get_orig_size_from_match_details</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="order_book_types.md#0x7_order_book_types_OrderMatchDetails">order_book_types::OrderMatchDetails</a>&lt;M&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_get_orig_size_from_match_details">get_orig_size_from_match_details</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="order_book_types.md#0x7_order_book_types_OrderMatchDetails">OrderMatchDetails</a>&lt;M&gt;,
): u64 {
    self.orig_size
}
</code></pre>



</details>

<a id="0x7_order_book_types_get_remaining_size_from_match_details"></a>

## Function `get_remaining_size_from_match_details`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_get_remaining_size_from_match_details">get_remaining_size_from_match_details</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="order_book_types.md#0x7_order_book_types_OrderMatchDetails">order_book_types::OrderMatchDetails</a>&lt;M&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_get_remaining_size_from_match_details">get_remaining_size_from_match_details</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="order_book_types.md#0x7_order_book_types_OrderMatchDetails">OrderMatchDetails</a>&lt;M&gt;,
): u64 {
    self.remaining_size
}
</code></pre>



</details>

<a id="0x7_order_book_types_get_time_in_force_from_match_details"></a>

## Function `get_time_in_force_from_match_details`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_get_time_in_force_from_match_details">get_time_in_force_from_match_details</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="order_book_types.md#0x7_order_book_types_OrderMatchDetails">order_book_types::OrderMatchDetails</a>&lt;M&gt;): <a href="order_book_types.md#0x7_order_book_types_TimeInForce">order_book_types::TimeInForce</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_get_time_in_force_from_match_details">get_time_in_force_from_match_details</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="order_book_types.md#0x7_order_book_types_OrderMatchDetails">OrderMatchDetails</a>&lt;M&gt;,
): <a href="order_book_types.md#0x7_order_book_types_TimeInForce">TimeInForce</a> {
    <b>if</b> (self is OrderMatchDetails::SingleOrder) {
        self.time_in_force
    } <b>else</b> {
        <a href="order_book_types.md#0x7_order_book_types_good_till_cancelled">good_till_cancelled</a>()
    }
}
</code></pre>



</details>

<a id="0x7_order_book_types_get_metadata_from_match_details"></a>

## Function `get_metadata_from_match_details`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_get_metadata_from_match_details">get_metadata_from_match_details</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="order_book_types.md#0x7_order_book_types_OrderMatchDetails">order_book_types::OrderMatchDetails</a>&lt;M&gt;): M
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_get_metadata_from_match_details">get_metadata_from_match_details</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="order_book_types.md#0x7_order_book_types_OrderMatchDetails">OrderMatchDetails</a>&lt;M&gt;,
): M {
    self.metadata
}
</code></pre>



</details>

<a id="0x7_order_book_types_get_client_order_id_from_match_details"></a>

## Function `get_client_order_id_from_match_details`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_get_client_order_id_from_match_details">get_client_order_id_from_match_details</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="order_book_types.md#0x7_order_book_types_OrderMatchDetails">order_book_types::OrderMatchDetails</a>&lt;M&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_get_client_order_id_from_match_details">get_client_order_id_from_match_details</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="order_book_types.md#0x7_order_book_types_OrderMatchDetails">OrderMatchDetails</a>&lt;M&gt;,
): Option&lt;String&gt; {
    <b>if</b> (self is OrderMatchDetails::SingleOrder) {
        <b>return</b> self.client_order_id
    } <b>else</b> {
        <b>return</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
    }
}
</code></pre>



</details>

<a id="0x7_order_book_types_is_bid_from_match_details"></a>

## Function `is_bid_from_match_details`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_is_bid_from_match_details">is_bid_from_match_details</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="order_book_types.md#0x7_order_book_types_OrderMatchDetails">order_book_types::OrderMatchDetails</a>&lt;M&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_is_bid_from_match_details">is_bid_from_match_details</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="order_book_types.md#0x7_order_book_types_OrderMatchDetails">OrderMatchDetails</a>&lt;M&gt;,
): bool {
    self.is_bid
}
</code></pre>



</details>

<a id="0x7_order_book_types_get_book_type_from_match_details"></a>

## Function `get_book_type_from_match_details`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_get_book_type_from_match_details">get_book_type_from_match_details</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="order_book_types.md#0x7_order_book_types_OrderMatchDetails">order_book_types::OrderMatchDetails</a>&lt;M&gt;): <a href="order_book_types.md#0x7_order_book_types_OrderType">order_book_types::OrderType</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_get_book_type_from_match_details">get_book_type_from_match_details</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="order_book_types.md#0x7_order_book_types_OrderMatchDetails">OrderMatchDetails</a>&lt;M&gt;,
): <a href="order_book_types.md#0x7_order_book_types_OrderType">OrderType</a> {
    <b>if</b> (self is OrderMatchDetails::SingleOrder) {
        <a href="order_book_types.md#0x7_order_book_types_single_order_type">single_order_type</a>()
    } <b>else</b> {
        <a href="order_book_types.md#0x7_order_book_types_bulk_order_type">bulk_order_type</a>()
    }
}
</code></pre>



</details>

<a id="0x7_order_book_types_is_bulk_order_from_match_details"></a>

## Function `is_bulk_order_from_match_details`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_is_bulk_order_from_match_details">is_bulk_order_from_match_details</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="order_book_types.md#0x7_order_book_types_OrderMatchDetails">order_book_types::OrderMatchDetails</a>&lt;M&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_is_bulk_order_from_match_details">is_bulk_order_from_match_details</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="order_book_types.md#0x7_order_book_types_OrderMatchDetails">OrderMatchDetails</a>&lt;M&gt;,
): bool {
    self is OrderMatchDetails::BulkOrder
}
</code></pre>



</details>

<a id="0x7_order_book_types_is_single_order_from_match_details"></a>

## Function `is_single_order_from_match_details`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_is_single_order_from_match_details">is_single_order_from_match_details</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="order_book_types.md#0x7_order_book_types_OrderMatchDetails">order_book_types::OrderMatchDetails</a>&lt;M&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_is_single_order_from_match_details">is_single_order_from_match_details</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="order_book_types.md#0x7_order_book_types_OrderMatchDetails">OrderMatchDetails</a>&lt;M&gt;,
): bool {
    self is OrderMatchDetails::SingleOrder
}
</code></pre>



</details>

<a id="0x7_order_book_types_get_sequence_number_from_match_details"></a>

## Function `get_sequence_number_from_match_details`

This should only be called on bulk orders, aborts if called for non-bulk order.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_get_sequence_number_from_match_details">get_sequence_number_from_match_details</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="order_book_types.md#0x7_order_book_types_OrderMatchDetails">order_book_types::OrderMatchDetails</a>&lt;M&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_get_sequence_number_from_match_details">get_sequence_number_from_match_details</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="order_book_types.md#0x7_order_book_types_OrderMatchDetails">OrderMatchDetails</a>&lt;M&gt;,
): u64 {
    self.sequence_number
}
</code></pre>



</details>

<a id="0x7_order_book_types_get_creation_time_micros_from_match_details"></a>

## Function `get_creation_time_micros_from_match_details`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_get_creation_time_micros_from_match_details">get_creation_time_micros_from_match_details</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="order_book_types.md#0x7_order_book_types_OrderMatchDetails">order_book_types::OrderMatchDetails</a>&lt;M&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_get_creation_time_micros_from_match_details">get_creation_time_micros_from_match_details</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="order_book_types.md#0x7_order_book_types_OrderMatchDetails">OrderMatchDetails</a>&lt;M&gt;,
): u64 {
    self.creation_time_micros
}
</code></pre>



</details>

<a id="0x7_order_book_types_new_single_order_match_details"></a>

## Function `new_single_order_match_details`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_new_single_order_match_details">new_single_order_match_details</a>&lt;M: <b>copy</b>, drop, store&gt;(order_id: <a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>, <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>, client_order_id: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, unique_priority_idx: <a href="order_book_types.md#0x7_order_book_types_UniqueIdxType">order_book_types::UniqueIdxType</a>, price: u64, orig_size: u64, remaining_size: u64, is_bid: bool, time_in_force: <a href="order_book_types.md#0x7_order_book_types_TimeInForce">order_book_types::TimeInForce</a>, creation_time_micros: u64, metadata: M): <a href="order_book_types.md#0x7_order_book_types_OrderMatchDetails">order_book_types::OrderMatchDetails</a>&lt;M&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_new_single_order_match_details">new_single_order_match_details</a>&lt;M: store + <b>copy</b> + drop&gt;(
    order_id: <a href="order_book_types.md#0x7_order_book_types_OrderIdType">OrderIdType</a>,
    <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>,
    client_order_id: Option&lt;String&gt;,
    unique_priority_idx: <a href="order_book_types.md#0x7_order_book_types_UniqueIdxType">UniqueIdxType</a>,
    price: u64,
    orig_size: u64,
    remaining_size: u64,
    is_bid: bool,
    time_in_force: <a href="order_book_types.md#0x7_order_book_types_TimeInForce">TimeInForce</a>,
    creation_time_micros: u64,
    metadata: M
): <a href="order_book_types.md#0x7_order_book_types_OrderMatchDetails">OrderMatchDetails</a>&lt;M&gt; {
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
        metadata,
    }
}
</code></pre>



</details>

<a id="0x7_order_book_types_new_bulk_order_match_details"></a>

## Function `new_bulk_order_match_details`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_new_bulk_order_match_details">new_bulk_order_match_details</a>&lt;M: <b>copy</b>, drop, store&gt;(order_id: <a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>, <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>, unique_priority_idx: <a href="order_book_types.md#0x7_order_book_types_UniqueIdxType">order_book_types::UniqueIdxType</a>, price: u64, remaining_size: u64, is_bid: bool, sequence_number: u64, creation_time_micros: u64, metadata: M): <a href="order_book_types.md#0x7_order_book_types_OrderMatchDetails">order_book_types::OrderMatchDetails</a>&lt;M&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_new_bulk_order_match_details">new_bulk_order_match_details</a>&lt;M: store + <b>copy</b> + drop&gt;(
    order_id: <a href="order_book_types.md#0x7_order_book_types_OrderIdType">OrderIdType</a>,
    <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>,
    unique_priority_idx: <a href="order_book_types.md#0x7_order_book_types_UniqueIdxType">UniqueIdxType</a>,
    price: u64,
    remaining_size: u64,
    is_bid: bool,
    sequence_number: u64,
    creation_time_micros: u64,
    metadata: M
): <a href="order_book_types.md#0x7_order_book_types_OrderMatchDetails">OrderMatchDetails</a>&lt;M&gt; {
    OrderMatchDetails::BulkOrder {
        order_id,
        <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>,
        unique_priority_idx,
        price,
        remaining_size,
        is_bid,
        sequence_number,
        creation_time_micros,
        metadata,
    }
}
</code></pre>



</details>

<a id="0x7_order_book_types_new_order_match_details_with_modified_size"></a>

## Function `new_order_match_details_with_modified_size`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_new_order_match_details_with_modified_size">new_order_match_details_with_modified_size</a>&lt;M: <b>copy</b>, drop, store&gt;(self: <a href="order_book_types.md#0x7_order_book_types_OrderMatchDetails">order_book_types::OrderMatchDetails</a>&lt;M&gt;, remaining_size: u64): <a href="order_book_types.md#0x7_order_book_types_OrderMatchDetails">order_book_types::OrderMatchDetails</a>&lt;M&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_new_order_match_details_with_modified_size">new_order_match_details_with_modified_size</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: <a href="order_book_types.md#0x7_order_book_types_OrderMatchDetails">OrderMatchDetails</a>&lt;M&gt;,
    remaining_size: u64
): <a href="order_book_types.md#0x7_order_book_types_OrderMatchDetails">OrderMatchDetails</a>&lt;M&gt; {
    self.remaining_size = remaining_size;
    self
}
</code></pre>



</details>

<a id="0x7_order_book_types_new_order_match"></a>

## Function `new_order_match`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_new_order_match">new_order_match</a>&lt;M: <b>copy</b>, drop, store&gt;(order: <a href="order_book_types.md#0x7_order_book_types_OrderMatchDetails">order_book_types::OrderMatchDetails</a>&lt;M&gt;, matched_size: u64): <a href="order_book_types.md#0x7_order_book_types_OrderMatch">order_book_types::OrderMatch</a>&lt;M&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_new_order_match">new_order_match</a>&lt;M: store + <b>copy</b> + drop&gt;(
    order: <a href="order_book_types.md#0x7_order_book_types_OrderMatchDetails">OrderMatchDetails</a>&lt;M&gt;,
    matched_size: u64
): <a href="order_book_types.md#0x7_order_book_types_OrderMatch">OrderMatch</a>&lt;M&gt; {
    OrderMatch::V1 {
        order,
        matched_size
    }
}
</code></pre>



</details>

<a id="0x7_order_book_types_validate_single_order_reinsertion_request"></a>

## Function `validate_single_order_reinsertion_request`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_validate_single_order_reinsertion_request">validate_single_order_reinsertion_request</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="order_book_types.md#0x7_order_book_types_OrderMatchDetails">order_book_types::OrderMatchDetails</a>&lt;M&gt;, other: &<a href="order_book_types.md#0x7_order_book_types_OrderMatchDetails">order_book_types::OrderMatchDetails</a>&lt;M&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_validate_single_order_reinsertion_request">validate_single_order_reinsertion_request</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="order_book_types.md#0x7_order_book_types_OrderMatchDetails">OrderMatchDetails</a>&lt;M&gt;,
    other: &<a href="order_book_types.md#0x7_order_book_types_OrderMatchDetails">OrderMatchDetails</a>&lt;M&gt;,
): bool {
    <b>assert</b>!(self is OrderMatchDetails::SingleOrder, <a href="order_book_types.md#0x7_order_book_types_E_REINSERT_ORDER_MISMATCH">E_REINSERT_ORDER_MISMATCH</a>);
    <b>assert</b>!(other is OrderMatchDetails::SingleOrder, <a href="order_book_types.md#0x7_order_book_types_E_REINSERT_ORDER_MISMATCH">E_REINSERT_ORDER_MISMATCH</a>);

    self.order_id == other.order_id &&
    self.<a href="../../aptos-framework/doc/account.md#0x1_account">account</a> == other.<a href="../../aptos-framework/doc/account.md#0x1_account">account</a> &&
    self.unique_priority_idx == other.unique_priority_idx &&
    self.price == other.price &&
    self.orig_size == other.orig_size &&
    self.is_bid == other.is_bid
}
</code></pre>



</details>

<a id="0x7_order_book_types_validate_bulk_order_reinsertion_request"></a>

## Function `validate_bulk_order_reinsertion_request`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_validate_bulk_order_reinsertion_request">validate_bulk_order_reinsertion_request</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="order_book_types.md#0x7_order_book_types_OrderMatchDetails">order_book_types::OrderMatchDetails</a>&lt;M&gt;, other: &<a href="order_book_types.md#0x7_order_book_types_OrderMatchDetails">order_book_types::OrderMatchDetails</a>&lt;M&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_validate_bulk_order_reinsertion_request">validate_bulk_order_reinsertion_request</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="order_book_types.md#0x7_order_book_types_OrderMatchDetails">OrderMatchDetails</a>&lt;M&gt;,
    other: &<a href="order_book_types.md#0x7_order_book_types_OrderMatchDetails">OrderMatchDetails</a>&lt;M&gt;,
): bool {
    <b>assert</b>!(self is OrderMatchDetails::BulkOrder, <a href="order_book_types.md#0x7_order_book_types_E_REINSERT_ORDER_MISMATCH">E_REINSERT_ORDER_MISMATCH</a>);
    <b>assert</b>!(other is OrderMatchDetails::BulkOrder, <a href="order_book_types.md#0x7_order_book_types_E_REINSERT_ORDER_MISMATCH">E_REINSERT_ORDER_MISMATCH</a>);

    self.order_id == other.order_id &&
    self.<a href="../../aptos-framework/doc/account.md#0x1_account">account</a> == other.<a href="../../aptos-framework/doc/account.md#0x1_account">account</a> &&
    self.unique_priority_idx == other.unique_priority_idx &&
    self.price == other.price &&
    self.is_bid == other.is_bid &&
    self.sequence_number == other.sequence_number
}
</code></pre>



</details>

<a id="0x7_order_book_types_new_active_matched_order"></a>

## Function `new_active_matched_order`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_new_active_matched_order">new_active_matched_order</a>(order_id: <a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>, matched_size: u64, remaining_size: u64, order_book_type: <a href="order_book_types.md#0x7_order_book_types_OrderType">order_book_types::OrderType</a>): <a href="order_book_types.md#0x7_order_book_types_ActiveMatchedOrder">order_book_types::ActiveMatchedOrder</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_new_active_matched_order">new_active_matched_order</a>(
    order_id: <a href="order_book_types.md#0x7_order_book_types_OrderIdType">OrderIdType</a>, matched_size: u64, remaining_size: u64, order_book_type: <a href="order_book_types.md#0x7_order_book_types_OrderType">OrderType</a>
): <a href="order_book_types.md#0x7_order_book_types_ActiveMatchedOrder">ActiveMatchedOrder</a> {
    <a href="order_book_types.md#0x7_order_book_types_ActiveMatchedOrder">ActiveMatchedOrder</a> { order_id, matched_size, remaining_size, order_book_type }
}
</code></pre>



</details>

<a id="0x7_order_book_types_destroy_active_matched_order"></a>

## Function `destroy_active_matched_order`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_destroy_active_matched_order">destroy_active_matched_order</a>(self: <a href="order_book_types.md#0x7_order_book_types_ActiveMatchedOrder">order_book_types::ActiveMatchedOrder</a>): (<a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>, u64, u64, <a href="order_book_types.md#0x7_order_book_types_OrderType">order_book_types::OrderType</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_destroy_active_matched_order">destroy_active_matched_order</a>(
    self: <a href="order_book_types.md#0x7_order_book_types_ActiveMatchedOrder">ActiveMatchedOrder</a>
): (<a href="order_book_types.md#0x7_order_book_types_OrderIdType">OrderIdType</a>, u64, u64, <a href="order_book_types.md#0x7_order_book_types_OrderType">OrderType</a>) {
    <b>let</b> <a href="order_book_types.md#0x7_order_book_types_ActiveMatchedOrder">ActiveMatchedOrder</a> { order_id, matched_size, remaining_size, order_book_type } = self;
    (order_id, matched_size, remaining_size, order_book_type)
}
</code></pre>



</details>

<a id="0x7_order_book_types_get_active_matched_size"></a>

## Function `get_active_matched_size`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_get_active_matched_size">get_active_matched_size</a>(self: &<a href="order_book_types.md#0x7_order_book_types_ActiveMatchedOrder">order_book_types::ActiveMatchedOrder</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_get_active_matched_size">get_active_matched_size</a>(self: &<a href="order_book_types.md#0x7_order_book_types_ActiveMatchedOrder">ActiveMatchedOrder</a>): u64 {
    self.matched_size
}
</code></pre>



</details>

<a id="0x7_order_book_types_is_active_matched_book_type_single_order"></a>

## Function `is_active_matched_book_type_single_order`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_is_active_matched_book_type_single_order">is_active_matched_book_type_single_order</a>(self: &<a href="order_book_types.md#0x7_order_book_types_ActiveMatchedOrder">order_book_types::ActiveMatchedOrder</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_is_active_matched_book_type_single_order">is_active_matched_book_type_single_order</a>(
    self: &<a href="order_book_types.md#0x7_order_book_types_ActiveMatchedOrder">ActiveMatchedOrder</a>
): bool {
    <a href="order_book_types.md#0x7_order_book_types_is_single_order_type">is_single_order_type</a>(&self.order_book_type)
}
</code></pre>



</details>

<a id="0x7_order_book_types_reverse_bits"></a>

## Function `reverse_bits`

Reverse the bits in a u128 value using divide and conquer approach
This is more efficient than the bit-by-bit approach, reducing from O(n) to O(log n)


<pre><code><b>fun</b> <a href="order_book_types.md#0x7_order_book_types_reverse_bits">reverse_bits</a>(value: u128): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="order_book_types.md#0x7_order_book_types_reverse_bits">reverse_bits</a>(value: u128): u128 {
    <b>let</b> v = value;

    // Swap odd and even bits
    v = ((v & 0x55555555555555555555555555555555) &lt;&lt; 1) | ((v &gt;&gt; 1) & 0x55555555555555555555555555555555);

    // Swap consecutive pairs
    v = ((v & 0x33333333333333333333333333333333) &lt;&lt; 2) | ((v &gt;&gt; 2) & 0x33333333333333333333333333333333);

    // Swap nibbles
    v = ((v & 0x0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f) &lt;&lt; 4) | ((v &gt;&gt; 4) & 0x0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f);

    // Swap bytes
    v = ((v & 0x00ff00ff00ff00ff00ff00ff00ff00ff) &lt;&lt; 8) | ((v &gt;&gt; 8) & 0x00ff00ff00ff00ff00ff00ff00ff00ff);

    // Swap 2-byte chunks
    v = ((v & 0x0000ffff0000ffff0000ffff0000ffff) &lt;&lt; 16) | ((v &gt;&gt; 16) & 0x0000ffff0000ffff0000ffff0000ffff);

    // Swap 4-byte chunks
    v = ((v & 0x00000000ffffffff00000000ffffffff) &lt;&lt; 32) | ((v &gt;&gt; 32) & 0x00000000ffffffff00000000ffffffff);

    // Swap 8-byte chunks
    v = (v &lt;&lt; 64) | (v &gt;&gt; 64);

    v
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
