
<a id="0x7_market_clearinghouse_order_info"></a>

# Module `0x7::market_clearinghouse_order_info`



-  [Enum `MarketClearinghouseOrderInfo`](#0x7_market_clearinghouse_order_info_MarketClearinghouseOrderInfo)
-  [Function `new_clearinghouse_order_info`](#0x7_market_clearinghouse_order_info_new_clearinghouse_order_info)
-  [Function `get_account`](#0x7_market_clearinghouse_order_info_get_account)
-  [Function `get_order_id`](#0x7_market_clearinghouse_order_info_get_order_id)
-  [Function `is_bid`](#0x7_market_clearinghouse_order_info_is_bid)
-  [Function `get_client_order_id`](#0x7_market_clearinghouse_order_info_get_client_order_id)
-  [Function `get_metadata`](#0x7_market_clearinghouse_order_info_get_metadata)
-  [Function `into_inner`](#0x7_market_clearinghouse_order_info_into_inner)


<pre><code><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;
<b>use</b> <a href="">0x5::order_book_types</a>;
</code></pre>



<a id="0x7_market_clearinghouse_order_info_MarketClearinghouseOrderInfo"></a>

## Enum `MarketClearinghouseOrderInfo`



<pre><code>enum <a href="market_clearinghouse_order_info.md#0x7_market_clearinghouse_order_info_MarketClearinghouseOrderInfo">MarketClearinghouseOrderInfo</a>&lt;M: <b>copy</b>, drop&gt; <b>has</b> <b>copy</b>, drop
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
<code>order_id: <a href="_OrderId">order_book_types::OrderId</a></code>
</dt>
<dd>

</dd>
<dt>
<code>client_order_id: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>is_bid: bool</code>
</dt>
<dd>

</dd>
<dt>
<code>limit_price: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>time_in_force: <a href="_TimeInForce">order_book_types::TimeInForce</a></code>
</dt>
<dd>

</dd>
<dt>
<code>order_type: <a href="_OrderType">order_book_types::OrderType</a></code>
</dt>
<dd>

</dd>
<dt>
<code>trigger_condition: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="_TriggerCondition">order_book_types::TriggerCondition</a>&gt;</code>
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

<a id="0x7_market_clearinghouse_order_info_new_clearinghouse_order_info"></a>

## Function `new_clearinghouse_order_info`



<pre><code><b>public</b> <b>fun</b> <a href="market_clearinghouse_order_info.md#0x7_market_clearinghouse_order_info_new_clearinghouse_order_info">new_clearinghouse_order_info</a>&lt;M: <b>copy</b>, drop&gt;(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>, order_id: <a href="_OrderId">order_book_types::OrderId</a>, client_order_id: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, is_bid: bool, limit_price: u64, time_in_force: <a href="_TimeInForce">order_book_types::TimeInForce</a>, order_type: <a href="_OrderType">order_book_types::OrderType</a>, trigger_condition: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="_TriggerCondition">order_book_types::TriggerCondition</a>&gt;, metadata: M): <a href="market_clearinghouse_order_info.md#0x7_market_clearinghouse_order_info_MarketClearinghouseOrderInfo">market_clearinghouse_order_info::MarketClearinghouseOrderInfo</a>&lt;M&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_clearinghouse_order_info.md#0x7_market_clearinghouse_order_info_new_clearinghouse_order_info">new_clearinghouse_order_info</a>&lt;M: <b>copy</b> + drop&gt;(
    <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>,
    order_id: OrderId,
    client_order_id: Option&lt;String&gt;,
    is_bid: bool,
    limit_price: u64,
    time_in_force: TimeInForce,
    order_type: OrderType,
    trigger_condition: Option&lt;TriggerCondition&gt;,
    metadata: M
): <a href="market_clearinghouse_order_info.md#0x7_market_clearinghouse_order_info_MarketClearinghouseOrderInfo">MarketClearinghouseOrderInfo</a>&lt;M&gt; {
    MarketClearinghouseOrderInfo::V1 {
        <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>,
        order_id,
        client_order_id,
        is_bid,
        limit_price,
        time_in_force,
        order_type,
        trigger_condition,
        metadata
    }
}
</code></pre>



</details>

<a id="0x7_market_clearinghouse_order_info_get_account"></a>

## Function `get_account`



<pre><code><b>public</b> <b>fun</b> <a href="market_clearinghouse_order_info.md#0x7_market_clearinghouse_order_info_get_account">get_account</a>&lt;M: <b>copy</b>, drop&gt;(self: &<a href="market_clearinghouse_order_info.md#0x7_market_clearinghouse_order_info_MarketClearinghouseOrderInfo">market_clearinghouse_order_info::MarketClearinghouseOrderInfo</a>&lt;M&gt;): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_clearinghouse_order_info.md#0x7_market_clearinghouse_order_info_get_account">get_account</a>&lt;M: <b>copy</b> + drop&gt;(
    self: &<a href="market_clearinghouse_order_info.md#0x7_market_clearinghouse_order_info_MarketClearinghouseOrderInfo">MarketClearinghouseOrderInfo</a>&lt;M&gt;
): <b>address</b> {
    self.<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>
}
</code></pre>



</details>

<a id="0x7_market_clearinghouse_order_info_get_order_id"></a>

## Function `get_order_id`



<pre><code><b>public</b> <b>fun</b> <a href="market_clearinghouse_order_info.md#0x7_market_clearinghouse_order_info_get_order_id">get_order_id</a>&lt;M: <b>copy</b>, drop&gt;(self: &<a href="market_clearinghouse_order_info.md#0x7_market_clearinghouse_order_info_MarketClearinghouseOrderInfo">market_clearinghouse_order_info::MarketClearinghouseOrderInfo</a>&lt;M&gt;): <a href="_OrderId">order_book_types::OrderId</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_clearinghouse_order_info.md#0x7_market_clearinghouse_order_info_get_order_id">get_order_id</a>&lt;M: <b>copy</b> + drop&gt;(
    self: &<a href="market_clearinghouse_order_info.md#0x7_market_clearinghouse_order_info_MarketClearinghouseOrderInfo">MarketClearinghouseOrderInfo</a>&lt;M&gt;
): OrderId {
    self.order_id
}
</code></pre>



</details>

<a id="0x7_market_clearinghouse_order_info_is_bid"></a>

## Function `is_bid`



<pre><code><b>public</b> <b>fun</b> <a href="market_clearinghouse_order_info.md#0x7_market_clearinghouse_order_info_is_bid">is_bid</a>&lt;M: <b>copy</b>, drop&gt;(self: &<a href="market_clearinghouse_order_info.md#0x7_market_clearinghouse_order_info_MarketClearinghouseOrderInfo">market_clearinghouse_order_info::MarketClearinghouseOrderInfo</a>&lt;M&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_clearinghouse_order_info.md#0x7_market_clearinghouse_order_info_is_bid">is_bid</a>&lt;M: <b>copy</b> + drop&gt;(
    self: &<a href="market_clearinghouse_order_info.md#0x7_market_clearinghouse_order_info_MarketClearinghouseOrderInfo">MarketClearinghouseOrderInfo</a>&lt;M&gt;
): bool {
    self.is_bid
}
</code></pre>



</details>

<a id="0x7_market_clearinghouse_order_info_get_client_order_id"></a>

## Function `get_client_order_id`



<pre><code><b>public</b> <b>fun</b> <a href="market_clearinghouse_order_info.md#0x7_market_clearinghouse_order_info_get_client_order_id">get_client_order_id</a>&lt;M: <b>copy</b>, drop&gt;(self: &<a href="market_clearinghouse_order_info.md#0x7_market_clearinghouse_order_info_MarketClearinghouseOrderInfo">market_clearinghouse_order_info::MarketClearinghouseOrderInfo</a>&lt;M&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_clearinghouse_order_info.md#0x7_market_clearinghouse_order_info_get_client_order_id">get_client_order_id</a>&lt;M: <b>copy</b> + drop&gt;(
    self: &<a href="market_clearinghouse_order_info.md#0x7_market_clearinghouse_order_info_MarketClearinghouseOrderInfo">MarketClearinghouseOrderInfo</a>&lt;M&gt;
): Option&lt;String&gt; {
    self.client_order_id
}
</code></pre>



</details>

<a id="0x7_market_clearinghouse_order_info_get_metadata"></a>

## Function `get_metadata`



<pre><code><b>public</b> <b>fun</b> <a href="market_clearinghouse_order_info.md#0x7_market_clearinghouse_order_info_get_metadata">get_metadata</a>&lt;M: <b>copy</b>, drop&gt;(self: &<a href="market_clearinghouse_order_info.md#0x7_market_clearinghouse_order_info_MarketClearinghouseOrderInfo">market_clearinghouse_order_info::MarketClearinghouseOrderInfo</a>&lt;M&gt;): &M
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_clearinghouse_order_info.md#0x7_market_clearinghouse_order_info_get_metadata">get_metadata</a>&lt;M: <b>copy</b> + drop&gt;(
    self: &<a href="market_clearinghouse_order_info.md#0x7_market_clearinghouse_order_info_MarketClearinghouseOrderInfo">MarketClearinghouseOrderInfo</a>&lt;M&gt;
): &M {
    &self.metadata
}
</code></pre>



</details>

<a id="0x7_market_clearinghouse_order_info_into_inner"></a>

## Function `into_inner`



<pre><code><b>public</b> <b>fun</b> <a href="market_clearinghouse_order_info.md#0x7_market_clearinghouse_order_info_into_inner">into_inner</a>&lt;M: <b>copy</b>, drop&gt;(self: <a href="market_clearinghouse_order_info.md#0x7_market_clearinghouse_order_info_MarketClearinghouseOrderInfo">market_clearinghouse_order_info::MarketClearinghouseOrderInfo</a>&lt;M&gt;): (<b>address</b>, <a href="_OrderId">order_book_types::OrderId</a>, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, bool, u64, <a href="_TimeInForce">order_book_types::TimeInForce</a>, <a href="_OrderType">order_book_types::OrderType</a>, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="_TriggerCondition">order_book_types::TriggerCondition</a>&gt;, M)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_clearinghouse_order_info.md#0x7_market_clearinghouse_order_info_into_inner">into_inner</a>&lt;M: <b>copy</b> + drop&gt;(
    self: <a href="market_clearinghouse_order_info.md#0x7_market_clearinghouse_order_info_MarketClearinghouseOrderInfo">MarketClearinghouseOrderInfo</a>&lt;M&gt;
): (
    <b>address</b>,
    OrderId,
    Option&lt;String&gt;,
    bool,
    u64,
    TimeInForce,
    OrderType,
    Option&lt;TriggerCondition&gt;,
    M
) {
    <b>let</b> MarketClearinghouseOrderInfo::V1 {
        <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>,
        order_id,
        client_order_id,
        is_bid,
        limit_price,
        time_in_force,
        order_type,
        trigger_condition,
        metadata
    } = self;
    (
        <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>,
        order_id,
        client_order_id,
        is_bid,
        limit_price,
        time_in_force,
        order_type,
        trigger_condition,
        metadata
    )
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
