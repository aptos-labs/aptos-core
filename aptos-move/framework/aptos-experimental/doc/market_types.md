
<a id="0x7_market_types"></a>

# Module `0x7::market_types`



-  [Enum `OrderCancellationReason`](#0x7_market_types_OrderCancellationReason)
-  [Enum `OrderStatus`](#0x7_market_types_OrderStatus)
-  [Enum `CallbackResult`](#0x7_market_types_CallbackResult)
-  [Enum `SettleTradeResult`](#0x7_market_types_SettleTradeResult)
-  [Enum `ValidationResult`](#0x7_market_types_ValidationResult)
-  [Enum `PlaceMakerOrderResult`](#0x7_market_types_PlaceMakerOrderResult)
-  [Enum `MarketClearinghouseCallbacks`](#0x7_market_types_MarketClearinghouseCallbacks)
-  [Enum `Market`](#0x7_market_types_Market)
-  [Enum `MarketConfig`](#0x7_market_types_MarketConfig)
-  [Enum `OrderEvent`](#0x7_market_types_OrderEvent)
-  [Enum `BulkOrderPlacedEvent`](#0x7_market_types_BulkOrderPlacedEvent)
-  [Enum `BulkOrderModifiedEvent`](#0x7_market_types_BulkOrderModifiedEvent)
-  [Enum `BulkOrderFilledEvent`](#0x7_market_types_BulkOrderFilledEvent)
-  [Enum `BulkOrderRejectionEvent`](#0x7_market_types_BulkOrderRejectionEvent)
-  [Constants](#@Constants_0)
-  [Function `order_cancellation_reason_post_only_violation`](#0x7_market_types_order_cancellation_reason_post_only_violation)
-  [Function `order_cancellation_reason_ioc_violation`](#0x7_market_types_order_cancellation_reason_ioc_violation)
-  [Function `order_cancellation_reason_position_update_violation`](#0x7_market_types_order_cancellation_reason_position_update_violation)
-  [Function `order_cancellation_reason_clearinghouse_settle_violation`](#0x7_market_types_order_cancellation_reason_clearinghouse_settle_violation)
-  [Function `order_cancellation_reason_max_fill_limit_violation`](#0x7_market_types_order_cancellation_reason_max_fill_limit_violation)
-  [Function `order_cancellation_reason_duplicate_client_order_id`](#0x7_market_types_order_cancellation_reason_duplicate_client_order_id)
-  [Function `order_cancellation_reason_order_pre_cancelled`](#0x7_market_types_order_cancellation_reason_order_pre_cancelled)
-  [Function `order_cancellation_reason_place_maker_order_violation`](#0x7_market_types_order_cancellation_reason_place_maker_order_violation)
-  [Function `order_cancellation_reason_dead_mans_switch_expired`](#0x7_market_types_order_cancellation_reason_dead_mans_switch_expired)
-  [Function `order_cancellation_reason_disallowed_self_trading`](#0x7_market_types_order_cancellation_reason_disallowed_self_trading)
-  [Function `order_cancellation_reason_cancelled_by_user`](#0x7_market_types_order_cancellation_reason_cancelled_by_user)
-  [Function `order_cancellation_reason_cancelled_by_system`](#0x7_market_types_order_cancellation_reason_cancelled_by_system)
-  [Function `order_cancellation_reason_cancelled_by_system_due_to_error`](#0x7_market_types_order_cancellation_reason_cancelled_by_system_due_to_error)
-  [Function `order_cancellation_reason_clearinghouse_stopped_matching`](#0x7_market_types_order_cancellation_reason_clearinghouse_stopped_matching)
-  [Function `order_status_open`](#0x7_market_types_order_status_open)
-  [Function `order_status_filled`](#0x7_market_types_order_status_filled)
-  [Function `order_status_cancelled`](#0x7_market_types_order_status_cancelled)
-  [Function `order_status_rejected`](#0x7_market_types_order_status_rejected)
-  [Function `order_status_size_reduced`](#0x7_market_types_order_status_size_reduced)
-  [Function `order_status_acknowledged`](#0x7_market_types_order_status_acknowledged)
-  [Function `new_settle_trade_result`](#0x7_market_types_new_settle_trade_result)
-  [Function `new_validation_result`](#0x7_market_types_new_validation_result)
-  [Function `new_place_maker_order_result`](#0x7_market_types_new_place_maker_order_result)
-  [Function `new_market_clearinghouse_callbacks`](#0x7_market_types_new_market_clearinghouse_callbacks)
-  [Function `get_settled_size`](#0x7_market_types_get_settled_size)
-  [Function `get_maker_cancellation_reason`](#0x7_market_types_get_maker_cancellation_reason)
-  [Function `get_taker_cancellation_reason`](#0x7_market_types_get_taker_cancellation_reason)
-  [Function `get_callback_result`](#0x7_market_types_get_callback_result)
-  [Function `is_validation_result_valid`](#0x7_market_types_is_validation_result_valid)
-  [Function `get_validation_failure_reason`](#0x7_market_types_get_validation_failure_reason)
-  [Function `get_place_maker_order_actions`](#0x7_market_types_get_place_maker_order_actions)
-  [Function `get_place_maker_order_cancellation_reason`](#0x7_market_types_get_place_maker_order_cancellation_reason)
-  [Function `extract_results`](#0x7_market_types_extract_results)
-  [Function `should_stop_matching`](#0x7_market_types_should_stop_matching)
-  [Function `new_callback_result_continue_matching`](#0x7_market_types_new_callback_result_continue_matching)
-  [Function `new_callback_result_stop_matching`](#0x7_market_types_new_callback_result_stop_matching)
-  [Function `new_callback_result_not_available`](#0x7_market_types_new_callback_result_not_available)
-  [Function `settle_trade`](#0x7_market_types_settle_trade)
-  [Function `validate_order_placement`](#0x7_market_types_validate_order_placement)
-  [Function `validate_bulk_order_placement`](#0x7_market_types_validate_bulk_order_placement)
-  [Function `place_maker_order`](#0x7_market_types_place_maker_order)
-  [Function `cleanup_order`](#0x7_market_types_cleanup_order)
-  [Function `cleanup_bulk_order_at_price`](#0x7_market_types_cleanup_bulk_order_at_price)
-  [Function `place_bulk_order`](#0x7_market_types_place_bulk_order)
-  [Function `decrease_order_size`](#0x7_market_types_decrease_order_size)
-  [Function `get_order_metadata_bytes`](#0x7_market_types_get_order_metadata_bytes)
-  [Function `new_market_config`](#0x7_market_types_new_market_config)
-  [Function `new_market`](#0x7_market_types_new_market)
-  [Function `set_allow_self_trade`](#0x7_market_types_set_allow_self_trade)
-  [Function `set_allow_events_emission`](#0x7_market_types_set_allow_events_emission)
-  [Function `set_allow_dead_mans_switch`](#0x7_market_types_set_allow_dead_mans_switch)
-  [Function `set_dead_mans_switch_min_keep_alive_time_secs`](#0x7_market_types_set_dead_mans_switch_min_keep_alive_time_secs)
-  [Function `get_order_book`](#0x7_market_types_get_order_book)
-  [Function `get_market_address`](#0x7_market_types_get_market_address)
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
-  [Function `emit_event_for_bulk_order_placed`](#0x7_market_types_emit_event_for_bulk_order_placed)
-  [Function `emit_event_for_bulk_order_cancelled`](#0x7_market_types_emit_event_for_bulk_order_cancelled)
-  [Function `emit_event_for_bulk_order_filled`](#0x7_market_types_emit_event_for_bulk_order_filled)
-  [Function `emit_event_for_bulk_order_modified`](#0x7_market_types_emit_event_for_bulk_order_modified)
-  [Function `emit_event_for_bulk_order_rejection`](#0x7_market_types_emit_event_for_bulk_order_rejection)
-  [Function `get_order_book_mut`](#0x7_market_types_get_order_book_mut)
-  [Function `get_pre_cancellation_tracker_mut`](#0x7_market_types_get_pre_cancellation_tracker_mut)
-  [Function `get_dead_mans_switch_tracker`](#0x7_market_types_get_dead_mans_switch_tracker)
-  [Function `get_dead_mans_switch_tracker_mut`](#0x7_market_types_get_dead_mans_switch_tracker_mut)
-  [Function `is_dead_mans_switch_enabled`](#0x7_market_types_is_dead_mans_switch_enabled)
-  [Function `get_parent`](#0x7_market_types_get_parent)
-  [Function `get_market`](#0x7_market_types_get_market)


<pre><code><b>use</b> <a href="../../aptos-framework/doc/event.md#0x1_event">0x1::event</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table">0x1::table</a>;
<b>use</b> <a href="">0x5::order_book_types</a>;
<b>use</b> <a href="">0x5::single_order_types</a>;
<b>use</b> <a href="dead_mans_switch_tracker.md#0x7_dead_mans_switch_tracker">0x7::dead_mans_switch_tracker</a>;
<b>use</b> <a href="market_clearinghouse_order_info.md#0x7_market_clearinghouse_order_info">0x7::market_clearinghouse_order_info</a>;
<b>use</b> <a href="order_book.md#0x7_order_book">0x7::order_book</a>;
<b>use</b> <a href="pre_cancellation_tracker.md#0x7_pre_cancellation_tracker">0x7::pre_cancellation_tracker</a>;
</code></pre>



<a id="0x7_market_types_OrderCancellationReason"></a>

## Enum `OrderCancellationReason`

Reasons why an order was cancelled


<pre><code>enum <a href="market_types.md#0x7_market_types_OrderCancellationReason">OrderCancellationReason</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>PostOnlyViolation</summary>


<details>
<summary>Fields</summary>


<dl>
</dl>


</details>

</details>

<details>
<summary>IOCViolation</summary>


<details>
<summary>Fields</summary>


<dl>
</dl>


</details>

</details>

<details>
<summary>PositionUpdateViolation</summary>


<details>
<summary>Fields</summary>


<dl>
</dl>


</details>

</details>

<details>
<summary>ReduceOnlyViolation</summary>


<details>
<summary>Fields</summary>


<dl>
</dl>


</details>

</details>

<details>
<summary>ClearinghouseSettleViolation</summary>


<details>
<summary>Fields</summary>


<dl>
</dl>


</details>

</details>

<details>
<summary>MaxFillLimitViolation</summary>


<details>
<summary>Fields</summary>


<dl>
</dl>


</details>

</details>

<details>
<summary>DuplicateClientOrderIdViolation</summary>


<details>
<summary>Fields</summary>


<dl>
</dl>


</details>

</details>

<details>
<summary>OrderPreCancelled</summary>


<details>
<summary>Fields</summary>


<dl>
</dl>


</details>

</details>

<details>
<summary>PlaceMakerOrderViolation</summary>


<details>
<summary>Fields</summary>


<dl>
</dl>


</details>

</details>

<details>
<summary>DeadMansSwitchExpired</summary>


<details>
<summary>Fields</summary>


<dl>
</dl>


</details>

</details>

<details>
<summary>DisallowedSelfTrading</summary>


<details>
<summary>Fields</summary>


<dl>
</dl>


</details>

</details>

<details>
<summary>OrderCancelledByUser</summary>


<details>
<summary>Fields</summary>


<dl>
</dl>


</details>

</details>

<details>
<summary>OrderCancelledBySystem</summary>


<details>
<summary>Fields</summary>


<dl>
</dl>


</details>

</details>

<details>
<summary>OrderCancelledBySystemDueToError</summary>


<details>
<summary>Fields</summary>


<dl>
</dl>


</details>

</details>

<details>
<summary>ClearinghouseStoppedMatching</summary>


<details>
<summary>Fields</summary>


<dl>
</dl>


</details>

</details>

</details>

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

<a id="0x7_market_types_CallbackResult"></a>

## Enum `CallbackResult`



<pre><code>enum <a href="market_types.md#0x7_market_types_CallbackResult">CallbackResult</a>&lt;R: <b>copy</b>, drop, store&gt; <b>has</b> <b>copy</b>, drop
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>NOT_AVAILABLE</summary>


<details>
<summary>Fields</summary>


<dl>
</dl>


</details>

</details>

<details>
<summary>CONTINUE_MATCHING</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>result: R</code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

<details>
<summary>STOP_MATCHING</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>result: R</code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

</details>

<a id="0x7_market_types_SettleTradeResult"></a>

## Enum `SettleTradeResult`



<pre><code>enum <a href="market_types.md#0x7_market_types_SettleTradeResult">SettleTradeResult</a>&lt;R: <b>copy</b>, drop, store&gt; <b>has</b> drop
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
<dt>
<code>callback_result: <a href="market_types.md#0x7_market_types_CallbackResult">market_types::CallbackResult</a>&lt;R&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

</details>

<a id="0x7_market_types_ValidationResult"></a>

## Enum `ValidationResult`



<pre><code>enum <a href="market_types.md#0x7_market_types_ValidationResult">ValidationResult</a> <b>has</b> <b>copy</b>, drop
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>V1</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>failure_reason: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

</details>

<a id="0x7_market_types_PlaceMakerOrderResult"></a>

## Enum `PlaceMakerOrderResult`



<pre><code>enum <a href="market_types.md#0x7_market_types_PlaceMakerOrderResult">PlaceMakerOrderResult</a>&lt;R: <b>copy</b>, drop, store&gt; <b>has</b> <b>copy</b>, drop
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>V1</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>cancellation_reason: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>action: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;R&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

</details>

<a id="0x7_market_types_MarketClearinghouseCallbacks"></a>

## Enum `MarketClearinghouseCallbacks`



<pre><code>enum <a href="market_types.md#0x7_market_types_MarketClearinghouseCallbacks">MarketClearinghouseCallbacks</a>&lt;M: <b>copy</b>, drop, store, R: <b>copy</b>, drop, store&gt; <b>has</b> drop
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>V1</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>settle_trade_f: |&<b>mut</b> <a href="market_types.md#0x7_market_types_Market">market_types::Market</a>&lt;M&gt;, <a href="market_clearinghouse_order_info.md#0x7_market_clearinghouse_order_info_MarketClearinghouseOrderInfo">market_clearinghouse_order_info::MarketClearinghouseOrderInfo</a>&lt;M&gt;, <a href="market_clearinghouse_order_info.md#0x7_market_clearinghouse_order_info_MarketClearinghouseOrderInfo">market_clearinghouse_order_info::MarketClearinghouseOrderInfo</a>&lt;M&gt;, u128, u64, u64|<a href="market_types.md#0x7_market_types_SettleTradeResult">market_types::SettleTradeResult</a>&lt;R&gt; <b>has</b> <b>copy</b> + drop</code>
</dt>
<dd>
 settle_trade_f arguments: market, taker, maker, fill_id, settled_price, settled_size,
</dd>
<dt>
<code>validate_order_placement_f: |<a href="market_clearinghouse_order_info.md#0x7_market_clearinghouse_order_info_MarketClearinghouseOrderInfo">market_clearinghouse_order_info::MarketClearinghouseOrderInfo</a>&lt;M&gt;, u64|<a href="market_types.md#0x7_market_types_ValidationResult">market_types::ValidationResult</a> <b>has</b> <b>copy</b> + drop</code>
</dt>
<dd>
 validate_settlement_update_f arguments: order_info, size
</dd>
<dt>
<code>validate_bulk_order_placement_f: |<b>address</b>, &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, &M|<a href="market_types.md#0x7_market_types_ValidationResult">market_types::ValidationResult</a> <b>has</b> <b>copy</b> + drop</code>
</dt>
<dd>
 Validate the bulk order placement arguments: account, bids_prices, bids_sizes, asks_prices, asks_sizes
</dd>
<dt>
<code>place_maker_order_f: |<a href="market_clearinghouse_order_info.md#0x7_market_clearinghouse_order_info_MarketClearinghouseOrderInfo">market_clearinghouse_order_info::MarketClearinghouseOrderInfo</a>&lt;M&gt;, u64|<a href="market_types.md#0x7_market_types_PlaceMakerOrderResult">market_types::PlaceMakerOrderResult</a>&lt;R&gt; <b>has</b> <b>copy</b> + drop</code>
</dt>
<dd>
 place_maker_order_f arguments: order_info, size
</dd>
<dt>
<code>cleanup_order_f: |<a href="market_clearinghouse_order_info.md#0x7_market_clearinghouse_order_info_MarketClearinghouseOrderInfo">market_clearinghouse_order_info::MarketClearinghouseOrderInfo</a>&lt;M&gt;, u64, bool| <b>has</b> <b>copy</b> + drop</code>
</dt>
<dd>
 cleanup_order_f arguments: order_info, cleanup_size, is_taker
</dd>
<dt>
<code>cleanup_bulk_order_at_price_f: |<b>address</b>, <a href="_OrderId">order_book_types::OrderId</a>, bool, u64, u64| <b>has</b> <b>copy</b> + drop</code>
</dt>
<dd>
 cleanup_bulk_orders_f arguments: account, is_bid, remaining_sizes
</dd>
<dt>
<code>place_bulk_order_f: |<b>address</b>, <a href="_OrderId">order_book_types::OrderId</a>, &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, &M| <b>has</b> <b>copy</b> + drop</code>
</dt>
<dd>
 place_bulk_order_f arguments: account, order_id, bid_prices, bid_sizes, ask_prices, ask_sizes,
 cancelled_bid_prices, cancelled_bid_sizes, cancelled_ask_prices, cancelled_ask_sizes, metadata
</dd>
<dt>
<code>decrease_order_size_f: |<a href="market_clearinghouse_order_info.md#0x7_market_clearinghouse_order_info_MarketClearinghouseOrderInfo">market_clearinghouse_order_info::MarketClearinghouseOrderInfo</a>&lt;M&gt;, u64| <b>has</b> <b>copy</b> + drop</code>
</dt>
<dd>
 decrease_order_size_f arguments: order_info, size
</dd>
<dt>
<code>get_order_metadata_bytes: |&M|<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; <b>has</b> <b>copy</b> + drop</code>
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
<dt>
<code><a href="dead_mans_switch_tracker.md#0x7_dead_mans_switch_tracker">dead_mans_switch_tracker</a>: <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_Table">table::Table</a>&lt;u8, <a href="dead_mans_switch_tracker.md#0x7_dead_mans_switch_tracker_DeadMansSwitchTracker">dead_mans_switch_tracker::DeadMansSwitchTracker</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

</details>

<a id="0x7_market_types_MarketConfig"></a>

## Enum `MarketConfig`



<pre><code>enum <a href="market_types.md#0x7_market_types_MarketConfig">MarketConfig</a> <b>has</b> drop, store
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
 Pre cancellation window in seconds
</dd>
<dt>
<code>enable_dead_mans_switch: bool</code>
</dt>
<dd>
 Enable dead man's switch functionality
</dd>
<dt>
<code>min_keep_alive_time_secs: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

</details>

<a id="0x7_market_types_OrderEvent"></a>

## Enum `OrderEvent`



<pre><code>#[<a href="../../aptos-framework/doc/event.md#0x1_event">event</a>]
enum <a href="market_types.md#0x7_market_types_OrderEvent">OrderEvent</a> <b>has</b> <b>copy</b>, drop, store
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
<code>client_order_id: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;</code>
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
<code>time_in_force: <a href="_TimeInForce">order_book_types::TimeInForce</a></code>
</dt>
<dd>

</dd>
<dt>
<code>trigger_condition: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="_TriggerCondition">order_book_types::TriggerCondition</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>cancellation_reason: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="market_types.md#0x7_market_types_OrderCancellationReason">market_types::OrderCancellationReason</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

</details>

<a id="0x7_market_types_BulkOrderPlacedEvent"></a>

## Enum `BulkOrderPlacedEvent`



<pre><code>#[<a href="../../aptos-framework/doc/event.md#0x1_event">event</a>]
enum <a href="market_types.md#0x7_market_types_BulkOrderPlacedEvent">BulkOrderPlacedEvent</a> <b>has</b> <b>copy</b>, drop, store
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
<code>sequence_number: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>user: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>bid_prices: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>bid_sizes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>ask_prices: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>ask_sizes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>cancelled_bid_prices: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>cancelled_bid_sizes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>cancelled_ask_prices: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>cancelled_ask_sizes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>previous_seq_num: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

</details>

<a id="0x7_market_types_BulkOrderModifiedEvent"></a>

## Enum `BulkOrderModifiedEvent`



<pre><code>#[<a href="../../aptos-framework/doc/event.md#0x1_event">event</a>]
enum <a href="market_types.md#0x7_market_types_BulkOrderModifiedEvent">BulkOrderModifiedEvent</a> <b>has</b> <b>copy</b>, drop, store
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
<code>sequence_number: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>user: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>bid_prices: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>bid_sizes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>ask_prices: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>ask_sizes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>cancelled_bid_prices: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>cancelled_bid_sizes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>cancelled_ask_prices: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>cancelled_ask_sizes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>previous_seq_num: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>cancellation_reason: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="market_types.md#0x7_market_types_OrderCancellationReason">market_types::OrderCancellationReason</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

</details>

<a id="0x7_market_types_BulkOrderFilledEvent"></a>

## Enum `BulkOrderFilledEvent`



<pre><code>#[<a href="../../aptos-framework/doc/event.md#0x1_event">event</a>]
enum <a href="market_types.md#0x7_market_types_BulkOrderFilledEvent">BulkOrderFilledEvent</a> <b>has</b> <b>copy</b>, drop, store
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
<code>sequence_number: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>user: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>filled_size: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>price: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>orig_price: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>is_bid: bool</code>
</dt>
<dd>

</dd>
<dt>
<code>fill_id: u128</code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

</details>

<a id="0x7_market_types_BulkOrderRejectionEvent"></a>

## Enum `BulkOrderRejectionEvent`



<pre><code>#[<a href="../../aptos-framework/doc/event.md#0x1_event">event</a>]
enum <a href="market_types.md#0x7_market_types_BulkOrderRejectionEvent">BulkOrderRejectionEvent</a> <b>has</b> <b>copy</b>, drop, store
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

</dd>
<dt>
<code>market: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>user: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>sequence_number: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>existing_sequence_number: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x7_market_types_EINVALID_TIME_IN_FORCE"></a>



<pre><code><b>const</b> <a href="market_types.md#0x7_market_types_EINVALID_TIME_IN_FORCE">EINVALID_TIME_IN_FORCE</a>: u64 = 3;
</code></pre>



<a id="0x7_market_types_DEAD_MANS_SWITCH_TRACKER_KEY"></a>



<pre><code><b>const</b> <a href="market_types.md#0x7_market_types_DEAD_MANS_SWITCH_TRACKER_KEY">DEAD_MANS_SWITCH_TRACKER_KEY</a>: u8 = 1;
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



<a id="0x7_market_types_order_cancellation_reason_post_only_violation"></a>

## Function `order_cancellation_reason_post_only_violation`



<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_order_cancellation_reason_post_only_violation">order_cancellation_reason_post_only_violation</a>(): <a href="market_types.md#0x7_market_types_OrderCancellationReason">market_types::OrderCancellationReason</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_order_cancellation_reason_post_only_violation">order_cancellation_reason_post_only_violation</a>(): <a href="market_types.md#0x7_market_types_OrderCancellationReason">OrderCancellationReason</a> {
    OrderCancellationReason::PostOnlyViolation
}
</code></pre>



</details>

<a id="0x7_market_types_order_cancellation_reason_ioc_violation"></a>

## Function `order_cancellation_reason_ioc_violation`



<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_order_cancellation_reason_ioc_violation">order_cancellation_reason_ioc_violation</a>(): <a href="market_types.md#0x7_market_types_OrderCancellationReason">market_types::OrderCancellationReason</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_order_cancellation_reason_ioc_violation">order_cancellation_reason_ioc_violation</a>(): <a href="market_types.md#0x7_market_types_OrderCancellationReason">OrderCancellationReason</a> {
    OrderCancellationReason::IOCViolation
}
</code></pre>



</details>

<a id="0x7_market_types_order_cancellation_reason_position_update_violation"></a>

## Function `order_cancellation_reason_position_update_violation`



<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_order_cancellation_reason_position_update_violation">order_cancellation_reason_position_update_violation</a>(): <a href="market_types.md#0x7_market_types_OrderCancellationReason">market_types::OrderCancellationReason</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_order_cancellation_reason_position_update_violation">order_cancellation_reason_position_update_violation</a>()
    : <a href="market_types.md#0x7_market_types_OrderCancellationReason">OrderCancellationReason</a> {
    OrderCancellationReason::PositionUpdateViolation
}
</code></pre>



</details>

<a id="0x7_market_types_order_cancellation_reason_clearinghouse_settle_violation"></a>

## Function `order_cancellation_reason_clearinghouse_settle_violation`



<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_order_cancellation_reason_clearinghouse_settle_violation">order_cancellation_reason_clearinghouse_settle_violation</a>(): <a href="market_types.md#0x7_market_types_OrderCancellationReason">market_types::OrderCancellationReason</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_order_cancellation_reason_clearinghouse_settle_violation">order_cancellation_reason_clearinghouse_settle_violation</a>()
    : <a href="market_types.md#0x7_market_types_OrderCancellationReason">OrderCancellationReason</a> {
    OrderCancellationReason::ClearinghouseSettleViolation
}
</code></pre>



</details>

<a id="0x7_market_types_order_cancellation_reason_max_fill_limit_violation"></a>

## Function `order_cancellation_reason_max_fill_limit_violation`



<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_order_cancellation_reason_max_fill_limit_violation">order_cancellation_reason_max_fill_limit_violation</a>(): <a href="market_types.md#0x7_market_types_OrderCancellationReason">market_types::OrderCancellationReason</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_order_cancellation_reason_max_fill_limit_violation">order_cancellation_reason_max_fill_limit_violation</a>()
    : <a href="market_types.md#0x7_market_types_OrderCancellationReason">OrderCancellationReason</a> {
    OrderCancellationReason::MaxFillLimitViolation
}
</code></pre>



</details>

<a id="0x7_market_types_order_cancellation_reason_duplicate_client_order_id"></a>

## Function `order_cancellation_reason_duplicate_client_order_id`



<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_order_cancellation_reason_duplicate_client_order_id">order_cancellation_reason_duplicate_client_order_id</a>(): <a href="market_types.md#0x7_market_types_OrderCancellationReason">market_types::OrderCancellationReason</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_order_cancellation_reason_duplicate_client_order_id">order_cancellation_reason_duplicate_client_order_id</a>()
    : <a href="market_types.md#0x7_market_types_OrderCancellationReason">OrderCancellationReason</a> {
    OrderCancellationReason::DuplicateClientOrderIdViolation
}
</code></pre>



</details>

<a id="0x7_market_types_order_cancellation_reason_order_pre_cancelled"></a>

## Function `order_cancellation_reason_order_pre_cancelled`



<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_order_cancellation_reason_order_pre_cancelled">order_cancellation_reason_order_pre_cancelled</a>(): <a href="market_types.md#0x7_market_types_OrderCancellationReason">market_types::OrderCancellationReason</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_order_cancellation_reason_order_pre_cancelled">order_cancellation_reason_order_pre_cancelled</a>(): <a href="market_types.md#0x7_market_types_OrderCancellationReason">OrderCancellationReason</a> {
    OrderCancellationReason::OrderPreCancelled
}
</code></pre>



</details>

<a id="0x7_market_types_order_cancellation_reason_place_maker_order_violation"></a>

## Function `order_cancellation_reason_place_maker_order_violation`



<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_order_cancellation_reason_place_maker_order_violation">order_cancellation_reason_place_maker_order_violation</a>(): <a href="market_types.md#0x7_market_types_OrderCancellationReason">market_types::OrderCancellationReason</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_order_cancellation_reason_place_maker_order_violation">order_cancellation_reason_place_maker_order_violation</a>()
    : <a href="market_types.md#0x7_market_types_OrderCancellationReason">OrderCancellationReason</a> {
    OrderCancellationReason::PlaceMakerOrderViolation
}
</code></pre>



</details>

<a id="0x7_market_types_order_cancellation_reason_dead_mans_switch_expired"></a>

## Function `order_cancellation_reason_dead_mans_switch_expired`



<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_order_cancellation_reason_dead_mans_switch_expired">order_cancellation_reason_dead_mans_switch_expired</a>(): <a href="market_types.md#0x7_market_types_OrderCancellationReason">market_types::OrderCancellationReason</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_order_cancellation_reason_dead_mans_switch_expired">order_cancellation_reason_dead_mans_switch_expired</a>()
    : <a href="market_types.md#0x7_market_types_OrderCancellationReason">OrderCancellationReason</a> {
    OrderCancellationReason::DeadMansSwitchExpired
}
</code></pre>



</details>

<a id="0x7_market_types_order_cancellation_reason_disallowed_self_trading"></a>

## Function `order_cancellation_reason_disallowed_self_trading`



<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_order_cancellation_reason_disallowed_self_trading">order_cancellation_reason_disallowed_self_trading</a>(): <a href="market_types.md#0x7_market_types_OrderCancellationReason">market_types::OrderCancellationReason</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_order_cancellation_reason_disallowed_self_trading">order_cancellation_reason_disallowed_self_trading</a>(): <a href="market_types.md#0x7_market_types_OrderCancellationReason">OrderCancellationReason</a> {
    OrderCancellationReason::DisallowedSelfTrading
}
</code></pre>



</details>

<a id="0x7_market_types_order_cancellation_reason_cancelled_by_user"></a>

## Function `order_cancellation_reason_cancelled_by_user`



<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_order_cancellation_reason_cancelled_by_user">order_cancellation_reason_cancelled_by_user</a>(): <a href="market_types.md#0x7_market_types_OrderCancellationReason">market_types::OrderCancellationReason</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_order_cancellation_reason_cancelled_by_user">order_cancellation_reason_cancelled_by_user</a>(): <a href="market_types.md#0x7_market_types_OrderCancellationReason">OrderCancellationReason</a> {
    OrderCancellationReason::OrderCancelledByUser
}
</code></pre>



</details>

<a id="0x7_market_types_order_cancellation_reason_cancelled_by_system"></a>

## Function `order_cancellation_reason_cancelled_by_system`



<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_order_cancellation_reason_cancelled_by_system">order_cancellation_reason_cancelled_by_system</a>(): <a href="market_types.md#0x7_market_types_OrderCancellationReason">market_types::OrderCancellationReason</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_order_cancellation_reason_cancelled_by_system">order_cancellation_reason_cancelled_by_system</a>(): <a href="market_types.md#0x7_market_types_OrderCancellationReason">OrderCancellationReason</a> {
    OrderCancellationReason::OrderCancelledBySystem
}
</code></pre>



</details>

<a id="0x7_market_types_order_cancellation_reason_cancelled_by_system_due_to_error"></a>

## Function `order_cancellation_reason_cancelled_by_system_due_to_error`



<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_order_cancellation_reason_cancelled_by_system_due_to_error">order_cancellation_reason_cancelled_by_system_due_to_error</a>(): <a href="market_types.md#0x7_market_types_OrderCancellationReason">market_types::OrderCancellationReason</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_order_cancellation_reason_cancelled_by_system_due_to_error">order_cancellation_reason_cancelled_by_system_due_to_error</a>()
    : <a href="market_types.md#0x7_market_types_OrderCancellationReason">OrderCancellationReason</a> {
    OrderCancellationReason::OrderCancelledBySystemDueToError
}
</code></pre>



</details>

<a id="0x7_market_types_order_cancellation_reason_clearinghouse_stopped_matching"></a>

## Function `order_cancellation_reason_clearinghouse_stopped_matching`



<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_order_cancellation_reason_clearinghouse_stopped_matching">order_cancellation_reason_clearinghouse_stopped_matching</a>(): <a href="market_types.md#0x7_market_types_OrderCancellationReason">market_types::OrderCancellationReason</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_order_cancellation_reason_clearinghouse_stopped_matching">order_cancellation_reason_clearinghouse_stopped_matching</a>()
    : <a href="market_types.md#0x7_market_types_OrderCancellationReason">OrderCancellationReason</a> {
    OrderCancellationReason::ClearinghouseStoppedMatching
}
</code></pre>



</details>

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



<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_new_settle_trade_result">new_settle_trade_result</a>&lt;R: <b>copy</b>, drop, store&gt;(settled_size: u64, maker_cancellation_reason: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, taker_cancellation_reason: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, callback_result: <a href="market_types.md#0x7_market_types_CallbackResult">market_types::CallbackResult</a>&lt;R&gt;): <a href="market_types.md#0x7_market_types_SettleTradeResult">market_types::SettleTradeResult</a>&lt;R&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_new_settle_trade_result">new_settle_trade_result</a>&lt;R: store + <b>copy</b> + drop&gt;(
    settled_size: u64,
    maker_cancellation_reason: Option&lt;String&gt;,
    taker_cancellation_reason: Option&lt;String&gt;,
    callback_result: <a href="market_types.md#0x7_market_types_CallbackResult">CallbackResult</a>&lt;R&gt;
): <a href="market_types.md#0x7_market_types_SettleTradeResult">SettleTradeResult</a>&lt;R&gt; {
    SettleTradeResult::V1 {
        settled_size,
        maker_cancellation_reason,
        taker_cancellation_reason,
        callback_result
    }
}
</code></pre>



</details>

<a id="0x7_market_types_new_validation_result"></a>

## Function `new_validation_result`



<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_new_validation_result">new_validation_result</a>(cancellation_reason: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;): <a href="market_types.md#0x7_market_types_ValidationResult">market_types::ValidationResult</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_new_validation_result">new_validation_result</a>(
    cancellation_reason: Option&lt;String&gt;
): <a href="market_types.md#0x7_market_types_ValidationResult">ValidationResult</a> {
    ValidationResult::V1 { failure_reason: cancellation_reason }
}
</code></pre>



</details>

<a id="0x7_market_types_new_place_maker_order_result"></a>

## Function `new_place_maker_order_result`



<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_new_place_maker_order_result">new_place_maker_order_result</a>&lt;R: <b>copy</b>, drop, store&gt;(cancellation_reason: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, actions: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;R&gt;): <a href="market_types.md#0x7_market_types_PlaceMakerOrderResult">market_types::PlaceMakerOrderResult</a>&lt;R&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_new_place_maker_order_result">new_place_maker_order_result</a>&lt;R: store + <b>copy</b> + drop&gt;(
    cancellation_reason: Option&lt;String&gt;, actions: Option&lt;R&gt;
): <a href="market_types.md#0x7_market_types_PlaceMakerOrderResult">PlaceMakerOrderResult</a>&lt;R&gt; {
    PlaceMakerOrderResult::V1 { cancellation_reason, action: actions }
}
</code></pre>



</details>

<a id="0x7_market_types_new_market_clearinghouse_callbacks"></a>

## Function `new_market_clearinghouse_callbacks`



<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_new_market_clearinghouse_callbacks">new_market_clearinghouse_callbacks</a>&lt;M: <b>copy</b>, drop, store, R: <b>copy</b>, drop, store&gt;(settle_trade_f: |&<b>mut</b> <a href="market_types.md#0x7_market_types_Market">market_types::Market</a>&lt;M&gt;, <a href="market_clearinghouse_order_info.md#0x7_market_clearinghouse_order_info_MarketClearinghouseOrderInfo">market_clearinghouse_order_info::MarketClearinghouseOrderInfo</a>&lt;M&gt;, <a href="market_clearinghouse_order_info.md#0x7_market_clearinghouse_order_info_MarketClearinghouseOrderInfo">market_clearinghouse_order_info::MarketClearinghouseOrderInfo</a>&lt;M&gt;, u128, u64, u64|<a href="market_types.md#0x7_market_types_SettleTradeResult">market_types::SettleTradeResult</a>&lt;R&gt; <b>has</b> <b>copy</b> + drop, validate_order_placement_f: |<a href="market_clearinghouse_order_info.md#0x7_market_clearinghouse_order_info_MarketClearinghouseOrderInfo">market_clearinghouse_order_info::MarketClearinghouseOrderInfo</a>&lt;M&gt;, u64|<a href="market_types.md#0x7_market_types_ValidationResult">market_types::ValidationResult</a> <b>has</b> <b>copy</b> + drop, validate_bulk_order_placement_f: |<b>address</b>, &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, &M|<a href="market_types.md#0x7_market_types_ValidationResult">market_types::ValidationResult</a> <b>has</b> <b>copy</b> + drop, place_maker_order_f: |<a href="market_clearinghouse_order_info.md#0x7_market_clearinghouse_order_info_MarketClearinghouseOrderInfo">market_clearinghouse_order_info::MarketClearinghouseOrderInfo</a>&lt;M&gt;, u64|<a href="market_types.md#0x7_market_types_PlaceMakerOrderResult">market_types::PlaceMakerOrderResult</a>&lt;R&gt; <b>has</b> <b>copy</b> + drop, cleanup_order_f: |<a href="market_clearinghouse_order_info.md#0x7_market_clearinghouse_order_info_MarketClearinghouseOrderInfo">market_clearinghouse_order_info::MarketClearinghouseOrderInfo</a>&lt;M&gt;, u64, bool| <b>has</b> <b>copy</b> + drop, cleanup_bulk_order_at_price_f: |<b>address</b>, <a href="_OrderId">order_book_types::OrderId</a>, bool, u64, u64| <b>has</b> <b>copy</b> + drop, place_bulk_order_f: |<b>address</b>, <a href="_OrderId">order_book_types::OrderId</a>, &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, &M| <b>has</b> <b>copy</b> + drop, decrease_order_size_f: |<a href="market_clearinghouse_order_info.md#0x7_market_clearinghouse_order_info_MarketClearinghouseOrderInfo">market_clearinghouse_order_info::MarketClearinghouseOrderInfo</a>&lt;M&gt;, u64| <b>has</b> <b>copy</b> + drop, get_order_metadata_bytes: |&M|<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; <b>has</b> <b>copy</b> + drop): <a href="market_types.md#0x7_market_types_MarketClearinghouseCallbacks">market_types::MarketClearinghouseCallbacks</a>&lt;M, R&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_new_market_clearinghouse_callbacks">new_market_clearinghouse_callbacks</a>&lt;M: store + <b>copy</b> + drop, R: store + <b>copy</b> + drop&gt;(
    settle_trade_f: |
        &<b>mut</b> <a href="market_types.md#0x7_market_types_Market">Market</a>&lt;M&gt;,
        MarketClearinghouseOrderInfo&lt;M&gt;,
        MarketClearinghouseOrderInfo&lt;M&gt;,
        u128,
        u64,
        u64
    | <a href="market_types.md#0x7_market_types_SettleTradeResult">SettleTradeResult</a>&lt;R&gt; <b>has</b> drop + <b>copy</b>,
    validate_order_placement_f: |
        MarketClearinghouseOrderInfo&lt;M&gt;,
        u64
    | <a href="market_types.md#0x7_market_types_ValidationResult">ValidationResult</a> <b>has</b> drop + <b>copy</b>,
    validate_bulk_order_placement_f: |
        <b>address</b>,
        &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
        &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
        &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
        &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
        &M
    | <a href="market_types.md#0x7_market_types_ValidationResult">ValidationResult</a> <b>has</b> drop + <b>copy</b>,
    place_maker_order_f: |MarketClearinghouseOrderInfo&lt;M&gt;, u64| <a href="market_types.md#0x7_market_types_PlaceMakerOrderResult">PlaceMakerOrderResult</a>&lt;R&gt; <b>has</b> drop
    + <b>copy</b>,
    cleanup_order_f: |
        MarketClearinghouseOrderInfo&lt;M&gt;,
        u64,
        bool
    | <b>has</b> drop + <b>copy</b>,
    cleanup_bulk_order_at_price_f: |
        <b>address</b>,
        OrderId,
        bool,
        u64,
        u64
    | <b>has</b> drop + <b>copy</b>,
    place_bulk_order_f: |
        <b>address</b>,
        OrderId,
        &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
        &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
        &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
        &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
        &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
        &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
        &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
        &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
        &M
    | <b>has</b> drop + <b>copy</b>,
    decrease_order_size_f: |
        MarketClearinghouseOrderInfo&lt;M&gt;,
        u64
    | <b>has</b> drop + <b>copy</b>,
    get_order_metadata_bytes: |&M| <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; <b>has</b> drop + <b>copy</b>
): <a href="market_types.md#0x7_market_types_MarketClearinghouseCallbacks">MarketClearinghouseCallbacks</a>&lt;M, R&gt; {
    MarketClearinghouseCallbacks::V1 {
        settle_trade_f,
        validate_order_placement_f,
        validate_bulk_order_placement_f,
        place_maker_order_f,
        cleanup_order_f,
        cleanup_bulk_order_at_price_f,
        place_bulk_order_f,
        decrease_order_size_f,
        get_order_metadata_bytes
    }
}
</code></pre>



</details>

<a id="0x7_market_types_get_settled_size"></a>

## Function `get_settled_size`



<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_get_settled_size">get_settled_size</a>&lt;R: <b>copy</b>, drop, store&gt;(self: &<a href="market_types.md#0x7_market_types_SettleTradeResult">market_types::SettleTradeResult</a>&lt;R&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_get_settled_size">get_settled_size</a>&lt;R: store + <b>copy</b> + drop&gt;(
    self: &<a href="market_types.md#0x7_market_types_SettleTradeResult">SettleTradeResult</a>&lt;R&gt;
): u64 {
    self.settled_size
}
</code></pre>



</details>

<a id="0x7_market_types_get_maker_cancellation_reason"></a>

## Function `get_maker_cancellation_reason`



<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_get_maker_cancellation_reason">get_maker_cancellation_reason</a>&lt;R: <b>copy</b>, drop, store&gt;(self: &<a href="market_types.md#0x7_market_types_SettleTradeResult">market_types::SettleTradeResult</a>&lt;R&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_get_maker_cancellation_reason">get_maker_cancellation_reason</a>&lt;R: store + <b>copy</b> + drop&gt;(
    self: &<a href="market_types.md#0x7_market_types_SettleTradeResult">SettleTradeResult</a>&lt;R&gt;
): Option&lt;String&gt; {
    self.maker_cancellation_reason
}
</code></pre>



</details>

<a id="0x7_market_types_get_taker_cancellation_reason"></a>

## Function `get_taker_cancellation_reason`



<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_get_taker_cancellation_reason">get_taker_cancellation_reason</a>&lt;R: <b>copy</b>, drop, store&gt;(self: &<a href="market_types.md#0x7_market_types_SettleTradeResult">market_types::SettleTradeResult</a>&lt;R&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_get_taker_cancellation_reason">get_taker_cancellation_reason</a>&lt;R: store + <b>copy</b> + drop&gt;(
    self: &<a href="market_types.md#0x7_market_types_SettleTradeResult">SettleTradeResult</a>&lt;R&gt;
): Option&lt;String&gt; {
    self.taker_cancellation_reason
}
</code></pre>



</details>

<a id="0x7_market_types_get_callback_result"></a>

## Function `get_callback_result`



<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_get_callback_result">get_callback_result</a>&lt;R: <b>copy</b>, drop, store&gt;(self: &<a href="market_types.md#0x7_market_types_SettleTradeResult">market_types::SettleTradeResult</a>&lt;R&gt;): &<a href="market_types.md#0x7_market_types_CallbackResult">market_types::CallbackResult</a>&lt;R&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_get_callback_result">get_callback_result</a>&lt;R: store + <b>copy</b> + drop&gt;(
    self: &<a href="market_types.md#0x7_market_types_SettleTradeResult">SettleTradeResult</a>&lt;R&gt;
): &<a href="market_types.md#0x7_market_types_CallbackResult">CallbackResult</a>&lt;R&gt; {
    &self.callback_result
}
</code></pre>



</details>

<a id="0x7_market_types_is_validation_result_valid"></a>

## Function `is_validation_result_valid`



<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_is_validation_result_valid">is_validation_result_valid</a>(self: &<a href="market_types.md#0x7_market_types_ValidationResult">market_types::ValidationResult</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_is_validation_result_valid">is_validation_result_valid</a>(self: &<a href="market_types.md#0x7_market_types_ValidationResult">ValidationResult</a>): bool {
    self.failure_reason.is_none()
}
</code></pre>



</details>

<a id="0x7_market_types_get_validation_failure_reason"></a>

## Function `get_validation_failure_reason`



<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_get_validation_failure_reason">get_validation_failure_reason</a>(self: &<a href="market_types.md#0x7_market_types_ValidationResult">market_types::ValidationResult</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_get_validation_failure_reason">get_validation_failure_reason</a>(self: &<a href="market_types.md#0x7_market_types_ValidationResult">ValidationResult</a>): Option&lt;String&gt; {
    self.failure_reason
}
</code></pre>



</details>

<a id="0x7_market_types_get_place_maker_order_actions"></a>

## Function `get_place_maker_order_actions`



<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_get_place_maker_order_actions">get_place_maker_order_actions</a>&lt;R: <b>copy</b>, drop, store&gt;(self: &<a href="market_types.md#0x7_market_types_PlaceMakerOrderResult">market_types::PlaceMakerOrderResult</a>&lt;R&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;R&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_get_place_maker_order_actions">get_place_maker_order_actions</a>&lt;R: store + <b>copy</b> + drop&gt;(
    self: &<a href="market_types.md#0x7_market_types_PlaceMakerOrderResult">PlaceMakerOrderResult</a>&lt;R&gt;
): Option&lt;R&gt; {
    self.action
}
</code></pre>



</details>

<a id="0x7_market_types_get_place_maker_order_cancellation_reason"></a>

## Function `get_place_maker_order_cancellation_reason`



<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_get_place_maker_order_cancellation_reason">get_place_maker_order_cancellation_reason</a>&lt;R: <b>copy</b>, drop, store&gt;(self: &<a href="market_types.md#0x7_market_types_PlaceMakerOrderResult">market_types::PlaceMakerOrderResult</a>&lt;R&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_get_place_maker_order_cancellation_reason">get_place_maker_order_cancellation_reason</a>&lt;R: store + <b>copy</b> + drop&gt;(
    self: &<a href="market_types.md#0x7_market_types_PlaceMakerOrderResult">PlaceMakerOrderResult</a>&lt;R&gt;
): Option&lt;String&gt; {
    self.cancellation_reason
}
</code></pre>



</details>

<a id="0x7_market_types_extract_results"></a>

## Function `extract_results`



<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_extract_results">extract_results</a>&lt;R: <b>copy</b>, drop, store&gt;(self: <a href="market_types.md#0x7_market_types_CallbackResult">market_types::CallbackResult</a>&lt;R&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;R&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_extract_results">extract_results</a>&lt;R: store + <b>copy</b> + drop&gt;(
    self: <a href="market_types.md#0x7_market_types_CallbackResult">CallbackResult</a>&lt;R&gt;
): Option&lt;R&gt; {
    match(self) {
        CallbackResult::NOT_AVAILABLE =&gt; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(),
        CallbackResult::CONTINUE_MATCHING { result } =&gt; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(result),
        CallbackResult::STOP_MATCHING { result } =&gt; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(result)
    }
}
</code></pre>



</details>

<a id="0x7_market_types_should_stop_matching"></a>

## Function `should_stop_matching`



<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_should_stop_matching">should_stop_matching</a>&lt;R: <b>copy</b>, drop, store&gt;(self: &<a href="market_types.md#0x7_market_types_CallbackResult">market_types::CallbackResult</a>&lt;R&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_should_stop_matching">should_stop_matching</a>&lt;R: store + <b>copy</b> + drop&gt;(
    self: &<a href="market_types.md#0x7_market_types_CallbackResult">CallbackResult</a>&lt;R&gt;
): bool {
    self is CallbackResult::STOP_MATCHING
}
</code></pre>



</details>

<a id="0x7_market_types_new_callback_result_continue_matching"></a>

## Function `new_callback_result_continue_matching`



<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_new_callback_result_continue_matching">new_callback_result_continue_matching</a>&lt;R: <b>copy</b>, drop, store&gt;(result: R): <a href="market_types.md#0x7_market_types_CallbackResult">market_types::CallbackResult</a>&lt;R&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_new_callback_result_continue_matching">new_callback_result_continue_matching</a>&lt;R: store + <b>copy</b> + drop&gt;(
    result: R
): <a href="market_types.md#0x7_market_types_CallbackResult">CallbackResult</a>&lt;R&gt; {
    CallbackResult::CONTINUE_MATCHING { result }
}
</code></pre>



</details>

<a id="0x7_market_types_new_callback_result_stop_matching"></a>

## Function `new_callback_result_stop_matching`



<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_new_callback_result_stop_matching">new_callback_result_stop_matching</a>&lt;R: <b>copy</b>, drop, store&gt;(result: R): <a href="market_types.md#0x7_market_types_CallbackResult">market_types::CallbackResult</a>&lt;R&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_new_callback_result_stop_matching">new_callback_result_stop_matching</a>&lt;R: store + <b>copy</b> + drop&gt;(
    result: R
): <a href="market_types.md#0x7_market_types_CallbackResult">CallbackResult</a>&lt;R&gt; {
    CallbackResult::STOP_MATCHING { result }
}
</code></pre>



</details>

<a id="0x7_market_types_new_callback_result_not_available"></a>

## Function `new_callback_result_not_available`



<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_new_callback_result_not_available">new_callback_result_not_available</a>&lt;R: <b>copy</b>, drop, store&gt;(): <a href="market_types.md#0x7_market_types_CallbackResult">market_types::CallbackResult</a>&lt;R&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_new_callback_result_not_available">new_callback_result_not_available</a>&lt;R: store + <b>copy</b> + drop&gt;()
    : <a href="market_types.md#0x7_market_types_CallbackResult">CallbackResult</a>&lt;R&gt; {
    CallbackResult::NOT_AVAILABLE
}
</code></pre>



</details>

<a id="0x7_market_types_settle_trade"></a>

## Function `settle_trade`



<pre><code>#[lint::skip(#[needless_mutable_reference])]
<b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_settle_trade">settle_trade</a>&lt;M: <b>copy</b>, drop, store, R: <b>copy</b>, drop, store&gt;(self: &<a href="market_types.md#0x7_market_types_MarketClearinghouseCallbacks">market_types::MarketClearinghouseCallbacks</a>&lt;M, R&gt;, market: &<b>mut</b> <a href="market_types.md#0x7_market_types_Market">market_types::Market</a>&lt;M&gt;, taker: <a href="market_clearinghouse_order_info.md#0x7_market_clearinghouse_order_info_MarketClearinghouseOrderInfo">market_clearinghouse_order_info::MarketClearinghouseOrderInfo</a>&lt;M&gt;, maker: <a href="market_clearinghouse_order_info.md#0x7_market_clearinghouse_order_info_MarketClearinghouseOrderInfo">market_clearinghouse_order_info::MarketClearinghouseOrderInfo</a>&lt;M&gt;, fill_id: u128, settled_price: u64, settled_size: u64): <a href="market_types.md#0x7_market_types_SettleTradeResult">market_types::SettleTradeResult</a>&lt;R&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_settle_trade">settle_trade</a>&lt;M: store + <b>copy</b> + drop, R: store + <b>copy</b> + drop&gt;(
    self: &<a href="market_types.md#0x7_market_types_MarketClearinghouseCallbacks">MarketClearinghouseCallbacks</a>&lt;M, R&gt;,
    market: &<b>mut</b> <a href="market_types.md#0x7_market_types_Market">Market</a>&lt;M&gt;,
    taker: MarketClearinghouseOrderInfo&lt;M&gt;,
    maker: MarketClearinghouseOrderInfo&lt;M&gt;,
    fill_id: u128,
    settled_price: u64,
    settled_size: u64
): <a href="market_types.md#0x7_market_types_SettleTradeResult">SettleTradeResult</a>&lt;R&gt; {
    (self.settle_trade_f) (market, taker, maker, fill_id, settled_price, settled_size)
}
</code></pre>



</details>

<a id="0x7_market_types_validate_order_placement"></a>

## Function `validate_order_placement`



<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_validate_order_placement">validate_order_placement</a>&lt;M: <b>copy</b>, drop, store, R: <b>copy</b>, drop, store&gt;(self: &<a href="market_types.md#0x7_market_types_MarketClearinghouseCallbacks">market_types::MarketClearinghouseCallbacks</a>&lt;M, R&gt;, order_info: <a href="market_clearinghouse_order_info.md#0x7_market_clearinghouse_order_info_MarketClearinghouseOrderInfo">market_clearinghouse_order_info::MarketClearinghouseOrderInfo</a>&lt;M&gt;, size: u64): <a href="market_types.md#0x7_market_types_ValidationResult">market_types::ValidationResult</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_validate_order_placement">validate_order_placement</a>&lt;M: store + <b>copy</b> + drop, R: store + <b>copy</b> + drop&gt;(
    self: &<a href="market_types.md#0x7_market_types_MarketClearinghouseCallbacks">MarketClearinghouseCallbacks</a>&lt;M, R&gt;,
    order_info: MarketClearinghouseOrderInfo&lt;M&gt;,
    size: u64
): <a href="market_types.md#0x7_market_types_ValidationResult">ValidationResult</a> {
    (self.validate_order_placement_f) (order_info, size)
}
</code></pre>



</details>

<a id="0x7_market_types_validate_bulk_order_placement"></a>

## Function `validate_bulk_order_placement`



<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_validate_bulk_order_placement">validate_bulk_order_placement</a>&lt;M: <b>copy</b>, drop, store, R: <b>copy</b>, drop, store&gt;(self: &<a href="market_types.md#0x7_market_types_MarketClearinghouseCallbacks">market_types::MarketClearinghouseCallbacks</a>&lt;M, R&gt;, <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>, bids_prices: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, bids_sizes: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, asks_prices: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, asks_sizes: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, order_metadata: &M): <a href="market_types.md#0x7_market_types_ValidationResult">market_types::ValidationResult</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_validate_bulk_order_placement">validate_bulk_order_placement</a>&lt;M: store + <b>copy</b> + drop, R: store + <b>copy</b> + drop&gt;(
    self: &<a href="market_types.md#0x7_market_types_MarketClearinghouseCallbacks">MarketClearinghouseCallbacks</a>&lt;M, R&gt;,
    <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>,
    bids_prices: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    bids_sizes: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    asks_prices: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    asks_sizes: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    order_metadata: &M
): <a href="market_types.md#0x7_market_types_ValidationResult">ValidationResult</a> {
    (self.validate_bulk_order_placement_f) (
        <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>, bids_prices, bids_sizes, asks_prices, asks_sizes, order_metadata
    )
}
</code></pre>



</details>

<a id="0x7_market_types_place_maker_order"></a>

## Function `place_maker_order`



<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_place_maker_order">place_maker_order</a>&lt;M: <b>copy</b>, drop, store, R: <b>copy</b>, drop, store&gt;(self: &<a href="market_types.md#0x7_market_types_MarketClearinghouseCallbacks">market_types::MarketClearinghouseCallbacks</a>&lt;M, R&gt;, order_info: <a href="market_clearinghouse_order_info.md#0x7_market_clearinghouse_order_info_MarketClearinghouseOrderInfo">market_clearinghouse_order_info::MarketClearinghouseOrderInfo</a>&lt;M&gt;, size: u64): <a href="market_types.md#0x7_market_types_PlaceMakerOrderResult">market_types::PlaceMakerOrderResult</a>&lt;R&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_place_maker_order">place_maker_order</a>&lt;M: store + <b>copy</b> + drop, R: store + <b>copy</b> + drop&gt;(
    self: &<a href="market_types.md#0x7_market_types_MarketClearinghouseCallbacks">MarketClearinghouseCallbacks</a>&lt;M, R&gt;,
    order_info: MarketClearinghouseOrderInfo&lt;M&gt;,
    size: u64
): <a href="market_types.md#0x7_market_types_PlaceMakerOrderResult">PlaceMakerOrderResult</a>&lt;R&gt; {
    (self.place_maker_order_f) (order_info, size)
}
</code></pre>



</details>

<a id="0x7_market_types_cleanup_order"></a>

## Function `cleanup_order`



<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_cleanup_order">cleanup_order</a>&lt;M: <b>copy</b>, drop, store, R: <b>copy</b>, drop, store&gt;(self: &<a href="market_types.md#0x7_market_types_MarketClearinghouseCallbacks">market_types::MarketClearinghouseCallbacks</a>&lt;M, R&gt;, order_info: <a href="market_clearinghouse_order_info.md#0x7_market_clearinghouse_order_info_MarketClearinghouseOrderInfo">market_clearinghouse_order_info::MarketClearinghouseOrderInfo</a>&lt;M&gt;, cleanup_size: u64, is_taker: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_cleanup_order">cleanup_order</a>&lt;M: store + <b>copy</b> + drop, R: store + <b>copy</b> + drop&gt;(
    self: &<a href="market_types.md#0x7_market_types_MarketClearinghouseCallbacks">MarketClearinghouseCallbacks</a>&lt;M, R&gt;,
    order_info: MarketClearinghouseOrderInfo&lt;M&gt;,
    cleanup_size: u64,
    is_taker: bool
) {
    (self.cleanup_order_f) (order_info, cleanup_size, is_taker)
}
</code></pre>



</details>

<a id="0x7_market_types_cleanup_bulk_order_at_price"></a>

## Function `cleanup_bulk_order_at_price`



<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_cleanup_bulk_order_at_price">cleanup_bulk_order_at_price</a>&lt;M: <b>copy</b>, drop, store, R: <b>copy</b>, drop, store&gt;(self: &<a href="market_types.md#0x7_market_types_MarketClearinghouseCallbacks">market_types::MarketClearinghouseCallbacks</a>&lt;M, R&gt;, <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>, order_id: <a href="_OrderId">order_book_types::OrderId</a>, is_bid: bool, price: u64, cleanup_size: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_cleanup_bulk_order_at_price">cleanup_bulk_order_at_price</a>&lt;M: store + <b>copy</b> + drop, R: store + <b>copy</b> + drop&gt;(
    self: &<a href="market_types.md#0x7_market_types_MarketClearinghouseCallbacks">MarketClearinghouseCallbacks</a>&lt;M, R&gt;,
    <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>,
    order_id: OrderId,
    is_bid: bool,
    price: u64,
    cleanup_size: u64
) {
    (self.cleanup_bulk_order_at_price_f) (
        <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>, order_id, is_bid, price, cleanup_size
    )
}
</code></pre>



</details>

<a id="0x7_market_types_place_bulk_order"></a>

## Function `place_bulk_order`



<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_place_bulk_order">place_bulk_order</a>&lt;M: <b>copy</b>, drop, store, R: <b>copy</b>, drop, store&gt;(self: &<a href="market_types.md#0x7_market_types_MarketClearinghouseCallbacks">market_types::MarketClearinghouseCallbacks</a>&lt;M, R&gt;, <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>, order_id: <a href="_OrderId">order_book_types::OrderId</a>, bid_prices: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, bid_sizes: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, ask_prices: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, ask_sizes: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, cancelled_bid_prices: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, cancelled_bid_sizes: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, cancelled_ask_prices: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, cancelled_ask_sizes: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, metadata: &M)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_place_bulk_order">place_bulk_order</a>&lt;M: store + <b>copy</b> + drop, R: store + <b>copy</b> + drop&gt;(
    self: &<a href="market_types.md#0x7_market_types_MarketClearinghouseCallbacks">MarketClearinghouseCallbacks</a>&lt;M, R&gt;,
    <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>,
    order_id: OrderId,
    bid_prices: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    bid_sizes: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    ask_prices: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    ask_sizes: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    cancelled_bid_prices: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    cancelled_bid_sizes: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    cancelled_ask_prices: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    cancelled_ask_sizes: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    metadata: &M
) {
    (self.place_bulk_order_f) (
        <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>,
        order_id,
        bid_prices,
        bid_sizes,
        ask_prices,
        ask_sizes,
        cancelled_bid_prices,
        cancelled_bid_sizes,
        cancelled_ask_prices,
        cancelled_ask_sizes,
        metadata
    )
}
</code></pre>



</details>

<a id="0x7_market_types_decrease_order_size"></a>

## Function `decrease_order_size`



<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_decrease_order_size">decrease_order_size</a>&lt;M: <b>copy</b>, drop, store, R: <b>copy</b>, drop, store&gt;(self: &<a href="market_types.md#0x7_market_types_MarketClearinghouseCallbacks">market_types::MarketClearinghouseCallbacks</a>&lt;M, R&gt;, order_info: <a href="market_clearinghouse_order_info.md#0x7_market_clearinghouse_order_info_MarketClearinghouseOrderInfo">market_clearinghouse_order_info::MarketClearinghouseOrderInfo</a>&lt;M&gt;, new_size: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_decrease_order_size">decrease_order_size</a>&lt;M: store + <b>copy</b> + drop, R: store + <b>copy</b> + drop&gt;(
    self: &<a href="market_types.md#0x7_market_types_MarketClearinghouseCallbacks">MarketClearinghouseCallbacks</a>&lt;M, R&gt;,
    order_info: MarketClearinghouseOrderInfo&lt;M&gt;,
    new_size: u64
) {
    (self.decrease_order_size_f) (order_info, new_size)
}
</code></pre>



</details>

<a id="0x7_market_types_get_order_metadata_bytes"></a>

## Function `get_order_metadata_bytes`



<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_get_order_metadata_bytes">get_order_metadata_bytes</a>&lt;M: <b>copy</b>, drop, store, R: <b>copy</b>, drop, store&gt;(self: &<a href="market_types.md#0x7_market_types_MarketClearinghouseCallbacks">market_types::MarketClearinghouseCallbacks</a>&lt;M, R&gt;, order_metadata: &M): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_get_order_metadata_bytes">get_order_metadata_bytes</a>&lt;M: store + <b>copy</b> + drop, R: store + <b>copy</b> + drop&gt;(
    self: &<a href="market_types.md#0x7_market_types_MarketClearinghouseCallbacks">MarketClearinghouseCallbacks</a>&lt;M, R&gt;,
    order_metadata: &M
): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    (self.get_order_metadata_bytes) (order_metadata)
}
</code></pre>



</details>

<a id="0x7_market_types_new_market_config"></a>

## Function `new_market_config`



<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_new_market_config">new_market_config</a>(allow_self_matching: bool, allow_events_emission: bool, pre_cancellation_window_secs: u64, enable_dead_mans_switch: bool, min_keep_alive_time_secs: u64): <a href="market_types.md#0x7_market_types_MarketConfig">market_types::MarketConfig</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_new_market_config">new_market_config</a>(
    allow_self_matching: bool,
    allow_events_emission: bool,
    pre_cancellation_window_secs: u64,
    enable_dead_mans_switch: bool,
    min_keep_alive_time_secs: u64
): <a href="market_types.md#0x7_market_types_MarketConfig">MarketConfig</a> {
    MarketConfig::V1 {
        allow_self_trade: allow_self_matching,
        allow_events_emission,
        pre_cancellation_window_secs,
        enable_dead_mans_switch,
        min_keep_alive_time_secs
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
    <b>let</b> <a href="dead_mans_switch_tracker.md#0x7_dead_mans_switch_tracker">dead_mans_switch_tracker</a> = <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_new">table::new</a>();
    <a href="dead_mans_switch_tracker.md#0x7_dead_mans_switch_tracker">dead_mans_switch_tracker</a>.add(
        <a href="market_types.md#0x7_market_types_DEAD_MANS_SWITCH_TRACKER_KEY">DEAD_MANS_SWITCH_TRACKER_KEY</a>,
        new_dead_mans_switch_tracker(config.min_keep_alive_time_secs)
    );
    Market::V1 {
        parent: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(parent),
        market: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(market),
        config,
        <a href="order_book.md#0x7_order_book">order_book</a>: new_order_book(),
        <a href="pre_cancellation_tracker.md#0x7_pre_cancellation_tracker">pre_cancellation_tracker</a>,
        <a href="dead_mans_switch_tracker.md#0x7_dead_mans_switch_tracker">dead_mans_switch_tracker</a>
    }
}
</code></pre>



</details>

<a id="0x7_market_types_set_allow_self_trade"></a>

## Function `set_allow_self_trade`



<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_set_allow_self_trade">set_allow_self_trade</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="market_types.md#0x7_market_types_Market">market_types::Market</a>&lt;M&gt;, allow_self_trade: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_set_allow_self_trade">set_allow_self_trade</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="market_types.md#0x7_market_types_Market">Market</a>&lt;M&gt;, allow_self_trade: bool
) {
    self.config.allow_self_trade = allow_self_trade;
}
</code></pre>



</details>

<a id="0x7_market_types_set_allow_events_emission"></a>

## Function `set_allow_events_emission`



<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_set_allow_events_emission">set_allow_events_emission</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="market_types.md#0x7_market_types_Market">market_types::Market</a>&lt;M&gt;, allow_events_emission: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_set_allow_events_emission">set_allow_events_emission</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="market_types.md#0x7_market_types_Market">Market</a>&lt;M&gt;, allow_events_emission: bool
) {
    self.config.allow_events_emission = allow_events_emission;
}
</code></pre>



</details>

<a id="0x7_market_types_set_allow_dead_mans_switch"></a>

## Function `set_allow_dead_mans_switch`



<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_set_allow_dead_mans_switch">set_allow_dead_mans_switch</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="market_types.md#0x7_market_types_Market">market_types::Market</a>&lt;M&gt;, enable_dead_mans_switch: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_set_allow_dead_mans_switch">set_allow_dead_mans_switch</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="market_types.md#0x7_market_types_Market">Market</a>&lt;M&gt;, enable_dead_mans_switch: bool
) {
    self.config.enable_dead_mans_switch = enable_dead_mans_switch;
}
</code></pre>



</details>

<a id="0x7_market_types_set_dead_mans_switch_min_keep_alive_time_secs"></a>

## Function `set_dead_mans_switch_min_keep_alive_time_secs`



<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_set_dead_mans_switch_min_keep_alive_time_secs">set_dead_mans_switch_min_keep_alive_time_secs</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="market_types.md#0x7_market_types_Market">market_types::Market</a>&lt;M&gt;, min_keep_alive_time_secs: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_set_dead_mans_switch_min_keep_alive_time_secs">set_dead_mans_switch_min_keep_alive_time_secs</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="market_types.md#0x7_market_types_Market">Market</a>&lt;M&gt;, min_keep_alive_time_secs: u64
) {
    self.config.min_keep_alive_time_secs = min_keep_alive_time_secs;
    <b>let</b> parent = self.parent;
    <b>let</b> market = self.market;
    <a href="dead_mans_switch_tracker.md#0x7_dead_mans_switch_tracker_set_min_keep_alive_time_secs">dead_mans_switch_tracker::set_min_keep_alive_time_secs</a>(
        self.<a href="market_types.md#0x7_market_types_get_dead_mans_switch_tracker_mut">get_dead_mans_switch_tracker_mut</a>(),
        parent,
        market,
        min_keep_alive_time_secs
    );
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



<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_is_taker_order">is_taker_order</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="market_types.md#0x7_market_types_Market">market_types::Market</a>&lt;M&gt;, price: u64, is_bid: bool, trigger_condition: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="_TriggerCondition">order_book_types::TriggerCondition</a>&gt;): bool
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


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_get_remaining_size">get_remaining_size</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="market_types.md#0x7_market_types_Market">market_types::Market</a>&lt;M&gt;, order_id: <a href="_OrderId">order_book_types::OrderId</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_get_remaining_size">get_remaining_size</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="market_types.md#0x7_market_types_Market">Market</a>&lt;M&gt;, order_id: OrderId
): u64 {
    self.<a href="order_book.md#0x7_order_book">order_book</a>.get_single_remaining_size(order_id)
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



<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_get_order_metadata">get_order_metadata</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="market_types.md#0x7_market_types_Market">market_types::Market</a>&lt;M&gt;, order_id: <a href="_OrderId">order_book_types::OrderId</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;M&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_get_order_metadata">get_order_metadata</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="market_types.md#0x7_market_types_Market">Market</a>&lt;M&gt;, order_id: OrderId
): Option&lt;M&gt; {
    self.<a href="order_book.md#0x7_order_book">order_book</a>.get_single_order_metadata(order_id)
}
</code></pre>



</details>

<a id="0x7_market_types_set_order_metadata"></a>

## Function `set_order_metadata`

Returns the order metadata for an order by order id.
It is up to the caller to perform necessary permissions checks


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_set_order_metadata">set_order_metadata</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="market_types.md#0x7_market_types_Market">market_types::Market</a>&lt;M&gt;, order_id: <a href="_OrderId">order_book_types::OrderId</a>, metadata: M)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_set_order_metadata">set_order_metadata</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="market_types.md#0x7_market_types_Market">Market</a>&lt;M&gt;, order_id: OrderId, metadata: M
) {
    self.<a href="order_book.md#0x7_order_book">order_book</a>.set_single_order_metadata(order_id, metadata);
}
</code></pre>



</details>

<a id="0x7_market_types_get_order_metadata_by_client_id"></a>

## Function `get_order_metadata_by_client_id`



<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_get_order_metadata_by_client_id">get_order_metadata_by_client_id</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="market_types.md#0x7_market_types_Market">market_types::Market</a>&lt;M&gt;, user: <b>address</b>, client_order_id: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;M&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_get_order_metadata_by_client_id">get_order_metadata_by_client_id</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="market_types.md#0x7_market_types_Market">Market</a>&lt;M&gt;, user: <b>address</b>, client_order_id: String
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


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_set_order_metadata_by_client_id">set_order_metadata_by_client_id</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="market_types.md#0x7_market_types_Market">market_types::Market</a>&lt;M&gt;, user: <b>address</b>, client_order_id: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, metadata: M)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_set_order_metadata_by_client_id">set_order_metadata_by_client_id</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="market_types.md#0x7_market_types_Market">Market</a>&lt;M&gt;,
    user: <b>address</b>,
    client_order_id: String,
    metadata: M
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


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_take_ready_price_based_orders">take_ready_price_based_orders</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="market_types.md#0x7_market_types_Market">market_types::Market</a>&lt;M&gt;, oracle_price: u64, order_limit: u64): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="_SingleOrder">single_order_types::SingleOrder</a>&lt;M&gt;&gt;
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


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_take_ready_time_based_orders">take_ready_time_based_orders</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="market_types.md#0x7_market_types_Market">market_types::Market</a>&lt;M&gt;, order_limit: u64): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="_SingleOrder">single_order_types::SingleOrder</a>&lt;M&gt;&gt;
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



<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_emit_event_for_order">emit_event_for_order</a>&lt;M: <b>copy</b>, drop, store, R: <b>copy</b>, drop, store&gt;(self: &<a href="market_types.md#0x7_market_types_Market">market_types::Market</a>&lt;M&gt;, order_id: <a href="_OrderId">order_book_types::OrderId</a>, client_order_id: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, user: <b>address</b>, orig_size: u64, remaining_size: u64, size_delta: u64, price: u64, is_bid: bool, is_taker: bool, status: <a href="market_types.md#0x7_market_types_OrderStatus">market_types::OrderStatus</a>, details: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, metadata: M, trigger_condition: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="_TriggerCondition">order_book_types::TriggerCondition</a>&gt;, time_in_force: <a href="_TimeInForce">order_book_types::TimeInForce</a>, cancellation_reason: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="market_types.md#0x7_market_types_OrderCancellationReason">market_types::OrderCancellationReason</a>&gt;, callbacks: &<a href="market_types.md#0x7_market_types_MarketClearinghouseCallbacks">market_types::MarketClearinghouseCallbacks</a>&lt;M, R&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_emit_event_for_order">emit_event_for_order</a>&lt;M: store + <b>copy</b> + drop, R: store + <b>copy</b> + drop&gt;(
    self: &<a href="market_types.md#0x7_market_types_Market">Market</a>&lt;M&gt;,
    order_id: OrderId,
    client_order_id: Option&lt;String&gt;,
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
    cancellation_reason: Option&lt;<a href="market_types.md#0x7_market_types_OrderCancellationReason">OrderCancellationReason</a>&gt;,
    callbacks: &<a href="market_types.md#0x7_market_types_MarketClearinghouseCallbacks">MarketClearinghouseCallbacks</a>&lt;M, R&gt;
) {
    // Final check whether <a href="../../aptos-framework/doc/event.md#0x1_event">event</a> sending is enabled
    <b>if</b> (self.config.allow_events_emission) {
        <b>let</b> metadata_bytes = callbacks.<a href="market_types.md#0x7_market_types_get_order_metadata_bytes">get_order_metadata_bytes</a>(&metadata);
        <a href="../../aptos-framework/doc/event.md#0x1_event_emit">event::emit</a>(
            OrderEvent::V1 {
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
                trigger_condition,
                cancellation_reason
            }
        );
    };
}
</code></pre>



</details>

<a id="0x7_market_types_emit_event_for_bulk_order_placed"></a>

## Function `emit_event_for_bulk_order_placed`



<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_emit_event_for_bulk_order_placed">emit_event_for_bulk_order_placed</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="market_types.md#0x7_market_types_Market">market_types::Market</a>&lt;M&gt;, order_id: <a href="_OrderId">order_book_types::OrderId</a>, sequence_number: u64, user: <b>address</b>, bid_prices: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, bid_sizes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, ask_prices: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, ask_sizes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, cancelled_bid_prices: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, cancelled_bid_sizes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, cancelled_ask_prices: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, cancelled_ask_sizes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, previous_seq_num: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_emit_event_for_bulk_order_placed">emit_event_for_bulk_order_placed</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="market_types.md#0x7_market_types_Market">Market</a>&lt;M&gt;,
    order_id: OrderId,
    sequence_number: u64,
    user: <b>address</b>,
    bid_prices: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    bid_sizes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    ask_prices: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    ask_sizes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    cancelled_bid_prices: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    cancelled_bid_sizes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    cancelled_ask_prices: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    cancelled_ask_sizes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    previous_seq_num: u64
) {
    // Final check whether <a href="../../aptos-framework/doc/event.md#0x1_event">event</a> sending is enabled
    <b>if</b> (self.config.allow_events_emission) {
        <a href="../../aptos-framework/doc/event.md#0x1_event_emit">event::emit</a>(
            BulkOrderPlacedEvent::V1 {
                parent: self.parent,
                market: self.market,
                order_id: order_id.get_order_id_value(),
                sequence_number,
                user,
                bid_prices,
                bid_sizes,
                ask_prices,
                ask_sizes,
                cancelled_bid_prices,
                cancelled_bid_sizes,
                cancelled_ask_prices,
                cancelled_ask_sizes,
                previous_seq_num
            }
        );
    };
}
</code></pre>



</details>

<a id="0x7_market_types_emit_event_for_bulk_order_cancelled"></a>

## Function `emit_event_for_bulk_order_cancelled`



<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_emit_event_for_bulk_order_cancelled">emit_event_for_bulk_order_cancelled</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="market_types.md#0x7_market_types_Market">market_types::Market</a>&lt;M&gt;, order_id: <a href="_OrderId">order_book_types::OrderId</a>, sequence_number: u64, user: <b>address</b>, cancelled_bid_prices: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, cancelled_bid_sizes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, cancelled_ask_prices: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, cancelled_ask_sizes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, cancellation_reason: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="market_types.md#0x7_market_types_OrderCancellationReason">market_types::OrderCancellationReason</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_emit_event_for_bulk_order_cancelled">emit_event_for_bulk_order_cancelled</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="market_types.md#0x7_market_types_Market">Market</a>&lt;M&gt;,
    order_id: OrderId,
    sequence_number: u64,
    user: <b>address</b>,
    cancelled_bid_prices: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    cancelled_bid_sizes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    cancelled_ask_prices: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    cancelled_ask_sizes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    cancellation_reason: Option&lt;<a href="market_types.md#0x7_market_types_OrderCancellationReason">OrderCancellationReason</a>&gt;
) {
    // Final check whether <a href="../../aptos-framework/doc/event.md#0x1_event">event</a> sending is enabled
    <b>if</b> (self.config.allow_events_emission) {
        <a href="../../aptos-framework/doc/event.md#0x1_event_emit">event::emit</a>(
            BulkOrderModifiedEvent::V1 {
                parent: self.parent,
                market: self.market,
                order_id: order_id.get_order_id_value(),
                sequence_number,
                user,
                bid_prices: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[],
                bid_sizes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[],
                ask_prices: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[],
                ask_sizes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[],
                cancelled_bid_prices,
                cancelled_bid_sizes,
                cancelled_ask_prices,
                cancelled_ask_sizes,
                previous_seq_num: sequence_number,
                cancellation_reason
            }
        )
    };
}
</code></pre>



</details>

<a id="0x7_market_types_emit_event_for_bulk_order_filled"></a>

## Function `emit_event_for_bulk_order_filled`



<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_emit_event_for_bulk_order_filled">emit_event_for_bulk_order_filled</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="market_types.md#0x7_market_types_Market">market_types::Market</a>&lt;M&gt;, order_id: <a href="_OrderId">order_book_types::OrderId</a>, sequence_number: u64, user: <b>address</b>, filled_size: u64, price: u64, orig_price: u64, is_bid: bool, fill_id: u128)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_emit_event_for_bulk_order_filled">emit_event_for_bulk_order_filled</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="market_types.md#0x7_market_types_Market">Market</a>&lt;M&gt;,
    order_id: OrderId,
    sequence_number: u64,
    user: <b>address</b>,
    filled_size: u64,
    price: u64,
    orig_price: u64,
    is_bid: bool,
    fill_id: u128
) {
    // Final check whether <a href="../../aptos-framework/doc/event.md#0x1_event">event</a> sending is enabled
    <b>if</b> (self.config.allow_events_emission) {
        <a href="../../aptos-framework/doc/event.md#0x1_event_emit">event::emit</a>(
            BulkOrderFilledEvent::V1 {
                parent: self.parent,
                market: self.market,
                order_id: order_id.get_order_id_value(),
                sequence_number,
                user,
                filled_size,
                price,
                orig_price,
                is_bid,
                fill_id
            }
        );
    };
}
</code></pre>



</details>

<a id="0x7_market_types_emit_event_for_bulk_order_modified"></a>

## Function `emit_event_for_bulk_order_modified`



<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_emit_event_for_bulk_order_modified">emit_event_for_bulk_order_modified</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="market_types.md#0x7_market_types_Market">market_types::Market</a>&lt;M&gt;, order_id: <a href="_OrderId">order_book_types::OrderId</a>, sequence_number: u64, user: <b>address</b>, bid_prices: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, bid_sizes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, ask_prices: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, ask_sizes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, cancelled_bid_prices: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, cancelled_bid_sizes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, cancelled_ask_prices: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, cancelled_ask_sizes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, cancellation_reason: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="market_types.md#0x7_market_types_OrderCancellationReason">market_types::OrderCancellationReason</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_emit_event_for_bulk_order_modified">emit_event_for_bulk_order_modified</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="market_types.md#0x7_market_types_Market">Market</a>&lt;M&gt;,
    order_id: OrderId,
    sequence_number: u64,
    user: <b>address</b>,
    bid_prices: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    bid_sizes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    ask_prices: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    ask_sizes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    cancelled_bid_prices: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    cancelled_bid_sizes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    cancelled_ask_prices: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    cancelled_ask_sizes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    cancellation_reason: Option&lt;<a href="market_types.md#0x7_market_types_OrderCancellationReason">OrderCancellationReason</a>&gt;
) {
    // Final check whether <a href="../../aptos-framework/doc/event.md#0x1_event">event</a> sending is enabled
    <b>if</b> (self.config.allow_events_emission) {
        <a href="../../aptos-framework/doc/event.md#0x1_event_emit">event::emit</a>(
            BulkOrderModifiedEvent::V1 {
                parent: self.parent,
                market: self.market,
                order_id: order_id.get_order_id_value(),
                sequence_number,
                user,
                bid_prices,
                bid_sizes,
                ask_prices,
                ask_sizes,
                cancelled_bid_prices,
                cancelled_bid_sizes,
                cancelled_ask_prices,
                cancelled_ask_sizes,
                previous_seq_num: sequence_number,
                cancellation_reason
            }
        );
    };
}
</code></pre>



</details>

<a id="0x7_market_types_emit_event_for_bulk_order_rejection"></a>

## Function `emit_event_for_bulk_order_rejection`



<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_emit_event_for_bulk_order_rejection">emit_event_for_bulk_order_rejection</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="market_types.md#0x7_market_types_Market">market_types::Market</a>&lt;M&gt;, user: <b>address</b>, sequence_number: u64, existing_sequence_number: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market_types.md#0x7_market_types_emit_event_for_bulk_order_rejection">emit_event_for_bulk_order_rejection</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="market_types.md#0x7_market_types_Market">Market</a>&lt;M&gt;,
    user: <b>address</b>,
    sequence_number: u64,
    existing_sequence_number: u64
) {
    // Final check whether <a href="../../aptos-framework/doc/event.md#0x1_event">event</a> sending is enabled
    <b>if</b> (self.config.allow_events_emission) {
        <a href="../../aptos-framework/doc/event.md#0x1_event_emit">event::emit</a>(
            BulkOrderRejectionEvent::V1 {
                parent: self.parent,
                market: self.market,
                user,
                sequence_number,
                existing_sequence_number
            }
        );
    };
}
</code></pre>



</details>

<a id="0x7_market_types_get_order_book_mut"></a>

## Function `get_order_book_mut`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="market_types.md#0x7_market_types_get_order_book_mut">get_order_book_mut</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="market_types.md#0x7_market_types_Market">market_types::Market</a>&lt;M&gt;): &<b>mut</b> <a href="order_book.md#0x7_order_book_OrderBook">order_book::OrderBook</a>&lt;M&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="market_types.md#0x7_market_types_get_order_book_mut">get_order_book_mut</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="market_types.md#0x7_market_types_Market">Market</a>&lt;M&gt;
): &<b>mut</b> OrderBook&lt;M&gt; {
    &<b>mut</b> self.<a href="order_book.md#0x7_order_book">order_book</a>
}
</code></pre>



</details>

<a id="0x7_market_types_get_pre_cancellation_tracker_mut"></a>

## Function `get_pre_cancellation_tracker_mut`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="market_types.md#0x7_market_types_get_pre_cancellation_tracker_mut">get_pre_cancellation_tracker_mut</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="market_types.md#0x7_market_types_Market">market_types::Market</a>&lt;M&gt;): &<b>mut</b> <a href="pre_cancellation_tracker.md#0x7_pre_cancellation_tracker_PreCancellationTracker">pre_cancellation_tracker::PreCancellationTracker</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="market_types.md#0x7_market_types_get_pre_cancellation_tracker_mut">get_pre_cancellation_tracker_mut</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="market_types.md#0x7_market_types_Market">Market</a>&lt;M&gt;
): &<b>mut</b> PreCancellationTracker {
    self.<a href="pre_cancellation_tracker.md#0x7_pre_cancellation_tracker">pre_cancellation_tracker</a>.borrow_mut(<a href="market_types.md#0x7_market_types_PRE_CANCELLATION_TRACKER_KEY">PRE_CANCELLATION_TRACKER_KEY</a>)
}
</code></pre>



</details>

<a id="0x7_market_types_get_dead_mans_switch_tracker"></a>

## Function `get_dead_mans_switch_tracker`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="market_types.md#0x7_market_types_get_dead_mans_switch_tracker">get_dead_mans_switch_tracker</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="market_types.md#0x7_market_types_Market">market_types::Market</a>&lt;M&gt;): &<a href="dead_mans_switch_tracker.md#0x7_dead_mans_switch_tracker_DeadMansSwitchTracker">dead_mans_switch_tracker::DeadMansSwitchTracker</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="market_types.md#0x7_market_types_get_dead_mans_switch_tracker">get_dead_mans_switch_tracker</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="market_types.md#0x7_market_types_Market">Market</a>&lt;M&gt;
): &DeadMansSwitchTracker {
    self.<a href="dead_mans_switch_tracker.md#0x7_dead_mans_switch_tracker">dead_mans_switch_tracker</a>.borrow(<a href="market_types.md#0x7_market_types_DEAD_MANS_SWITCH_TRACKER_KEY">DEAD_MANS_SWITCH_TRACKER_KEY</a>)
}
</code></pre>



</details>

<a id="0x7_market_types_get_dead_mans_switch_tracker_mut"></a>

## Function `get_dead_mans_switch_tracker_mut`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="market_types.md#0x7_market_types_get_dead_mans_switch_tracker_mut">get_dead_mans_switch_tracker_mut</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="market_types.md#0x7_market_types_Market">market_types::Market</a>&lt;M&gt;): &<b>mut</b> <a href="dead_mans_switch_tracker.md#0x7_dead_mans_switch_tracker_DeadMansSwitchTracker">dead_mans_switch_tracker::DeadMansSwitchTracker</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="market_types.md#0x7_market_types_get_dead_mans_switch_tracker_mut">get_dead_mans_switch_tracker_mut</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="market_types.md#0x7_market_types_Market">Market</a>&lt;M&gt;
): &<b>mut</b> DeadMansSwitchTracker {
    self.<a href="dead_mans_switch_tracker.md#0x7_dead_mans_switch_tracker">dead_mans_switch_tracker</a>.borrow_mut(<a href="market_types.md#0x7_market_types_DEAD_MANS_SWITCH_TRACKER_KEY">DEAD_MANS_SWITCH_TRACKER_KEY</a>)
}
</code></pre>



</details>

<a id="0x7_market_types_is_dead_mans_switch_enabled"></a>

## Function `is_dead_mans_switch_enabled`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="market_types.md#0x7_market_types_is_dead_mans_switch_enabled">is_dead_mans_switch_enabled</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="market_types.md#0x7_market_types_Market">market_types::Market</a>&lt;M&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="market_types.md#0x7_market_types_is_dead_mans_switch_enabled">is_dead_mans_switch_enabled</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="market_types.md#0x7_market_types_Market">Market</a>&lt;M&gt;
): bool {
    self.config.enable_dead_mans_switch
}
</code></pre>



</details>

<a id="0x7_market_types_get_parent"></a>

## Function `get_parent`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="market_types.md#0x7_market_types_get_parent">get_parent</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="market_types.md#0x7_market_types_Market">market_types::Market</a>&lt;M&gt;): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="market_types.md#0x7_market_types_get_parent">get_parent</a>&lt;M: store + <b>copy</b> + drop&gt;(self: &<a href="market_types.md#0x7_market_types_Market">Market</a>&lt;M&gt;): <b>address</b> {
    self.parent
}
</code></pre>



</details>

<a id="0x7_market_types_get_market"></a>

## Function `get_market`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="market_types.md#0x7_market_types_get_market">get_market</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="market_types.md#0x7_market_types_Market">market_types::Market</a>&lt;M&gt;): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="market_types.md#0x7_market_types_get_market">get_market</a>&lt;M: store + <b>copy</b> + drop&gt;(self: &<a href="market_types.md#0x7_market_types_Market">Market</a>&lt;M&gt;): <b>address</b> {
    self.market
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
