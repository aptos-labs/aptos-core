
<a id="0x5_order_book_types"></a>

# Module `0x5::order_book_types`

Order book type definitions


-  [Struct `OrderId`](#0x5_order_book_types_OrderId)
-  [Struct `AccountClientOrderId`](#0x5_order_book_types_AccountClientOrderId)
-  [Struct `IncreasingIdx`](#0x5_order_book_types_IncreasingIdx)
-  [Struct `DecreasingIdx`](#0x5_order_book_types_DecreasingIdx)
-  [Struct `OrderType`](#0x5_order_book_types_OrderType)
-  [Enum `TimeInForce`](#0x5_order_book_types_TimeInForce)
-  [Enum `TriggerCondition`](#0x5_order_book_types_TriggerCondition)
-  [Constants](#@Constants_0)
-  [Function `single_order_type`](#0x5_order_book_types_single_order_type)
-  [Function `bulk_order_type`](#0x5_order_book_types_bulk_order_type)
-  [Function `is_bulk_order_type`](#0x5_order_book_types_is_bulk_order_type)
-  [Function `is_single_order_type`](#0x5_order_book_types_is_single_order_type)
-  [Function `next_order_id`](#0x5_order_book_types_next_order_id)
-  [Function `new_order_id_type`](#0x5_order_book_types_new_order_id_type)
-  [Function `new_account_client_order_id`](#0x5_order_book_types_new_account_client_order_id)
-  [Function `next_increasing_idx_type`](#0x5_order_book_types_next_increasing_idx_type)
-  [Function `into_decreasing_idx_type`](#0x5_order_book_types_into_decreasing_idx_type)
-  [Function `get_order_id_value`](#0x5_order_book_types_get_order_id_value)
-  [Function `time_in_force_from_index`](#0x5_order_book_types_time_in_force_from_index)
-  [Function `good_till_cancelled`](#0x5_order_book_types_good_till_cancelled)
-  [Function `post_only`](#0x5_order_book_types_post_only)
-  [Function `immediate_or_cancel`](#0x5_order_book_types_immediate_or_cancel)
-  [Function `new_time_based_trigger_condition`](#0x5_order_book_types_new_time_based_trigger_condition)
-  [Function `price_move_up_condition`](#0x5_order_book_types_price_move_up_condition)
-  [Function `price_move_down_condition`](#0x5_order_book_types_price_move_down_condition)
-  [Function `get_trigger_condition_indices`](#0x5_order_book_types_get_trigger_condition_indices)
-  [Function `reverse_bits`](#0x5_order_book_types_reverse_bits)


<pre><code><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;
<b>use</b> <a href="../../aptos-framework/doc/transaction_context.md#0x1_transaction_context">0x1::transaction_context</a>;
</code></pre>



<a id="0x5_order_book_types_OrderId"></a>

## Struct `OrderId`



<pre><code><b>struct</b> <a href="order_book_types.md#0x5_order_book_types_OrderId">OrderId</a> <b>has</b> <b>copy</b>, drop, store
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

<a id="0x5_order_book_types_AccountClientOrderId"></a>

## Struct `AccountClientOrderId`



<pre><code><b>struct</b> <a href="order_book_types.md#0x5_order_book_types_AccountClientOrderId">AccountClientOrderId</a> <b>has</b> <b>copy</b>, drop, store
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

<a id="0x5_order_book_types_IncreasingIdx"></a>

## Struct `IncreasingIdx`



<pre><code><b>struct</b> <a href="order_book_types.md#0x5_order_book_types_IncreasingIdx">IncreasingIdx</a> <b>has</b> <b>copy</b>, drop, store
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

<a id="0x5_order_book_types_DecreasingIdx"></a>

## Struct `DecreasingIdx`



<pre><code><b>struct</b> <a href="order_book_types.md#0x5_order_book_types_DecreasingIdx">DecreasingIdx</a> <b>has</b> <b>copy</b>, drop, store
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

<a id="0x5_order_book_types_OrderType"></a>

## Struct `OrderType`



<pre><code><b>struct</b> <a href="order_book_types.md#0x5_order_book_types_OrderType">OrderType</a> <b>has</b> <b>copy</b>, drop, store
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

<a id="0x5_order_book_types_TimeInForce"></a>

## Enum `TimeInForce`

Order time in force


<pre><code>enum <a href="order_book_types.md#0x5_order_book_types_TimeInForce">TimeInForce</a> <b>has</b> <b>copy</b>, drop, store
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

<a id="0x5_order_book_types_TriggerCondition"></a>

## Enum `TriggerCondition`



<pre><code>enum <a href="order_book_types.md#0x5_order_book_types_TriggerCondition">TriggerCondition</a> <b>has</b> <b>copy</b>, drop, store
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

<a id="@Constants_0"></a>

## Constants


<a id="0x5_order_book_types_BULK_ORDER_TYPE"></a>



<pre><code><b>const</b> <a href="order_book_types.md#0x5_order_book_types_BULK_ORDER_TYPE">BULK_ORDER_TYPE</a>: u16 = 1;
</code></pre>



<a id="0x5_order_book_types_EINVALID_TIME_IN_FORCE"></a>



<pre><code><b>const</b> <a href="order_book_types.md#0x5_order_book_types_EINVALID_TIME_IN_FORCE">EINVALID_TIME_IN_FORCE</a>: u64 = 5;
</code></pre>



<a id="0x5_order_book_types_SINGLE_ORDER_TYPE"></a>



<pre><code><b>const</b> <a href="order_book_types.md#0x5_order_book_types_SINGLE_ORDER_TYPE">SINGLE_ORDER_TYPE</a>: u16 = 0;
</code></pre>



<a id="0x5_order_book_types_U128_MAX"></a>



<pre><code><b>const</b> <a href="order_book_types.md#0x5_order_book_types_U128_MAX">U128_MAX</a>: u128 = 340282366920938463463374607431768211455;
</code></pre>



<a id="0x5_order_book_types_single_order_type"></a>

## Function `single_order_type`



<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x5_order_book_types_single_order_type">single_order_type</a>(): <a href="order_book_types.md#0x5_order_book_types_OrderType">order_book_types::OrderType</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x5_order_book_types_single_order_type">single_order_type</a>(): <a href="order_book_types.md#0x5_order_book_types_OrderType">OrderType</a> {
    <a href="order_book_types.md#0x5_order_book_types_OrderType">OrderType</a> { type: <a href="order_book_types.md#0x5_order_book_types_SINGLE_ORDER_TYPE">SINGLE_ORDER_TYPE</a> }
}
</code></pre>



</details>

<a id="0x5_order_book_types_bulk_order_type"></a>

## Function `bulk_order_type`



<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x5_order_book_types_bulk_order_type">bulk_order_type</a>(): <a href="order_book_types.md#0x5_order_book_types_OrderType">order_book_types::OrderType</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x5_order_book_types_bulk_order_type">bulk_order_type</a>(): <a href="order_book_types.md#0x5_order_book_types_OrderType">OrderType</a> {
    <a href="order_book_types.md#0x5_order_book_types_OrderType">OrderType</a> { type: <a href="order_book_types.md#0x5_order_book_types_BULK_ORDER_TYPE">BULK_ORDER_TYPE</a> }
}
</code></pre>



</details>

<a id="0x5_order_book_types_is_bulk_order_type"></a>

## Function `is_bulk_order_type`



<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x5_order_book_types_is_bulk_order_type">is_bulk_order_type</a>(order_type: &<a href="order_book_types.md#0x5_order_book_types_OrderType">order_book_types::OrderType</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x5_order_book_types_is_bulk_order_type">is_bulk_order_type</a>(order_type: &<a href="order_book_types.md#0x5_order_book_types_OrderType">OrderType</a>): bool {
    order_type.type == <a href="order_book_types.md#0x5_order_book_types_BULK_ORDER_TYPE">BULK_ORDER_TYPE</a>
}
</code></pre>



</details>

<a id="0x5_order_book_types_is_single_order_type"></a>

## Function `is_single_order_type`



<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x5_order_book_types_is_single_order_type">is_single_order_type</a>(order_type: &<a href="order_book_types.md#0x5_order_book_types_OrderType">order_book_types::OrderType</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x5_order_book_types_is_single_order_type">is_single_order_type</a>(order_type: &<a href="order_book_types.md#0x5_order_book_types_OrderType">OrderType</a>): bool {
    order_type.type == <a href="order_book_types.md#0x5_order_book_types_SINGLE_ORDER_TYPE">SINGLE_ORDER_TYPE</a>
}
</code></pre>



</details>

<a id="0x5_order_book_types_next_order_id"></a>

## Function `next_order_id`



<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x5_order_book_types_next_order_id">next_order_id</a>(): <a href="order_book_types.md#0x5_order_book_types_OrderId">order_book_types::OrderId</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x5_order_book_types_next_order_id">next_order_id</a>(): <a href="order_book_types.md#0x5_order_book_types_OrderId">OrderId</a> {
    // reverse bits <b>to</b> make order ids random, so indices on top of them are shuffled.
    <a href="order_book_types.md#0x5_order_book_types_OrderId">OrderId</a> {
        order_id: <a href="order_book_types.md#0x5_order_book_types_reverse_bits">reverse_bits</a>(
            <a href="../../aptos-framework/doc/transaction_context.md#0x1_transaction_context_monotonically_increasing_counter">transaction_context::monotonically_increasing_counter</a>()
        )
    }
}
</code></pre>



</details>

<a id="0x5_order_book_types_new_order_id_type"></a>

## Function `new_order_id_type`



<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x5_order_book_types_new_order_id_type">new_order_id_type</a>(order_id: u128): <a href="order_book_types.md#0x5_order_book_types_OrderId">order_book_types::OrderId</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x5_order_book_types_new_order_id_type">new_order_id_type</a>(order_id: u128): <a href="order_book_types.md#0x5_order_book_types_OrderId">OrderId</a> {
    <a href="order_book_types.md#0x5_order_book_types_OrderId">OrderId</a> { order_id }
}
</code></pre>



</details>

<a id="0x5_order_book_types_new_account_client_order_id"></a>

## Function `new_account_client_order_id`



<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x5_order_book_types_new_account_client_order_id">new_account_client_order_id</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>, client_order_id: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <a href="order_book_types.md#0x5_order_book_types_AccountClientOrderId">order_book_types::AccountClientOrderId</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x5_order_book_types_new_account_client_order_id">new_account_client_order_id</a>(
    <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>, client_order_id: String
): <a href="order_book_types.md#0x5_order_book_types_AccountClientOrderId">AccountClientOrderId</a> {
    <a href="order_book_types.md#0x5_order_book_types_AccountClientOrderId">AccountClientOrderId</a> { <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>, client_order_id }
}
</code></pre>



</details>

<a id="0x5_order_book_types_next_increasing_idx_type"></a>

## Function `next_increasing_idx_type`



<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x5_order_book_types_next_increasing_idx_type">next_increasing_idx_type</a>(): <a href="order_book_types.md#0x5_order_book_types_IncreasingIdx">order_book_types::IncreasingIdx</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x5_order_book_types_next_increasing_idx_type">next_increasing_idx_type</a>(): <a href="order_book_types.md#0x5_order_book_types_IncreasingIdx">IncreasingIdx</a> {
    <a href="order_book_types.md#0x5_order_book_types_IncreasingIdx">IncreasingIdx</a> { idx: <a href="../../aptos-framework/doc/transaction_context.md#0x1_transaction_context_monotonically_increasing_counter">transaction_context::monotonically_increasing_counter</a>() }
}
</code></pre>



</details>

<a id="0x5_order_book_types_into_decreasing_idx_type"></a>

## Function `into_decreasing_idx_type`



<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x5_order_book_types_into_decreasing_idx_type">into_decreasing_idx_type</a>(self: &<a href="order_book_types.md#0x5_order_book_types_IncreasingIdx">order_book_types::IncreasingIdx</a>): <a href="order_book_types.md#0x5_order_book_types_DecreasingIdx">order_book_types::DecreasingIdx</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x5_order_book_types_into_decreasing_idx_type">into_decreasing_idx_type</a>(self: &<a href="order_book_types.md#0x5_order_book_types_IncreasingIdx">IncreasingIdx</a>): <a href="order_book_types.md#0x5_order_book_types_DecreasingIdx">DecreasingIdx</a> {
    <a href="order_book_types.md#0x5_order_book_types_DecreasingIdx">DecreasingIdx</a> { idx: <a href="order_book_types.md#0x5_order_book_types_U128_MAX">U128_MAX</a> - self.idx }
}
</code></pre>



</details>

<a id="0x5_order_book_types_get_order_id_value"></a>

## Function `get_order_id_value`



<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x5_order_book_types_get_order_id_value">get_order_id_value</a>(self: &<a href="order_book_types.md#0x5_order_book_types_OrderId">order_book_types::OrderId</a>): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x5_order_book_types_get_order_id_value">get_order_id_value</a>(self: &<a href="order_book_types.md#0x5_order_book_types_OrderId">OrderId</a>): u128 {
    self.order_id
}
</code></pre>



</details>

<a id="0x5_order_book_types_time_in_force_from_index"></a>

## Function `time_in_force_from_index`



<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x5_order_book_types_time_in_force_from_index">time_in_force_from_index</a>(index: u8): <a href="order_book_types.md#0x5_order_book_types_TimeInForce">order_book_types::TimeInForce</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x5_order_book_types_time_in_force_from_index">time_in_force_from_index</a>(index: u8): <a href="order_book_types.md#0x5_order_book_types_TimeInForce">TimeInForce</a> {
    <b>if</b> (index == 0) {
        TimeInForce::GTC
    } <b>else</b> <b>if</b> (index == 1) {
        TimeInForce::POST_ONLY
    } <b>else</b> <b>if</b> (index == 2) {
        TimeInForce::IOC
    } <b>else</b> {
        <b>abort</b> <a href="order_book_types.md#0x5_order_book_types_EINVALID_TIME_IN_FORCE">EINVALID_TIME_IN_FORCE</a>
    }
}
</code></pre>



</details>

<a id="0x5_order_book_types_good_till_cancelled"></a>

## Function `good_till_cancelled`



<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x5_order_book_types_good_till_cancelled">good_till_cancelled</a>(): <a href="order_book_types.md#0x5_order_book_types_TimeInForce">order_book_types::TimeInForce</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x5_order_book_types_good_till_cancelled">good_till_cancelled</a>(): <a href="order_book_types.md#0x5_order_book_types_TimeInForce">TimeInForce</a> {
    TimeInForce::GTC
}
</code></pre>



</details>

<a id="0x5_order_book_types_post_only"></a>

## Function `post_only`



<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x5_order_book_types_post_only">post_only</a>(): <a href="order_book_types.md#0x5_order_book_types_TimeInForce">order_book_types::TimeInForce</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x5_order_book_types_post_only">post_only</a>(): <a href="order_book_types.md#0x5_order_book_types_TimeInForce">TimeInForce</a> {
    TimeInForce::POST_ONLY
}
</code></pre>



</details>

<a id="0x5_order_book_types_immediate_or_cancel"></a>

## Function `immediate_or_cancel`



<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x5_order_book_types_immediate_or_cancel">immediate_or_cancel</a>(): <a href="order_book_types.md#0x5_order_book_types_TimeInForce">order_book_types::TimeInForce</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x5_order_book_types_immediate_or_cancel">immediate_or_cancel</a>(): <a href="order_book_types.md#0x5_order_book_types_TimeInForce">TimeInForce</a> {
    TimeInForce::IOC
}
</code></pre>



</details>

<a id="0x5_order_book_types_new_time_based_trigger_condition"></a>

## Function `new_time_based_trigger_condition`



<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x5_order_book_types_new_time_based_trigger_condition">new_time_based_trigger_condition</a>(time_secs: u64): <a href="order_book_types.md#0x5_order_book_types_TriggerCondition">order_book_types::TriggerCondition</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x5_order_book_types_new_time_based_trigger_condition">new_time_based_trigger_condition</a>(time_secs: u64): <a href="order_book_types.md#0x5_order_book_types_TriggerCondition">TriggerCondition</a> {
    TriggerCondition::TimeBased(time_secs)
}
</code></pre>



</details>

<a id="0x5_order_book_types_price_move_up_condition"></a>

## Function `price_move_up_condition`



<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x5_order_book_types_price_move_up_condition">price_move_up_condition</a>(price: u64): <a href="order_book_types.md#0x5_order_book_types_TriggerCondition">order_book_types::TriggerCondition</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x5_order_book_types_price_move_up_condition">price_move_up_condition</a>(price: u64): <a href="order_book_types.md#0x5_order_book_types_TriggerCondition">TriggerCondition</a> {
    TriggerCondition::PriceMoveAbove(price)
}
</code></pre>



</details>

<a id="0x5_order_book_types_price_move_down_condition"></a>

## Function `price_move_down_condition`



<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x5_order_book_types_price_move_down_condition">price_move_down_condition</a>(price: u64): <a href="order_book_types.md#0x5_order_book_types_TriggerCondition">order_book_types::TriggerCondition</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x5_order_book_types_price_move_down_condition">price_move_down_condition</a>(price: u64): <a href="order_book_types.md#0x5_order_book_types_TriggerCondition">TriggerCondition</a> {
    TriggerCondition::PriceMoveBelow(price)
}
</code></pre>



</details>

<a id="0x5_order_book_types_get_trigger_condition_indices"></a>

## Function `get_trigger_condition_indices`



<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x5_order_book_types_get_trigger_condition_indices">get_trigger_condition_indices</a>(self: &<a href="order_book_types.md#0x5_order_book_types_TriggerCondition">order_book_types::TriggerCondition</a>): (<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x5_order_book_types_get_trigger_condition_indices">get_trigger_condition_indices</a>(
    self: &<a href="order_book_types.md#0x5_order_book_types_TriggerCondition">TriggerCondition</a>
): (<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;) {
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

<a id="0x5_order_book_types_reverse_bits"></a>

## Function `reverse_bits`

Reverse the bits in a u128 value using divide and conquer approach
This is more efficient than the bit-by-bit approach, reducing from O(n) to O(log n)


<pre><code><b>fun</b> <a href="order_book_types.md#0x5_order_book_types_reverse_bits">reverse_bits</a>(value: u128): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="order_book_types.md#0x5_order_book_types_reverse_bits">reverse_bits</a>(value: u128): u128 {
    <b>let</b> v = value;

    // Swap odd and even bits
    v =
        ((v & 0x55555555555555555555555555555555) &lt;&lt; 1)
            | ((v &gt;&gt; 1) & 0x55555555555555555555555555555555);

    // Swap consecutive pairs
    v =
        ((v & 0x33333333333333333333333333333333) &lt;&lt; 2)
            | ((v &gt;&gt; 2) & 0x33333333333333333333333333333333);

    // Swap nibbles
    v =
        ((v & 0x0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f) &lt;&lt; 4)
            | ((v &gt;&gt; 4) & 0x0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f);

    // Swap bytes
    v =
        ((v & 0x00ff00ff00ff00ff00ff00ff00ff00ff) &lt;&lt; 8)
            | ((v &gt;&gt; 8) & 0x00ff00ff00ff00ff00ff00ff00ff00ff);

    // Swap 2-byte chunks
    v =
        ((v & 0x0000ffff0000ffff0000ffff0000ffff) &lt;&lt; 16)
            | ((v &gt;&gt; 16) & 0x0000ffff0000ffff0000ffff0000ffff);

    // Swap 4-byte chunks
    v =
        ((v & 0x00000000ffffffff00000000ffffffff) &lt;&lt; 32)
            | ((v &gt;&gt; 32) & 0x00000000ffffffff00000000ffffffff);

    // Swap 8-byte chunks
    v = (v &lt;&lt; 64) | (v &gt;&gt; 64);

    v
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
