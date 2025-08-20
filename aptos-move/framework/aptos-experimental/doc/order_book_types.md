
<a id="0x7_order_book_types"></a>

# Module `0x7::order_book_types`

Order book type definitions


-  [Struct `OrderIdType`](#0x7_order_book_types_OrderIdType)
-  [Struct `AccountClientOrderId`](#0x7_order_book_types_AccountClientOrderId)
-  [Struct `UniqueIdxType`](#0x7_order_book_types_UniqueIdxType)
-  [Enum `AscendingIdGenerator`](#0x7_order_book_types_AscendingIdGenerator)
-  [Enum `TimeInForce`](#0x7_order_book_types_TimeInForce)
-  [Enum `TriggerCondition`](#0x7_order_book_types_TriggerCondition)
-  [Constants](#@Constants_0)
-  [Function `new_default_big_ordered_map`](#0x7_order_book_types_new_default_big_ordered_map)
-  [Function `new_order_id_type`](#0x7_order_book_types_new_order_id_type)
-  [Function `new_account_client_order_id`](#0x7_order_book_types_new_account_client_order_id)
-  [Function `new_ascending_id_generator`](#0x7_order_book_types_new_ascending_id_generator)
-  [Function `next_ascending_id`](#0x7_order_book_types_next_ascending_id)
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


<pre><code><b>use</b> <a href="../../aptos-framework/doc/big_ordered_map.md#0x1_big_ordered_map">0x1::big_ordered_map</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
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
<code>client_order_id: u64</code>
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

<a id="0x7_order_book_types_AscendingIdGenerator"></a>

## Enum `AscendingIdGenerator`



<pre><code>enum <a href="order_book_types.md#0x7_order_book_types_AscendingIdGenerator">AscendingIdGenerator</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>FromCounter</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>value: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

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

<a id="@Constants_0"></a>

## Constants


<a id="0x7_order_book_types_BIG_MAP_INNER_DEGREE"></a>



<pre><code><b>const</b> <a href="order_book_types.md#0x7_order_book_types_BIG_MAP_INNER_DEGREE">BIG_MAP_INNER_DEGREE</a>: u16 = 64;
</code></pre>



<a id="0x7_order_book_types_BIG_MAP_LEAF_DEGREE"></a>



<pre><code><b>const</b> <a href="order_book_types.md#0x7_order_book_types_BIG_MAP_LEAF_DEGREE">BIG_MAP_LEAF_DEGREE</a>: u16 = 32;
</code></pre>



<a id="0x7_order_book_types_EINVALID_TIME_IN_FORCE"></a>



<pre><code><b>const</b> <a href="order_book_types.md#0x7_order_book_types_EINVALID_TIME_IN_FORCE">EINVALID_TIME_IN_FORCE</a>: u64 = 5;
</code></pre>



<a id="0x7_order_book_types_U128_MAX"></a>



<pre><code><b>const</b> <a href="order_book_types.md#0x7_order_book_types_U128_MAX">U128_MAX</a>: u128 = 340282366920938463463374607431768211455;
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



<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_new_account_client_order_id">new_account_client_order_id</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>, client_order_id: u64): <a href="order_book_types.md#0x7_order_book_types_AccountClientOrderId">order_book_types::AccountClientOrderId</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_new_account_client_order_id">new_account_client_order_id</a>(
    <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>, client_order_id: u64
): <a href="order_book_types.md#0x7_order_book_types_AccountClientOrderId">AccountClientOrderId</a> {
    <a href="order_book_types.md#0x7_order_book_types_AccountClientOrderId">AccountClientOrderId</a> { <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>, client_order_id }
}
</code></pre>



</details>

<a id="0x7_order_book_types_new_ascending_id_generator"></a>

## Function `new_ascending_id_generator`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_new_ascending_id_generator">new_ascending_id_generator</a>(): <a href="order_book_types.md#0x7_order_book_types_AscendingIdGenerator">order_book_types::AscendingIdGenerator</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_new_ascending_id_generator">new_ascending_id_generator</a>(): <a href="order_book_types.md#0x7_order_book_types_AscendingIdGenerator">AscendingIdGenerator</a> {
    AscendingIdGenerator::FromCounter { value: 0 }
}
</code></pre>



</details>

<a id="0x7_order_book_types_next_ascending_id"></a>

## Function `next_ascending_id`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_next_ascending_id">next_ascending_id</a>(self: &<b>mut</b> <a href="order_book_types.md#0x7_order_book_types_AscendingIdGenerator">order_book_types::AscendingIdGenerator</a>): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_next_ascending_id">next_ascending_id</a>(self: &<b>mut</b> <a href="order_book_types.md#0x7_order_book_types_AscendingIdGenerator">AscendingIdGenerator</a>): u128 {
    self.value += 1;
    self.value <b>as</b> u128
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



<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_new_time_based_trigger_condition">new_time_based_trigger_condition</a>(time: u64): <a href="order_book_types.md#0x7_order_book_types_TriggerCondition">order_book_types::TriggerCondition</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_new_time_based_trigger_condition">new_time_based_trigger_condition</a>(time: u64): <a href="order_book_types.md#0x7_order_book_types_TriggerCondition">TriggerCondition</a> {
    TriggerCondition::TimeBased(time)
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



<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_index">index</a>(self: &<a href="order_book_types.md#0x7_order_book_types_TriggerCondition">order_book_types::TriggerCondition</a>): (<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book_types.md#0x7_order_book_types_index">index</a>(self: &<a href="order_book_types.md#0x7_order_book_types_TriggerCondition">TriggerCondition</a>):
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


[move-book]: https://aptos.dev/move/book/SUMMARY
