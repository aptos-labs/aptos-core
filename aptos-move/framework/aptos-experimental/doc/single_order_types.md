
<a id="0x7_single_order_types"></a>

# Module `0x7::single_order_types`

(work in progress)


-  [Struct `ActiveMatchedOrder`](#0x7_single_order_types_ActiveMatchedOrder)
-  [Enum `SingleOrderMatch`](#0x7_single_order_types_SingleOrderMatch)
-  [Enum `Order`](#0x7_single_order_types_Order)
-  [Enum `OrderWithState`](#0x7_single_order_types_OrderWithState)
-  [Constants](#@Constants_0)
-  [Function `get_slippage_pct_precision`](#0x7_single_order_types_get_slippage_pct_precision)
-  [Function `new_active_matched_order`](#0x7_single_order_types_new_active_matched_order)
-  [Function `destroy_active_matched_order`](#0x7_single_order_types_destroy_active_matched_order)
-  [Function `new_order`](#0x7_single_order_types_new_order)
-  [Function `new_single_order_match`](#0x7_single_order_types_new_single_order_match)
-  [Function `get_active_matched_size`](#0x7_single_order_types_get_active_matched_size)
-  [Function `get_matched_size`](#0x7_single_order_types_get_matched_size)
-  [Function `new_order_with_state`](#0x7_single_order_types_new_order_with_state)
-  [Function `get_order_from_state`](#0x7_single_order_types_get_order_from_state)
-  [Function `get_metadata_from_state`](#0x7_single_order_types_get_metadata_from_state)
-  [Function `set_metadata_in_state`](#0x7_single_order_types_set_metadata_in_state)
-  [Function `get_order_id`](#0x7_single_order_types_get_order_id)
-  [Function `get_account`](#0x7_single_order_types_get_account)
-  [Function `get_unique_priority_idx`](#0x7_single_order_types_get_unique_priority_idx)
-  [Function `get_metadata_from_order`](#0x7_single_order_types_get_metadata_from_order)
-  [Function `get_time_in_force`](#0x7_single_order_types_get_time_in_force)
-  [Function `get_trigger_condition_from_order`](#0x7_single_order_types_get_trigger_condition_from_order)
-  [Function `increase_remaining_size`](#0x7_single_order_types_increase_remaining_size)
-  [Function `decrease_remaining_size`](#0x7_single_order_types_decrease_remaining_size)
-  [Function `set_remaining_size`](#0x7_single_order_types_set_remaining_size)
-  [Function `get_remaining_size_from_state`](#0x7_single_order_types_get_remaining_size_from_state)
-  [Function `get_unique_priority_idx_from_state`](#0x7_single_order_types_get_unique_priority_idx_from_state)
-  [Function `get_remaining_size`](#0x7_single_order_types_get_remaining_size)
-  [Function `get_orig_size`](#0x7_single_order_types_get_orig_size)
-  [Function `get_client_order_id`](#0x7_single_order_types_get_client_order_id)
-  [Function `destroy_order_from_state`](#0x7_single_order_types_destroy_order_from_state)
-  [Function `destroy_active_match_order`](#0x7_single_order_types_destroy_active_match_order)
-  [Function `destroy_order`](#0x7_single_order_types_destroy_order)
-  [Function `destroy_single_order_match`](#0x7_single_order_types_destroy_single_order_match)
-  [Function `is_active_order`](#0x7_single_order_types_is_active_order)
-  [Function `get_price`](#0x7_single_order_types_get_price)
-  [Function `is_bid`](#0x7_single_order_types_is_bid)


<pre><code><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="order_book_types.md#0x7_order_book_types">0x7::order_book_types</a>;
</code></pre>



<a id="0x7_single_order_types_ActiveMatchedOrder"></a>

## Struct `ActiveMatchedOrder`



<pre><code><b>struct</b> <a href="single_order_types.md#0x7_single_order_types_ActiveMatchedOrder">ActiveMatchedOrder</a> <b>has</b> <b>copy</b>, drop
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

<a id="0x7_single_order_types_SingleOrderMatch"></a>

## Enum `SingleOrderMatch`



<pre><code>enum <a href="single_order_types.md#0x7_single_order_types_SingleOrderMatch">SingleOrderMatch</a>&lt;M: <b>copy</b>, drop, store&gt; <b>has</b> <b>copy</b>, drop
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>V1</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>order: <a href="single_order_types.md#0x7_single_order_types_Order">single_order_types::Order</a>&lt;M&gt;</code>
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

<a id="0x7_single_order_types_Order"></a>

## Enum `Order`



<pre><code>enum <a href="single_order_types.md#0x7_single_order_types_Order">Order</a>&lt;M: <b>copy</b>, drop, store&gt; <b>has</b> <b>copy</b>, drop, store
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
<code>client_order_id: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;</code>
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
<code>trigger_condition: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="order_book_types.md#0x7_order_book_types_TriggerCondition">order_book_types::TriggerCondition</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>time_in_force: <a href="order_book_types.md#0x7_order_book_types_TimeInForce">order_book_types::TimeInForce</a></code>
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

<a id="0x7_single_order_types_OrderWithState"></a>

## Enum `OrderWithState`



<pre><code>enum <a href="single_order_types.md#0x7_single_order_types_OrderWithState">OrderWithState</a>&lt;M: <b>copy</b>, drop, store&gt; <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>V1</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>order: <a href="single_order_types.md#0x7_single_order_types_Order">single_order_types::Order</a>&lt;M&gt;</code>
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

</details>

</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x7_single_order_types_EINVALID_ORDER_SIZE_DECREASE"></a>



<pre><code><b>const</b> <a href="single_order_types.md#0x7_single_order_types_EINVALID_ORDER_SIZE_DECREASE">EINVALID_ORDER_SIZE_DECREASE</a>: u64 = 4;
</code></pre>



<a id="0x7_single_order_types_EINVALID_TRIGGER_CONDITION"></a>



<pre><code><b>const</b> <a href="single_order_types.md#0x7_single_order_types_EINVALID_TRIGGER_CONDITION">EINVALID_TRIGGER_CONDITION</a>: u64 = 2;
</code></pre>



<a id="0x7_single_order_types_EORDER_ALREADY_EXISTS"></a>



<pre><code><b>const</b> <a href="single_order_types.md#0x7_single_order_types_EORDER_ALREADY_EXISTS">EORDER_ALREADY_EXISTS</a>: u64 = 1;
</code></pre>



<a id="0x7_single_order_types_INVALID_MATCH_RESULT"></a>



<pre><code><b>const</b> <a href="single_order_types.md#0x7_single_order_types_INVALID_MATCH_RESULT">INVALID_MATCH_RESULT</a>: u64 = 3;
</code></pre>



<a id="0x7_single_order_types_SLIPPAGE_PCT_PRECISION"></a>



<pre><code><b>const</b> <a href="single_order_types.md#0x7_single_order_types_SLIPPAGE_PCT_PRECISION">SLIPPAGE_PCT_PRECISION</a>: u64 = 100;
</code></pre>



<a id="0x7_single_order_types_get_slippage_pct_precision"></a>

## Function `get_slippage_pct_precision`



<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x7_single_order_types_get_slippage_pct_precision">get_slippage_pct_precision</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x7_single_order_types_get_slippage_pct_precision">get_slippage_pct_precision</a>(): u64 {
    <a href="single_order_types.md#0x7_single_order_types_SLIPPAGE_PCT_PRECISION">SLIPPAGE_PCT_PRECISION</a>
}
</code></pre>



</details>

<a id="0x7_single_order_types_new_active_matched_order"></a>

## Function `new_active_matched_order`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="single_order_types.md#0x7_single_order_types_new_active_matched_order">new_active_matched_order</a>(order_id: <a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>, matched_size: u64, remaining_size: u64): <a href="single_order_types.md#0x7_single_order_types_ActiveMatchedOrder">single_order_types::ActiveMatchedOrder</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="single_order_types.md#0x7_single_order_types_new_active_matched_order">new_active_matched_order</a>(
    order_id: OrderIdType, matched_size: u64, remaining_size: u64
): <a href="single_order_types.md#0x7_single_order_types_ActiveMatchedOrder">ActiveMatchedOrder</a> {
    <a href="single_order_types.md#0x7_single_order_types_ActiveMatchedOrder">ActiveMatchedOrder</a> { order_id, matched_size, remaining_size }
}
</code></pre>



</details>

<a id="0x7_single_order_types_destroy_active_matched_order"></a>

## Function `destroy_active_matched_order`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="single_order_types.md#0x7_single_order_types_destroy_active_matched_order">destroy_active_matched_order</a>(self: <a href="single_order_types.md#0x7_single_order_types_ActiveMatchedOrder">single_order_types::ActiveMatchedOrder</a>): (<a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>, u64, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="single_order_types.md#0x7_single_order_types_destroy_active_matched_order">destroy_active_matched_order</a>(
    self: <a href="single_order_types.md#0x7_single_order_types_ActiveMatchedOrder">ActiveMatchedOrder</a>
): (OrderIdType, u64, u64) {
    (self.order_id, self.matched_size, self.remaining_size)
}
</code></pre>



</details>

<a id="0x7_single_order_types_new_order"></a>

## Function `new_order`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="single_order_types.md#0x7_single_order_types_new_order">new_order</a>&lt;M: <b>copy</b>, drop, store&gt;(order_id: <a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>, <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>, unique_priority_idx: <a href="order_book_types.md#0x7_order_book_types_UniqueIdxType">order_book_types::UniqueIdxType</a>, client_order_id: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;, price: u64, orig_size: u64, size: u64, is_bid: bool, trigger_condition: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="order_book_types.md#0x7_order_book_types_TriggerCondition">order_book_types::TriggerCondition</a>&gt;, time_in_force: <a href="order_book_types.md#0x7_order_book_types_TimeInForce">order_book_types::TimeInForce</a>, metadata: M): <a href="single_order_types.md#0x7_single_order_types_Order">single_order_types::Order</a>&lt;M&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="single_order_types.md#0x7_single_order_types_new_order">new_order</a>&lt;M: store + <b>copy</b> + drop&gt;(
    order_id: OrderIdType,
    <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>,
    unique_priority_idx: UniqueIdxType,
    client_order_id: Option&lt;u64&gt;,
    price: u64,
    orig_size: u64,
    size: u64,
    is_bid: bool,
    trigger_condition: Option&lt;TriggerCondition&gt;,
    time_in_force: TimeInForce,
    metadata: M
): <a href="single_order_types.md#0x7_single_order_types_Order">Order</a>&lt;M&gt; {
    Order::V1 {
        order_id,
        <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>,
        unique_priority_idx,
        client_order_id,
        price,
        orig_size,
        remaining_size: size,
        is_bid,
        trigger_condition,
        time_in_force,
        metadata
    }
}
</code></pre>



</details>

<a id="0x7_single_order_types_new_single_order_match"></a>

## Function `new_single_order_match`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="single_order_types.md#0x7_single_order_types_new_single_order_match">new_single_order_match</a>&lt;M: <b>copy</b>, drop, store&gt;(order: <a href="single_order_types.md#0x7_single_order_types_Order">single_order_types::Order</a>&lt;M&gt;, matched_size: u64): <a href="single_order_types.md#0x7_single_order_types_SingleOrderMatch">single_order_types::SingleOrderMatch</a>&lt;M&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="single_order_types.md#0x7_single_order_types_new_single_order_match">new_single_order_match</a>&lt;M: store + <b>copy</b> + drop&gt;(
    order: <a href="single_order_types.md#0x7_single_order_types_Order">Order</a>&lt;M&gt;, matched_size: u64
): <a href="single_order_types.md#0x7_single_order_types_SingleOrderMatch">SingleOrderMatch</a>&lt;M&gt; {
    SingleOrderMatch::V1 { order, matched_size }
}
</code></pre>



</details>

<a id="0x7_single_order_types_get_active_matched_size"></a>

## Function `get_active_matched_size`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="single_order_types.md#0x7_single_order_types_get_active_matched_size">get_active_matched_size</a>(self: &<a href="single_order_types.md#0x7_single_order_types_ActiveMatchedOrder">single_order_types::ActiveMatchedOrder</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="single_order_types.md#0x7_single_order_types_get_active_matched_size">get_active_matched_size</a>(self: &<a href="single_order_types.md#0x7_single_order_types_ActiveMatchedOrder">ActiveMatchedOrder</a>): u64 {
    self.matched_size
}
</code></pre>



</details>

<a id="0x7_single_order_types_get_matched_size"></a>

## Function `get_matched_size`



<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x7_single_order_types_get_matched_size">get_matched_size</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="single_order_types.md#0x7_single_order_types_SingleOrderMatch">single_order_types::SingleOrderMatch</a>&lt;M&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x7_single_order_types_get_matched_size">get_matched_size</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="single_order_types.md#0x7_single_order_types_SingleOrderMatch">SingleOrderMatch</a>&lt;M&gt;
): u64 {
    self.matched_size
}
</code></pre>



</details>

<a id="0x7_single_order_types_new_order_with_state"></a>

## Function `new_order_with_state`



<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x7_single_order_types_new_order_with_state">new_order_with_state</a>&lt;M: <b>copy</b>, drop, store&gt;(order: <a href="single_order_types.md#0x7_single_order_types_Order">single_order_types::Order</a>&lt;M&gt;, is_active: bool): <a href="single_order_types.md#0x7_single_order_types_OrderWithState">single_order_types::OrderWithState</a>&lt;M&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x7_single_order_types_new_order_with_state">new_order_with_state</a>&lt;M: store + <b>copy</b> + drop&gt;(
    order: <a href="single_order_types.md#0x7_single_order_types_Order">Order</a>&lt;M&gt;, is_active: bool
): <a href="single_order_types.md#0x7_single_order_types_OrderWithState">OrderWithState</a>&lt;M&gt; {
    OrderWithState::V1 { order, is_active }
}
</code></pre>



</details>

<a id="0x7_single_order_types_get_order_from_state"></a>

## Function `get_order_from_state`



<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x7_single_order_types_get_order_from_state">get_order_from_state</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="single_order_types.md#0x7_single_order_types_OrderWithState">single_order_types::OrderWithState</a>&lt;M&gt;): &<a href="single_order_types.md#0x7_single_order_types_Order">single_order_types::Order</a>&lt;M&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x7_single_order_types_get_order_from_state">get_order_from_state</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="single_order_types.md#0x7_single_order_types_OrderWithState">OrderWithState</a>&lt;M&gt;
): &<a href="single_order_types.md#0x7_single_order_types_Order">Order</a>&lt;M&gt; {
    &self.order
}
</code></pre>



</details>

<a id="0x7_single_order_types_get_metadata_from_state"></a>

## Function `get_metadata_from_state`



<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x7_single_order_types_get_metadata_from_state">get_metadata_from_state</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="single_order_types.md#0x7_single_order_types_OrderWithState">single_order_types::OrderWithState</a>&lt;M&gt;): M
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x7_single_order_types_get_metadata_from_state">get_metadata_from_state</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="single_order_types.md#0x7_single_order_types_OrderWithState">OrderWithState</a>&lt;M&gt;
): M {
    self.order.metadata
}
</code></pre>



</details>

<a id="0x7_single_order_types_set_metadata_in_state"></a>

## Function `set_metadata_in_state`



<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x7_single_order_types_set_metadata_in_state">set_metadata_in_state</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="single_order_types.md#0x7_single_order_types_OrderWithState">single_order_types::OrderWithState</a>&lt;M&gt;, metadata: M)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x7_single_order_types_set_metadata_in_state">set_metadata_in_state</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="single_order_types.md#0x7_single_order_types_OrderWithState">OrderWithState</a>&lt;M&gt;, metadata: M
) {
    self.order.metadata = metadata;
}
</code></pre>



</details>

<a id="0x7_single_order_types_get_order_id"></a>

## Function `get_order_id`



<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x7_single_order_types_get_order_id">get_order_id</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="single_order_types.md#0x7_single_order_types_Order">single_order_types::Order</a>&lt;M&gt;): <a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x7_single_order_types_get_order_id">get_order_id</a>&lt;M: store + <b>copy</b> + drop&gt;(self: &<a href="single_order_types.md#0x7_single_order_types_Order">Order</a>&lt;M&gt;): OrderIdType {
    self.order_id
}
</code></pre>



</details>

<a id="0x7_single_order_types_get_account"></a>

## Function `get_account`



<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x7_single_order_types_get_account">get_account</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="single_order_types.md#0x7_single_order_types_Order">single_order_types::Order</a>&lt;M&gt;): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x7_single_order_types_get_account">get_account</a>&lt;M: store + <b>copy</b> + drop&gt;(self: &<a href="single_order_types.md#0x7_single_order_types_Order">Order</a>&lt;M&gt;): <b>address</b> {
    self.<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>
}
</code></pre>



</details>

<a id="0x7_single_order_types_get_unique_priority_idx"></a>

## Function `get_unique_priority_idx`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="single_order_types.md#0x7_single_order_types_get_unique_priority_idx">get_unique_priority_idx</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="single_order_types.md#0x7_single_order_types_Order">single_order_types::Order</a>&lt;M&gt;): <a href="order_book_types.md#0x7_order_book_types_UniqueIdxType">order_book_types::UniqueIdxType</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="single_order_types.md#0x7_single_order_types_get_unique_priority_idx">get_unique_priority_idx</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="single_order_types.md#0x7_single_order_types_Order">Order</a>&lt;M&gt;
): UniqueIdxType {
    self.unique_priority_idx
}
</code></pre>



</details>

<a id="0x7_single_order_types_get_metadata_from_order"></a>

## Function `get_metadata_from_order`



<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x7_single_order_types_get_metadata_from_order">get_metadata_from_order</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="single_order_types.md#0x7_single_order_types_Order">single_order_types::Order</a>&lt;M&gt;): M
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x7_single_order_types_get_metadata_from_order">get_metadata_from_order</a>&lt;M: store + <b>copy</b> + drop&gt;(self: &<a href="single_order_types.md#0x7_single_order_types_Order">Order</a>&lt;M&gt;): M {
    self.metadata
}
</code></pre>



</details>

<a id="0x7_single_order_types_get_time_in_force"></a>

## Function `get_time_in_force`



<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x7_single_order_types_get_time_in_force">get_time_in_force</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="single_order_types.md#0x7_single_order_types_Order">single_order_types::Order</a>&lt;M&gt;): <a href="order_book_types.md#0x7_order_book_types_TimeInForce">order_book_types::TimeInForce</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x7_single_order_types_get_time_in_force">get_time_in_force</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="single_order_types.md#0x7_single_order_types_Order">Order</a>&lt;M&gt;
): TimeInForce {
    self.time_in_force
}
</code></pre>



</details>

<a id="0x7_single_order_types_get_trigger_condition_from_order"></a>

## Function `get_trigger_condition_from_order`



<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x7_single_order_types_get_trigger_condition_from_order">get_trigger_condition_from_order</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="single_order_types.md#0x7_single_order_types_Order">single_order_types::Order</a>&lt;M&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="order_book_types.md#0x7_order_book_types_TriggerCondition">order_book_types::TriggerCondition</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x7_single_order_types_get_trigger_condition_from_order">get_trigger_condition_from_order</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="single_order_types.md#0x7_single_order_types_Order">Order</a>&lt;M&gt;
): Option&lt;TriggerCondition&gt; {
    self.trigger_condition
}
</code></pre>



</details>

<a id="0x7_single_order_types_increase_remaining_size"></a>

## Function `increase_remaining_size`



<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x7_single_order_types_increase_remaining_size">increase_remaining_size</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="single_order_types.md#0x7_single_order_types_OrderWithState">single_order_types::OrderWithState</a>&lt;M&gt;, size: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x7_single_order_types_increase_remaining_size">increase_remaining_size</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="single_order_types.md#0x7_single_order_types_OrderWithState">OrderWithState</a>&lt;M&gt;, size: u64
) {
    self.order.remaining_size += size;
}
</code></pre>



</details>

<a id="0x7_single_order_types_decrease_remaining_size"></a>

## Function `decrease_remaining_size`



<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x7_single_order_types_decrease_remaining_size">decrease_remaining_size</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="single_order_types.md#0x7_single_order_types_OrderWithState">single_order_types::OrderWithState</a>&lt;M&gt;, size: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x7_single_order_types_decrease_remaining_size">decrease_remaining_size</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="single_order_types.md#0x7_single_order_types_OrderWithState">OrderWithState</a>&lt;M&gt;, size: u64
) {
    <b>assert</b>!(self.order.remaining_size &gt; size, <a href="single_order_types.md#0x7_single_order_types_EINVALID_ORDER_SIZE_DECREASE">EINVALID_ORDER_SIZE_DECREASE</a>);
    self.order.remaining_size -= size;
}
</code></pre>



</details>

<a id="0x7_single_order_types_set_remaining_size"></a>

## Function `set_remaining_size`



<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x7_single_order_types_set_remaining_size">set_remaining_size</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="single_order_types.md#0x7_single_order_types_OrderWithState">single_order_types::OrderWithState</a>&lt;M&gt;, remaining_size: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x7_single_order_types_set_remaining_size">set_remaining_size</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="single_order_types.md#0x7_single_order_types_OrderWithState">OrderWithState</a>&lt;M&gt;, remaining_size: u64
) {
    self.order.remaining_size = remaining_size;
}
</code></pre>



</details>

<a id="0x7_single_order_types_get_remaining_size_from_state"></a>

## Function `get_remaining_size_from_state`



<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x7_single_order_types_get_remaining_size_from_state">get_remaining_size_from_state</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="single_order_types.md#0x7_single_order_types_OrderWithState">single_order_types::OrderWithState</a>&lt;M&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x7_single_order_types_get_remaining_size_from_state">get_remaining_size_from_state</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="single_order_types.md#0x7_single_order_types_OrderWithState">OrderWithState</a>&lt;M&gt;
): u64 {
    self.order.remaining_size
}
</code></pre>



</details>

<a id="0x7_single_order_types_get_unique_priority_idx_from_state"></a>

## Function `get_unique_priority_idx_from_state`



<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x7_single_order_types_get_unique_priority_idx_from_state">get_unique_priority_idx_from_state</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="single_order_types.md#0x7_single_order_types_OrderWithState">single_order_types::OrderWithState</a>&lt;M&gt;): <a href="order_book_types.md#0x7_order_book_types_UniqueIdxType">order_book_types::UniqueIdxType</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x7_single_order_types_get_unique_priority_idx_from_state">get_unique_priority_idx_from_state</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="single_order_types.md#0x7_single_order_types_OrderWithState">OrderWithState</a>&lt;M&gt;
): UniqueIdxType {
    self.order.unique_priority_idx
}
</code></pre>



</details>

<a id="0x7_single_order_types_get_remaining_size"></a>

## Function `get_remaining_size`



<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x7_single_order_types_get_remaining_size">get_remaining_size</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="single_order_types.md#0x7_single_order_types_Order">single_order_types::Order</a>&lt;M&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x7_single_order_types_get_remaining_size">get_remaining_size</a>&lt;M: store + <b>copy</b> + drop&gt;(self: &<a href="single_order_types.md#0x7_single_order_types_Order">Order</a>&lt;M&gt;): u64 {
    self.remaining_size
}
</code></pre>



</details>

<a id="0x7_single_order_types_get_orig_size"></a>

## Function `get_orig_size`



<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x7_single_order_types_get_orig_size">get_orig_size</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="single_order_types.md#0x7_single_order_types_Order">single_order_types::Order</a>&lt;M&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x7_single_order_types_get_orig_size">get_orig_size</a>&lt;M: store + <b>copy</b> + drop&gt;(self: &<a href="single_order_types.md#0x7_single_order_types_Order">Order</a>&lt;M&gt;): u64 {
    self.orig_size
}
</code></pre>



</details>

<a id="0x7_single_order_types_get_client_order_id"></a>

## Function `get_client_order_id`



<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x7_single_order_types_get_client_order_id">get_client_order_id</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="single_order_types.md#0x7_single_order_types_Order">single_order_types::Order</a>&lt;M&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x7_single_order_types_get_client_order_id">get_client_order_id</a>&lt;M: store + <b>copy</b> + drop&gt;(self: &<a href="single_order_types.md#0x7_single_order_types_Order">Order</a>&lt;M&gt;): Option&lt;u64&gt; {
    self.client_order_id
}
</code></pre>



</details>

<a id="0x7_single_order_types_destroy_order_from_state"></a>

## Function `destroy_order_from_state`



<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x7_single_order_types_destroy_order_from_state">destroy_order_from_state</a>&lt;M: <b>copy</b>, drop, store&gt;(self: <a href="single_order_types.md#0x7_single_order_types_OrderWithState">single_order_types::OrderWithState</a>&lt;M&gt;): (<a href="single_order_types.md#0x7_single_order_types_Order">single_order_types::Order</a>&lt;M&gt;, bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x7_single_order_types_destroy_order_from_state">destroy_order_from_state</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: <a href="single_order_types.md#0x7_single_order_types_OrderWithState">OrderWithState</a>&lt;M&gt;
): (<a href="single_order_types.md#0x7_single_order_types_Order">Order</a>&lt;M&gt;, bool) {
    (self.order, self.is_active)
}
</code></pre>



</details>

<a id="0x7_single_order_types_destroy_active_match_order"></a>

## Function `destroy_active_match_order`



<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x7_single_order_types_destroy_active_match_order">destroy_active_match_order</a>(self: <a href="single_order_types.md#0x7_single_order_types_ActiveMatchedOrder">single_order_types::ActiveMatchedOrder</a>): (<a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>, u64, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x7_single_order_types_destroy_active_match_order">destroy_active_match_order</a>(self: <a href="single_order_types.md#0x7_single_order_types_ActiveMatchedOrder">ActiveMatchedOrder</a>): (OrderIdType, u64, u64) {
    (self.order_id, self.matched_size, self.remaining_size)
}
</code></pre>



</details>

<a id="0x7_single_order_types_destroy_order"></a>

## Function `destroy_order`



<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x7_single_order_types_destroy_order">destroy_order</a>&lt;M: <b>copy</b>, drop, store&gt;(self: <a href="single_order_types.md#0x7_single_order_types_Order">single_order_types::Order</a>&lt;M&gt;): (<b>address</b>, <a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;, u64, u64, u64, bool, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="order_book_types.md#0x7_order_book_types_TriggerCondition">order_book_types::TriggerCondition</a>&gt;, <a href="order_book_types.md#0x7_order_book_types_TimeInForce">order_book_types::TimeInForce</a>, M)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x7_single_order_types_destroy_order">destroy_order</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: <a href="single_order_types.md#0x7_single_order_types_Order">Order</a>&lt;M&gt;
): (
    <b>address</b>, OrderIdType, Option&lt;u64&gt;, u64, u64, u64, bool, Option&lt;TriggerCondition&gt;, TimeInForce, M
) {
    <b>let</b> Order::V1 {
        order_id,
        <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>,
        client_order_id,
        unique_priority_idx: _,
        price,
        orig_size,
        remaining_size,
        is_bid,
        trigger_condition,
        time_in_force,
        metadata
    } = self;
    (
        <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>,
        order_id,
        client_order_id,
        price,
        orig_size,
        remaining_size,
        is_bid,
        trigger_condition,
        time_in_force,
        metadata
    )
}
</code></pre>



</details>

<a id="0x7_single_order_types_destroy_single_order_match"></a>

## Function `destroy_single_order_match`



<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x7_single_order_types_destroy_single_order_match">destroy_single_order_match</a>&lt;M: <b>copy</b>, drop, store&gt;(self: <a href="single_order_types.md#0x7_single_order_types_SingleOrderMatch">single_order_types::SingleOrderMatch</a>&lt;M&gt;): (<a href="single_order_types.md#0x7_single_order_types_Order">single_order_types::Order</a>&lt;M&gt;, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x7_single_order_types_destroy_single_order_match">destroy_single_order_match</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: <a href="single_order_types.md#0x7_single_order_types_SingleOrderMatch">SingleOrderMatch</a>&lt;M&gt;
): (<a href="single_order_types.md#0x7_single_order_types_Order">Order</a>&lt;M&gt;, u64) {
    (self.order, self.matched_size)
}
</code></pre>



</details>

<a id="0x7_single_order_types_is_active_order"></a>

## Function `is_active_order`



<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x7_single_order_types_is_active_order">is_active_order</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="single_order_types.md#0x7_single_order_types_OrderWithState">single_order_types::OrderWithState</a>&lt;M&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x7_single_order_types_is_active_order">is_active_order</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="single_order_types.md#0x7_single_order_types_OrderWithState">OrderWithState</a>&lt;M&gt;
): bool {
    self.is_active
}
</code></pre>



</details>

<a id="0x7_single_order_types_get_price"></a>

## Function `get_price`



<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x7_single_order_types_get_price">get_price</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="single_order_types.md#0x7_single_order_types_Order">single_order_types::Order</a>&lt;M&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x7_single_order_types_get_price">get_price</a>&lt;M: store + <b>copy</b> + drop&gt;(self: &<a href="single_order_types.md#0x7_single_order_types_Order">Order</a>&lt;M&gt;): u64 {
    self.price
}
</code></pre>



</details>

<a id="0x7_single_order_types_is_bid"></a>

## Function `is_bid`



<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x7_single_order_types_is_bid">is_bid</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="single_order_types.md#0x7_single_order_types_Order">single_order_types::Order</a>&lt;M&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x7_single_order_types_is_bid">is_bid</a>&lt;M: store + <b>copy</b> + drop&gt;(self: &<a href="single_order_types.md#0x7_single_order_types_Order">Order</a>&lt;M&gt;): bool {
    self.is_bid
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
