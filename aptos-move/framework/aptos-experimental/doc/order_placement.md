
<a id="0x7_order_placement"></a>

# Module `0x7::order_placement`

This module provides a generic trading engine implementation for a market. On a high level, it's a data structure,
that stores an order book and provides APIs to place orders, cancel orders, and match orders. The market also acts
as a wrapper around the order book and pluggable clearinghouse implementation.
A clearing house implementation is expected to implement the following APIs
- settle_trade(taker, taker_order_id, maker, maker_order_id, fill_id, is_taker_long, price, size): SettleTradeResult ->
Called by the market when there is a match between taker and maker. The clearinghouse is expected to settle the trade
and return the result. Please note that the clearing house settlement size might not be the same as the order match size and
the settlement might also fail. The fill_id is an incremental counter for matched orders and can be used to track specific fills
- validate_order_placement(account, is_taker, is_long, price, size): bool -> Called by the market to validate
an order when it's placed. The clearinghouse is expected to validate the order and return true if the order is valid.
This API is called for both maker and taker order placements.
Check out clearinghouse_test as an example of the simplest form of clearing house implementation that just tracks
the position size of the user and does not do any validation.

- place_maker_order(account, order_id, is_bid, price, size, metadata) -> Called by the market before placing the
maker order in the order book. The clearinghouse can use this to track pending orders in the order book and perform
any other book keeping operations.

- cleanup_order(account, order_id, is_bid, remaining_size, order_metadata) -> Called by the market when an order is cancelled or fully filled
The clearinhouse can perform any cleanup operations like removing the order from the pending orders list. For every order placement
that passes the validate_order_placement check,
the market guarantees that the cleanup_order API will be called once and only once with the remaining size of the order.
the remaining size of the order being cleaned up - it can be 0, if the order was fully matched

- decrease_order_size(account, order_id, is_bid, price, size) -> Called by the market when a maker order is decreased
in size by the user. Please note that this API will only be called after place_maker_order is called and the order is
already in the order book. Size in this case is the remaining size of the order after the decrease.

Following are some valid sequence of API calls that the market makes to the clearinghouse:
1. validate_order_placement(10)
2. settle_trade(2)
3. settle_trade(3)
4. place_maker_order(5)
5. decrease_order_size(2)
6. decrease_order_size(1)
7. cleanup_order(2)
or
1. validate_order_placement(10)
2. cleanup_order(10)

Upon placement of an order, the market generates an order id and emits an event with the order details - the order id
is a unique id for the order that can be used to later get the status of the order or cancel the order.

Market also supports various conditions for order matching like Good Till Cancelled (GTC), Post Only, Immediate or Cancel (IOC).
GTC orders are orders that are valid until they are cancelled or filled. Post Only orders are orders that are valid only if they are not
taker orders. IOC orders are orders that are valid only if they are taker orders.

In addition, the market also supports trigger conditions for orders. An order with trigger condition is not put
on the order book until its trigger conditions are met. The following trigger conditions are supported:
TakeProfit(price): If it's a buy order it's triggered when the market price is greater than or equal to the price. If
it's a sell order it's triggered when the market price is less than or equal to the price.
StopLoss(price): If it's a buy order it's triggered when the market price is less than or equal to the price. If it's
a sell order it's triggered when the market price is greater than or equal to the price.
TimeBased(time): The order is triggered when the current time is greater than or equal to the time.


