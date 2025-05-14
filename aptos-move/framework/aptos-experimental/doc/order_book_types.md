
<a id="0x7_order_book_types"></a>

# Module `0x7::order_book_types`

(work in progress)


-  [Struct `OrderIdType`](#0x7_order_book_types_OrderIdType)
-  [Struct `UniqueIdxType`](#0x7_order_book_types_UniqueIdxType)
-  [Struct `ActiveMatchedOrder`](#0x7_order_book_types_ActiveMatchedOrder)
-  [Struct `SingleOrderMatch`](#0x7_order_book_types_SingleOrderMatch)
-  [Struct `Order`](#0x7_order_book_types_Order)
-  [Enum `TriggerCondition`](#0x7_order_book_types_TriggerCondition)
-  [Struct `OrderWithState`](#0x7_order_book_types_OrderWithState)
-  [Constants](#@Constants_0)
-  [Function `new_default_big_ordered_map`](#0x7_order_book_types_new_default_big_ordered_map)
-  [Function `get_slippage_pct_precision`](#0x7_order_book_types_get_slippage_pct_precision)
-  [Function `new_time_based_trigger_condition`](#0x7_order_book_types_new_time_based_trigger_condition)
-  [Function `new_order_id_type`](#0x7_order_book_types_new_order_id_type)
-  [Function `generate_unique_idx_fifo_tiebraker`](#0x7_order_book_types_generate_unique_idx_fifo_tiebraker)
-  [Function `new_unique_idx_type`](#0x7_order_book_types_new_unique_idx_type)
-  [Function `descending_idx`](#0x7_order_book_types_descending_idx)
-  [Function `new_active_matched_order`](#0x7_order_book_types_new_active_matched_order)
-  [Function `destroy_active_matched_order`](#0x7_order_book_types_destroy_active_matched_order)
-  [Function `new_order`](#0x7_order_book_types_new_order)
-  [Function `new_single_order_match`](#0x7_order_book_types_new_single_order_match)
-  [Function `get_active_matched_size`](#0x7_order_book_types_get_active_matched_size)
-  [Function `get_matched_size`](#0x7_order_book_types_get_matched_size)
-  [Function `new_order_with_state`](#0x7_order_book_types_new_order_with_state)
-  [Function `tp_trigger_condition`](#0x7_order_book_types_tp_trigger_condition)
-  [Function `sl_trigger_condition`](#0x7_order_book_types_sl_trigger_condition)
-  [Function `index`](#0x7_order_book_types_index)
-  [Function `get_order_from_state`](#0x7_order_book_types_get_order_from_state)
-  [Function `get_metadata_from_state`](#0x7_order_book_types_get_metadata_from_state)
-  [Function `get_order_id`](#0x7_order_book_types_get_order_id)
-  [Function `get_unique_priority_idx`](#0x7_order_book_types_get_unique_priority_idx)
-  [Function `get_metadata_from_order`](#0x7_order_book_types_get_metadata_from_order)
-  [Function `get_trigger_condition_from_order`](#0x7_order_book_types_get_trigger_condition_from_order)
-  [Function `increase_remaining_size`](#0x7_order_book_types_increase_remaining_size)
-  [Function `decrease_remaining_size`](#0x7_order_book_types_decrease_remaining_size)
-  [Function `set_remaining_size`](#0x7_order_book_types_set_remaining_size)
-  [Function `get_remaining_size_from_state`](#0x7_order_book_types_get_remaining_size_from_state)
-  [Function `get_unique_priority_idx_from_state`](#0x7_order_book_types_get_unique_priority_idx_from_state)
-  [Function `get_remaining_size`](#0x7_order_book_types_get_remaining_size)
-  [Function `get_orig_size`](#0x7_order_book_types_get_orig_size)
-  [Function `destroy_order_from_state`](#0x7_order_book_types_destroy_order_from_state)
-  [Function `destroy_active_match_order`](#0x7_order_book_types_destroy_active_match_order)
-  [Function `destroy_order`](#0x7_order_book_types_destroy_order)
-  [Function `destroy_single_order_match`](#0x7_order_book_types_destroy_single_order_match)
-  [Function `destroy_order_id_type`](#0x7_order_book_types_destroy_order_id_type)
-  [Function `is_active_order`](#0x7_order_book_types_is_active_order)
-  [Function `get_price`](#0x7_order_book_types_get_price)
-  [Function `is_buy`](#0x7_order_book_types_is_buy)


<pre><code><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">0x1::bcs</a>;
<b>use</b> <a href="../../aptos-framework/doc/big_ordered_map.md#0x1_big_ordered_map">0x1::big_ordered_map</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs">0x1::from_bcs</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
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
<code><a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>account_order_id: u64</code>
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
<code>idx: u256</code>
</dt>
<dd>

</dd>
</dl>


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
</dl>


</details>

<a id="0x7_order_book_types_SingleOrderMatch"></a>

## Struct `SingleOrderMatch`



<pre><code><b>struct</b> <a href="order_book_types.md#0x7_order_book_types_SingleOrderMatch">SingleOrderMatch</a>&lt;M: <b>copy</b>, drop, store&gt; <b>has</b> <b>copy</b>, drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>order: <a href="order_book_types.md#0x7_order_book_types_Order">order_book_types::Order</a>&lt;M&gt;</code>
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

<a id="0x7_order_book_types_Order"></a>

## Struct `Order`



<pre><code><b>struct</b> <a href="order_book_types.md#0x7_order_book_types_Order">Order</a>&lt;M: <b>copy</b>, drop, store&gt; <b>has</b> <b>copy</b>, drop, store
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
<code>is_buy: bool</code>
</dt>
<dd>

</dd>
<dt>
<code>trigger_condition: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="order_book_types.md#0x7_order_book_types_TriggerCondition">order_book_types::TriggerCondition</a>&gt;</code>
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

<a id="0x7_order_book_types_TriggerCondition"></a>

## Enum `TriggerCondition`



<pre><code>enum <a href="order_book_types.md#0x7_order_book_types_TriggerCondition">TriggerCondition</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>TakeProfit</summary>


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
<summary>StopLoss</summary>


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

<a id="0x7_order_book_types_OrderWithState"></a>

## Struct `OrderWithState`



<pre><code><b>struct</b> <a href="order_book_types.md#0x7_order_book_types_OrderWithState">OrderWithState</a>&lt;M: <b>copy</b>, drop, store&gt; <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>order: <a href="order_book_types.md#0x7_order_book_types_Order">order_book_types::Order</a>&lt;M&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>is_active: bool</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x7_order_book_types_U256_MAX"></a>



<pre><code><b>const</b> <a href="order_book_types.md#0x7_order_book_types_U256_MAX">U256_MAX</a>: u256 = 115792089237316195423570985008687907853269984665640564039457584007913129639935;
</code></pre>



<a id="0x7_order_book_types_BIG_MAP_INNER_DEGREE"></a>



<pre><code><b>const</b> <a href="order_book_types.md#0x7_order_book_types_BIG_MAP_INNER_DEGREE">BIG_MAP_INNER_DEGREE</a>: u16 = 64;
</code></pre>



<a id="0x7_order_book_types_BIG_MAP_LEAF_DEGREE"></a>



<pre><code><b>const</b> <a href="order_book_types.md#0x7_order_book_types_BIG_MAP_LEAF_DEGREE">BIG_MAP_LEAF_DEGREE</a>: u16 = 32;
</code></pre>



<a id="0x7_order_book_types_EINVALID_ORDER_SIZE_DECREASE"></a>



<pre><code><b>const</b> <a href="order_book_types.md#0x7_order_book_types_EINVALID_ORDER_SIZE_DECREASE">EINVALID_ORDER_SIZE_DECREASE</a>: u64 = 4;
</code></pre>



<a id="0x7_order_book_types_EINVALID_TRIGGER_CONDITION"></a>



<pre><code><b>const</b> <a href="order_book_types.md#0x7_order_book_types_EINVALID_TRIGGER_CONDITION">EINVALID_TRIGGER_CONDITION</a>: u64 = 2;
</code></pre>



<a id="0x7_order_book_types_EORDER_ALREADY_EXISTS"></a>



<pre><code><b>const</b> <a href="order_book_types.md#0x7_order_book_types_EORDER_ALREADY_EXISTS">EORDER_ALREADY_EXISTS</a>: u64 = 1;
</code></pre>



<a id="0x7_order_book_types_INVALID_MATCH_RESULT"></a>



<pre><code><b>const</b> <a href="order_book_types.md#0x7_order_book_types_INVALID_MATCH_RESULT">INVALID_MATCH_RESULT</a>: u64 = 3;
</code></pre>



<a id="0x7_order_book_types_SLIPPAGE_PCT_PRECISION"></a>



<pre><code><b>const</b> <a href="order_book_types.md#0x7_order_book_types_SLIPPAGE_PCT_PRECISION">SLIPPAGE_PCT_PRECISION</a>: u64 = 100;
</code></pre>



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

<a id="0x7_order_book_types_get_slippage_pct_precision"></a>

## Function `get_slippage_pct_precision`



<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_get_slippage_pct_precision">get_slippage_pct_precision</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_get_slippage_pct_precision">get_slippage_pct_precision</a>(): u64 {
    <a href="order_book_types.md#0x7_order_book_types_SLIPPAGE_PCT_PRECISION">SLIPPAGE_PCT_PRECISION</a>
}
</code></pre>



</details>

<a id="0x7_order_book_types_new_time_based_trigger_condition"></a>

## Function `new_time_based_trigger_condition`



<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_new_time_based_trigger_condition">new_time_based_trigger_condition</a>(time: u64): <a href="order_book_types.md#0x7_order_book_types_TriggerCondition">order_book_types::TriggerCondition</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_new_time_based_trigger_condition">new_time_based_trigger_condition</a>(time: u64): <a href="order_book_types.md#0x7_order_book_types_TriggerCondition">TriggerCondition</a> {
    TriggerCondition::TimeBased(time)
}
</code></pre>



</details>

<a id="0x7_order_book_types_new_order_id_type"></a>

## Function `new_order_id_type`



<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_new_order_id_type">new_order_id_type</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>, account_order_id: u64): <a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_new_order_id_type">new_order_id_type</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>, account_order_id: u64): <a href="order_book_types.md#0x7_order_book_types_OrderIdType">OrderIdType</a> {
    <a href="order_book_types.md#0x7_order_book_types_OrderIdType">OrderIdType</a> { <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>, account_order_id }
}
</code></pre>



</details>

<a id="0x7_order_book_types_generate_unique_idx_fifo_tiebraker"></a>

## Function `generate_unique_idx_fifo_tiebraker`



<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_generate_unique_idx_fifo_tiebraker">generate_unique_idx_fifo_tiebraker</a>(): <a href="order_book_types.md#0x7_order_book_types_UniqueIdxType">order_book_types::UniqueIdxType</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_generate_unique_idx_fifo_tiebraker">generate_unique_idx_fifo_tiebraker</a>(): <a href="order_book_types.md#0x7_order_book_types_UniqueIdxType">UniqueIdxType</a> {
    // TODO change from random <b>to</b> monothonically increasing value
    <a href="order_book_types.md#0x7_order_book_types_new_unique_idx_type">new_unique_idx_type</a>(
        <a href="../../aptos-framework/../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_to_u256">from_bcs::to_u256</a>(
            <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(&<a href="../../aptos-framework/doc/transaction_context.md#0x1_transaction_context_generate_auid_address">transaction_context::generate_auid_address</a>())
        )
    )
}
</code></pre>



</details>

<a id="0x7_order_book_types_new_unique_idx_type"></a>

## Function `new_unique_idx_type`



<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_new_unique_idx_type">new_unique_idx_type</a>(idx: u256): <a href="order_book_types.md#0x7_order_book_types_UniqueIdxType">order_book_types::UniqueIdxType</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_new_unique_idx_type">new_unique_idx_type</a>(idx: u256): <a href="order_book_types.md#0x7_order_book_types_UniqueIdxType">UniqueIdxType</a> {
    <a href="order_book_types.md#0x7_order_book_types_UniqueIdxType">UniqueIdxType</a> { idx }
}
</code></pre>



</details>

<a id="0x7_order_book_types_descending_idx"></a>

## Function `descending_idx`



<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_descending_idx">descending_idx</a>(self: &<a href="order_book_types.md#0x7_order_book_types_UniqueIdxType">order_book_types::UniqueIdxType</a>): <a href="order_book_types.md#0x7_order_book_types_UniqueIdxType">order_book_types::UniqueIdxType</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_descending_idx">descending_idx</a>(self: &<a href="order_book_types.md#0x7_order_book_types_UniqueIdxType">UniqueIdxType</a>): <a href="order_book_types.md#0x7_order_book_types_UniqueIdxType">UniqueIdxType</a> {
    <a href="order_book_types.md#0x7_order_book_types_UniqueIdxType">UniqueIdxType</a> { idx: <a href="order_book_types.md#0x7_order_book_types_U256_MAX">U256_MAX</a> - self.idx }
}
</code></pre>



</details>

<a id="0x7_order_book_types_new_active_matched_order"></a>

## Function `new_active_matched_order`



<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_new_active_matched_order">new_active_matched_order</a>(order_id: <a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>, matched_size: u64, remaining_size: u64): <a href="order_book_types.md#0x7_order_book_types_ActiveMatchedOrder">order_book_types::ActiveMatchedOrder</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_new_active_matched_order">new_active_matched_order</a>(
    order_id: <a href="order_book_types.md#0x7_order_book_types_OrderIdType">OrderIdType</a>, matched_size: u64, remaining_size: u64
): <a href="order_book_types.md#0x7_order_book_types_ActiveMatchedOrder">ActiveMatchedOrder</a> {
    <a href="order_book_types.md#0x7_order_book_types_ActiveMatchedOrder">ActiveMatchedOrder</a> { order_id, matched_size, remaining_size }
}
</code></pre>



</details>

<a id="0x7_order_book_types_destroy_active_matched_order"></a>

## Function `destroy_active_matched_order`



<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_destroy_active_matched_order">destroy_active_matched_order</a>(self: <a href="order_book_types.md#0x7_order_book_types_ActiveMatchedOrder">order_book_types::ActiveMatchedOrder</a>): (<a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>, u64, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_destroy_active_matched_order">destroy_active_matched_order</a>(self: <a href="order_book_types.md#0x7_order_book_types_ActiveMatchedOrder">ActiveMatchedOrder</a>): (<a href="order_book_types.md#0x7_order_book_types_OrderIdType">OrderIdType</a>, u64, u64) {
    (self.order_id, self.matched_size, self.remaining_size)
}
</code></pre>



</details>

<a id="0x7_order_book_types_new_order"></a>

## Function `new_order`



<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_new_order">new_order</a>&lt;M: <b>copy</b>, drop, store&gt;(order_id: <a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>, unique_priority_idx: <a href="order_book_types.md#0x7_order_book_types_UniqueIdxType">order_book_types::UniqueIdxType</a>, price: u64, orig_size: u64, size: u64, is_buy: bool, trigger_condition: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="order_book_types.md#0x7_order_book_types_TriggerCondition">order_book_types::TriggerCondition</a>&gt;, metadata: M): <a href="order_book_types.md#0x7_order_book_types_Order">order_book_types::Order</a>&lt;M&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_new_order">new_order</a>&lt;M: store + <b>copy</b> + drop&gt;(
    order_id: <a href="order_book_types.md#0x7_order_book_types_OrderIdType">OrderIdType</a>,
    unique_priority_idx: <a href="order_book_types.md#0x7_order_book_types_UniqueIdxType">UniqueIdxType</a>,
    price: u64,
    orig_size: u64,
    size: u64,
    is_buy: bool,
    trigger_condition: Option&lt;<a href="order_book_types.md#0x7_order_book_types_TriggerCondition">TriggerCondition</a>&gt;,
    metadata: M
): <a href="order_book_types.md#0x7_order_book_types_Order">Order</a>&lt;M&gt; {
    <a href="order_book_types.md#0x7_order_book_types_Order">Order</a> {
        order_id,
        unique_priority_idx,
        price,
        orig_size,
        remaining_size: size,
        is_buy,
        trigger_condition,
        metadata
    }
}
</code></pre>



</details>

<a id="0x7_order_book_types_new_single_order_match"></a>

## Function `new_single_order_match`



<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_new_single_order_match">new_single_order_match</a>&lt;M: <b>copy</b>, drop, store&gt;(order: <a href="order_book_types.md#0x7_order_book_types_Order">order_book_types::Order</a>&lt;M&gt;, matched_size: u64): <a href="order_book_types.md#0x7_order_book_types_SingleOrderMatch">order_book_types::SingleOrderMatch</a>&lt;M&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_new_single_order_match">new_single_order_match</a>&lt;M: store + <b>copy</b> + drop&gt;(
    order: <a href="order_book_types.md#0x7_order_book_types_Order">Order</a>&lt;M&gt;, matched_size: u64
): <a href="order_book_types.md#0x7_order_book_types_SingleOrderMatch">SingleOrderMatch</a>&lt;M&gt; {
    <a href="order_book_types.md#0x7_order_book_types_SingleOrderMatch">SingleOrderMatch</a> { order, matched_size }
}
</code></pre>



</details>

<a id="0x7_order_book_types_get_active_matched_size"></a>

## Function `get_active_matched_size`



<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_get_active_matched_size">get_active_matched_size</a>(self: &<a href="order_book_types.md#0x7_order_book_types_ActiveMatchedOrder">order_book_types::ActiveMatchedOrder</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_get_active_matched_size">get_active_matched_size</a>(self: &<a href="order_book_types.md#0x7_order_book_types_ActiveMatchedOrder">ActiveMatchedOrder</a>): u64 {
    self.matched_size
}
</code></pre>



</details>

<a id="0x7_order_book_types_get_matched_size"></a>

## Function `get_matched_size`



<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_get_matched_size">get_matched_size</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="order_book_types.md#0x7_order_book_types_SingleOrderMatch">order_book_types::SingleOrderMatch</a>&lt;M&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_get_matched_size">get_matched_size</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="order_book_types.md#0x7_order_book_types_SingleOrderMatch">SingleOrderMatch</a>&lt;M&gt;
): u64 {
    self.matched_size
}
</code></pre>



</details>

<a id="0x7_order_book_types_new_order_with_state"></a>

## Function `new_order_with_state`



<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_new_order_with_state">new_order_with_state</a>&lt;M: <b>copy</b>, drop, store&gt;(order: <a href="order_book_types.md#0x7_order_book_types_Order">order_book_types::Order</a>&lt;M&gt;, is_active: bool): <a href="order_book_types.md#0x7_order_book_types_OrderWithState">order_book_types::OrderWithState</a>&lt;M&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_new_order_with_state">new_order_with_state</a>&lt;M: store + <b>copy</b> + drop&gt;(
    order: <a href="order_book_types.md#0x7_order_book_types_Order">Order</a>&lt;M&gt;, is_active: bool
): <a href="order_book_types.md#0x7_order_book_types_OrderWithState">OrderWithState</a>&lt;M&gt; {
    <a href="order_book_types.md#0x7_order_book_types_OrderWithState">OrderWithState</a> { order, is_active }
}
</code></pre>



</details>

<a id="0x7_order_book_types_tp_trigger_condition"></a>

## Function `tp_trigger_condition`



<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_tp_trigger_condition">tp_trigger_condition</a>(take_profit: u64): <a href="order_book_types.md#0x7_order_book_types_TriggerCondition">order_book_types::TriggerCondition</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_tp_trigger_condition">tp_trigger_condition</a>(take_profit: u64): <a href="order_book_types.md#0x7_order_book_types_TriggerCondition">TriggerCondition</a> {
    TriggerCondition::TakeProfit(take_profit)
}
</code></pre>



</details>

<a id="0x7_order_book_types_sl_trigger_condition"></a>

## Function `sl_trigger_condition`



<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_sl_trigger_condition">sl_trigger_condition</a>(stop_loss: u64): <a href="order_book_types.md#0x7_order_book_types_TriggerCondition">order_book_types::TriggerCondition</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_sl_trigger_condition">sl_trigger_condition</a>(stop_loss: u64): <a href="order_book_types.md#0x7_order_book_types_TriggerCondition">TriggerCondition</a> {
    TriggerCondition::StopLoss(stop_loss)
}
</code></pre>



</details>

<a id="0x7_order_book_types_index"></a>

## Function `index`



<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_index">index</a>(self: &<a href="order_book_types.md#0x7_order_book_types_TriggerCondition">order_book_types::TriggerCondition</a>, is_buy: bool): (<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_index">index</a>(self: &<a href="order_book_types.md#0x7_order_book_types_TriggerCondition">TriggerCondition</a>, is_buy: bool):
    (Option&lt;u64&gt;, Option&lt;u64&gt;, Option&lt;u64&gt;) {
    match(self) {
        TriggerCondition::TakeProfit(tp) =&gt; {
            <b>if</b> (is_buy) {
                (<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(*tp), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>())
            } <b>else</b> {
                (<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(*tp), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>())
            }
        }
        TriggerCondition::StopLoss(sl) =&gt; {
            <b>if</b> (is_buy) {
                (<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(*sl), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>())
            } <b>else</b> {
                (<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(*sl), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>())
            }
        }
        TriggerCondition::TimeBased(time) =&gt; {
            (<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(*time))
        }
    }
}
</code></pre>



</details>

<a id="0x7_order_book_types_get_order_from_state"></a>

## Function `get_order_from_state`



<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_get_order_from_state">get_order_from_state</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="order_book_types.md#0x7_order_book_types_OrderWithState">order_book_types::OrderWithState</a>&lt;M&gt;): &<a href="order_book_types.md#0x7_order_book_types_Order">order_book_types::Order</a>&lt;M&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_get_order_from_state">get_order_from_state</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="order_book_types.md#0x7_order_book_types_OrderWithState">OrderWithState</a>&lt;M&gt;
): &<a href="order_book_types.md#0x7_order_book_types_Order">Order</a>&lt;M&gt; {
    &self.order
}
</code></pre>



</details>

<a id="0x7_order_book_types_get_metadata_from_state"></a>

## Function `get_metadata_from_state`



<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_get_metadata_from_state">get_metadata_from_state</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="order_book_types.md#0x7_order_book_types_OrderWithState">order_book_types::OrderWithState</a>&lt;M&gt;): M
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_get_metadata_from_state">get_metadata_from_state</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="order_book_types.md#0x7_order_book_types_OrderWithState">OrderWithState</a>&lt;M&gt;
): M {
    self.order.metadata
}
</code></pre>



</details>

<a id="0x7_order_book_types_get_order_id"></a>

## Function `get_order_id`



<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_get_order_id">get_order_id</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="order_book_types.md#0x7_order_book_types_Order">order_book_types::Order</a>&lt;M&gt;): <a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_get_order_id">get_order_id</a>&lt;M: store + <b>copy</b> + drop&gt;(self: &<a href="order_book_types.md#0x7_order_book_types_Order">Order</a>&lt;M&gt;): <a href="order_book_types.md#0x7_order_book_types_OrderIdType">OrderIdType</a> {
    self.order_id
}
</code></pre>



</details>

<a id="0x7_order_book_types_get_unique_priority_idx"></a>

## Function `get_unique_priority_idx`



<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_get_unique_priority_idx">get_unique_priority_idx</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="order_book_types.md#0x7_order_book_types_Order">order_book_types::Order</a>&lt;M&gt;): <a href="order_book_types.md#0x7_order_book_types_UniqueIdxType">order_book_types::UniqueIdxType</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_get_unique_priority_idx">get_unique_priority_idx</a>&lt;M: store + <b>copy</b> + drop&gt;(self: &<a href="order_book_types.md#0x7_order_book_types_Order">Order</a>&lt;M&gt;): <a href="order_book_types.md#0x7_order_book_types_UniqueIdxType">UniqueIdxType</a> {
    self.unique_priority_idx
}
</code></pre>



</details>

<a id="0x7_order_book_types_get_metadata_from_order"></a>

## Function `get_metadata_from_order`



<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_get_metadata_from_order">get_metadata_from_order</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="order_book_types.md#0x7_order_book_types_Order">order_book_types::Order</a>&lt;M&gt;): M
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_get_metadata_from_order">get_metadata_from_order</a>&lt;M: store + <b>copy</b> + drop&gt;(self: &<a href="order_book_types.md#0x7_order_book_types_Order">Order</a>&lt;M&gt;): M {
    self.metadata
}
</code></pre>



</details>

<a id="0x7_order_book_types_get_trigger_condition_from_order"></a>

## Function `get_trigger_condition_from_order`



<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_get_trigger_condition_from_order">get_trigger_condition_from_order</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="order_book_types.md#0x7_order_book_types_Order">order_book_types::Order</a>&lt;M&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="order_book_types.md#0x7_order_book_types_TriggerCondition">order_book_types::TriggerCondition</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_get_trigger_condition_from_order">get_trigger_condition_from_order</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="order_book_types.md#0x7_order_book_types_Order">Order</a>&lt;M&gt;
): Option&lt;<a href="order_book_types.md#0x7_order_book_types_TriggerCondition">TriggerCondition</a>&gt; {
    self.trigger_condition
}
</code></pre>



</details>

<a id="0x7_order_book_types_increase_remaining_size"></a>

## Function `increase_remaining_size`



<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_increase_remaining_size">increase_remaining_size</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="order_book_types.md#0x7_order_book_types_OrderWithState">order_book_types::OrderWithState</a>&lt;M&gt;, size: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_increase_remaining_size">increase_remaining_size</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="order_book_types.md#0x7_order_book_types_OrderWithState">OrderWithState</a>&lt;M&gt;, size: u64
) {
    self.order.remaining_size += size;
}
</code></pre>



</details>

<a id="0x7_order_book_types_decrease_remaining_size"></a>

## Function `decrease_remaining_size`



<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_decrease_remaining_size">decrease_remaining_size</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="order_book_types.md#0x7_order_book_types_OrderWithState">order_book_types::OrderWithState</a>&lt;M&gt;, size: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_decrease_remaining_size">decrease_remaining_size</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="order_book_types.md#0x7_order_book_types_OrderWithState">OrderWithState</a>&lt;M&gt;, size: u64
) {
    <b>assert</b>!(self.order.remaining_size &gt; size, <a href="order_book_types.md#0x7_order_book_types_EINVALID_ORDER_SIZE_DECREASE">EINVALID_ORDER_SIZE_DECREASE</a>);
    self.order.remaining_size -= size;
}
</code></pre>



</details>

<a id="0x7_order_book_types_set_remaining_size"></a>

## Function `set_remaining_size`



<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_set_remaining_size">set_remaining_size</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="order_book_types.md#0x7_order_book_types_OrderWithState">order_book_types::OrderWithState</a>&lt;M&gt;, remaining_size: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_set_remaining_size">set_remaining_size</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="order_book_types.md#0x7_order_book_types_OrderWithState">OrderWithState</a>&lt;M&gt;, remaining_size: u64
) {
    self.order.remaining_size = remaining_size;
}
</code></pre>



</details>

<a id="0x7_order_book_types_get_remaining_size_from_state"></a>

## Function `get_remaining_size_from_state`



<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_get_remaining_size_from_state">get_remaining_size_from_state</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="order_book_types.md#0x7_order_book_types_OrderWithState">order_book_types::OrderWithState</a>&lt;M&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_get_remaining_size_from_state">get_remaining_size_from_state</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="order_book_types.md#0x7_order_book_types_OrderWithState">OrderWithState</a>&lt;M&gt;
): u64 {
    self.order.remaining_size
}
</code></pre>



</details>

<a id="0x7_order_book_types_get_unique_priority_idx_from_state"></a>

## Function `get_unique_priority_idx_from_state`



<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_get_unique_priority_idx_from_state">get_unique_priority_idx_from_state</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="order_book_types.md#0x7_order_book_types_OrderWithState">order_book_types::OrderWithState</a>&lt;M&gt;): <a href="order_book_types.md#0x7_order_book_types_UniqueIdxType">order_book_types::UniqueIdxType</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_get_unique_priority_idx_from_state">get_unique_priority_idx_from_state</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="order_book_types.md#0x7_order_book_types_OrderWithState">OrderWithState</a>&lt;M&gt;
): <a href="order_book_types.md#0x7_order_book_types_UniqueIdxType">UniqueIdxType</a> {
    self.order.unique_priority_idx
}
</code></pre>



</details>

<a id="0x7_order_book_types_get_remaining_size"></a>

## Function `get_remaining_size`



<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_get_remaining_size">get_remaining_size</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="order_book_types.md#0x7_order_book_types_Order">order_book_types::Order</a>&lt;M&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_get_remaining_size">get_remaining_size</a>&lt;M: store + <b>copy</b> + drop&gt;(self: &<a href="order_book_types.md#0x7_order_book_types_Order">Order</a>&lt;M&gt;): u64 {
    self.remaining_size
}
</code></pre>



</details>

<a id="0x7_order_book_types_get_orig_size"></a>

## Function `get_orig_size`



<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_get_orig_size">get_orig_size</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="order_book_types.md#0x7_order_book_types_Order">order_book_types::Order</a>&lt;M&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_get_orig_size">get_orig_size</a>&lt;M: store + <b>copy</b> + drop&gt;(self: &<a href="order_book_types.md#0x7_order_book_types_Order">Order</a>&lt;M&gt;): u64 {
    self.orig_size
}
</code></pre>



</details>

<a id="0x7_order_book_types_destroy_order_from_state"></a>

## Function `destroy_order_from_state`



<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_destroy_order_from_state">destroy_order_from_state</a>&lt;M: <b>copy</b>, drop, store&gt;(self: <a href="order_book_types.md#0x7_order_book_types_OrderWithState">order_book_types::OrderWithState</a>&lt;M&gt;): (<a href="order_book_types.md#0x7_order_book_types_Order">order_book_types::Order</a>&lt;M&gt;, bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_destroy_order_from_state">destroy_order_from_state</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: <a href="order_book_types.md#0x7_order_book_types_OrderWithState">OrderWithState</a>&lt;M&gt;
): (<a href="order_book_types.md#0x7_order_book_types_Order">Order</a>&lt;M&gt;, bool) {
    (self.order, self.is_active)
}
</code></pre>



</details>

<a id="0x7_order_book_types_destroy_active_match_order"></a>

## Function `destroy_active_match_order`



<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_destroy_active_match_order">destroy_active_match_order</a>(self: <a href="order_book_types.md#0x7_order_book_types_ActiveMatchedOrder">order_book_types::ActiveMatchedOrder</a>): (<a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>, u64, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_destroy_active_match_order">destroy_active_match_order</a>(self: <a href="order_book_types.md#0x7_order_book_types_ActiveMatchedOrder">ActiveMatchedOrder</a>): (<a href="order_book_types.md#0x7_order_book_types_OrderIdType">OrderIdType</a>, u64, u64) {
    (self.order_id, self.matched_size, self.remaining_size)
}
</code></pre>



</details>

<a id="0x7_order_book_types_destroy_order"></a>

## Function `destroy_order`



<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_destroy_order">destroy_order</a>&lt;M: <b>copy</b>, drop, store&gt;(self: <a href="order_book_types.md#0x7_order_book_types_Order">order_book_types::Order</a>&lt;M&gt;): (<a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>, <a href="order_book_types.md#0x7_order_book_types_UniqueIdxType">order_book_types::UniqueIdxType</a>, u64, u64, u64, bool, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="order_book_types.md#0x7_order_book_types_TriggerCondition">order_book_types::TriggerCondition</a>&gt;, M)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_destroy_order">destroy_order</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: <a href="order_book_types.md#0x7_order_book_types_Order">Order</a>&lt;M&gt;
): (<a href="order_book_types.md#0x7_order_book_types_OrderIdType">OrderIdType</a>, <a href="order_book_types.md#0x7_order_book_types_UniqueIdxType">UniqueIdxType</a>, u64, u64, u64, bool, Option&lt;<a href="order_book_types.md#0x7_order_book_types_TriggerCondition">TriggerCondition</a>&gt;, M) {
    (
        self.order_id,
        self.unique_priority_idx,
        self.price,
        self.orig_size,
        self.remaining_size,
        self.is_buy,
        self.trigger_condition,
        self.metadata
    )
}
</code></pre>



</details>

<a id="0x7_order_book_types_destroy_single_order_match"></a>

## Function `destroy_single_order_match`



<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_destroy_single_order_match">destroy_single_order_match</a>&lt;M: <b>copy</b>, drop, store&gt;(self: <a href="order_book_types.md#0x7_order_book_types_SingleOrderMatch">order_book_types::SingleOrderMatch</a>&lt;M&gt;): (<a href="order_book_types.md#0x7_order_book_types_Order">order_book_types::Order</a>&lt;M&gt;, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_destroy_single_order_match">destroy_single_order_match</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: <a href="order_book_types.md#0x7_order_book_types_SingleOrderMatch">SingleOrderMatch</a>&lt;M&gt;
): (<a href="order_book_types.md#0x7_order_book_types_Order">Order</a>&lt;M&gt;, u64) {
    (self.order, self.matched_size)
}
</code></pre>



</details>

<a id="0x7_order_book_types_destroy_order_id_type"></a>

## Function `destroy_order_id_type`



<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_destroy_order_id_type">destroy_order_id_type</a>(self: <a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>): (<b>address</b>, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_destroy_order_id_type">destroy_order_id_type</a>(self: <a href="order_book_types.md#0x7_order_book_types_OrderIdType">OrderIdType</a>): (<b>address</b>, u64) {
    (self.<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>, self.account_order_id)
}
</code></pre>



</details>

<a id="0x7_order_book_types_is_active_order"></a>

## Function `is_active_order`



<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_is_active_order">is_active_order</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="order_book_types.md#0x7_order_book_types_OrderWithState">order_book_types::OrderWithState</a>&lt;M&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_is_active_order">is_active_order</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="order_book_types.md#0x7_order_book_types_OrderWithState">OrderWithState</a>&lt;M&gt;
): bool {
    self.is_active
}
</code></pre>



</details>

<a id="0x7_order_book_types_get_price"></a>

## Function `get_price`



<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_get_price">get_price</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="order_book_types.md#0x7_order_book_types_Order">order_book_types::Order</a>&lt;M&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_get_price">get_price</a>&lt;M: store + <b>copy</b> + drop&gt;(self: &<a href="order_book_types.md#0x7_order_book_types_Order">Order</a>&lt;M&gt;): u64 {
    self.price
}
</code></pre>



</details>

<a id="0x7_order_book_types_is_buy"></a>

## Function `is_buy`



<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_is_buy">is_buy</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="order_book_types.md#0x7_order_book_types_Order">order_book_types::Order</a>&lt;M&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_is_buy">is_buy</a>&lt;M: store + <b>copy</b> + drop&gt;(self: &<a href="order_book_types.md#0x7_order_book_types_Order">Order</a>&lt;M&gt;): bool {
    self.is_buy
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
