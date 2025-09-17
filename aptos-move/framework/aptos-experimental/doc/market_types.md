
<a id="0x7_market_types"></a>

# Module `0x7::market_types`



-  [Enum `OrderStatus`](#0x7_market_types_OrderStatus)
-  [Enum `SettleTradeResult`](#0x7_market_types_SettleTradeResult)
-  [Enum `MarketClearinghouseCallbacks`](#0x7_market_types_MarketClearinghouseCallbacks)
-  [Enum `Market`](#0x7_market_types_Market)
-  [Enum `MarketConfig`](#0x7_market_types_MarketConfig)
-  [Struct `OrderEvent`](#0x7_market_types_OrderEvent)
-  [Constants](#@Constants_0)
-  [Function `order_status_open`](#0x7_market_types_order_status_open)
-  [Function `order_status_filled`](#0x7_market_types_order_status_filled)
-  [Function `order_status_cancelled`](#0x7_market_types_order_status_cancelled)
-  [Function `order_status_rejected`](#0x7_market_types_order_status_rejected)
-  [Function `order_status_size_reduced`](#0x7_market_types_order_status_size_reduced)
-  [Function `order_status_acknowledged`](#0x7_market_types_order_status_acknowledged)
-  [Function `new_settle_trade_result`](#0x7_market_types_new_settle_trade_result)
-  [Function `new_market_clearinghouse_callbacks`](#0x7_market_types_new_market_clearinghouse_callbacks)
-  [Function `get_settled_size`](#0x7_market_types_get_settled_size)
-  [Function `get_maker_cancellation_reason`](#0x7_market_types_get_maker_cancellation_reason)
-  [Function `get_taker_cancellation_reason`](#0x7_market_types_get_taker_cancellation_reason)
-  [Function `settle_trade`](#0x7_market_types_settle_trade)
-  [Function `validate_order_placement`](#0x7_market_types_validate_order_placement)
-  [Function `validate_bulk_order_placement`](#0x7_market_types_validate_bulk_order_placement)
-  [Function `place_maker_order`](#0x7_market_types_place_maker_order)
-  [Function `cleanup_order`](#0x7_market_types_cleanup_order)
-  [Function `cleanup_bulk_orders`](#0x7_market_types_cleanup_bulk_orders)
-  [Function `decrease_order_size`](#0x7_market_types_decrease_order_size)
-  [Function `get_order_metadata_bytes`](#0x7_market_types_get_order_metadata_bytes)
-  [Function `new_market_config`](#0x7_market_types_new_market_config)
-  [Function `new_market`](#0x7_market_types_new_market)
-  [Function `next_order_id`](#0x7_market_types_next_order_id)
-  [Function `next_fill_id`](#0x7_market_types_next_fill_id)
-  [Function `get_order_book`](#0x7_market_types_get_order_book)
-  [Function `get_order_book_mut`](#0x7_market_types_get_order_book_mut)
-  [Function `get_market_address`](#0x7_market_types_get_market_address)
-  [Function `get_pre_cancellation_tracker_mut`](#0x7_market_types_get_pre_cancellation_tracker_mut)
-  [Function `best_bid_price`](#0x7_market_types_best_bid_price)
-  [Function `best_ask_price`](#0x7_market_types_best_ask_price)
-  [Function `is_taker_order`](#0x7_market_types_is_taker_order)
-  [Function `is_allowed_self_trade`](#0x7_market_types_is_allowed_self_trade)
-  [Function `get_remaining_size`](#0x7_market_types_get_remaining_size)
-  [Function `get_bulk_order_remaining_size`](#0x7_market_types_get_bulk_order_remaining_size)
-  [Function `get_order_metadata`](#0x7_market_types_get_order_metadata)
-  [Function `set_order_metadata`](#0x7_market_types_set_order_metadata)
-  [Function `get_order_metadata_by_client_id`](#0x7_market_types_get_order_metadata_by_client_id)
-  [Function `set_order_metadata_by_client_id`](#0x7_market_types_set_order_metadata_by_client_id)
-  [Function `take_ready_price_based_orders`](#0x7_market_types_take_ready_price_based_orders)
-  [Function `take_ready_time_based_orders`](#0x7_market_types_take_ready_time_based_orders)
-  [Function `emit_event_for_order`](#0x7_market_types_emit_event_for_order)


<pre><code><b>use</b> <a href="../../aptos-framework/doc/event.md#0x1_event">0x1::event</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table">0x1::table</a>;
<b>use</b> <a href="order_book.md#0x7_order_book">0x7::order_book</a>;
<b>use</b> <a href="order_book_types.md#0x7_order_book_types">0x7::order_book_types</a>;
<b>use</b> <a href="pre_cancellation_tracker.md#0x7_pre_cancellation_tracker">0x7::pre_cancellation_tracker</a>;
<b>use</b> <a href="single_order_types.md#0x7_single_order_types">0x7::single_order_types</a>;
</code></pre>



<a id="0x7_market_types_OrderStatus"></a>

## Enum `OrderStatus`



<pre><code>enum <a href="market_types.md#0x7_market_types_OrderStatus">OrderStatus</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>OPEN</summary>


<details>
<summary>Fields</summary>


<dl>
</dl>


</details>

</details>

<details>
<summary>FILLED</summary>


<details>
<summary>Fields</summary>


<dl>
</dl>


</details>

</details>

<details>
<summary>CANCELLED</summary>


<details>
<summary>Fields</summary>


<dl>
</dl>


</details>

</details>

<details>
<summary>REJECTED</summary>


<details>
<summary>Fields</summary>


<dl>
</dl>


</details>

</details>

<details>
<summary>SIZE_REDUCED</summary>


<details>
<summary>Fields</summary>


<dl>
</dl>


</details>

</details>

<details>
<summary>ACKNOWLEDGED</summary>


<details>
<summary>Fields</summary>


<dl>
</dl>


</details>

</details>

</details>

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
<code>settle_trade_f: |&<b>mut</b> <a href="market_types.md#0x7_market_types_Market">market_types::Market</a>&lt;M&gt;, <b>address</b>, <a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>, <b>address</b>, <a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>, u64, bool, u64, u64, M, M|<a href="market_types.md#0x7_market_types_SettleTradeResult">market_types::SettleTradeResult</a> <b>has</b> <b>copy</b> + drop</code>
</dt>
<dd>
 settle_trade_f arguments: taker, taker_order_id, maker, maker_order_id, fill_id, is_taker_long, price, size
</dd>
<dt>
<code>validate_order_placement_f: |<b>address</b>, <a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>, bool, bool, u64, <a href="order_book_types.md#0x7_order_book_types_TimeInForce">order_book_types::TimeInForce</a>, u64, M|bool <b>has</b> <b>copy</b> + drop</code>
</dt>
<dd>
 validate_settlement_update_f arguments: account, order_id, is_taker, is_long, price, size
</dd>
<dt>
<code>validate_bulk_order_placement_f: |<b>address</b>, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, M|bool <b>has</b> <b>copy</b> + drop</code>
</dt>
<dd>
 Validate the bulk order placement arguments: account, bids_prices, bids_sizes, asks_prices, asks_sizes
</dd>
<dt>
<code>place_maker_order_f: |<b>address</b>, <a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>, bool, u64, u64, M| <b>has</b> <b>copy</b> + drop</code>
</dt>
<dd>
 place_maker_order_f arguments: account, order_id, is_bid, price, size, order_metadata
</dd>
<dt>
<code>cleanup_order_f: |<b>address</b>, <a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>, bool, u64, M| <b>has</b> <b>copy</b> + drop</code>
</dt>
<dd>
 cleanup_order_f arguments: account, order_id, is_bid, remaining_size, order_metadata
</dd>
<dt>
<code>cleanup_bulk_orders_f: |<b>address</b>, bool, u64| <b>has</b> <b>copy</b> + drop</code>
</dt>
<dd>
 cleanup_bulk_orders_f arguments: account, is_bid, remaining_sizes
</dd>
<dt>
<code>decrease_order_size_f: |<b>address</b>, <a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>, bool, u64, u64| <b>has</b> <b>copy</b> + drop</code>
</dt>
<dd>
 decrease_order_size_f arguments: account, order_id, is_bid, price, size
</dd>
<dt>
<code>get_order_metadata_bytes: |M|<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; <b>has</b> <b>copy</b> + drop</code>
</dt>
<dd>
 get a string representation of order metadata to be used in events
</dd>
</dl>


</details>

</details>

</details>

<a id="0x7_market_types_Market"></a>

## Enum `Market`



<pre><code>enum <a href="market_types.md#0x7_market_types_Market">Market</a>&lt;M: <b>copy</b>, drop, store&gt; <b>has</b> store
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>V1</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>parent: <b>address</b></code>
</dt>
<dd>
 Address of the parent object that created this market
 Purely for grouping events based on the source DEX, not used otherwise
</dd>
<dt>
<code>market: <b>address</b></code>
</dt>
<dd>
 Address of the market object of this market.
</dd>
<dt>
<code>order_id_generator: <a href="order_book_types.md#0x7_order_book_types_AscendingIdGenerator">order_book_types::AscendingIdGenerator</a></code>
</dt>
<dd>

</dd>
<dt>
<code>next_fill_id: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>config: <a href="market_types.md#0x7_market_types_MarketConfig">market_types::MarketConfig</a></code>
</dt>
<dd>

</dd>
<dt>
<code><a href="order_book.md#0x7_order_book">order_book</a>: <a href="order_book.md#0x7_order_book_OrderBook">order_book::OrderBook</a>&lt;M&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code><a href="pre_cancellation_tracker.md#0x7_pre_cancellation_tracker">pre_cancellation_tracker</a>: <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_Table">table::Table</a>&lt;u8, <a href="pre_cancellation_tracker.md#0x7_pre_cancellation_tracker_PreCancellationTracker">pre_cancellation_tracker::PreCancellationTracker</a>&gt;</code>
</dt>
<dd>
 Pre cancellation tracker for the market, it is wrapped inside a table
 as otherwise any insertion/deletion from the tracker would cause conflict
 with the order book.
</dd>
</dl>


</details>

</details>

</details>

<a id="0x7_market_types_MarketConfig"></a>

## Enum `MarketConfig`



<pre><code>enum <a href="market_types.md#0x7_market_types_MarketConfig">MarketConfig</a> <b>has</b> store
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>V1</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>allow_self_trade: bool</code>
</dt>
<dd>
 Weather to allow self matching orders
</dd>
<dt>
<code>allow_events_emission: bool</code>
</dt>
<dd>
 Whether to allow sending all events for the markett
</dd>
<dt>
<code>pre_cancellation_window_secs: u64</code>
</dt>
<dd>
 Pre cancellation window in microseconds
</dd>
</dl>


</details>

</details>

</details>

<a id="0x7_market_types_OrderEvent"></a>

## Struct `OrderEvent`



<pre><code>#[<a href="../../aptos-framework/doc/event.md#0x1_event">event</a>]
<b>struct</b> <a href="market_types.md#0x7_market_types_OrderEvent">OrderEvent</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>parent: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>market: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>order_id: u128</code>
</dt>
<dd>

</dd>
<dt>
<code>client_order_id: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>user: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>orig_size: u64</code>
</dt>
<dd>
 Original size of the order
</dd>
<dt>
<code>remaining_size: u64</code>
</dt>
<dd>
 Remaining size of the order in the order book
</dd>
<dt>
<code>size_delta: u64</code>
</dt>
<dd>
 OPEN - size_delta will be amount of size added
 CANCELLED - size_delta will be amount of size removed
 FILLED - size_delta will be amount of size filled
 REJECTED - size_delta will always be 0
</dd>
<dt>
<code>price: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>is_bid: bool</code>
</dt>
<dd>

</dd>
<dt>
<code>is_taker: bool</code>
</dt>
<dd>
 Whether the order crosses the orderbook.
</dd>
<dt>
<code>status: <a href="market_types.md#0x7_market_types_OrderStatus">market_types::OrderStatus</a></code>
</dt>
<dd>

</dd>
<dt>
<code>details: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
<dt>
<code>metadata_bytes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>time_in_force: <a href="order_book_types.md#0x7_order_book_types_TimeInForce">order_book_types::TimeInForce</a></code>
</dt>
<dd>

</dd>
<dt>
<code>trigger_condition: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="order_book_types.md#0x7_order_book_types_TriggerCondition">order_book_types::TriggerCondition</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x7_market_types_EINVALID_TIME_IN_FORCE"></a>



<pre><code><b>const</b> <a href="market_types.md#0x7_market_types_EINVALID_TIME_IN_FORCE">EINVALID_TIME_IN_FORCE</a>: u64 = 3;
</code></pre>



<a id="0x7_market_types_EINVALID_ADDRESS"></a>



<pre><code><b>const</b> <a href="market_types.md#0x7_market_types_EINVALID_ADDRESS">EINVALID_ADDRESS</a>: u64 = 1;
</code></pre>



<a id="0x7_market_types_EINVALID_SETTLE_RESULT"></a>



<pre><code><b>const</b> <a href="market_types.md#0x7_market_types_EINVALID_SETTLE_RESULT">EINVALID_SETTLE_RESULT</a>: u64 = 2;
</code></pre>



<a id="0x7_market_types_EORDER_DOES_NOT_EXIST"></a>



<pre><code><b>const</b> <a href="market_types.md#0x7_market_types_EORDER_DOES_NOT_EXIST">EORDER_DOES_NOT_EXIST</a>: u64 = 6;
</code></pre>



<a id="0x7_market_types_PRE_CANCELLATION_TRACKER_KEY"></a>



<pre><code><b>const</b> <a href="market_types.md#0x7_market_types_PRE_CANCELLATION_TRACKER_KEY">PRE_CANCELLATION_TRACKER_KEY</a>: u8 = 0;
</code></pre>



<a id="0x7_market_types_order_status_open"></a>

## Function `order_status_open`



<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_order_status_open">order_status_open</a>(): <a href="market_types.md#0x7_market_types_OrderStatus">market_types::OrderStatus</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_order_status_open">order_status_open</a>(): <a href="market_types.md#0x7_market_types_OrderStatus">OrderStatus</a> {
    OrderStatus::OPEN
}
</code></pre>



</details>

<a id="0x7_market_types_order_status_filled"></a>

## Function `order_status_filled`



<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_order_status_filled">order_status_filled</a>(): <a href="market_types.md#0x7_market_types_OrderStatus">market_types::OrderStatus</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_order_status_filled">order_status_filled</a>(): <a href="market_types.md#0x7_market_types_OrderStatus">OrderStatus</a> {
    OrderStatus::FILLED
}
</code></pre>



</details>

<a id="0x7_market_types_order_status_cancelled"></a>

## Function `order_status_cancelled`



<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_order_status_cancelled">order_status_cancelled</a>(): <a href="market_types.md#0x7_market_types_OrderStatus">market_types::OrderStatus</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_order_status_cancelled">order_status_cancelled</a>(): <a href="market_types.md#0x7_market_types_OrderStatus">OrderStatus</a> {
    OrderStatus::CANCELLED
}
</code></pre>



</details>

<a id="0x7_market_types_order_status_rejected"></a>

## Function `order_status_rejected`



<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_order_status_rejected">order_status_rejected</a>(): <a href="market_types.md#0x7_market_types_OrderStatus">market_types::OrderStatus</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_order_status_rejected">order_status_rejected</a>(): <a href="market_types.md#0x7_market_types_OrderStatus">OrderStatus</a> {
    OrderStatus::REJECTED
}
</code></pre>



</details>

<a id="0x7_market_types_order_status_size_reduced"></a>

## Function `order_status_size_reduced`



<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_order_status_size_reduced">order_status_size_reduced</a>(): <a href="market_types.md#0x7_market_types_OrderStatus">market_types::OrderStatus</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_order_status_size_reduced">order_status_size_reduced</a>(): <a href="market_types.md#0x7_market_types_OrderStatus">OrderStatus</a> {
    OrderStatus::SIZE_REDUCED
}
</code></pre>



</details>

<a id="0x7_market_types_order_status_acknowledged"></a>

## Function `order_status_acknowledged`



<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_order_status_acknowledged">order_status_acknowledged</a>(): <a href="market_types.md#0x7_market_types_OrderStatus">market_types::OrderStatus</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_order_status_acknowledged">order_status_acknowledged</a>(): <a href="market_types.md#0x7_market_types_OrderStatus">OrderStatus</a> {
    OrderStatus::ACKNOWLEDGED
}
</code></pre>



</details>

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



<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_new_market_clearinghouse_callbacks">new_market_clearinghouse_callbacks</a>&lt;M: <b>copy</b>, drop, store&gt;(settle_trade_f: |&<b>mut</b> <a href="market_types.md#0x7_market_types_Market">market_types::Market</a>&lt;M&gt;, <b>address</b>, <a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>, <b>address</b>, <a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>, u64, bool, u64, u64, M, M|<a href="market_types.md#0x7_market_types_SettleTradeResult">market_types::SettleTradeResult</a> <b>has</b> <b>copy</b> + drop, validate_order_placement_f: |<b>address</b>, <a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>, bool, bool, u64, <a href="order_book_types.md#0x7_order_book_types_TimeInForce">order_book_types::TimeInForce</a>, u64, M|bool <b>has</b> <b>copy</b> + drop, validate_bulk_order_placement_f: |<b>address</b>, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, M|bool <b>has</b> <b>copy</b> + drop, place_maker_order_f: |<b>address</b>, <a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>, bool, u64, u64, M| <b>has</b> <b>copy</b> + drop, cleanup_order_f: |<b>address</b>, <a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>, bool, u64, M| <b>has</b> <b>copy</b> + drop, cleanup_bulk_orders_f: |<b>address</b>, bool, u64| <b>has</b> <b>copy</b> + drop, decrease_order_size_f: |<b>address</b>, <a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>, bool, u64, u64| <b>has</b> <b>copy</b> + drop, get_order_metadata_bytes: |M|<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; <b>has</b> <b>copy</b> + drop): <a href="market_types.md#0x7_market_types_MarketClearinghouseCallbacks">market_types::MarketClearinghouseCallbacks</a>&lt;M&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_new_market_clearinghouse_callbacks">new_market_clearinghouse_callbacks</a>&lt;M: store + <b>copy</b> + drop&gt;(
    // settle_trade_f arguments: taker, taker_order_id, maker, maker_order_id, fill_id, is_taker_long, price, size
    settle_trade_f: |&<b>mut</b> <a href="market_types.md#0x7_market_types_Market">Market</a>&lt;M&gt;, <b>address</b>, OrderIdType, <b>address</b>, OrderIdType, u64, bool, u64, u64, M, M| <a href="market_types.md#0x7_market_types_SettleTradeResult">SettleTradeResult</a> <b>has</b> drop + <b>copy</b>,
    // validate_settlement_update_f arguments: <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>, order_id, is_taker, is_long, price, size
    validate_order_placement_f: |<b>address</b>, OrderIdType, bool, bool, u64,  TimeInForce, u64, M| bool <b>has</b> drop + <b>copy</b>,
    // Validate the bulk order placement
    validate_bulk_order_placement_f: |<b>address</b>, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, M| bool <b>has</b> drop + <b>copy</b>,
    // place_maker_order_f arguments: <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>, order_id, is_bid, price, size, order_metadata
    place_maker_order_f: |<b>address</b>, OrderIdType, bool, u64, u64, M| <b>has</b> drop + <b>copy</b>,
    // cleanup_order_f arguments: <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>, order_id, is_bid, remaining_size, order_metadata
    cleanup_order_f: |<b>address</b>, OrderIdType, bool, u64, M| <b>has</b> drop + <b>copy</b>,
    // cleanup_bulk_orders_f arguments: <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>, is_bid, remaining_sizes
    cleanup_bulk_orders_f: |<b>address</b>, bool, u64| <b>has</b> drop + <b>copy</b>,
    // decrease_order_size_f arguments: <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>, order_id, is_bid, price, size
    decrease_order_size_f: |<b>address</b>, OrderIdType, bool, u64, u64| <b>has</b> drop + <b>copy</b>,
    // get a <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">string</a> representation of order metadata <b>to</b> be used in events
    get_order_metadata_bytes: |M| <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; <b>has</b> drop + <b>copy</b>
): <a href="market_types.md#0x7_market_types_MarketClearinghouseCallbacks">MarketClearinghouseCallbacks</a>&lt;M&gt; {
    MarketClearinghouseCallbacks::V1 {
        settle_trade_f,
        validate_order_placement_f,
        validate_bulk_order_placement_f,
        place_maker_order_f,
        cleanup_order_f,
        cleanup_bulk_orders_f,
        decrease_order_size_f,
        get_order_metadata_bytes
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



<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_settle_trade">settle_trade</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="market_types.md#0x7_market_types_MarketClearinghouseCallbacks">market_types::MarketClearinghouseCallbacks</a>&lt;M&gt;, market: &<b>mut</b> <a href="market_types.md#0x7_market_types_Market">market_types::Market</a>&lt;M&gt;, taker: <b>address</b>, taker_order_id: <a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>, maker: <b>address</b>, maker_order_id: <a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>, fill_id: u64, is_taker_long: bool, price: u64, size: u64, taker_metadata: M, maker_metadata: M): <a href="market_types.md#0x7_market_types_SettleTradeResult">market_types::SettleTradeResult</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_settle_trade">settle_trade</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="market_types.md#0x7_market_types_MarketClearinghouseCallbacks">MarketClearinghouseCallbacks</a>&lt;M&gt;,
    market: &<b>mut</b> <a href="market_types.md#0x7_market_types_Market">Market</a>&lt;M&gt;,
    taker: <b>address</b>,
    taker_order_id: OrderIdType,
    maker: <b>address</b>,
    maker_order_id: OrderIdType,
    fill_id: u64,
    is_taker_long: bool,
    price: u64,
    size: u64,
    taker_metadata: M,
    maker_metadata: M): <a href="market_types.md#0x7_market_types_SettleTradeResult">SettleTradeResult</a> {
    (self.settle_trade_f)(market, taker, taker_order_id, maker, maker_order_id, fill_id, is_taker_long, price, size, taker_metadata, maker_metadata)
}
</code></pre>



</details>

<a id="0x7_market_types_validate_order_placement"></a>

## Function `validate_order_placement`



<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_validate_order_placement">validate_order_placement</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="market_types.md#0x7_market_types_MarketClearinghouseCallbacks">market_types::MarketClearinghouseCallbacks</a>&lt;M&gt;, <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>, order_id: <a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>, is_taker: bool, is_bid: bool, price: u64, time_in_force: <a href="order_book_types.md#0x7_order_book_types_TimeInForce">order_book_types::TimeInForce</a>, size: u64, order_metadata: M): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_validate_order_placement">validate_order_placement</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="market_types.md#0x7_market_types_MarketClearinghouseCallbacks">MarketClearinghouseCallbacks</a>&lt;M&gt;,
    <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>,
    order_id: OrderIdType,
    is_taker: bool,
    is_bid: bool,
    price: u64,
    time_in_force: TimeInForce,
    size: u64,
    order_metadata: M): bool {
    (self.validate_order_placement_f)(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>, order_id, is_taker, is_bid, price, time_in_force, size, order_metadata)
}
</code></pre>



</details>

<a id="0x7_market_types_validate_bulk_order_placement"></a>

## Function `validate_bulk_order_placement`



<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_validate_bulk_order_placement">validate_bulk_order_placement</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="market_types.md#0x7_market_types_MarketClearinghouseCallbacks">market_types::MarketClearinghouseCallbacks</a>&lt;M&gt;, <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>, bids_prices: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, bids_sizes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, asks_prices: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, asks_sizes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, order_metadata: M): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_validate_bulk_order_placement">validate_bulk_order_placement</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="market_types.md#0x7_market_types_MarketClearinghouseCallbacks">MarketClearinghouseCallbacks</a>&lt;M&gt;,
    <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>,
    bids_prices: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    bids_sizes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    asks_prices: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    asks_sizes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    order_metadata: M,
): bool {
    (self.validate_bulk_order_placement_f)(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>, bids_prices, bids_sizes, asks_prices, asks_sizes, order_metadata)
}
</code></pre>



</details>

<a id="0x7_market_types_place_maker_order"></a>

## Function `place_maker_order`



<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_place_maker_order">place_maker_order</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="market_types.md#0x7_market_types_MarketClearinghouseCallbacks">market_types::MarketClearinghouseCallbacks</a>&lt;M&gt;, <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>, order_id: <a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>, is_bid: bool, price: u64, size: u64, order_metadata: M)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_place_maker_order">place_maker_order</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="market_types.md#0x7_market_types_MarketClearinghouseCallbacks">MarketClearinghouseCallbacks</a>&lt;M&gt;,
    <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>,
    order_id: OrderIdType,
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



<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_cleanup_order">cleanup_order</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="market_types.md#0x7_market_types_MarketClearinghouseCallbacks">market_types::MarketClearinghouseCallbacks</a>&lt;M&gt;, <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>, order_id: <a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>, is_bid: bool, remaining_size: u64, order_metadata: M)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_cleanup_order">cleanup_order</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="market_types.md#0x7_market_types_MarketClearinghouseCallbacks">MarketClearinghouseCallbacks</a>&lt;M&gt;,
    <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>,
    order_id: OrderIdType,
    is_bid: bool,
    remaining_size: u64,
    order_metadata: M) {
    (self.cleanup_order_f)(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>, order_id, is_bid, remaining_size, order_metadata)
}
</code></pre>



</details>

<a id="0x7_market_types_cleanup_bulk_orders"></a>

## Function `cleanup_bulk_orders`



<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_cleanup_bulk_orders">cleanup_bulk_orders</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="market_types.md#0x7_market_types_MarketClearinghouseCallbacks">market_types::MarketClearinghouseCallbacks</a>&lt;M&gt;, <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>, is_bid: bool, remaining_sizes: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_cleanup_bulk_orders">cleanup_bulk_orders</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="market_types.md#0x7_market_types_MarketClearinghouseCallbacks">MarketClearinghouseCallbacks</a>&lt;M&gt;,
    <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>,
    is_bid: bool,
    remaining_sizes: u64) {
    (self.cleanup_bulk_orders_f)(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>, is_bid, remaining_sizes)
}
</code></pre>



</details>

<a id="0x7_market_types_decrease_order_size"></a>

## Function `decrease_order_size`



<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_decrease_order_size">decrease_order_size</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="market_types.md#0x7_market_types_MarketClearinghouseCallbacks">market_types::MarketClearinghouseCallbacks</a>&lt;M&gt;, <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>, order_id: <a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>, is_bid: bool, price: u64, size: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_decrease_order_size">decrease_order_size</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="market_types.md#0x7_market_types_MarketClearinghouseCallbacks">MarketClearinghouseCallbacks</a>&lt;M&gt;,
    <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>,
    order_id: OrderIdType,
    is_bid: bool,
    price: u64,
    size: u64) {
    (self.decrease_order_size_f)(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>, order_id, is_bid, price, size)
}
</code></pre>



</details>

<a id="0x7_market_types_get_order_metadata_bytes"></a>

## Function `get_order_metadata_bytes`



<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_get_order_metadata_bytes">get_order_metadata_bytes</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="market_types.md#0x7_market_types_MarketClearinghouseCallbacks">market_types::MarketClearinghouseCallbacks</a>&lt;M&gt;, order_metadata: M): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_get_order_metadata_bytes">get_order_metadata_bytes</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="market_types.md#0x7_market_types_MarketClearinghouseCallbacks">MarketClearinghouseCallbacks</a>&lt;M&gt;,
    order_metadata: M): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    (self.get_order_metadata_bytes)(order_metadata)
}
</code></pre>



</details>

<a id="0x7_market_types_new_market_config"></a>

## Function `new_market_config`



<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_new_market_config">new_market_config</a>(allow_self_matching: bool, allow_events_emission: bool, pre_cancellation_window_secs: u64): <a href="market_types.md#0x7_market_types_MarketConfig">market_types::MarketConfig</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_new_market_config">new_market_config</a>(
    allow_self_matching: bool, allow_events_emission: bool, pre_cancellation_window_secs: u64
): <a href="market_types.md#0x7_market_types_MarketConfig">MarketConfig</a> {
    MarketConfig::V1 {
        allow_self_trade: allow_self_matching,
        allow_events_emission,
        pre_cancellation_window_secs,
    }
}
</code></pre>



</details>

<a id="0x7_market_types_new_market"></a>

## Function `new_market`



<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_new_market">new_market</a>&lt;M: <b>copy</b>, drop, store&gt;(parent: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, market: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, config: <a href="market_types.md#0x7_market_types_MarketConfig">market_types::MarketConfig</a>): <a href="market_types.md#0x7_market_types_Market">market_types::Market</a>&lt;M&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_new_market">new_market</a>&lt;M: store + <b>copy</b> + drop&gt;(
    parent: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, market: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, config: <a href="market_types.md#0x7_market_types_MarketConfig">MarketConfig</a>
): <a href="market_types.md#0x7_market_types_Market">Market</a>&lt;M&gt; {
    // requiring signers, and not addresses, purely <b>to</b> guarantee different dexes
    // cannot polute events <b>to</b> each other, accidentally or maliciously.
    <b>let</b> pre_cancellation_window = config.pre_cancellation_window_secs;
    <b>let</b> <a href="pre_cancellation_tracker.md#0x7_pre_cancellation_tracker">pre_cancellation_tracker</a> = <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_new">table::new</a>();
    <a href="pre_cancellation_tracker.md#0x7_pre_cancellation_tracker">pre_cancellation_tracker</a>.add(
        <a href="market_types.md#0x7_market_types_PRE_CANCELLATION_TRACKER_KEY">PRE_CANCELLATION_TRACKER_KEY</a>,
        new_pre_cancellation_tracker(pre_cancellation_window)
    );
    Market::V1 {
        parent: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(parent),
        market: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(market),
        order_id_generator: new_ascending_id_generator(),
        next_fill_id: 0,
        config,
        <a href="order_book.md#0x7_order_book">order_book</a>: new_order_book(),
        <a href="pre_cancellation_tracker.md#0x7_pre_cancellation_tracker">pre_cancellation_tracker</a>,
    }
}
</code></pre>



</details>

<a id="0x7_market_types_next_order_id"></a>

## Function `next_order_id`



<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_next_order_id">next_order_id</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="market_types.md#0x7_market_types_Market">market_types::Market</a>&lt;M&gt;): <a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_next_order_id">next_order_id</a>&lt;M: store + <b>copy</b> + drop&gt;(self: &<b>mut</b> <a href="market_types.md#0x7_market_types_Market">Market</a>&lt;M&gt;): OrderIdType {
    new_order_id_type(self.order_id_generator.next_ascending_id())
}
</code></pre>



</details>

<a id="0x7_market_types_next_fill_id"></a>

## Function `next_fill_id`



<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_next_fill_id">next_fill_id</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="market_types.md#0x7_market_types_Market">market_types::Market</a>&lt;M&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_next_fill_id">next_fill_id</a>&lt;M: store + <b>copy</b> + drop&gt;(self: &<b>mut</b> <a href="market_types.md#0x7_market_types_Market">Market</a>&lt;M&gt;): u64 {
    <b>let</b> next_fill_id = self.next_fill_id;
    self.next_fill_id += 1;
    next_fill_id
}
</code></pre>



</details>

<a id="0x7_market_types_get_order_book"></a>

## Function `get_order_book`



<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_get_order_book">get_order_book</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="market_types.md#0x7_market_types_Market">market_types::Market</a>&lt;M&gt;): &<a href="order_book.md#0x7_order_book_OrderBook">order_book::OrderBook</a>&lt;M&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_get_order_book">get_order_book</a>&lt;M: store + <b>copy</b> + drop&gt;(self: &<a href="market_types.md#0x7_market_types_Market">Market</a>&lt;M&gt;): &OrderBook&lt;M&gt; {
    &self.<a href="order_book.md#0x7_order_book">order_book</a>
}
</code></pre>



</details>

<a id="0x7_market_types_get_order_book_mut"></a>

## Function `get_order_book_mut`



<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_get_order_book_mut">get_order_book_mut</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="market_types.md#0x7_market_types_Market">market_types::Market</a>&lt;M&gt;): &<b>mut</b> <a href="order_book.md#0x7_order_book_OrderBook">order_book::OrderBook</a>&lt;M&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_get_order_book_mut">get_order_book_mut</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="market_types.md#0x7_market_types_Market">Market</a>&lt;M&gt;
): &<b>mut</b> OrderBook&lt;M&gt; {
    &<b>mut</b> self.<a href="order_book.md#0x7_order_book">order_book</a>
}
</code></pre>



</details>

<a id="0x7_market_types_get_market_address"></a>

## Function `get_market_address`



<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_get_market_address">get_market_address</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="market_types.md#0x7_market_types_Market">market_types::Market</a>&lt;M&gt;): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_get_market_address">get_market_address</a>&lt;M: store + <b>copy</b> + drop&gt;(self: &<a href="market_types.md#0x7_market_types_Market">Market</a>&lt;M&gt;): <b>address</b> {
    self.market
}
</code></pre>



</details>

<a id="0x7_market_types_get_pre_cancellation_tracker_mut"></a>

## Function `get_pre_cancellation_tracker_mut`



<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_get_pre_cancellation_tracker_mut">get_pre_cancellation_tracker_mut</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="market_types.md#0x7_market_types_Market">market_types::Market</a>&lt;M&gt;): &<b>mut</b> <a href="pre_cancellation_tracker.md#0x7_pre_cancellation_tracker_PreCancellationTracker">pre_cancellation_tracker::PreCancellationTracker</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_get_pre_cancellation_tracker_mut">get_pre_cancellation_tracker_mut</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="market_types.md#0x7_market_types_Market">Market</a>&lt;M&gt;
): &<b>mut</b> PreCancellationTracker {
    self.<a href="pre_cancellation_tracker.md#0x7_pre_cancellation_tracker">pre_cancellation_tracker</a>.borrow_mut(<a href="market_types.md#0x7_market_types_PRE_CANCELLATION_TRACKER_KEY">PRE_CANCELLATION_TRACKER_KEY</a>)
}
</code></pre>



</details>

<a id="0x7_market_types_best_bid_price"></a>

## Function `best_bid_price`



<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_best_bid_price">best_bid_price</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="market_types.md#0x7_market_types_Market">market_types::Market</a>&lt;M&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_best_bid_price">best_bid_price</a>&lt;M: store + <b>copy</b> + drop&gt;(self: &<a href="market_types.md#0x7_market_types_Market">Market</a>&lt;M&gt;): Option&lt;u64&gt; {
    self.<a href="order_book.md#0x7_order_book">order_book</a>.<a href="market_types.md#0x7_market_types_best_bid_price">best_bid_price</a>()
}
</code></pre>



</details>

<a id="0x7_market_types_best_ask_price"></a>

## Function `best_ask_price`



<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_best_ask_price">best_ask_price</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="market_types.md#0x7_market_types_Market">market_types::Market</a>&lt;M&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_best_ask_price">best_ask_price</a>&lt;M: store + <b>copy</b> + drop&gt;(self: &<a href="market_types.md#0x7_market_types_Market">Market</a>&lt;M&gt;): Option&lt;u64&gt; {
    self.<a href="order_book.md#0x7_order_book">order_book</a>.<a href="market_types.md#0x7_market_types_best_ask_price">best_ask_price</a>()
}
</code></pre>



</details>

<a id="0x7_market_types_is_taker_order"></a>

## Function `is_taker_order`



<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_is_taker_order">is_taker_order</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="market_types.md#0x7_market_types_Market">market_types::Market</a>&lt;M&gt;, price: u64, is_bid: bool, trigger_condition: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="order_book_types.md#0x7_order_book_types_TriggerCondition">order_book_types::TriggerCondition</a>&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_is_taker_order">is_taker_order</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="market_types.md#0x7_market_types_Market">Market</a>&lt;M&gt;,
    price: u64,
    is_bid: bool,
    trigger_condition: Option&lt;TriggerCondition&gt;
): bool {
    self.<a href="order_book.md#0x7_order_book">order_book</a>.<a href="market_types.md#0x7_market_types_is_taker_order">is_taker_order</a>(price, is_bid, trigger_condition)
}
</code></pre>



</details>

<a id="0x7_market_types_is_allowed_self_trade"></a>

## Function `is_allowed_self_trade`



<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_is_allowed_self_trade">is_allowed_self_trade</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="market_types.md#0x7_market_types_Market">market_types::Market</a>&lt;M&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_is_allowed_self_trade">is_allowed_self_trade</a>&lt;M: store + <b>copy</b> + drop&gt;(self: &<a href="market_types.md#0x7_market_types_Market">Market</a>&lt;M&gt;): bool {
    self.config.allow_self_trade
}
</code></pre>



</details>

<a id="0x7_market_types_get_remaining_size"></a>

## Function `get_remaining_size`

Remaining size of the order in the order book.


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_get_remaining_size">get_remaining_size</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="market_types.md#0x7_market_types_Market">market_types::Market</a>&lt;M&gt;, order_id: <a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_get_remaining_size">get_remaining_size</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="market_types.md#0x7_market_types_Market">Market</a>&lt;M&gt;, order_id: OrderIdType
): u64 {
    self.<a href="order_book.md#0x7_order_book">order_book</a>.<a href="market_types.md#0x7_market_types_get_remaining_size">get_remaining_size</a>(order_id)
}
</code></pre>



</details>

<a id="0x7_market_types_get_bulk_order_remaining_size"></a>

## Function `get_bulk_order_remaining_size`



<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_get_bulk_order_remaining_size">get_bulk_order_remaining_size</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="market_types.md#0x7_market_types_Market">market_types::Market</a>&lt;M&gt;, user: <b>address</b>, is_bid: bool): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_get_bulk_order_remaining_size">get_bulk_order_remaining_size</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="market_types.md#0x7_market_types_Market">Market</a>&lt;M&gt;, user: <b>address</b>, is_bid: bool
): u64 {
    self.<a href="order_book.md#0x7_order_book">order_book</a>.<a href="market_types.md#0x7_market_types_get_bulk_order_remaining_size">get_bulk_order_remaining_size</a>(user, is_bid)
}
</code></pre>



</details>

<a id="0x7_market_types_get_order_metadata"></a>

## Function `get_order_metadata`



<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_get_order_metadata">get_order_metadata</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="market_types.md#0x7_market_types_Market">market_types::Market</a>&lt;M&gt;, order_id: <a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;M&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_get_order_metadata">get_order_metadata</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="market_types.md#0x7_market_types_Market">Market</a>&lt;M&gt;, order_id: OrderIdType
): Option&lt;M&gt; {
    self.<a href="order_book.md#0x7_order_book">order_book</a>.<a href="market_types.md#0x7_market_types_get_order_metadata">get_order_metadata</a>(order_id)
}
</code></pre>



</details>

<a id="0x7_market_types_set_order_metadata"></a>

## Function `set_order_metadata`

Returns the order metadata for an order by order id.
It is up to the caller to perform necessary permissions checks


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_set_order_metadata">set_order_metadata</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="market_types.md#0x7_market_types_Market">market_types::Market</a>&lt;M&gt;, order_id: <a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>, metadata: M)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_set_order_metadata">set_order_metadata</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="market_types.md#0x7_market_types_Market">Market</a>&lt;M&gt;, order_id: OrderIdType, metadata: M
) {
    self.<a href="order_book.md#0x7_order_book">order_book</a>.<a href="market_types.md#0x7_market_types_set_order_metadata">set_order_metadata</a>(order_id, metadata);
}
</code></pre>



</details>

<a id="0x7_market_types_get_order_metadata_by_client_id"></a>

## Function `get_order_metadata_by_client_id`



<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_get_order_metadata_by_client_id">get_order_metadata_by_client_id</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="market_types.md#0x7_market_types_Market">market_types::Market</a>&lt;M&gt;, user: <b>address</b>, client_order_id: u64): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;M&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_get_order_metadata_by_client_id">get_order_metadata_by_client_id</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="market_types.md#0x7_market_types_Market">Market</a>&lt;M&gt;, user: <b>address</b>, client_order_id: u64
): Option&lt;M&gt; {
    <b>let</b> order_id = self.<a href="order_book.md#0x7_order_book">order_book</a>.get_order_id_by_client_id(user, client_order_id);
    <b>if</b> (order_id.is_none()) {
        <b>return</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>();
    };
    <b>return</b> self.<a href="market_types.md#0x7_market_types_get_order_metadata">get_order_metadata</a>(order_id.destroy_some())
}
</code></pre>



</details>

<a id="0x7_market_types_set_order_metadata_by_client_id"></a>

## Function `set_order_metadata_by_client_id`

Sets the order metadata for an order by client id. It is up to the caller to perform necessary permissions checks
around ownership of the order.


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_set_order_metadata_by_client_id">set_order_metadata_by_client_id</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="market_types.md#0x7_market_types_Market">market_types::Market</a>&lt;M&gt;, user: <b>address</b>, client_order_id: u64, metadata: M)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_set_order_metadata_by_client_id">set_order_metadata_by_client_id</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="market_types.md#0x7_market_types_Market">Market</a>&lt;M&gt;, user: <b>address</b>, client_order_id: u64, metadata: M
) {
    <b>let</b> order_id = self.<a href="order_book.md#0x7_order_book">order_book</a>.get_order_id_by_client_id(user, client_order_id);
    <b>assert</b>!(order_id.is_some(), <a href="market_types.md#0x7_market_types_EORDER_DOES_NOT_EXIST">EORDER_DOES_NOT_EXIST</a>);
    self.<a href="market_types.md#0x7_market_types_set_order_metadata">set_order_metadata</a>(order_id.destroy_some(), metadata);
}
</code></pre>



</details>

<a id="0x7_market_types_take_ready_price_based_orders"></a>

## Function `take_ready_price_based_orders`

Returns all the pending order ready to be executed based on the oracle price. The caller is responsible to
call the <code>place_order_with_order_id</code> API to place the order with the order id returned from this API.


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_take_ready_price_based_orders">take_ready_price_based_orders</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="market_types.md#0x7_market_types_Market">market_types::Market</a>&lt;M&gt;, oracle_price: u64, order_limit: u64): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="single_order_types.md#0x7_single_order_types_SingleOrder">single_order_types::SingleOrder</a>&lt;M&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_take_ready_price_based_orders">take_ready_price_based_orders</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="market_types.md#0x7_market_types_Market">Market</a>&lt;M&gt;, oracle_price: u64, order_limit: u64
): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;SingleOrder&lt;M&gt;&gt; {
    self.<a href="order_book.md#0x7_order_book">order_book</a>.<a href="market_types.md#0x7_market_types_take_ready_price_based_orders">take_ready_price_based_orders</a>(oracle_price, order_limit)
}
</code></pre>



</details>

<a id="0x7_market_types_take_ready_time_based_orders"></a>

## Function `take_ready_time_based_orders`

Returns all the pending order that are ready to be executed based on current time stamp. The caller is responsible to
call the <code>place_order_with_order_id</code> API to place the order with the order id returned from this API.


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_take_ready_time_based_orders">take_ready_time_based_orders</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="market_types.md#0x7_market_types_Market">market_types::Market</a>&lt;M&gt;, order_limit: u64): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="single_order_types.md#0x7_single_order_types_SingleOrder">single_order_types::SingleOrder</a>&lt;M&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_take_ready_time_based_orders">take_ready_time_based_orders</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="market_types.md#0x7_market_types_Market">Market</a>&lt;M&gt;, order_limit: u64
): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;SingleOrder&lt;M&gt;&gt; {
    self.<a href="order_book.md#0x7_order_book">order_book</a>.<a href="market_types.md#0x7_market_types_take_ready_time_based_orders">take_ready_time_based_orders</a>(order_limit)
}
</code></pre>



</details>

<a id="0x7_market_types_emit_event_for_order"></a>

## Function `emit_event_for_order`



<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_emit_event_for_order">emit_event_for_order</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="market_types.md#0x7_market_types_Market">market_types::Market</a>&lt;M&gt;, order_id: <a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>, client_order_id: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;, user: <b>address</b>, orig_size: u64, remaining_size: u64, size_delta: u64, price: u64, is_bid: bool, is_taker: bool, status: <a href="market_types.md#0x7_market_types_OrderStatus">market_types::OrderStatus</a>, details: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, metadata: M, trigger_condition: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="order_book_types.md#0x7_order_book_types_TriggerCondition">order_book_types::TriggerCondition</a>&gt;, time_in_force: <a href="order_book_types.md#0x7_order_book_types_TimeInForce">order_book_types::TimeInForce</a>, callbacks: &<a href="market_types.md#0x7_market_types_MarketClearinghouseCallbacks">market_types::MarketClearinghouseCallbacks</a>&lt;M&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_emit_event_for_order">emit_event_for_order</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="market_types.md#0x7_market_types_Market">Market</a>&lt;M&gt;,
    order_id: OrderIdType,
    client_order_id: Option&lt;u64&gt;,
    user: <b>address</b>,
    orig_size: u64,
    remaining_size: u64,
    size_delta: u64,
    price: u64,
    is_bid: bool,
    is_taker: bool,
    status: <a href="market_types.md#0x7_market_types_OrderStatus">OrderStatus</a>,
    details: String,
    metadata: M,
    trigger_condition: Option&lt;TriggerCondition&gt;,
    time_in_force: TimeInForce,
    callbacks: &<a href="market_types.md#0x7_market_types_MarketClearinghouseCallbacks">MarketClearinghouseCallbacks</a>&lt;M&gt;
) {
    // Final check whether <a href="../../aptos-framework/doc/event.md#0x1_event">event</a> sending is enabled
    <b>if</b> (self.config.allow_events_emission) {
        <b>let</b> metadata_bytes =
            callbacks.<a href="market_types.md#0x7_market_types_get_order_metadata_bytes">get_order_metadata_bytes</a>(metadata);
        <a href="../../aptos-framework/doc/event.md#0x1_event_emit">event::emit</a>(
            <a href="market_types.md#0x7_market_types_OrderEvent">OrderEvent</a> {
                parent: self.parent,
                market: self.market,
                order_id: order_id.get_order_id_value(),
                client_order_id,
                user,
                orig_size,
                remaining_size,
                size_delta,
                price,
                is_bid,
                is_taker,
                status,
                details,
                metadata_bytes,
                time_in_force,
                trigger_condition
            }
        );
    };
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