-  [Enum `OrderMatchResult`](#0x7_order_placement_OrderMatchResult)
-  [Constants](#@Constants_0)
-  [Function `destroy_order_match_result`](#0x7_order_placement_destroy_order_match_result)
-  [Function `number_of_fills`](#0x7_order_placement_number_of_fills)
-  [Function `number_of_matches`](#0x7_order_placement_number_of_matches)
-  [Function `total_fill_size`](#0x7_order_placement_total_fill_size)
-  [Function `get_cancel_reason`](#0x7_order_placement_get_cancel_reason)
-  [Function `get_remaining_size_from_result`](#0x7_order_placement_get_remaining_size_from_result)
-  [Function `is_ioc_violation`](#0x7_order_placement_is_ioc_violation)
-  [Function `is_fill_limit_violation`](#0x7_order_placement_is_fill_limit_violation)
-  [Function `is_dead_mans_switch_expired`](#0x7_order_placement_is_dead_mans_switch_expired)
-  [Function `is_clearinghouse_stopped_matching`](#0x7_order_placement_is_clearinghouse_stopped_matching)
-  [Function `get_order_id`](#0x7_order_placement_get_order_id)
-  [Function `place_limit_order`](#0x7_order_placement_place_limit_order)
-  [Function `place_market_order`](#0x7_order_placement_place_market_order)
-  [Function `place_maker_order_internal`](#0x7_order_placement_place_maker_order_internal)
-  [Function `cancel_bulk_maker_order_internal`](#0x7_order_placement_cancel_bulk_maker_order_internal)
-  [Function `cancel_maker_order_internal`](#0x7_order_placement_cancel_maker_order_internal)
-  [Function `cancel_taker_order_internal`](#0x7_order_placement_cancel_taker_order_internal)
-  [Function `cleanup_order_internal`](#0x7_order_placement_cleanup_order_internal)
-  [Function `settle_single_trade`](#0x7_order_placement_settle_single_trade)
-  [Function `place_order_with_order_id`](#0x7_order_placement_place_order_with_order_id)


<pre><code><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;
<b>use</b> <a href="../../aptos-framework/doc/transaction_context.md#0x1_transaction_context">0x1::transaction_context</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
<b>use</b> <a href="">0x5::bulk_order_types</a>;
<b>use</b> <a href="">0x5::order_book_types</a>;
<b>use</b> <a href="">0x5::order_match_types</a>;
<b>use</b> <a href="">0x5::single_order_types</a>;
<b>use</b> <a href="dead_mans_switch_tracker.md#0x7_dead_mans_switch_tracker">0x7::dead_mans_switch_tracker</a>;
<b>use</b> <a href="market_clearinghouse_order_info.md#0x7_market_clearinghouse_order_info">0x7::market_clearinghouse_order_info</a>;
<b>use</b> <a href="market_types.md#0x7_market_types">0x7::market_types</a>;
<b>use</b> <a href="order_book.md#0x7_order_book">0x7::order_book</a>;
<b>use</b> <a href="pre_cancellation_tracker.md#0x7_pre_cancellation_tracker">0x7::pre_cancellation_tracker</a>;
</code></pre>



<a id="0x7_order_placement_OrderMatchResult"></a>

## Enum `OrderMatchResult`



<pre><code>enum <a href="order_placement.md#0x7_order_placement_OrderMatchResult">OrderMatchResult</a>&lt;R: <b>copy</b>, drop, store&gt; <b>has</b> drop
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>V1</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>order_id: <a href="_OrderId">order_book_types::OrderId</a></code>
</dt>
<dd>

</dd>
<dt>
<code>remaining_size: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>cancel_reason: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="market_types.md#0x7_market_types_OrderCancellationReason">market_types::OrderCancellationReason</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>callback_results: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;R&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>fill_sizes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>match_count: u32</code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x7_order_placement_U64_MAX"></a>



<pre><code><b>const</b> <a href="order_placement.md#0x7_order_placement_U64_MAX">U64_MAX</a>: u64 = 18446744073709551615;
</code></pre>



<a id="0x7_order_placement_ECLEARINGHOUSE_SETTLEMENT_VIOLATION"></a>



<pre><code><b>const</b> <a href="order_placement.md#0x7_order_placement_ECLEARINGHOUSE_SETTLEMENT_VIOLATION">ECLEARINGHOUSE_SETTLEMENT_VIOLATION</a>: u64 = 2;
</code></pre>



<a id="0x7_order_placement_ECLIENT_ORDER_ID_LENGTH_EXCEEDED"></a>



<pre><code><b>const</b> <a href="order_placement.md#0x7_order_placement_ECLIENT_ORDER_ID_LENGTH_EXCEEDED">ECLIENT_ORDER_ID_LENGTH_EXCEEDED</a>: u64 = 3;
</code></pre>



<a id="0x7_order_placement_EINVALID_ORDER"></a>



<pre><code><b>const</b> <a href="order_placement.md#0x7_order_placement_EINVALID_ORDER">EINVALID_ORDER</a>: u64 = 1;
</code></pre>



<a id="0x7_order_placement_MAX_CLIENT_ORDER_ID_LENGTH"></a>



<pre><code><b>const</b> <a href="order_placement.md#0x7_order_placement_MAX_CLIENT_ORDER_ID_LENGTH">MAX_CLIENT_ORDER_ID_LENGTH</a>: u64 = 32;
</code></pre>



<a id="0x7_order_placement_destroy_order_match_result"></a>

## Function `destroy_order_match_result`



<pre><code><b>public</b> <b>fun</b> <a href="order_placement.md#0x7_order_placement_destroy_order_match_result">destroy_order_match_result</a>&lt;R: <b>copy</b>, drop, store&gt;(self: <a href="order_placement.md#0x7_order_placement_OrderMatchResult">order_placement::OrderMatchResult</a>&lt;R&gt;): (<a href="_OrderId">order_book_types::OrderId</a>, u64, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="market_types.md#0x7_market_types_OrderCancellationReason">market_types::OrderCancellationReason</a>&gt;, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;R&gt;, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, u32)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_placement.md#0x7_order_placement_destroy_order_match_result">destroy_order_match_result</a>&lt;R: store + <b>copy</b> + drop&gt;(
    self: <a href="order_placement.md#0x7_order_placement_OrderMatchResult">OrderMatchResult</a>&lt;R&gt;
): (OrderId, u64, Option&lt;OrderCancellationReason&gt;, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;R&gt;, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, u32) {
    <b>let</b> OrderMatchResult::V1 {
        order_id,
        remaining_size,
        cancel_reason,
        callback_results,
        fill_sizes,
        match_count
    } = self;
    (
        order_id,
        remaining_size,
        cancel_reason,
        callback_results,
        fill_sizes,
        match_count
    )
}
</code></pre>



</details>

<a id="0x7_order_placement_number_of_fills"></a>

## Function `number_of_fills`



<pre><code><b>public</b> <b>fun</b> <a href="order_placement.md#0x7_order_placement_number_of_fills">number_of_fills</a>&lt;R: <b>copy</b>, drop, store&gt;(self: &<a href="order_placement.md#0x7_order_placement_OrderMatchResult">order_placement::OrderMatchResult</a>&lt;R&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_placement.md#0x7_order_placement_number_of_fills">number_of_fills</a>&lt;R: store + <b>copy</b> + drop&gt;(
    self: &<a href="order_placement.md#0x7_order_placement_OrderMatchResult">OrderMatchResult</a>&lt;R&gt;
): u64 {
    self.fill_sizes.length()
}
</code></pre>



</details>

<a id="0x7_order_placement_number_of_matches"></a>

## Function `number_of_matches`

Includes fills and cancels


<pre><code><b>public</b> <b>fun</b> <a href="order_placement.md#0x7_order_placement_number_of_matches">number_of_matches</a>&lt;R: <b>copy</b>, drop, store&gt;(self: &<a href="order_placement.md#0x7_order_placement_OrderMatchResult">order_placement::OrderMatchResult</a>&lt;R&gt;): u32
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_placement.md#0x7_order_placement_number_of_matches">number_of_matches</a>&lt;R: store + <b>copy</b> + drop&gt;(
    self: &<a href="order_placement.md#0x7_order_placement_OrderMatchResult">OrderMatchResult</a>&lt;R&gt;
): u32 {
    self.match_count
}
</code></pre>



</details>

<a id="0x7_order_placement_total_fill_size"></a>

## Function `total_fill_size`



<pre><code><b>public</b> <b>fun</b> <a href="order_placement.md#0x7_order_placement_total_fill_size">total_fill_size</a>&lt;R: <b>copy</b>, drop, store&gt;(self: &<a href="order_placement.md#0x7_order_placement_OrderMatchResult">order_placement::OrderMatchResult</a>&lt;R&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_placement.md#0x7_order_placement_total_fill_size">total_fill_size</a>&lt;R: store + <b>copy</b> + drop&gt;(
    self: &<a href="order_placement.md#0x7_order_placement_OrderMatchResult">OrderMatchResult</a>&lt;R&gt;
): u64 {
    self.fill_sizes.fold(0, |acc, fill_size| acc + fill_size)
}
</code></pre>



</details>

<a id="0x7_order_placement_get_cancel_reason"></a>

## Function `get_cancel_reason`



<pre><code><b>public</b> <b>fun</b> <a href="order_placement.md#0x7_order_placement_get_cancel_reason">get_cancel_reason</a>&lt;R: <b>copy</b>, drop, store&gt;(self: &<a href="order_placement.md#0x7_order_placement_OrderMatchResult">order_placement::OrderMatchResult</a>&lt;R&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="market_types.md#0x7_market_types_OrderCancellationReason">market_types::OrderCancellationReason</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_placement.md#0x7_order_placement_get_cancel_reason">get_cancel_reason</a>&lt;R: store + <b>copy</b> + drop&gt;(
    self: &<a href="order_placement.md#0x7_order_placement_OrderMatchResult">OrderMatchResult</a>&lt;R&gt;
): Option&lt;OrderCancellationReason&gt; {
    self.cancel_reason
}
</code></pre>



</details>

<a id="0x7_order_placement_get_remaining_size_from_result"></a>

## Function `get_remaining_size_from_result`



<pre><code><b>public</b> <b>fun</b> <a href="order_placement.md#0x7_order_placement_get_remaining_size_from_result">get_remaining_size_from_result</a>&lt;R: <b>copy</b>, drop, store&gt;(self: &<a href="order_placement.md#0x7_order_placement_OrderMatchResult">order_placement::OrderMatchResult</a>&lt;R&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_placement.md#0x7_order_placement_get_remaining_size_from_result">get_remaining_size_from_result</a>&lt;R: store + <b>copy</b> + drop&gt;(
    self: &<a href="order_placement.md#0x7_order_placement_OrderMatchResult">OrderMatchResult</a>&lt;R&gt;
): u64 {
    self.remaining_size
}
</code></pre>



</details>

<a id="0x7_order_placement_is_ioc_violation"></a>

## Function `is_ioc_violation`



<pre><code><b>public</b> <b>fun</b> <a href="order_placement.md#0x7_order_placement_is_ioc_violation">is_ioc_violation</a>(reason: <a href="market_types.md#0x7_market_types_OrderCancellationReason">market_types::OrderCancellationReason</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_placement.md#0x7_order_placement_is_ioc_violation">is_ioc_violation</a>(reason: OrderCancellationReason): bool {
    reason == <a href="market_types.md#0x7_market_types_order_cancellation_reason_ioc_violation">market_types::order_cancellation_reason_ioc_violation</a>()
}
</code></pre>



</details>

<a id="0x7_order_placement_is_fill_limit_violation"></a>

## Function `is_fill_limit_violation`



<pre><code><b>public</b> <b>fun</b> <a href="order_placement.md#0x7_order_placement_is_fill_limit_violation">is_fill_limit_violation</a>(cancel_reason: <a href="market_types.md#0x7_market_types_OrderCancellationReason">market_types::OrderCancellationReason</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_placement.md#0x7_order_placement_is_fill_limit_violation">is_fill_limit_violation</a>(
    cancel_reason: OrderCancellationReason
): bool {
    cancel_reason
        == <a href="market_types.md#0x7_market_types_order_cancellation_reason_max_fill_limit_violation">market_types::order_cancellation_reason_max_fill_limit_violation</a>()
}
</code></pre>



</details>

<a id="0x7_order_placement_is_dead_mans_switch_expired"></a>

## Function `is_dead_mans_switch_expired`



<pre><code><b>public</b> <b>fun</b> <a href="order_placement.md#0x7_order_placement_is_dead_mans_switch_expired">is_dead_mans_switch_expired</a>(cancel_reason: <a href="market_types.md#0x7_market_types_OrderCancellationReason">market_types::OrderCancellationReason</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_placement.md#0x7_order_placement_is_dead_mans_switch_expired">is_dead_mans_switch_expired</a>(
    cancel_reason: OrderCancellationReason
): bool {
    cancel_reason
        == <a href="market_types.md#0x7_market_types_order_cancellation_reason_dead_mans_switch_expired">market_types::order_cancellation_reason_dead_mans_switch_expired</a>()
}
</code></pre>



</details>

<a id="0x7_order_placement_is_clearinghouse_stopped_matching"></a>

## Function `is_clearinghouse_stopped_matching`



<pre><code><b>public</b> <b>fun</b> <a href="order_placement.md#0x7_order_placement_is_clearinghouse_stopped_matching">is_clearinghouse_stopped_matching</a>(cancel_reason: <a href="market_types.md#0x7_market_types_OrderCancellationReason">market_types::OrderCancellationReason</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_placement.md#0x7_order_placement_is_clearinghouse_stopped_matching">is_clearinghouse_stopped_matching</a>(
    cancel_reason: OrderCancellationReason
): bool {
    cancel_reason
        == <a href="market_types.md#0x7_market_types_order_cancellation_reason_clearinghouse_stopped_matching">market_types::order_cancellation_reason_clearinghouse_stopped_matching</a>()
}
</code></pre>



</details>

<a id="0x7_order_placement_get_order_id"></a>

## Function `get_order_id`



<pre><code><b>public</b> <b>fun</b> <a href="order_placement.md#0x7_order_placement_get_order_id">get_order_id</a>&lt;R: <b>copy</b>, drop, store&gt;(self: <a href="order_placement.md#0x7_order_placement_OrderMatchResult">order_placement::OrderMatchResult</a>&lt;R&gt;): <a href="_OrderId">order_book_types::OrderId</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_placement.md#0x7_order_placement_get_order_id">get_order_id</a>&lt;R: store + <b>copy</b> + drop&gt;(self: <a href="order_placement.md#0x7_order_placement_OrderMatchResult">OrderMatchResult</a>&lt;R&gt;): OrderId {
    self.order_id
}
</code></pre>



</details>

<a id="0x7_order_placement_place_limit_order"></a>

## Function `place_limit_order`

Places a limit order - If it's a taker order, it will be matched immediately and if it's a maker order, it will simply
be placed in the order book. An order id is generated when the order is placed and this id can be used to
uniquely identify the order for this market and can also be used to get the status of the order or cancel the order.
The order is placed with the following parameters:
- user: The user who is placing the order
- price: The price at which the order is placed
- orig_size: The original size of the order
- is_bid: Whether the order is a buy order or a sell order
- time_in_force: The time in force for the order. This can be one of the following:
- TimeInForce::GTC: Good till cancelled order type
- TimeInForce::POST_ONLY: Post Only order type - ensures that the order is not a taker order
- TimeInForce::IOC: Immediate or Cancel order type - ensures that the order is a taker order. Try to match as much of the
order as possible as taker order and cancel the rest.
- trigger_condition: The trigger condition
- metadata: The metadata for the order. This can be any type that the clearing house implementation supports.
- client_order_id: The client order id for the order. This is an optional field that can be specified by the client
is solely used for their own tracking of the order. client order id doesn't have semantic meaning and
is not be inspected by the orderbook internally.
- max_match_limit: The maximum match limit for the order. This is the maximum number of matches (fills or cancels) to trigger for this order.
This knob is present to configure maximum amount of gas any order placement transaction might consume and avoid
hitting the maximum has limit of the blockchain.
- cancel_on_match_limit: bool: Whether to cancel the given order when the match limit is reached.
This is useful as the caller might not want to cancel the order when the limit is reached and can continue
that order in a separate transaction.
- callbacks: The callbacks for the market clearinghouse. This is a struct that implements the MarketClearinghouseCallbacks
interface. This is used to validate the order and settle the trade.
Returns the order id, remaining size, cancel reason and number of fills for the order.


<pre><code><b>public</b> <b>fun</b> <a href="order_placement.md#0x7_order_placement_place_limit_order">place_limit_order</a>&lt;M: <b>copy</b>, drop, store, R: <b>copy</b>, drop, store&gt;(market: &<b>mut</b> <a href="market_types.md#0x7_market_types_Market">market_types::Market</a>&lt;M&gt;, user: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, limit_price: u64, orig_size: u64, is_bid: bool, time_in_force: <a href="_TimeInForce">order_book_types::TimeInForce</a>, trigger_condition: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="_TriggerCondition">order_book_types::TriggerCondition</a>&gt;, metadata: M, client_order_id: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, max_match_limit: u32, cancel_on_match_limit: bool, callbacks: &<a href="market_types.md#0x7_market_types_MarketClearinghouseCallbacks">market_types::MarketClearinghouseCallbacks</a>&lt;M, R&gt;): <a href="order_placement.md#0x7_order_placement_OrderMatchResult">order_placement::OrderMatchResult</a>&lt;R&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_placement.md#0x7_order_placement_place_limit_order">place_limit_order</a>&lt;M: store + <b>copy</b> + drop, R: store + <b>copy</b> + drop&gt;(
    market: &<b>mut</b> Market&lt;M&gt;,
    user: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    limit_price: u64,
    orig_size: u64,
    is_bid: bool,
    time_in_force: TimeInForce,
    trigger_condition: Option&lt;TriggerCondition&gt;,
    metadata: M,
    client_order_id: Option&lt;String&gt;,
    max_match_limit: u32,
    cancel_on_match_limit: bool,
    callbacks: &MarketClearinghouseCallbacks&lt;M, R&gt;
): <a href="order_placement.md#0x7_order_placement_OrderMatchResult">OrderMatchResult</a>&lt;R&gt; {
    <a href="order_placement.md#0x7_order_placement_place_order_with_order_id">place_order_with_order_id</a>(
        market,
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(user),
        limit_price,
        orig_size,
        orig_size,
        is_bid,
        time_in_force,
        trigger_condition,
        metadata,
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(), // order_id
        client_order_id,
        max_match_limit,
        cancel_on_match_limit,
        <b>true</b>,
        callbacks
    )
}
</code></pre>



</details>

<a id="0x7_order_placement_place_market_order"></a>

## Function `place_market_order`

Places a market order - The order is guaranteed to be a taker order and will be matched immediately.


<pre><code><b>public</b> <b>fun</b> <a href="order_placement.md#0x7_order_placement_place_market_order">place_market_order</a>&lt;M: <b>copy</b>, drop, store, R: <b>copy</b>, drop, store&gt;(market: &<b>mut</b> <a href="market_types.md#0x7_market_types_Market">market_types::Market</a>&lt;M&gt;, user: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, orig_size: u64, is_bid: bool, metadata: M, client_order_id: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, max_match_limit: u32, cancel_on_match_limit: bool, callbacks: &<a href="market_types.md#0x7_market_types_MarketClearinghouseCallbacks">market_types::MarketClearinghouseCallbacks</a>&lt;M, R&gt;): <a href="order_placement.md#0x7_order_placement_OrderMatchResult">order_placement::OrderMatchResult</a>&lt;R&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_placement.md#0x7_order_placement_place_market_order">place_market_order</a>&lt;M: store + <b>copy</b> + drop, R: store + <b>copy</b> + drop&gt;(
    market: &<b>mut</b> Market&lt;M&gt;,
    user: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    orig_size: u64,
    is_bid: bool,
    metadata: M,
    client_order_id: Option&lt;String&gt;,
    max_match_limit: u32,
    cancel_on_match_limit: bool,
    callbacks: &MarketClearinghouseCallbacks&lt;M, R&gt;
): <a href="order_placement.md#0x7_order_placement_OrderMatchResult">OrderMatchResult</a>&lt;R&gt; {
    <a href="order_placement.md#0x7_order_placement_place_order_with_order_id">place_order_with_order_id</a>(
        market,
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(user),
        <b>if</b> (is_bid) {
            <a href="order_placement.md#0x7_order_placement_U64_MAX">U64_MAX</a>
        } <b>else</b> { 1 },
        orig_size,
        orig_size,
        is_bid,
        immediate_or_cancel(), // market orders are always IOC
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(), // trigger_condition
        metadata,
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(), // order_id
        client_order_id,
        max_match_limit,
        cancel_on_match_limit,
        <b>true</b>,
        callbacks
    )
}
</code></pre>



</details>

<a id="0x7_order_placement_place_maker_order_internal"></a>

## Function `place_maker_order_internal`



<pre><code><b>fun</b> <a href="order_placement.md#0x7_order_placement_place_maker_order_internal">place_maker_order_internal</a>&lt;M: <b>copy</b>, drop, store, R: <b>copy</b>, drop, store&gt;(market: &<b>mut</b> <a href="market_types.md#0x7_market_types_Market">market_types::Market</a>&lt;M&gt;, user_addr: <b>address</b>, limit_price: u64, orig_size: u64, remaining_size: u64, fill_sizes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, match_count: u32, is_bid: bool, time_in_force: <a href="_TimeInForce">order_book_types::TimeInForce</a>, trigger_condition: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="_TriggerCondition">order_book_types::TriggerCondition</a>&gt;, metadata: M, order_id: <a href="_OrderId">order_book_types::OrderId</a>, client_order_id: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, emit_open_for_cancellation: bool, callbacks: &<a href="market_types.md#0x7_market_types_MarketClearinghouseCallbacks">market_types::MarketClearinghouseCallbacks</a>&lt;M, R&gt;, callback_results: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;R&gt;): <a href="order_placement.md#0x7_order_placement_OrderMatchResult">order_placement::OrderMatchResult</a>&lt;R&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="order_placement.md#0x7_order_placement_place_maker_order_internal">place_maker_order_internal</a>&lt;M: store + <b>copy</b> + drop, R: store + <b>copy</b> + drop&gt;(
    market: &<b>mut</b> Market&lt;M&gt;,
    user_addr: <b>address</b>,
    limit_price: u64,
    orig_size: u64,
    remaining_size: u64,
    fill_sizes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    match_count: u32,
    is_bid: bool,
    time_in_force: TimeInForce,
    trigger_condition: Option&lt;TriggerCondition&gt;,
    metadata: M,
    order_id: OrderId,
    client_order_id: Option&lt;String&gt;,
    emit_open_for_cancellation: bool,
    callbacks: &MarketClearinghouseCallbacks&lt;M, R&gt;,
    callback_results: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;R&gt;
): <a href="order_placement.md#0x7_order_placement_OrderMatchResult">OrderMatchResult</a>&lt;R&gt; {
    <b>if</b> (time_in_force == immediate_or_cancel() && trigger_condition.is_none()) {
        <b>return</b> <a href="order_placement.md#0x7_order_placement_cancel_taker_order_internal">cancel_taker_order_internal</a>(
            market,
            user_addr,
            limit_price,
            order_id,
            client_order_id,
            orig_size,
            remaining_size,
            fill_sizes,
            match_count,
            is_bid,
            <a href="market_types.md#0x7_market_types_order_cancellation_reason_ioc_violation">market_types::order_cancellation_reason_ioc_violation</a>(),
            std::string::utf8(b"IOC Violation"),
            trigger_condition,
            metadata,
            time_in_force,
            emit_open_for_cancellation,
            callbacks,
            callback_results
        );
    };

    <b>if</b> (trigger_condition.is_some()) {
        // Do not emit an open <a href="../../aptos-framework/doc/event.md#0x1_event">event</a> for orders <b>with</b> trigger conditions <b>as</b> they are not live in the order book yet
        market.get_order_book_mut().place_maker_order(
            new_single_order_request(
                user_addr,
                order_id,
                client_order_id,
                limit_price,
                orig_size,
                remaining_size,
                is_bid,
                trigger_condition,
                time_in_force,
                metadata
            )
        );
        <b>return</b> OrderMatchResult::V1 {
            order_id,
            remaining_size,
            cancel_reason: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(),
            callback_results,
            fill_sizes,
            match_count
        }
    };

    <b>let</b> result =
        callbacks.place_maker_order(
            new_clearinghouse_order_info(
                user_addr,
                order_id,
                client_order_id,
                is_bid,
                limit_price,
                time_in_force,
                single_order_type(),
                <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(),
                metadata
            ),
            remaining_size
        );
    <b>if</b> (result.get_place_maker_order_cancellation_reason().is_some()) {
        <b>return</b> <a href="order_placement.md#0x7_order_placement_cancel_taker_order_internal">cancel_taker_order_internal</a>(
            market,
            user_addr,
            limit_price,
            order_id,
            client_order_id,
            orig_size,
            remaining_size,
            fill_sizes,
            match_count,
            is_bid,
            <a href="market_types.md#0x7_market_types_order_cancellation_reason_place_maker_order_violation">market_types::order_cancellation_reason_place_maker_order_violation</a>(),
            result.get_place_maker_order_cancellation_reason().destroy_some(),
            <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(), // trigger_condition
            metadata,
            time_in_force,
            emit_open_for_cancellation,
            callbacks,
            callback_results
        );
    };

    // Emit order open <a href="../../aptos-framework/doc/event.md#0x1_event">event</a> for the maker order
    market.emit_event_for_order(
        order_id,
        client_order_id,
        user_addr,
        orig_size,
        remaining_size,
        remaining_size,
        limit_price,
        is_bid,
        <b>false</b>,
        <a href="market_types.md#0x7_market_types_order_status_open">market_types::order_status_open</a>(),
        std::string::utf8(b""),
        metadata,
        trigger_condition,
        time_in_force,
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(),
        callbacks
    );

    <b>let</b> actions = result.get_place_maker_order_actions();
    <b>if</b> (actions.is_some()) {
        callback_results.push_back(actions.destroy_some());
    };
    market.get_order_book_mut().place_maker_order(
        new_single_order_request(
            user_addr,
            order_id,
            client_order_id,
            limit_price,
            orig_size,
            remaining_size,
            is_bid,
            trigger_condition,
            time_in_force,
            metadata
        )
    );
    <b>return</b> OrderMatchResult::V1 {
        order_id,
        remaining_size,
        cancel_reason: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(),
        callback_results,
        fill_sizes,
        match_count
    }
}
</code></pre>



</details>

<a id="0x7_order_placement_cancel_bulk_maker_order_internal"></a>

## Function `cancel_bulk_maker_order_internal`



<pre><code><b>fun</b> <a href="order_placement.md#0x7_order_placement_cancel_bulk_maker_order_internal">cancel_bulk_maker_order_internal</a>&lt;M: <b>copy</b>, drop, store, R: <b>copy</b>, drop, store&gt;(market: &<b>mut</b> <a href="market_types.md#0x7_market_types_Market">market_types::Market</a>&lt;M&gt;, maker_order: &<a href="_OrderMatchDetails">order_match_types::OrderMatchDetails</a>&lt;M&gt;, maker_address: <b>address</b>, order_id: <a href="_OrderId">order_book_types::OrderId</a>, unsettled_size: u64, cancellation_reason: <a href="market_types.md#0x7_market_types_OrderCancellationReason">market_types::OrderCancellationReason</a>, callbacks: &<a href="market_types.md#0x7_market_types_MarketClearinghouseCallbacks">market_types::MarketClearinghouseCallbacks</a>&lt;M, R&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="order_placement.md#0x7_order_placement_cancel_bulk_maker_order_internal">cancel_bulk_maker_order_internal</a>&lt;M: store + <b>copy</b> + drop, R: store + <b>copy</b> + drop&gt;(
    market: &<b>mut</b> Market&lt;M&gt;,
    maker_order: &OrderMatchDetails&lt;M&gt;,
    maker_address: <b>address</b>,
    order_id: OrderId,
    unsettled_size: u64,
    cancellation_reason: OrderCancellationReason,
    callbacks: &MarketClearinghouseCallbacks&lt;M, R&gt;
) {
    <b>let</b> remaining_size = maker_order.get_remaining_size_from_match_details();
    <b>let</b> price = maker_order.get_price_from_match_details();
    <b>let</b> is_bid = maker_order.is_bid_from_match_details();
    <b>let</b> cancelled_size = unsettled_size + remaining_size;

    // Cancel only at the specific price level instead of cancelling the entire bulk order
    <b>let</b> (_actual_cancelled_size, modified_order) =
        <b>if</b> (remaining_size != 0) {
            market.get_order_book_mut().cancel_bulk_order_at_price(
                maker_address, price, is_bid
            )
        } <b>else</b> {
            // If remaining size is 0, just get the current order state for <a href="../../aptos-framework/doc/event.md#0x1_event">event</a> emission
            (0, market.get_order_book().get_bulk_order(maker_address))
        };

    callbacks.cleanup_bulk_order_at_price(
        maker_address,
        order_id,
        is_bid,
        price,
        cancelled_size
    );

    // Emit <a href="../../aptos-framework/doc/event.md#0x1_event">event</a> <b>with</b> the cancelled price level
    <b>let</b> (
        modified_order_request, _order_id, _unique_priority_idx, _creation_time_micros
    ) = modified_order.destroy_bulk_order();
    <b>let</b> (
        _account,
        order_sequence_number,
        bid_prices,
        bid_sizes,
        ask_prices,
        ask_sizes,
        _metadata
    ) = modified_order_request.destroy_bulk_order_request();

    // Build cancelled price/size vectors for the specific level that was cancelled
    <b>let</b> (
        cancelled_bid_prices,
        cancelled_bid_sizes,
        cancelled_ask_prices,
        cancelled_ask_sizes
    ) =
        <b>if</b> (is_bid) {
            (<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[price], <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[cancelled_size], <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[], <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[])
        } <b>else</b> {
            (<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[], <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[], <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[price], <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[cancelled_size])
        };

    market.emit_event_for_bulk_order_modified(
        order_id,
        order_sequence_number,
        maker_address,
        bid_prices,
        bid_sizes,
        ask_prices,
        ask_sizes,
        cancelled_bid_prices,
        cancelled_bid_sizes,
        cancelled_ask_prices,
        cancelled_ask_sizes,
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(cancellation_reason)
    );
}
</code></pre>



</details>

<a id="0x7_order_placement_cancel_maker_order_internal"></a>

## Function `cancel_maker_order_internal`



<pre><code><b>fun</b> <a href="order_placement.md#0x7_order_placement_cancel_maker_order_internal">cancel_maker_order_internal</a>&lt;M: <b>copy</b>, drop, store, R: <b>copy</b>, drop, store&gt;(market: &<b>mut</b> <a href="market_types.md#0x7_market_types_Market">market_types::Market</a>&lt;M&gt;, maker_order: &<a href="_OrderMatchDetails">order_match_types::OrderMatchDetails</a>&lt;M&gt;, client_order_id: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, maker_address: <b>address</b>, order_id: <a href="_OrderId">order_book_types::OrderId</a>, cancellation_reason: <a href="market_types.md#0x7_market_types_OrderCancellationReason">market_types::OrderCancellationReason</a>, maker_cancellation_reason: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, unsettled_size: u64, metadata: M, time_in_force: <a href="_TimeInForce">order_book_types::TimeInForce</a>, callbacks: &<a href="market_types.md#0x7_market_types_MarketClearinghouseCallbacks">market_types::MarketClearinghouseCallbacks</a>&lt;M, R&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="order_placement.md#0x7_order_placement_cancel_maker_order_internal">cancel_maker_order_internal</a>&lt;M: store + <b>copy</b> + drop, R: store + <b>copy</b> + drop&gt;(
    market: &<b>mut</b> Market&lt;M&gt;,
    maker_order: &OrderMatchDetails&lt;M&gt;,
    client_order_id: Option&lt;String&gt;,
    maker_address: <b>address</b>,
    order_id: OrderId,
    cancellation_reason: OrderCancellationReason,
    maker_cancellation_reason: String,
    unsettled_size: u64,
    metadata: M,
    time_in_force: TimeInForce,
    callbacks: &MarketClearinghouseCallbacks&lt;M, R&gt;
) {
    <b>if</b> (maker_order.is_bulk_order_from_match_details()) {
        <b>return</b> <a href="order_placement.md#0x7_order_placement_cancel_bulk_maker_order_internal">cancel_bulk_maker_order_internal</a>(
            market,
            maker_order,
            maker_address,
            order_id,
            unsettled_size,
            cancellation_reason,
            callbacks
        );
    };
    <b>let</b> maker_cancel_size =
        unsettled_size + maker_order.get_remaining_size_from_match_details();
    market.emit_event_for_order(
        order_id,
        client_order_id,
        maker_address,
        maker_order.get_orig_size_from_match_details(),
        0,
        maker_cancel_size,
        maker_order.get_price_from_match_details(),
        maker_order.is_bid_from_match_details(),
        <b>false</b>,
        <a href="market_types.md#0x7_market_types_order_status_cancelled">market_types::order_status_cancelled</a>(),
        maker_cancellation_reason,
        metadata,
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(), // trigger_condition
        time_in_force,
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(cancellation_reason),
        callbacks
    );
    // If the maker is invalid cancel the maker order and <b>continue</b> <b>to</b> the next maker order
    <b>if</b> (maker_order.get_remaining_size_from_match_details() != 0) {
        market.get_order_book_mut().cancel_single_order(maker_address, order_id);
    };
    <a href="order_placement.md#0x7_order_placement_cleanup_order_internal">cleanup_order_internal</a>(
        maker_address,
        order_id,
        client_order_id,
        maker_order.get_book_type_from_match_details(),
        maker_order.is_bid_from_match_details(),
        time_in_force,
        maker_cancel_size,
        maker_order.get_price_from_match_details(),
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(), // trigger_condition
        metadata,
        callbacks,
        <b>false</b> // is_taker is <b>false</b> <b>as</b> this is a maker order
    );
}
</code></pre>



</details>

<a id="0x7_order_placement_cancel_taker_order_internal"></a>

## Function `cancel_taker_order_internal`



<pre><code>#[lint::skip(#[needless_mutable_reference])]
<b>fun</b> <a href="order_placement.md#0x7_order_placement_cancel_taker_order_internal">cancel_taker_order_internal</a>&lt;M: <b>copy</b>, drop, store, R: <b>copy</b>, drop, store&gt;(market: &<b>mut</b> <a href="market_types.md#0x7_market_types_Market">market_types::Market</a>&lt;M&gt;, user_addr: <b>address</b>, limit_price: u64, order_id: <a href="_OrderId">order_book_types::OrderId</a>, client_order_id: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, orig_size: u64, size_delta: u64, fill_sizes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, match_count: u32, is_bid: bool, cancel_reason: <a href="market_types.md#0x7_market_types_OrderCancellationReason">market_types::OrderCancellationReason</a>, cancel_details: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, trigger_condition: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="_TriggerCondition">order_book_types::TriggerCondition</a>&gt;, metadata: M, time_in_force: <a href="_TimeInForce">order_book_types::TimeInForce</a>, emit_order_open: bool, callbacks: &<a href="market_types.md#0x7_market_types_MarketClearinghouseCallbacks">market_types::MarketClearinghouseCallbacks</a>&lt;M, R&gt;, callback_results: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;R&gt;): <a href="order_placement.md#0x7_order_placement_OrderMatchResult">order_placement::OrderMatchResult</a>&lt;R&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="order_placement.md#0x7_order_placement_cancel_taker_order_internal">cancel_taker_order_internal</a>&lt;M: store + <b>copy</b> + drop, R: store + <b>copy</b> + drop&gt;(
    market: &<b>mut</b> Market&lt;M&gt;,
    user_addr: <b>address</b>,
    limit_price: u64,
    order_id: OrderId,
    client_order_id: Option&lt;String&gt;,
    orig_size: u64,
    size_delta: u64,
    fill_sizes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    match_count: u32,
    is_bid: bool,
    cancel_reason: OrderCancellationReason,
    cancel_details: String,
    trigger_condition: Option&lt;TriggerCondition&gt;,
    metadata: M,
    time_in_force: TimeInForce,
    emit_order_open: bool,
    callbacks: &MarketClearinghouseCallbacks&lt;M, R&gt;,
    callback_results: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;R&gt;
): <a href="order_placement.md#0x7_order_placement_OrderMatchResult">OrderMatchResult</a>&lt;R&gt; {
    <b>if</b> (emit_order_open) {
        market.emit_event_for_order(
            order_id,
            client_order_id,
            user_addr,
            orig_size,
            size_delta,
            orig_size,
            limit_price,
            is_bid,
            <b>true</b>, // is_taker - always <b>true</b> for taker orders
            <a href="market_types.md#0x7_market_types_order_status_open">market_types::order_status_open</a>(),
            std::string::utf8(b""),
            metadata,
            <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(), // trigger_condition
            time_in_force,
            <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(),
            callbacks
        );
    };
    market.emit_event_for_order(
        order_id,
        client_order_id,
        user_addr,
        orig_size,
        0,
        size_delta,
        limit_price,
        is_bid,
        <b>true</b>, // is_taker - always <b>true</b> for taker orders
        <a href="market_types.md#0x7_market_types_order_status_cancelled">market_types::order_status_cancelled</a>(),
        cancel_details,
        metadata,
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(), // trigger_condition
        time_in_force,
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(cancel_reason),
        callbacks
    );
    callbacks.cleanup_order(
        new_clearinghouse_order_info(
            user_addr,
            order_id,
            client_order_id,
            is_bid,
            limit_price,
            time_in_force,
            single_order_type(),
            trigger_condition,
            metadata
        ),
        size_delta,
        <b>true</b> // is_taker - always <b>true</b> for taker orders
    );
    OrderMatchResult::V1 {
        order_id,
        remaining_size: 0,
        cancel_reason: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(cancel_reason),
        fill_sizes,
        callback_results,
        match_count
    }
}
</code></pre>



</details>

<a id="0x7_order_placement_cleanup_order_internal"></a>

## Function `cleanup_order_internal`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_placement.md#0x7_order_placement_cleanup_order_internal">cleanup_order_internal</a>&lt;M: <b>copy</b>, drop, store, R: <b>copy</b>, drop, store&gt;(user_addr: <b>address</b>, order_id: <a href="_OrderId">order_book_types::OrderId</a>, client_order_id: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, order_type: <a href="_OrderType">order_book_types::OrderType</a>, is_bid: bool, time_in_force: <a href="_TimeInForce">order_book_types::TimeInForce</a>, cleanup_size: u64, price: u64, trigger_condition: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="_TriggerCondition">order_book_types::TriggerCondition</a>&gt;, metadata: M, callbacks: &<a href="market_types.md#0x7_market_types_MarketClearinghouseCallbacks">market_types::MarketClearinghouseCallbacks</a>&lt;M, R&gt;, is_taker: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_placement.md#0x7_order_placement_cleanup_order_internal">cleanup_order_internal</a>&lt;M: store + <b>copy</b> + drop, R: store + <b>copy</b> + drop&gt;(
    user_addr: <b>address</b>,
    order_id: OrderId,
    client_order_id: Option&lt;String&gt;,
    order_type: OrderType,
    is_bid: bool,
    time_in_force: TimeInForce,
    cleanup_size: u64,
    price: u64,
    trigger_condition: Option&lt;TriggerCondition&gt;,
    metadata: M,
    callbacks: &MarketClearinghouseCallbacks&lt;M, R&gt;,
    is_taker: bool
) {
    <b>if</b> (order_type == single_order_type()) {
        callbacks.cleanup_order(
            new_clearinghouse_order_info(
                user_addr,
                order_id,
                client_order_id,
                is_bid,
                price,
                time_in_force,
                single_order_type(),
                trigger_condition,
                metadata
            ),
            cleanup_size,
            is_taker
        );
    } <b>else</b> {
        callbacks.cleanup_bulk_order_at_price(
            user_addr,
            order_id,
            is_bid,
            price,
            cleanup_size
        );
    }
}
</code></pre>



</details>

<a id="0x7_order_placement_settle_single_trade"></a>

## Function `settle_single_trade`



<pre><code><b>fun</b> <a href="order_placement.md#0x7_order_placement_settle_single_trade">settle_single_trade</a>&lt;M: <b>copy</b>, drop, store, R: <b>copy</b>, drop, store&gt;(market: &<b>mut</b> <a href="market_types.md#0x7_market_types_Market">market_types::Market</a>&lt;M&gt;, user_addr: <b>address</b>, price: u64, orig_size: u64, remaining_size: &<b>mut</b> u64, is_bid: bool, metadata: M, order_id: <a href="_OrderId">order_book_types::OrderId</a>, client_order_id: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, callbacks: &<a href="market_types.md#0x7_market_types_MarketClearinghouseCallbacks">market_types::MarketClearinghouseCallbacks</a>&lt;M, R&gt;, time_in_force: <a href="_TimeInForce">order_book_types::TimeInForce</a>, fill_sizes: &<b>mut</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;): (<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="market_types.md#0x7_market_types_OrderCancellationReason">market_types::OrderCancellationReason</a>&gt;, <a href="market_types.md#0x7_market_types_CallbackResult">market_types::CallbackResult</a>&lt;R&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="order_placement.md#0x7_order_placement_settle_single_trade">settle_single_trade</a>&lt;M: store + <b>copy</b> + drop, R: store + <b>copy</b> + drop&gt;(
    market: &<b>mut</b> Market&lt;M&gt;,
    user_addr: <b>address</b>,
    price: u64,
    orig_size: u64,
    remaining_size: &<b>mut</b> u64,
    is_bid: bool,
    metadata: M,
    order_id: OrderId,
    client_order_id: Option&lt;String&gt;,
    callbacks: &MarketClearinghouseCallbacks&lt;M, R&gt;,
    time_in_force: TimeInForce,
    fill_sizes: &<b>mut</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;
): (Option&lt;OrderCancellationReason&gt;, CallbackResult&lt;R&gt;) {
    <b>let</b> dead_mans_switch_enabled = market.is_dead_mans_switch_enabled();
    <b>if</b> (dead_mans_switch_enabled
        && !is_order_valid(
            market.get_dead_mans_switch_tracker(), user_addr, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
        )) {
        <b>let</b> taker_cancellation_reason =
            std::string::utf8(b"Order invalidated due <b>to</b> dead man's switch expiry");
        <a href="order_placement.md#0x7_order_placement_cancel_taker_order_internal">cancel_taker_order_internal</a>(
            market,
            user_addr,
            price,
            order_id,
            client_order_id,
            orig_size,
            *remaining_size,
            *fill_sizes,
            0, // match_count - doesn't matter <b>as</b> we don't <b>use</b> the result.
            is_bid,
            <a href="market_types.md#0x7_market_types_order_cancellation_reason_dead_mans_switch_expired">market_types::order_cancellation_reason_dead_mans_switch_expired</a>(),
            taker_cancellation_reason,
            <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(), // trigger_condition
            metadata,
            time_in_force,
            <b>false</b>, // emit_order_open is <b>false</b> <b>as</b> the order was already open
            callbacks,
            <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[]
        );
        <b>return</b> (
            <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(
                <a href="market_types.md#0x7_market_types_order_cancellation_reason_dead_mans_switch_expired">market_types::order_cancellation_reason_dead_mans_switch_expired</a>()
            ),
            new_callback_result_not_available()
        );
    };
    <b>let</b> result =
        market.get_order_book_mut().get_single_match_for_taker(
            price, *remaining_size, is_bid
        );
    <b>let</b> (maker_order, maker_matched_size) = result.destroy_order_match();
    <b>if</b> (!market.is_allowed_self_trade()
        && maker_order.get_account_from_match_details() == user_addr) {
        <a href="order_placement.md#0x7_order_placement_cancel_maker_order_internal">cancel_maker_order_internal</a>(
            market,
            &maker_order,
            maker_order.get_client_order_id_from_match_details(),
            maker_order.get_account_from_match_details(),
            maker_order.get_order_id_from_match_details(),
            <a href="market_types.md#0x7_market_types_order_cancellation_reason_disallowed_self_trading">market_types::order_cancellation_reason_disallowed_self_trading</a>(),
            std::string::utf8(b"Disallowed self trading"),
            maker_matched_size,
            maker_order.get_metadata_from_match_details(),
            maker_order.get_time_in_force_from_match_details(),
            callbacks
        );
        <b>return</b> (<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(), new_callback_result_not_available());
    };
    <b>if</b> (dead_mans_switch_enabled
        && !is_order_valid(
            market.get_dead_mans_switch_tracker(),
            maker_order.get_account_from_match_details(),
            <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(
                maker_order.get_creation_time_micros_from_match_details() / 1000000
            )
        )) {
        <a href="order_placement.md#0x7_order_placement_cancel_maker_order_internal">cancel_maker_order_internal</a>(
            market,
            &maker_order,
            maker_order.get_client_order_id_from_match_details(),
            maker_order.get_account_from_match_details(),
            maker_order.get_order_id_from_match_details(),
            <a href="market_types.md#0x7_market_types_order_cancellation_reason_dead_mans_switch_expired">market_types::order_cancellation_reason_dead_mans_switch_expired</a>(),
            std::string::utf8(b"Order invalidated due <b>to</b> dead man's switch expiry"),
            maker_matched_size,
            maker_order.get_metadata_from_match_details(),
            maker_order.get_time_in_force_from_match_details(),
            callbacks
        );
        <b>return</b> (<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(), new_callback_result_not_available());
    };
    <b>let</b> fill_id = <a href="../../aptos-framework/doc/transaction_context.md#0x1_transaction_context_monotonically_increasing_counter">transaction_context::monotonically_increasing_counter</a>();
    <b>let</b> settle_result =
        callbacks.settle_trade(
            market,
            new_clearinghouse_order_info(
                user_addr,
                order_id,
                client_order_id,
                is_bid,
                price,
                time_in_force,
                single_order_type(),
                <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(), // trigger_condition
                metadata
            ),
            new_clearinghouse_order_info(
                maker_order.get_account_from_match_details(),
                maker_order.get_order_id_from_match_details(),
                maker_order.get_client_order_id_from_match_details(),
                maker_order.is_bid_from_match_details(),
                maker_order.get_price_from_match_details(),
                maker_order.get_time_in_force_from_match_details(),
                maker_order.get_book_type_from_match_details(),
                <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(), // trigger_condition
                maker_order.get_metadata_from_match_details()
            ),
            fill_id,
            maker_order.get_price_from_match_details(), // Order is always matched at the price of the maker
            maker_matched_size
        );

    <b>let</b> unsettled_maker_size = maker_matched_size;
    <b>let</b> settled_size = settle_result.get_settled_size();
    <b>if</b> (settled_size &gt; 0) {
        *remaining_size -= settled_size;
        unsettled_maker_size -= settled_size;
        fill_sizes.push_back(settled_size);
        // Event for taker fill
        market.emit_event_for_order(
            order_id,
            client_order_id,
            user_addr,
            orig_size,
            *remaining_size,
            settled_size,
            maker_order.get_price_from_match_details(),
            is_bid,
            <b>true</b>,
            <a href="market_types.md#0x7_market_types_order_status_filled">market_types::order_status_filled</a>(),
            std::string::utf8(b""),
            metadata,
            <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(), // trigger_condition
            time_in_force,
            <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(),
            callbacks
        );
        // Event for maker fill
        <b>if</b> (maker_order.is_bulk_order_from_match_details()) {
            market.emit_event_for_bulk_order_filled(
                maker_order.get_order_id_from_match_details(),
                maker_order.get_sequence_number_from_match_details(),
                maker_order.get_account_from_match_details(),
                settled_size,
                maker_order.get_price_from_match_details(),
                maker_order.get_price_from_match_details(),
                !is_bid,
                fill_id
            );
        } <b>else</b> {
            market.emit_event_for_order(
                maker_order.get_order_id_from_match_details(),
                maker_order.get_client_order_id_from_match_details(),
                maker_order.get_account_from_match_details(),
                maker_order.get_orig_size_from_match_details(),
                maker_order.get_remaining_size_from_match_details()
                    + unsettled_maker_size,
                settled_size,
                maker_order.get_price_from_match_details(),
                !is_bid,
                <b>false</b>,
                <a href="market_types.md#0x7_market_types_order_status_filled">market_types::order_status_filled</a>(),
                std::string::utf8(b""),
                maker_order.get_metadata_from_match_details(),
                <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(),
                maker_order.get_time_in_force_from_match_details(),
                <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(),
                callbacks
            );
        };
    };

    <b>let</b> maker_cancellation_reason_str = settle_result.get_maker_cancellation_reason();
    <b>let</b> taker_cancellation_reason_str = settle_result.get_taker_cancellation_reason();
    <b>if</b> (settled_size &lt; maker_matched_size) {
        // If the order is partially settled, the expectation is that the clearinghouse
        // provides cancellation reason for at least one of the orders.
        <b>assert</b>!(
            maker_cancellation_reason_str.is_some()
                || taker_cancellation_reason_str.is_some(),
            <a href="order_placement.md#0x7_order_placement_ECLEARINGHOUSE_SETTLEMENT_VIOLATION">ECLEARINGHOUSE_SETTLEMENT_VIOLATION</a>
        );
    };
    <b>let</b> taker_cancellation_reason =
        <b>if</b> (taker_cancellation_reason_str.is_some()) {
            <a href="order_placement.md#0x7_order_placement_cancel_taker_order_internal">cancel_taker_order_internal</a>(
                market,
                user_addr,
                price,
                order_id,
                client_order_id,
                orig_size,
                *remaining_size,
                *fill_sizes,
                0, // match_count - doesn't matter <b>as</b> we don't <b>use</b> the result.
                is_bid,
                <a href="market_types.md#0x7_market_types_order_cancellation_reason_clearinghouse_settle_violation">market_types::order_cancellation_reason_clearinghouse_settle_violation</a>(),
                taker_cancellation_reason_str.destroy_some(),
                <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(), // trigger_condition
                metadata,
                time_in_force,
                <b>false</b>, // emit_order_open is <b>false</b> <b>as</b> the order was already open
                callbacks,
                <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[]
            );
            <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(
                <a href="market_types.md#0x7_market_types_order_cancellation_reason_clearinghouse_settle_violation">market_types::order_cancellation_reason_clearinghouse_settle_violation</a>()
            )
        } <b>else</b> {
            <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
        };
    <b>if</b> (maker_cancellation_reason_str.is_some()) {
        <a href="order_placement.md#0x7_order_placement_cancel_maker_order_internal">cancel_maker_order_internal</a>(
            market,
            &maker_order,
            maker_order.get_client_order_id_from_match_details(),
            maker_order.get_account_from_match_details(),
            maker_order.get_order_id_from_match_details(),
            <a href="market_types.md#0x7_market_types_order_cancellation_reason_clearinghouse_settle_violation">market_types::order_cancellation_reason_clearinghouse_settle_violation</a>(),
            maker_cancellation_reason_str.destroy_some(),
            unsettled_maker_size,
            maker_order.get_metadata_from_match_details(),
            maker_order.get_time_in_force_from_match_details(),
            callbacks
        );
    } <b>else</b> {
        <b>if</b> (unsettled_maker_size &gt; 0) {
            //  we need <b>to</b> re-insert the maker order back into the order book
            <b>let</b> reinsertion_request =
                maker_order.new_order_match_details_with_modified_size(
                    unsettled_maker_size
                );
            market.get_order_book_mut().reinsert_order(
                reinsertion_request, &maker_order
            );
        } <b>else</b> <b>if</b> (maker_order.get_remaining_size_from_match_details() == 0) {
            <a href="order_placement.md#0x7_order_placement_cleanup_order_internal">cleanup_order_internal</a>(
                maker_order.get_account_from_match_details(),
                maker_order.get_order_id_from_match_details(),
                maker_order.get_client_order_id_from_match_details(),
                maker_order.get_book_type_from_match_details(),
                !is_bid, // is_bid is inverted for maker orders
                maker_order.get_time_in_force_from_match_details(),
                0, // 0 because the order is fully filled
                maker_order.get_price_from_match_details(),
                <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(), // trigger_condition
                maker_order.get_metadata_from_match_details(),
                callbacks,
                <b>false</b> // is_taker is <b>false</b> for maker orders
            );
        }
    };
    (taker_cancellation_reason, *settle_result.get_callback_result())
}
</code></pre>



</details>

<a id="0x7_order_placement_place_order_with_order_id"></a>

## Function `place_order_with_order_id`

Core function to place an order with a given order id. If the order id is not provided, a new order id is generated.
The function itself doesn't do any validation of the user_address, it's up to the caller to ensure that signer validation
is done before calling this function if needed.


<pre><code><b>public</b> <b>fun</b> <a href="order_placement.md#0x7_order_placement_place_order_with_order_id">place_order_with_order_id</a>&lt;M: <b>copy</b>, drop, store, R: <b>copy</b>, drop, store&gt;(market: &<b>mut</b> <a href="market_types.md#0x7_market_types_Market">market_types::Market</a>&lt;M&gt;, user_addr: <b>address</b>, limit_price: u64, orig_size: u64, remaining_size: u64, is_bid: bool, time_in_force: <a href="_TimeInForce">order_book_types::TimeInForce</a>, trigger_condition: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="_TriggerCondition">order_book_types::TriggerCondition</a>&gt;, metadata: M, order_id: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="_OrderId">order_book_types::OrderId</a>&gt;, client_order_id: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, max_match_limit: u32, cancel_on_match_limit: bool, emit_taker_order_open: bool, callbacks: &<a href="market_types.md#0x7_market_types_MarketClearinghouseCallbacks">market_types::MarketClearinghouseCallbacks</a>&lt;M, R&gt;): <a href="order_placement.md#0x7_order_placement_OrderMatchResult">order_placement::OrderMatchResult</a>&lt;R&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_placement.md#0x7_order_placement_place_order_with_order_id">place_order_with_order_id</a>&lt;M: store + <b>copy</b> + drop, R: store + <b>copy</b> + drop&gt;(
    market: &<b>mut</b> Market&lt;M&gt;,
    user_addr: <b>address</b>,
    limit_price: u64,
    orig_size: u64,
    remaining_size: u64,
    is_bid: bool,
    time_in_force: TimeInForce,
    trigger_condition: Option&lt;TriggerCondition&gt;,
    metadata: M,
    order_id: Option&lt;OrderId&gt;,
    client_order_id: Option&lt;String&gt;,
    max_match_limit: u32,
    cancel_on_match_limit: bool,
    emit_taker_order_open: bool,
    callbacks: &MarketClearinghouseCallbacks&lt;M, R&gt;
): <a href="order_placement.md#0x7_order_placement_OrderMatchResult">OrderMatchResult</a>&lt;R&gt; {
    <b>assert</b>!(
        orig_size &gt; 0
            && remaining_size &gt; 0
            && orig_size &gt;= remaining_size,
        <a href="order_placement.md#0x7_order_placement_EINVALID_ORDER">EINVALID_ORDER</a>
    );
    <b>assert</b>!(max_match_limit &gt; 0, <a href="order_placement.md#0x7_order_placement_EINVALID_ORDER">EINVALID_ORDER</a>);
    <b>assert</b>!(limit_price &gt; 0, <a href="order_placement.md#0x7_order_placement_EINVALID_ORDER">EINVALID_ORDER</a>);
    <b>if</b> (client_order_id.is_some()) {
        <b>assert</b>!(
            client_order_id.borrow().length() &lt;= <a href="order_placement.md#0x7_order_placement_MAX_CLIENT_ORDER_ID_LENGTH">MAX_CLIENT_ORDER_ID_LENGTH</a>,
            <a href="order_placement.md#0x7_order_placement_ECLIENT_ORDER_ID_LENGTH_EXCEEDED">ECLIENT_ORDER_ID_LENGTH_EXCEEDED</a>
        );
    };
    <b>if</b> (order_id.is_none()) {
        // If order id is not provided, generate a new order id
        order_id = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(next_order_id());
    };
    <b>let</b> order_id = order_id.destroy_some();
    <b>let</b> callback_results = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>();
    <b>let</b> validation_result =
        callbacks.validate_order_placement(
            new_clearinghouse_order_info(
                user_addr,
                order_id,
                client_order_id,
                is_bid,
                limit_price,
                time_in_force,
                single_order_type(),
                trigger_condition,
                metadata
            ),
            remaining_size
        );
    <b>if</b> (!validation_result.is_validation_result_valid()) {
        <b>return</b> <a href="order_placement.md#0x7_order_placement_cancel_taker_order_internal">cancel_taker_order_internal</a>(
            market,
            user_addr,
            limit_price,
            order_id,
            client_order_id,
            orig_size,
            remaining_size,
            <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[],
            0, // match_count
            is_bid,
            <a href="market_types.md#0x7_market_types_order_cancellation_reason_position_update_violation">market_types::order_cancellation_reason_position_update_violation</a>(),
            validation_result.get_validation_failure_reason().destroy_some(),
            trigger_condition,
            metadata,
            time_in_force,
            <b>true</b>, // emit_order_open
            callbacks,
            <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[]
        );
    };

    <b>if</b> (client_order_id.is_some()) {
        <b>if</b> (market.get_order_book().client_order_id_exists(
            user_addr, client_order_id.destroy_some()
        )) {
            // Client provided a client order id that already <b>exists</b> in the order book
            <b>return</b> <a href="order_placement.md#0x7_order_placement_cancel_taker_order_internal">cancel_taker_order_internal</a>(
                market,
                user_addr,
                limit_price,
                order_id,
                client_order_id,
                orig_size,
                remaining_size,
                <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[],
                0, // match_count
                is_bid,
                <a href="market_types.md#0x7_market_types_order_cancellation_reason_duplicate_client_order_id">market_types::order_cancellation_reason_duplicate_client_order_id</a>(),
                std::string::utf8(b"Duplicate client order id"),
                trigger_condition,
                metadata,
                time_in_force,
                <b>true</b>, // emit_order_open
                callbacks,
                <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[]
            );
        };

        <b>if</b> (is_pre_cancelled(
            market.get_pre_cancellation_tracker_mut(),
            user_addr,
            client_order_id.destroy_some()
        )) {
            <b>return</b> <a href="order_placement.md#0x7_order_placement_cancel_taker_order_internal">cancel_taker_order_internal</a>(
                market,
                user_addr,
                limit_price,
                order_id,
                client_order_id,
                orig_size,
                remaining_size,
                <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[],
                0, // match_count
                is_bid,
                <a href="market_types.md#0x7_market_types_order_cancellation_reason_order_pre_cancelled">market_types::order_cancellation_reason_order_pre_cancelled</a>(),
                std::string::utf8(b"Order pre cancelled"),
                trigger_condition,
                metadata,
                time_in_force,
                <b>true</b>, // emit_order_open
                callbacks,
                <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[]
            );
        };
    };
    <b>let</b> is_taker_order =
        market.get_order_book().is_taker_order(
            limit_price, is_bid, trigger_condition
        );

    <b>if</b> (!is_taker_order) {
        <b>return</b> <a href="order_placement.md#0x7_order_placement_place_maker_order_internal">place_maker_order_internal</a>(
            market,
            user_addr,
            limit_price,
            orig_size,
            remaining_size,
            <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[],
            0, // match_count
            is_bid,
            time_in_force,
            trigger_condition,
            metadata,
            order_id,
            client_order_id,
            emit_taker_order_open, // order_open_emitted
            callbacks,
            <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[]
        );
    };

    // NOTE: We should always <b>use</b> is_taker: <b>true</b> for this order past this
    // point so that indexer can consistently track the order's status
    <b>if</b> (time_in_force == post_only()) {
        <b>return</b> <a href="order_placement.md#0x7_order_placement_cancel_taker_order_internal">cancel_taker_order_internal</a>(
            market,
            user_addr,
            limit_price,
            order_id,
            client_order_id,
            orig_size,
            remaining_size,
            <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[],
            0, // match_count
            is_bid,
            <a href="market_types.md#0x7_market_types_order_cancellation_reason_post_only_violation">market_types::order_cancellation_reason_post_only_violation</a>(),
            std::string::utf8(b"Post Only violation"),
            <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(), // trigger_condition
            metadata,
            time_in_force,
            <b>true</b>, // emit_order_open
            callbacks,
            <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[]
        );
    };

    <b>if</b> (emit_taker_order_open) {
        // We don't emit order open events for orders <b>with</b> trigger conditions <b>as</b> they are not
        // actually placed in the order book until they are triggered.
        market.emit_event_for_order(
            order_id,
            client_order_id,
            user_addr,
            orig_size,
            remaining_size,
            orig_size,
            limit_price,
            is_bid,
            is_taker_order,
            <a href="market_types.md#0x7_market_types_order_status_open">market_types::order_status_open</a>(),
            std::string::utf8(b""),
            metadata,
            trigger_condition,
            time_in_force,
            <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(),
            callbacks
        );
    };

    <b>let</b> fill_sizes = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>();
    <b>let</b> match_count = 0;
    <b>loop</b> {
        match_count += 1;
        <b>let</b> (taker_cancellation_reason, callback_result) =
            <a href="order_placement.md#0x7_order_placement_settle_single_trade">settle_single_trade</a>(
                market,
                user_addr,
                limit_price,
                orig_size,
                &<b>mut</b> remaining_size,
                is_bid,
                metadata,
                order_id,
                client_order_id,
                callbacks,
                time_in_force,
                &<b>mut</b> fill_sizes
            );
        <b>let</b> should_stop = callback_result.should_stop_matching();
        <b>let</b> result = callback_result.extract_results();
        <b>if</b> (result.is_some()) {
            callback_results.push_back(result.destroy_some());
        };
        <b>if</b> (taker_cancellation_reason.is_some()) {
            <b>return</b> OrderMatchResult::V1 {
                order_id,
                remaining_size: 0, // 0 because the order is cancelled
                cancel_reason: taker_cancellation_reason,
                fill_sizes,
                callback_results,
                match_count
            }
        };
        <b>if</b> (remaining_size == 0) {
            <a href="order_placement.md#0x7_order_placement_cleanup_order_internal">cleanup_order_internal</a>(
                user_addr,
                order_id,
                client_order_id,
                single_order_type(),
                is_bid,
                time_in_force,
                0,
                limit_price,
                trigger_condition,
                metadata,
                callbacks,
                <b>true</b>
            );
            <b>break</b>;
        };
        <b>if</b> (should_stop) {
            <b>return</b> OrderMatchResult::V1 {
                order_id,
                remaining_size,
                cancel_reason: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(
                    order_cancellation_reason_clearinghouse_stopped_matching()
                ),
                fill_sizes,
                callback_results,
                match_count
            }
        };
        // Check <b>if</b> the next iteration will still match
        <b>let</b> is_taker_order = market.is_taker_order(
            limit_price, is_bid, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
        );
        <b>if</b> (!is_taker_order) {
            <b>if</b> (time_in_force == immediate_or_cancel()) {
                <b>return</b> <a href="order_placement.md#0x7_order_placement_cancel_taker_order_internal">cancel_taker_order_internal</a>(
                    market,
                    user_addr,
                    limit_price,
                    order_id,
                    client_order_id,
                    orig_size,
                    remaining_size,
                    fill_sizes,
                    match_count,
                    is_bid,
                    <a href="market_types.md#0x7_market_types_order_cancellation_reason_ioc_violation">market_types::order_cancellation_reason_ioc_violation</a>(),
                    std::string::utf8(b"IOC_VIOLATION"),
                    <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(), // trigger_condition
                    metadata,
                    time_in_force,
                    <b>false</b>, // emit_order_open is <b>false</b> <b>as</b> the order was already open
                    callbacks,
                    callback_results
                );
            } <b>else</b> {
                // If the order is not a taker order, then we can place it <b>as</b> a maker order
                <b>return</b> <a href="order_placement.md#0x7_order_placement_place_maker_order_internal">place_maker_order_internal</a>(
                    market,
                    user_addr,
                    limit_price,
                    orig_size,
                    remaining_size,
                    fill_sizes,
                    match_count,
                    is_bid,
                    time_in_force,
                    <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(), // trigger_condition
                    metadata,
                    order_id,
                    client_order_id,
                    <b>false</b>,
                    callbacks,
                    callback_results
                );
            };
        };

        <b>if</b> (match_count &gt;= max_match_limit) {
            <b>if</b> (cancel_on_match_limit) {
                <b>return</b> <a href="order_placement.md#0x7_order_placement_cancel_taker_order_internal">cancel_taker_order_internal</a>(
                    market,
                    user_addr,
                    limit_price,
                    order_id,
                    client_order_id,
                    orig_size,
                    remaining_size,
                    fill_sizes,
                    match_count,
                    is_bid,
                    <a href="market_types.md#0x7_market_types_order_cancellation_reason_max_fill_limit_violation">market_types::order_cancellation_reason_max_fill_limit_violation</a>(),
                    std::string::utf8(b"Max fill limit reached"),
                    <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(), // trigger_condition
                    metadata,
                    time_in_force,
                    <b>false</b>, // emit_order_open is <b>false</b> <b>as</b> the order was already open
                    callbacks,
                    callback_results
                );
            } <b>else</b> {
                <b>return</b> OrderMatchResult::V1 {
                    order_id,
                    remaining_size,
                    cancel_reason: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(
                        <a href="market_types.md#0x7_market_types_order_cancellation_reason_max_fill_limit_violation">market_types::order_cancellation_reason_max_fill_limit_violation</a>()
                    ),
                    callback_results,
                    fill_sizes,
                    match_count
                }
            };
        };
    };
    OrderMatchResult::V1 {
        order_id,
        remaining_size,
        cancel_reason: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(),
        fill_sizes,
        callback_results,
        match_count
    }
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
