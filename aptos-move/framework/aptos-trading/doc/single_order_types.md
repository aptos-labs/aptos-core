
<a id="0x5_single_order_types"></a>

# Module `0x5::single_order_types`

Single Order Types Module


-  [Enum `SingleOrderRequest`](#0x5_single_order_types_SingleOrderRequest)
-  [Enum `SingleOrder`](#0x5_single_order_types_SingleOrder)
-  [Enum `OrderWithState`](#0x5_single_order_types_OrderWithState)
-  [Constants](#@Constants_0)
-  [Function `new_order_request_from_match_details`](#0x5_single_order_types_new_order_request_from_match_details)
-  [Function `new_single_order`](#0x5_single_order_types_new_single_order)
-  [Function `new_order_with_state`](#0x5_single_order_types_new_order_with_state)
-  [Function `get_order_id`](#0x5_single_order_types_get_order_id)
-  [Function `get_account`](#0x5_single_order_types_get_account)
-  [Function `get_trigger_condition`](#0x5_single_order_types_get_trigger_condition)
-  [Function `get_remaining_size`](#0x5_single_order_types_get_remaining_size)
-  [Function `get_client_order_id`](#0x5_single_order_types_get_client_order_id)
-  [Function `get_price`](#0x5_single_order_types_get_price)
-  [Function `is_bid`](#0x5_single_order_types_is_bid)
-  [Function `get_creation_time_micros`](#0x5_single_order_types_get_creation_time_micros)
-  [Function `get_unique_priority_idx`](#0x5_single_order_types_get_unique_priority_idx)
-  [Function `get_order_request`](#0x5_single_order_types_get_order_request)
-  [Function `get_order_request_mut`](#0x5_single_order_types_get_order_request_mut)
-  [Function `get_order_from_state`](#0x5_single_order_types_get_order_from_state)
-  [Function `get_order_from_state_mut`](#0x5_single_order_types_get_order_from_state_mut)
-  [Function `get_metadata_from_state`](#0x5_single_order_types_get_metadata_from_state)
-  [Function `set_metadata_in_state`](#0x5_single_order_types_set_metadata_in_state)
-  [Function `increase_remaining_size_from_state`](#0x5_single_order_types_increase_remaining_size_from_state)
-  [Function `decrease_remaining_size_from_state`](#0x5_single_order_types_decrease_remaining_size_from_state)
-  [Function `set_remaining_size_from_state`](#0x5_single_order_types_set_remaining_size_from_state)
-  [Function `get_remaining_size_from_state`](#0x5_single_order_types_get_remaining_size_from_state)
-  [Function `get_unique_priority_idx_from_state`](#0x5_single_order_types_get_unique_priority_idx_from_state)
-  [Function `is_active_order`](#0x5_single_order_types_is_active_order)
-  [Function `destroy_order_from_state`](#0x5_single_order_types_destroy_order_from_state)
-  [Function `destroy_single_order`](#0x5_single_order_types_destroy_single_order)
-  [Function `destroy_single_order_request`](#0x5_single_order_types_destroy_single_order_request)
-  [Function `new_single_order_request`](#0x5_single_order_types_new_single_order_request)


<pre><code><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;
<b>use</b> <a href="../../aptos-framework/doc/timestamp.md#0x1_timestamp">0x1::timestamp</a>;
<b>use</b> <a href="order_book_types.md#0x5_order_book_types">0x5::order_book_types</a>;
<b>use</b> <a href="order_match_types.md#0x5_order_match_types">0x5::order_match_types</a>;
</code></pre>



<a id="0x5_single_order_types_SingleOrderRequest"></a>

## Enum `SingleOrderRequest`



<pre><code>enum <a href="single_order_types.md#0x5_single_order_types_SingleOrderRequest">SingleOrderRequest</a>&lt;M: <b>copy</b>, drop, store&gt; <b>has</b> <b>copy</b>, drop, store
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
<code>order_id: <a href="order_book_types.md#0x5_order_book_types_OrderId">order_book_types::OrderId</a></code>
</dt>
<dd>

</dd>
<dt>
<code>client_order_id: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;</code>
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
<code>trigger_condition: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="order_book_types.md#0x5_order_book_types_TriggerCondition">order_book_types::TriggerCondition</a>&gt;</code>
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

</details>

<a id="0x5_single_order_types_SingleOrder"></a>

## Enum `SingleOrder`



<pre><code>enum <a href="single_order_types.md#0x5_single_order_types_SingleOrder">SingleOrder</a>&lt;M: <b>copy</b>, drop, store&gt; <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>V1</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>order_request: <a href="single_order_types.md#0x5_single_order_types_SingleOrderRequest">single_order_types::SingleOrderRequest</a>&lt;M&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>unique_priority_idx: <a href="order_book_types.md#0x5_order_book_types_IncreasingIdx">order_book_types::IncreasingIdx</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

</details>

<a id="0x5_single_order_types_OrderWithState"></a>

## Enum `OrderWithState`



<pre><code>enum <a href="single_order_types.md#0x5_single_order_types_OrderWithState">OrderWithState</a>&lt;M: <b>copy</b>, drop, store&gt; <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>V1</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>order: <a href="single_order_types.md#0x5_single_order_types_SingleOrder">single_order_types::SingleOrder</a>&lt;M&gt;</code>
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


<a id="0x5_single_order_types_EINVALID_ORDER_SIZE_DECREASE"></a>



<pre><code><b>const</b> <a href="single_order_types.md#0x5_single_order_types_EINVALID_ORDER_SIZE_DECREASE">EINVALID_ORDER_SIZE_DECREASE</a>: u64 = 4;
</code></pre>



<a id="0x5_single_order_types_EINVALID_TRIGGER_CONDITION"></a>



<pre><code><b>const</b> <a href="single_order_types.md#0x5_single_order_types_EINVALID_TRIGGER_CONDITION">EINVALID_TRIGGER_CONDITION</a>: u64 = 2;
</code></pre>



<a id="0x5_single_order_types_EORDER_ALREADY_EXISTS"></a>



<pre><code><b>const</b> <a href="single_order_types.md#0x5_single_order_types_EORDER_ALREADY_EXISTS">EORDER_ALREADY_EXISTS</a>: u64 = 1;
</code></pre>



<a id="0x5_single_order_types_INVALID_MATCH_RESULT"></a>



<pre><code><b>const</b> <a href="single_order_types.md#0x5_single_order_types_INVALID_MATCH_RESULT">INVALID_MATCH_RESULT</a>: u64 = 3;
</code></pre>



<a id="0x5_single_order_types_new_order_request_from_match_details"></a>

## Function `new_order_request_from_match_details`



<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x5_single_order_types_new_order_request_from_match_details">new_order_request_from_match_details</a>&lt;M: <b>copy</b>, drop, store&gt;(order_match_details: <a href="order_match_types.md#0x5_order_match_types_OrderMatchDetails">order_match_types::OrderMatchDetails</a>&lt;M&gt;): <a href="single_order_types.md#0x5_single_order_types_SingleOrderRequest">single_order_types::SingleOrderRequest</a>&lt;M&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x5_single_order_types_new_order_request_from_match_details">new_order_request_from_match_details</a>&lt;M: store + <b>copy</b> + drop&gt;(
    order_match_details: OrderMatchDetails&lt;M&gt;
): <a href="single_order_types.md#0x5_single_order_types_SingleOrderRequest">SingleOrderRequest</a>&lt;M&gt; {
    <b>let</b> (
        order_id,
        <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>,
        client_order_id,
        _unique_priority_idx,
        price,
        orig_size,
        remaining_size,
        is_bid,
        time_in_force,
        creation_time_micros,
        metadata
    ) = order_match_details.destroy_single_order_match_details();
    SingleOrderRequest::V1 {
        <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>,
        order_id,
        client_order_id,
        price,
        orig_size,
        remaining_size,
        is_bid,
        trigger_condition: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(),
        time_in_force,
        creation_time_micros,
        metadata
    }
}
</code></pre>



</details>

<a id="0x5_single_order_types_new_single_order"></a>

## Function `new_single_order`



<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x5_single_order_types_new_single_order">new_single_order</a>&lt;M: <b>copy</b>, drop, store&gt;(order_request: <a href="single_order_types.md#0x5_single_order_types_SingleOrderRequest">single_order_types::SingleOrderRequest</a>&lt;M&gt;, unique_priority_idx: <a href="order_book_types.md#0x5_order_book_types_IncreasingIdx">order_book_types::IncreasingIdx</a>): <a href="single_order_types.md#0x5_single_order_types_SingleOrder">single_order_types::SingleOrder</a>&lt;M&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x5_single_order_types_new_single_order">new_single_order</a>&lt;M: store + <b>copy</b> + drop&gt;(
    order_request: <a href="single_order_types.md#0x5_single_order_types_SingleOrderRequest">SingleOrderRequest</a>&lt;M&gt;, unique_priority_idx: IncreasingIdx
): <a href="single_order_types.md#0x5_single_order_types_SingleOrder">SingleOrder</a>&lt;M&gt; {
    SingleOrder::V1 { order_request, unique_priority_idx }
}
</code></pre>



</details>

<a id="0x5_single_order_types_new_order_with_state"></a>

## Function `new_order_with_state`



<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x5_single_order_types_new_order_with_state">new_order_with_state</a>&lt;M: <b>copy</b>, drop, store&gt;(order: <a href="single_order_types.md#0x5_single_order_types_SingleOrder">single_order_types::SingleOrder</a>&lt;M&gt;, is_active: bool): <a href="single_order_types.md#0x5_single_order_types_OrderWithState">single_order_types::OrderWithState</a>&lt;M&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x5_single_order_types_new_order_with_state">new_order_with_state</a>&lt;M: store + <b>copy</b> + drop&gt;(
    order: <a href="single_order_types.md#0x5_single_order_types_SingleOrder">SingleOrder</a>&lt;M&gt;, is_active: bool
): <a href="single_order_types.md#0x5_single_order_types_OrderWithState">OrderWithState</a>&lt;M&gt; {
    OrderWithState::V1 { order, is_active }
}
</code></pre>



</details>

<a id="0x5_single_order_types_get_order_id"></a>

## Function `get_order_id`



<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x5_single_order_types_get_order_id">get_order_id</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="single_order_types.md#0x5_single_order_types_SingleOrderRequest">single_order_types::SingleOrderRequest</a>&lt;M&gt;): <a href="order_book_types.md#0x5_order_book_types_OrderId">order_book_types::OrderId</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x5_single_order_types_get_order_id">get_order_id</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="single_order_types.md#0x5_single_order_types_SingleOrderRequest">SingleOrderRequest</a>&lt;M&gt;
): OrderId {
    self.order_id
}
</code></pre>



</details>

<a id="0x5_single_order_types_get_account"></a>

## Function `get_account`



<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x5_single_order_types_get_account">get_account</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="single_order_types.md#0x5_single_order_types_SingleOrderRequest">single_order_types::SingleOrderRequest</a>&lt;M&gt;): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x5_single_order_types_get_account">get_account</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="single_order_types.md#0x5_single_order_types_SingleOrderRequest">SingleOrderRequest</a>&lt;M&gt;
): <b>address</b> {
    self.<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>
}
</code></pre>



</details>

<a id="0x5_single_order_types_get_trigger_condition"></a>

## Function `get_trigger_condition`



<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x5_single_order_types_get_trigger_condition">get_trigger_condition</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="single_order_types.md#0x5_single_order_types_SingleOrderRequest">single_order_types::SingleOrderRequest</a>&lt;M&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="order_book_types.md#0x5_order_book_types_TriggerCondition">order_book_types::TriggerCondition</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x5_single_order_types_get_trigger_condition">get_trigger_condition</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="single_order_types.md#0x5_single_order_types_SingleOrderRequest">SingleOrderRequest</a>&lt;M&gt;
): Option&lt;TriggerCondition&gt; {
    self.trigger_condition
}
</code></pre>



</details>

<a id="0x5_single_order_types_get_remaining_size"></a>

## Function `get_remaining_size`



<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x5_single_order_types_get_remaining_size">get_remaining_size</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="single_order_types.md#0x5_single_order_types_SingleOrderRequest">single_order_types::SingleOrderRequest</a>&lt;M&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x5_single_order_types_get_remaining_size">get_remaining_size</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="single_order_types.md#0x5_single_order_types_SingleOrderRequest">SingleOrderRequest</a>&lt;M&gt;
): u64 {
    self.remaining_size
}
</code></pre>



</details>

<a id="0x5_single_order_types_get_client_order_id"></a>

## Function `get_client_order_id`



<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x5_single_order_types_get_client_order_id">get_client_order_id</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="single_order_types.md#0x5_single_order_types_SingleOrderRequest">single_order_types::SingleOrderRequest</a>&lt;M&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x5_single_order_types_get_client_order_id">get_client_order_id</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="single_order_types.md#0x5_single_order_types_SingleOrderRequest">SingleOrderRequest</a>&lt;M&gt;
): Option&lt;String&gt; {
    self.client_order_id
}
</code></pre>



</details>

<a id="0x5_single_order_types_get_price"></a>

## Function `get_price`



<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x5_single_order_types_get_price">get_price</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="single_order_types.md#0x5_single_order_types_SingleOrderRequest">single_order_types::SingleOrderRequest</a>&lt;M&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x5_single_order_types_get_price">get_price</a>&lt;M: store + <b>copy</b> + drop&gt;(self: &<a href="single_order_types.md#0x5_single_order_types_SingleOrderRequest">SingleOrderRequest</a>&lt;M&gt;): u64 {
    self.price
}
</code></pre>



</details>

<a id="0x5_single_order_types_is_bid"></a>

## Function `is_bid`



<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x5_single_order_types_is_bid">is_bid</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="single_order_types.md#0x5_single_order_types_SingleOrderRequest">single_order_types::SingleOrderRequest</a>&lt;M&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x5_single_order_types_is_bid">is_bid</a>&lt;M: store + <b>copy</b> + drop&gt;(self: &<a href="single_order_types.md#0x5_single_order_types_SingleOrderRequest">SingleOrderRequest</a>&lt;M&gt;): bool {
    self.is_bid
}
</code></pre>



</details>

<a id="0x5_single_order_types_get_creation_time_micros"></a>

## Function `get_creation_time_micros`



<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x5_single_order_types_get_creation_time_micros">get_creation_time_micros</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="single_order_types.md#0x5_single_order_types_SingleOrderRequest">single_order_types::SingleOrderRequest</a>&lt;M&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x5_single_order_types_get_creation_time_micros">get_creation_time_micros</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="single_order_types.md#0x5_single_order_types_SingleOrderRequest">SingleOrderRequest</a>&lt;M&gt;
): u64 {
    self.creation_time_micros
}
</code></pre>



</details>

<a id="0x5_single_order_types_get_unique_priority_idx"></a>

## Function `get_unique_priority_idx`



<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x5_single_order_types_get_unique_priority_idx">get_unique_priority_idx</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="single_order_types.md#0x5_single_order_types_SingleOrder">single_order_types::SingleOrder</a>&lt;M&gt;): <a href="order_book_types.md#0x5_order_book_types_IncreasingIdx">order_book_types::IncreasingIdx</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x5_single_order_types_get_unique_priority_idx">get_unique_priority_idx</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="single_order_types.md#0x5_single_order_types_SingleOrder">SingleOrder</a>&lt;M&gt;
): IncreasingIdx {
    self.unique_priority_idx
}
</code></pre>



</details>

<a id="0x5_single_order_types_get_order_request"></a>

## Function `get_order_request`



<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x5_single_order_types_get_order_request">get_order_request</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="single_order_types.md#0x5_single_order_types_SingleOrder">single_order_types::SingleOrder</a>&lt;M&gt;): &<a href="single_order_types.md#0x5_single_order_types_SingleOrderRequest">single_order_types::SingleOrderRequest</a>&lt;M&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x5_single_order_types_get_order_request">get_order_request</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="single_order_types.md#0x5_single_order_types_SingleOrder">SingleOrder</a>&lt;M&gt;
): &<a href="single_order_types.md#0x5_single_order_types_SingleOrderRequest">SingleOrderRequest</a>&lt;M&gt; {
    &self.order_request
}
</code></pre>



</details>

<a id="0x5_single_order_types_get_order_request_mut"></a>

## Function `get_order_request_mut`



<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x5_single_order_types_get_order_request_mut">get_order_request_mut</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="single_order_types.md#0x5_single_order_types_SingleOrder">single_order_types::SingleOrder</a>&lt;M&gt;): &<b>mut</b> <a href="single_order_types.md#0x5_single_order_types_SingleOrderRequest">single_order_types::SingleOrderRequest</a>&lt;M&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x5_single_order_types_get_order_request_mut">get_order_request_mut</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="single_order_types.md#0x5_single_order_types_SingleOrder">SingleOrder</a>&lt;M&gt;
): &<b>mut</b> <a href="single_order_types.md#0x5_single_order_types_SingleOrderRequest">SingleOrderRequest</a>&lt;M&gt; {
    &<b>mut</b> self.order_request
}
</code></pre>



</details>

<a id="0x5_single_order_types_get_order_from_state"></a>

## Function `get_order_from_state`



<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x5_single_order_types_get_order_from_state">get_order_from_state</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="single_order_types.md#0x5_single_order_types_OrderWithState">single_order_types::OrderWithState</a>&lt;M&gt;): &<a href="single_order_types.md#0x5_single_order_types_SingleOrder">single_order_types::SingleOrder</a>&lt;M&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x5_single_order_types_get_order_from_state">get_order_from_state</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="single_order_types.md#0x5_single_order_types_OrderWithState">OrderWithState</a>&lt;M&gt;
): &<a href="single_order_types.md#0x5_single_order_types_SingleOrder">SingleOrder</a>&lt;M&gt; {
    &self.order
}
</code></pre>



</details>

<a id="0x5_single_order_types_get_order_from_state_mut"></a>

## Function `get_order_from_state_mut`



<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x5_single_order_types_get_order_from_state_mut">get_order_from_state_mut</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="single_order_types.md#0x5_single_order_types_OrderWithState">single_order_types::OrderWithState</a>&lt;M&gt;): &<b>mut</b> <a href="single_order_types.md#0x5_single_order_types_SingleOrder">single_order_types::SingleOrder</a>&lt;M&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x5_single_order_types_get_order_from_state_mut">get_order_from_state_mut</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="single_order_types.md#0x5_single_order_types_OrderWithState">OrderWithState</a>&lt;M&gt;
): &<b>mut</b> <a href="single_order_types.md#0x5_single_order_types_SingleOrder">SingleOrder</a>&lt;M&gt; {
    &<b>mut</b> self.order
}
</code></pre>



</details>

<a id="0x5_single_order_types_get_metadata_from_state"></a>

## Function `get_metadata_from_state`



<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x5_single_order_types_get_metadata_from_state">get_metadata_from_state</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="single_order_types.md#0x5_single_order_types_OrderWithState">single_order_types::OrderWithState</a>&lt;M&gt;): M
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x5_single_order_types_get_metadata_from_state">get_metadata_from_state</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="single_order_types.md#0x5_single_order_types_OrderWithState">OrderWithState</a>&lt;M&gt;
): M {
    self.order.order_request.metadata
}
</code></pre>



</details>

<a id="0x5_single_order_types_set_metadata_in_state"></a>

## Function `set_metadata_in_state`



<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x5_single_order_types_set_metadata_in_state">set_metadata_in_state</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="single_order_types.md#0x5_single_order_types_OrderWithState">single_order_types::OrderWithState</a>&lt;M&gt;, metadata: M)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x5_single_order_types_set_metadata_in_state">set_metadata_in_state</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="single_order_types.md#0x5_single_order_types_OrderWithState">OrderWithState</a>&lt;M&gt;, metadata: M
) {
    self.order.order_request.metadata = metadata;
}
</code></pre>



</details>

<a id="0x5_single_order_types_increase_remaining_size_from_state"></a>

## Function `increase_remaining_size_from_state`



<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x5_single_order_types_increase_remaining_size_from_state">increase_remaining_size_from_state</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="single_order_types.md#0x5_single_order_types_OrderWithState">single_order_types::OrderWithState</a>&lt;M&gt;, size: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x5_single_order_types_increase_remaining_size_from_state">increase_remaining_size_from_state</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="single_order_types.md#0x5_single_order_types_OrderWithState">OrderWithState</a>&lt;M&gt;, size: u64
) {
    self.order.order_request.remaining_size += size;
}
</code></pre>



</details>

<a id="0x5_single_order_types_decrease_remaining_size_from_state"></a>

## Function `decrease_remaining_size_from_state`



<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x5_single_order_types_decrease_remaining_size_from_state">decrease_remaining_size_from_state</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="single_order_types.md#0x5_single_order_types_OrderWithState">single_order_types::OrderWithState</a>&lt;M&gt;, size: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x5_single_order_types_decrease_remaining_size_from_state">decrease_remaining_size_from_state</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="single_order_types.md#0x5_single_order_types_OrderWithState">OrderWithState</a>&lt;M&gt;, size: u64
) {
    <b>assert</b>!(
        self.order.order_request.remaining_size &gt; size,
        <a href="single_order_types.md#0x5_single_order_types_EINVALID_ORDER_SIZE_DECREASE">EINVALID_ORDER_SIZE_DECREASE</a>
    );
    self.order.order_request.remaining_size -= size;
}
</code></pre>



</details>

<a id="0x5_single_order_types_set_remaining_size_from_state"></a>

## Function `set_remaining_size_from_state`



<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x5_single_order_types_set_remaining_size_from_state">set_remaining_size_from_state</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="single_order_types.md#0x5_single_order_types_OrderWithState">single_order_types::OrderWithState</a>&lt;M&gt;, remaining_size: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x5_single_order_types_set_remaining_size_from_state">set_remaining_size_from_state</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="single_order_types.md#0x5_single_order_types_OrderWithState">OrderWithState</a>&lt;M&gt;, remaining_size: u64
) {
    self.order.order_request.remaining_size = remaining_size;
}
</code></pre>



</details>

<a id="0x5_single_order_types_get_remaining_size_from_state"></a>

## Function `get_remaining_size_from_state`



<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x5_single_order_types_get_remaining_size_from_state">get_remaining_size_from_state</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="single_order_types.md#0x5_single_order_types_OrderWithState">single_order_types::OrderWithState</a>&lt;M&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x5_single_order_types_get_remaining_size_from_state">get_remaining_size_from_state</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="single_order_types.md#0x5_single_order_types_OrderWithState">OrderWithState</a>&lt;M&gt;
): u64 {
    self.order.order_request.remaining_size
}
</code></pre>



</details>

<a id="0x5_single_order_types_get_unique_priority_idx_from_state"></a>

## Function `get_unique_priority_idx_from_state`



<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x5_single_order_types_get_unique_priority_idx_from_state">get_unique_priority_idx_from_state</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="single_order_types.md#0x5_single_order_types_OrderWithState">single_order_types::OrderWithState</a>&lt;M&gt;): <a href="order_book_types.md#0x5_order_book_types_IncreasingIdx">order_book_types::IncreasingIdx</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x5_single_order_types_get_unique_priority_idx_from_state">get_unique_priority_idx_from_state</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="single_order_types.md#0x5_single_order_types_OrderWithState">OrderWithState</a>&lt;M&gt;
): IncreasingIdx {
    self.order.unique_priority_idx
}
</code></pre>



</details>

<a id="0x5_single_order_types_is_active_order"></a>

## Function `is_active_order`



<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x5_single_order_types_is_active_order">is_active_order</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="single_order_types.md#0x5_single_order_types_OrderWithState">single_order_types::OrderWithState</a>&lt;M&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x5_single_order_types_is_active_order">is_active_order</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="single_order_types.md#0x5_single_order_types_OrderWithState">OrderWithState</a>&lt;M&gt;
): bool {
    self.is_active
}
</code></pre>



</details>

<a id="0x5_single_order_types_destroy_order_from_state"></a>

## Function `destroy_order_from_state`



<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x5_single_order_types_destroy_order_from_state">destroy_order_from_state</a>&lt;M: <b>copy</b>, drop, store&gt;(self: <a href="single_order_types.md#0x5_single_order_types_OrderWithState">single_order_types::OrderWithState</a>&lt;M&gt;): (<a href="single_order_types.md#0x5_single_order_types_SingleOrder">single_order_types::SingleOrder</a>&lt;M&gt;, bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x5_single_order_types_destroy_order_from_state">destroy_order_from_state</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: <a href="single_order_types.md#0x5_single_order_types_OrderWithState">OrderWithState</a>&lt;M&gt;
): (<a href="single_order_types.md#0x5_single_order_types_SingleOrder">SingleOrder</a>&lt;M&gt;, bool) {
    <b>let</b> OrderWithState::V1 { order, is_active } = self;
    (order, is_active)
}
</code></pre>



</details>

<a id="0x5_single_order_types_destroy_single_order"></a>

## Function `destroy_single_order`



<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x5_single_order_types_destroy_single_order">destroy_single_order</a>&lt;M: <b>copy</b>, drop, store&gt;(self: <a href="single_order_types.md#0x5_single_order_types_SingleOrder">single_order_types::SingleOrder</a>&lt;M&gt;): (<a href="single_order_types.md#0x5_single_order_types_SingleOrderRequest">single_order_types::SingleOrderRequest</a>&lt;M&gt;, <a href="order_book_types.md#0x5_order_book_types_IncreasingIdx">order_book_types::IncreasingIdx</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x5_single_order_types_destroy_single_order">destroy_single_order</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: <a href="single_order_types.md#0x5_single_order_types_SingleOrder">SingleOrder</a>&lt;M&gt;
): (<a href="single_order_types.md#0x5_single_order_types_SingleOrderRequest">SingleOrderRequest</a>&lt;M&gt;, IncreasingIdx) {
    <b>let</b> SingleOrder::V1 { order_request, unique_priority_idx } = self;
    (order_request, unique_priority_idx)
}
</code></pre>



</details>

<a id="0x5_single_order_types_destroy_single_order_request"></a>

## Function `destroy_single_order_request`



<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x5_single_order_types_destroy_single_order_request">destroy_single_order_request</a>&lt;M: <b>copy</b>, drop, store&gt;(self: <a href="single_order_types.md#0x5_single_order_types_SingleOrderRequest">single_order_types::SingleOrderRequest</a>&lt;M&gt;): (<b>address</b>, <a href="order_book_types.md#0x5_order_book_types_OrderId">order_book_types::OrderId</a>, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, u64, u64, u64, bool, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="order_book_types.md#0x5_order_book_types_TriggerCondition">order_book_types::TriggerCondition</a>&gt;, <a href="order_book_types.md#0x5_order_book_types_TimeInForce">order_book_types::TimeInForce</a>, u64, M)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x5_single_order_types_destroy_single_order_request">destroy_single_order_request</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: <a href="single_order_types.md#0x5_single_order_types_SingleOrderRequest">SingleOrderRequest</a>&lt;M&gt;
): (
    <b>address</b>,
    OrderId,
    Option&lt;String&gt;,
    u64,
    u64,
    u64,
    bool,
    Option&lt;TriggerCondition&gt;,
    TimeInForce,
    u64,
    M
) {
    <b>let</b> SingleOrderRequest::V1 {
        <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>,
        order_id,
        client_order_id,
        price,
        orig_size,
        remaining_size,
        is_bid,
        trigger_condition,
        time_in_force,
        metadata,
        creation_time_micros
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
        creation_time_micros,
        metadata
    )
}
</code></pre>



</details>

<a id="0x5_single_order_types_new_single_order_request"></a>

## Function `new_single_order_request`



<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x5_single_order_types_new_single_order_request">new_single_order_request</a>&lt;M: <b>copy</b>, drop, store&gt;(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>, order_id: <a href="order_book_types.md#0x5_order_book_types_OrderId">order_book_types::OrderId</a>, client_order_id: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, price: u64, orig_size: u64, remaining_size: u64, is_bid: bool, trigger_condition: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="order_book_types.md#0x5_order_book_types_TriggerCondition">order_book_types::TriggerCondition</a>&gt;, time_in_force: <a href="order_book_types.md#0x5_order_book_types_TimeInForce">order_book_types::TimeInForce</a>, metadata: M): <a href="single_order_types.md#0x5_single_order_types_SingleOrderRequest">single_order_types::SingleOrderRequest</a>&lt;M&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="single_order_types.md#0x5_single_order_types_new_single_order_request">new_single_order_request</a>&lt;M: store + <b>copy</b> + drop&gt;(
    <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>,
    order_id: OrderId,
    client_order_id: Option&lt;String&gt;,
    price: u64,
    orig_size: u64,
    remaining_size: u64,
    is_bid: bool,
    trigger_condition: Option&lt;TriggerCondition&gt;,
    time_in_force: TimeInForce,
    metadata: M
): <a href="single_order_types.md#0x5_single_order_types_SingleOrderRequest">SingleOrderRequest</a>&lt;M&gt; {
    SingleOrderRequest::V1 {
        <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>,
        order_id,
        client_order_id,
        price,
        orig_size,
        remaining_size,
        is_bid,
        trigger_condition,
        time_in_force,
        creation_time_micros: <a href="../../aptos-framework/doc/timestamp.md#0x1_timestamp_now_microseconds">timestamp::now_microseconds</a>(),
        metadata
    }
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
