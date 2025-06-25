
<a id="0x7_market_types"></a>

# Module `0x7::market_types`



-  [Enum `SettleTradeResult`](#0x7_market_types_SettleTradeResult)
-  [Enum `MarketClearinghouseCallbacks`](#0x7_market_types_MarketClearinghouseCallbacks)
-  [Constants](#@Constants_0)
-  [Function `new_settle_trade_result`](#0x7_market_types_new_settle_trade_result)
-  [Function `new_market_clearinghouse_callbacks`](#0x7_market_types_new_market_clearinghouse_callbacks)
-  [Function `get_settled_size`](#0x7_market_types_get_settled_size)
-  [Function `get_maker_cancellation_reason`](#0x7_market_types_get_maker_cancellation_reason)
-  [Function `get_taker_cancellation_reason`](#0x7_market_types_get_taker_cancellation_reason)
-  [Function `settle_trade`](#0x7_market_types_settle_trade)
-  [Function `validate_order_placement`](#0x7_market_types_validate_order_placement)
-  [Function `place_maker_order`](#0x7_market_types_place_maker_order)
-  [Function `cleanup_order`](#0x7_market_types_cleanup_order)
-  [Function `decrease_order_size`](#0x7_market_types_decrease_order_size)


<pre><code><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;
</code></pre>



<a id="0x7_market_types_SettleTradeResult"></a>

## Enum `SettleTradeResult`



<pre><code>enum <a href="market_types.md#0x7_market_types_SettleTradeResult">SettleTradeResult</a> <b>has</b> drop
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>V1</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>settled_size: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>maker_cancellation_reason: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>taker_cancellation_reason: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

</details>

<a id="0x7_market_types_MarketClearinghouseCallbacks"></a>

## Enum `MarketClearinghouseCallbacks`



<pre><code>enum <a href="market_types.md#0x7_market_types_MarketClearinghouseCallbacks">MarketClearinghouseCallbacks</a>&lt;M: <b>copy</b>, drop, store&gt; <b>has</b> drop
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>V1</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>settle_trade_f: |(<b>address</b>, <b>address</b>, u64, u64, u64, bool, u64, u64, M, M)|<a href="market_types.md#0x7_market_types_SettleTradeResult">market_types::SettleTradeResult</a> <b>has</b> <b>copy</b> + drop</code>
</dt>
<dd>

</dd>
<dt>
<code>validate_order_placement_f: |(<b>address</b>, u64, bool, bool, u64, u64, M)|bool <b>has</b> <b>copy</b> + drop</code>
</dt>
<dd>

</dd>
<dt>
<code>place_maker_order_f: |(<b>address</b>, u64, bool, u64, u64, M)| <b>has</b> <b>copy</b> + drop</code>
</dt>
<dd>

</dd>
<dt>
<code>cleanup_order_f: |(<b>address</b>, u64, bool, u64)| <b>has</b> <b>copy</b> + drop</code>
</dt>
<dd>

</dd>
<dt>
<code>decrease_order_size_f: |(<b>address</b>, u64, bool, u64, u64)| <b>has</b> <b>copy</b> + drop</code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x7_market_types_EINVALID_ADDRESS"></a>



<pre><code><b>const</b> <a href="market_types.md#0x7_market_types_EINVALID_ADDRESS">EINVALID_ADDRESS</a>: u64 = 1;
</code></pre>



<a id="0x7_market_types_EINVALID_SETTLE_RESULT"></a>



<pre><code><b>const</b> <a href="market_types.md#0x7_market_types_EINVALID_SETTLE_RESULT">EINVALID_SETTLE_RESULT</a>: u64 = 2;
</code></pre>



<a id="0x7_market_types_new_settle_trade_result"></a>

## Function `new_settle_trade_result`



<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_new_settle_trade_result">new_settle_trade_result</a>(settled_size: u64, maker_cancellation_reason: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, taker_cancellation_reason: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;): <a href="market_types.md#0x7_market_types_SettleTradeResult">market_types::SettleTradeResult</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_new_settle_trade_result">new_settle_trade_result</a>(
    settled_size: u64,
    maker_cancellation_reason: Option&lt;String&gt;,
    taker_cancellation_reason: Option&lt;String&gt;
): <a href="market_types.md#0x7_market_types_SettleTradeResult">SettleTradeResult</a> {
    SettleTradeResult::V1 {
        settled_size,
        maker_cancellation_reason,
        taker_cancellation_reason
    }
}
</code></pre>



</details>

<a id="0x7_market_types_new_market_clearinghouse_callbacks"></a>

## Function `new_market_clearinghouse_callbacks`



<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_new_market_clearinghouse_callbacks">new_market_clearinghouse_callbacks</a>&lt;M: <b>copy</b>, drop, store&gt;(settle_trade_f: |(<b>address</b>, <b>address</b>, u64, u64, u64, bool, u64, u64, M, M)|<a href="market_types.md#0x7_market_types_SettleTradeResult">market_types::SettleTradeResult</a> <b>has</b> <b>copy</b> + drop, validate_order_placement_f: |(<b>address</b>, u64, bool, bool, u64, u64, M)|bool <b>has</b> <b>copy</b> + drop, place_maker_order_f: |(<b>address</b>, u64, bool, u64, u64, M)| <b>has</b> <b>copy</b> + drop, cleanup_order_f: |(<b>address</b>, u64, bool, u64)| <b>has</b> <b>copy</b> + drop, decrease_order_size_f: |(<b>address</b>, u64, bool, u64, u64)| <b>has</b> <b>copy</b> + drop): <a href="market_types.md#0x7_market_types_MarketClearinghouseCallbacks">market_types::MarketClearinghouseCallbacks</a>&lt;M&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_new_market_clearinghouse_callbacks">new_market_clearinghouse_callbacks</a>&lt;M: store + <b>copy</b> + drop&gt;(
    // settle_trade_f arguments: taker, maker, taker_order_id, maker_order_id, fill_id, is_taker_long, price, size
    settle_trade_f: |<b>address</b>, <b>address</b>, u64, u64, u64, bool, u64, u64, M, M| <a href="market_types.md#0x7_market_types_SettleTradeResult">SettleTradeResult</a> <b>has</b> drop + <b>copy</b>,
    // validate_settlement_update_f arguments: accoun, is_taker, is_long, price, size
    validate_order_placement_f: |<b>address</b>, u64, bool, bool, u64, u64, M| bool <b>has</b> drop + <b>copy</b>,
    place_maker_order_f: |<b>address</b>, u64, bool, u64, u64, M| <b>has</b> drop + <b>copy</b>,
    cleanup_order_f: |<b>address</b>, u64, bool, u64| <b>has</b> drop + <b>copy</b>,
    decrease_order_size_f: |<b>address</b>, u64, bool, u64, u64| <b>has</b> drop + <b>copy</b>,
): <a href="market_types.md#0x7_market_types_MarketClearinghouseCallbacks">MarketClearinghouseCallbacks</a>&lt;M&gt; {
    MarketClearinghouseCallbacks::V1 {
        settle_trade_f,
        validate_order_placement_f,
        place_maker_order_f,
        cleanup_order_f,
        decrease_order_size_f
    }
}
</code></pre>



</details>

<a id="0x7_market_types_get_settled_size"></a>

## Function `get_settled_size`



<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_get_settled_size">get_settled_size</a>(self: &<a href="market_types.md#0x7_market_types_SettleTradeResult">market_types::SettleTradeResult</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_get_settled_size">get_settled_size</a>(self: &<a href="market_types.md#0x7_market_types_SettleTradeResult">SettleTradeResult</a>): u64 {
    self.settled_size
}
</code></pre>



</details>

<a id="0x7_market_types_get_maker_cancellation_reason"></a>

## Function `get_maker_cancellation_reason`



<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_get_maker_cancellation_reason">get_maker_cancellation_reason</a>(self: &<a href="market_types.md#0x7_market_types_SettleTradeResult">market_types::SettleTradeResult</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_get_maker_cancellation_reason">get_maker_cancellation_reason</a>(self: &<a href="market_types.md#0x7_market_types_SettleTradeResult">SettleTradeResult</a>): Option&lt;String&gt; {
    self.maker_cancellation_reason
}
</code></pre>



</details>

<a id="0x7_market_types_get_taker_cancellation_reason"></a>

## Function `get_taker_cancellation_reason`



<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_get_taker_cancellation_reason">get_taker_cancellation_reason</a>(self: &<a href="market_types.md#0x7_market_types_SettleTradeResult">market_types::SettleTradeResult</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_get_taker_cancellation_reason">get_taker_cancellation_reason</a>(self: &<a href="market_types.md#0x7_market_types_SettleTradeResult">SettleTradeResult</a>): Option&lt;String&gt; {
    self.taker_cancellation_reason
}
</code></pre>



</details>

<a id="0x7_market_types_settle_trade"></a>

## Function `settle_trade`



<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_settle_trade">settle_trade</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="market_types.md#0x7_market_types_MarketClearinghouseCallbacks">market_types::MarketClearinghouseCallbacks</a>&lt;M&gt;, taker: <b>address</b>, maker: <b>address</b>, taker_order_id: u64, maker_order_id: u64, fill_id: u64, is_taker_long: bool, price: u64, size: u64, taker_metadata: M, maker_metadata: M): <a href="market_types.md#0x7_market_types_SettleTradeResult">market_types::SettleTradeResult</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_settle_trade">settle_trade</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="market_types.md#0x7_market_types_MarketClearinghouseCallbacks">MarketClearinghouseCallbacks</a>&lt;M&gt;,
    taker: <b>address</b>,
    maker: <b>address</b>,
    taker_order_id: u64,
    maker_order_id:u64,
    fill_id: u64,
    is_taker_long: bool,
    price: u64,
    size: u64,
    taker_metadata: M,
    maker_metadata: M): <a href="market_types.md#0x7_market_types_SettleTradeResult">SettleTradeResult</a> {
    (self.settle_trade_f)(taker, maker, taker_order_id, maker_order_id, fill_id, is_taker_long, price, size, taker_metadata, maker_metadata)
}
</code></pre>



</details>

<a id="0x7_market_types_validate_order_placement"></a>

## Function `validate_order_placement`



<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_validate_order_placement">validate_order_placement</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="market_types.md#0x7_market_types_MarketClearinghouseCallbacks">market_types::MarketClearinghouseCallbacks</a>&lt;M&gt;, <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>, order_id: u64, is_taker: bool, is_bid: bool, price: u64, size: u64, order_metadata: M): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_validate_order_placement">validate_order_placement</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="market_types.md#0x7_market_types_MarketClearinghouseCallbacks">MarketClearinghouseCallbacks</a>&lt;M&gt;,
    <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>,
    order_id: u64,
    is_taker: bool,
    is_bid: bool,
    price: u64,
    size: u64,
    order_metadata: M): bool {
    (self.validate_order_placement_f)(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>, order_id, is_taker, is_bid, price, size, order_metadata)
}
</code></pre>



</details>

<a id="0x7_market_types_place_maker_order"></a>

## Function `place_maker_order`



<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_place_maker_order">place_maker_order</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="market_types.md#0x7_market_types_MarketClearinghouseCallbacks">market_types::MarketClearinghouseCallbacks</a>&lt;M&gt;, <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>, order_id: u64, is_bid: bool, price: u64, size: u64, order_metadata: M)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_place_maker_order">place_maker_order</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="market_types.md#0x7_market_types_MarketClearinghouseCallbacks">MarketClearinghouseCallbacks</a>&lt;M&gt;,
    <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>,
    order_id: u64,
    is_bid: bool,
    price: u64,
    size: u64,
    order_metadata: M) {
    (self.place_maker_order_f)(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>, order_id, is_bid, price, size, order_metadata)
}
</code></pre>



</details>

<a id="0x7_market_types_cleanup_order"></a>

## Function `cleanup_order`



<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_cleanup_order">cleanup_order</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="market_types.md#0x7_market_types_MarketClearinghouseCallbacks">market_types::MarketClearinghouseCallbacks</a>&lt;M&gt;, <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>, order_id: u64, is_bid: bool, remaining_size: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_cleanup_order">cleanup_order</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="market_types.md#0x7_market_types_MarketClearinghouseCallbacks">MarketClearinghouseCallbacks</a>&lt;M&gt;,
    <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>,
    order_id: u64,
    is_bid: bool,
    remaining_size: u64) {
    (self.cleanup_order_f)(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>, order_id, is_bid, remaining_size)
}
</code></pre>



</details>

<a id="0x7_market_types_decrease_order_size"></a>

## Function `decrease_order_size`



<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_decrease_order_size">decrease_order_size</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="market_types.md#0x7_market_types_MarketClearinghouseCallbacks">market_types::MarketClearinghouseCallbacks</a>&lt;M&gt;, <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>, order_id: u64, is_bid: bool, price: u64, size: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_decrease_order_size">decrease_order_size</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="market_types.md#0x7_market_types_MarketClearinghouseCallbacks">MarketClearinghouseCallbacks</a>&lt;M&gt;,
    <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>,
    order_id: u64,
    is_bid: bool,
    price: u64,
    size: u64,) {
    (self.decrease_order_size_f)(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>, order_id, is_bid, price, size)
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
