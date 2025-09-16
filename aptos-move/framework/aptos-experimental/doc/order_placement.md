
<a id="0x7_order_placement"></a>

# Module `0x7::order_placement`

This module provides a generic trading engine implementation for a market. On a high level, its a data structure,
that stores an order book and provides APIs to place orders, cancel orders, and match orders. The market also acts
as a wrapper around the order book and pluggable clearinghouse implementation.
A clearing house implementation is expected to implement the following APIs
- settle_trade(taker, taker_order_id, maker, maker_order_id, fill_id, is_taker_long, price, size): SettleTradeResult ->
Called by the market when there is an match between taker and maker. The clearinghouse is expected to settle the trade
and return the result. Please note that the clearing house settlment size might not be the same as the order match size and
the settlement might also fail. The fill_id is an incremental counter for matched orders and can be used to track specific fills
- validate_order_placement(account, is_taker, is_long, price, size): bool -> Called by the market to validate
an order when its placed. The clearinghouse is expected to validate the order and return true if the order is valid.
Checkout clearinghouse_test as an example of the simplest form of clearing house implementation that just tracks
the position size of the user and does not do any validation.

- place_maker_order(account, order_id, is_bid, price, size, metadata) -> Called by the market before placing the
maker order in the order book. The clearinghouse can use this to track pending orders in the order book and perform
any other book keeping operations.

- cleanup_order(account, order_id, is_bid, remaining_size, order_metadata) -> Called by the market when an order is cancelled or fully filled
The clearinhouse can perform any cleanup operations like removing the order from the pending orders list. For every order placement
that passes the validate_order_placement check,
the market guarantees that the cleanup_order API will be called once and only once with the remaining size of the order.

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
on the order book until its trigger conditions are met. Following trigger conditions are supported:
TakeProfit(price): If its a buy order its triggered when the market price is greater than or equal to the price. If
its a sell order its triggered when the market price is less than or equal to the price.
StopLoss(price): If its a buy order its triggered when the market price is less than or equal to the price. If its
a sell order its triggered when the market price is greater than or equal to the price.
TimeBased(time): The order is triggered when the current time is greater than or equal to the time.


-  [Enum `OrderCancellationReason`](#0x7_order_placement_OrderCancellationReason)
-  [Struct `OrderMatchResult`](#0x7_order_placement_OrderMatchResult)
-  [Constants](#@Constants_0)
-  [Function `destroy_order_match_result`](#0x7_order_placement_destroy_order_match_result)
-  [Function `number_of_fills`](#0x7_order_placement_number_of_fills)
-  [Function `number_of_matches`](#0x7_order_placement_number_of_matches)
-  [Function `total_fill_size`](#0x7_order_placement_total_fill_size)
-  [Function `get_cancel_reason`](#0x7_order_placement_get_cancel_reason)
-  [Function `get_remaining_size_from_result`](#0x7_order_placement_get_remaining_size_from_result)
-  [Function `is_ioc_violation`](#0x7_order_placement_is_ioc_violation)
-  [Function `is_fill_limit_violation`](#0x7_order_placement_is_fill_limit_violation)
-  [Function `get_order_id`](#0x7_order_placement_get_order_id)
-  [Function `place_limit_order`](#0x7_order_placement_place_limit_order)
-  [Function `place_market_order`](#0x7_order_placement_place_market_order)
-  [Function `place_maker_order_internal`](#0x7_order_placement_place_maker_order_internal)
-  [Function `cancel_maker_order_internal`](#0x7_order_placement_cancel_maker_order_internal)
-  [Function `cancel_single_order_internal`](#0x7_order_placement_cancel_single_order_internal)
-  [Function `cleanup_order_internal`](#0x7_order_placement_cleanup_order_internal)
-  [Function `settle_single_trade`](#0x7_order_placement_settle_single_trade)
-  [Function `place_order_with_order_id`](#0x7_order_placement_place_order_with_order_id)


<pre><code><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
<b>use</b> <a href="market_types.md#0x7_market_types">0x7::market_types</a>;
<b>use</b> <a href="order_book.md#0x7_order_book">0x7::order_book</a>;
<b>use</b> <a href="order_book_types.md#0x7_order_book_types">0x7::order_book_types</a>;
<b>use</b> <a href="pre_cancellation_tracker.md#0x7_pre_cancellation_tracker">0x7::pre_cancellation_tracker</a>;
<b>use</b> <a href="single_order_book.md#0x7_single_order_book">0x7::single_order_book</a>;
<b>use</b> <a href="single_order_types.md#0x7_single_order_types">0x7::single_order_types</a>;
</code></pre>



<a id="0x7_order_placement_OrderCancellationReason"></a>

## Enum `OrderCancellationReason`



<pre><code>enum <a href="order_placement.md#0x7_order_placement_OrderCancellationReason">OrderCancellationReason</a> <b>has</b> <b>copy</b>, drop
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

</details>

<a id="0x7_order_placement_OrderMatchResult"></a>

## Struct `OrderMatchResult`



<pre><code><b>struct</b> <a href="order_placement.md#0x7_order_placement_OrderMatchResult">OrderMatchResult</a> <b>has</b> drop
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
<code>remaining_size: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>cancel_reason: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="order_placement.md#0x7_order_placement_OrderCancellationReason">order_placement::OrderCancellationReason</a>&gt;</code>
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

<a id="@Constants_0"></a>

## Constants


<a id="0x7_order_placement_ENOT_ADMIN"></a>



<pre><code><b>const</b> <a href="order_placement.md#0x7_order_placement_ENOT_ADMIN">ENOT_ADMIN</a>: u64 = 4;
</code></pre>



<a id="0x7_order_placement_U64_MAX"></a>



<pre><code><b>const</b> <a href="order_placement.md#0x7_order_placement_U64_MAX">U64_MAX</a>: u64 = 18446744073709551615;
</code></pre>



<a id="0x7_order_placement_EORDER_DOES_NOT_EXIST"></a>



<pre><code><b>const</b> <a href="order_placement.md#0x7_order_placement_EORDER_DOES_NOT_EXIST">EORDER_DOES_NOT_EXIST</a>: u64 = 6;
</code></pre>



<a id="0x7_order_placement_PRE_CANCELLATION_TRACKER_KEY"></a>



<pre><code><b>const</b> <a href="order_placement.md#0x7_order_placement_PRE_CANCELLATION_TRACKER_KEY">PRE_CANCELLATION_TRACKER_KEY</a>: u8 = 0;
</code></pre>



<a id="0x7_order_placement_EINVALID_FEE_TIER"></a>



<pre><code><b>const</b> <a href="order_placement.md#0x7_order_placement_EINVALID_FEE_TIER">EINVALID_FEE_TIER</a>: u64 = 5;
</code></pre>



<a id="0x7_order_placement_EINVALID_LIQUIDATION"></a>



<pre><code><b>const</b> <a href="order_placement.md#0x7_order_placement_EINVALID_LIQUIDATION">EINVALID_LIQUIDATION</a>: u64 = 11;
</code></pre>



<a id="0x7_order_placement_EINVALID_MATCHING_FOR_MAKER_REINSERT"></a>



<pre><code><b>const</b> <a href="order_placement.md#0x7_order_placement_EINVALID_MATCHING_FOR_MAKER_REINSERT">EINVALID_MATCHING_FOR_MAKER_REINSERT</a>: u64 = 9;
</code></pre>



<a id="0x7_order_placement_EINVALID_ORDER"></a>



<pre><code><b>const</b> <a href="order_placement.md#0x7_order_placement_EINVALID_ORDER">EINVALID_ORDER</a>: u64 = 1;
</code></pre>



<a id="0x7_order_placement_EINVALID_SETTLE_PRICE"></a>



<pre><code><b>const</b> <a href="order_placement.md#0x7_order_placement_EINVALID_SETTLE_PRICE">EINVALID_SETTLE_PRICE</a>: u64 = 13;
</code></pre>



<a id="0x7_order_placement_EINVALID_TAKER_POSITION_UPDATE"></a>



<pre><code><b>const</b> <a href="order_placement.md#0x7_order_placement_EINVALID_TAKER_POSITION_UPDATE">EINVALID_TAKER_POSITION_UPDATE</a>: u64 = 10;
</code></pre>



<a id="0x7_order_placement_EMARKET_NOT_FOUND"></a>



<pre><code><b>const</b> <a href="order_placement.md#0x7_order_placement_EMARKET_NOT_FOUND">EMARKET_NOT_FOUND</a>: u64 = 3;
</code></pre>



<a id="0x7_order_placement_ENOT_ORDER_CREATOR"></a>



<pre><code><b>const</b> <a href="order_placement.md#0x7_order_placement_ENOT_ORDER_CREATOR">ENOT_ORDER_CREATOR</a>: u64 = 12;
</code></pre>



<a id="0x7_order_placement_EORDER_BOOK_FULL"></a>



<pre><code><b>const</b> <a href="order_placement.md#0x7_order_placement_EORDER_BOOK_FULL">EORDER_BOOK_FULL</a>: u64 = 2;
</code></pre>



<a id="0x7_order_placement_destroy_order_match_result"></a>

## Function `destroy_order_match_result`



<pre><code><b>public</b> <b>fun</b> <a href="order_placement.md#0x7_order_placement_destroy_order_match_result">destroy_order_match_result</a>(self: <a href="order_placement.md#0x7_order_placement_OrderMatchResult">order_placement::OrderMatchResult</a>): (<a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>, u64, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="order_placement.md#0x7_order_placement_OrderCancellationReason">order_placement::OrderCancellationReason</a>&gt;, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, u32)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_placement.md#0x7_order_placement_destroy_order_match_result">destroy_order_match_result</a>(
    self: <a href="order_placement.md#0x7_order_placement_OrderMatchResult">OrderMatchResult</a>
): (OrderIdType, u64, Option&lt;<a href="order_placement.md#0x7_order_placement_OrderCancellationReason">OrderCancellationReason</a>&gt;, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, u32) {
    <b>let</b> <a href="order_placement.md#0x7_order_placement_OrderMatchResult">OrderMatchResult</a> { order_id, remaining_size, cancel_reason, fill_sizes, match_count } =
        self;
    (order_id, remaining_size, cancel_reason, fill_sizes, match_count)
}
</code></pre>



</details>

<a id="0x7_order_placement_number_of_fills"></a>

## Function `number_of_fills`



<pre><code><b>public</b> <b>fun</b> <a href="order_placement.md#0x7_order_placement_number_of_fills">number_of_fills</a>(self: &<a href="order_placement.md#0x7_order_placement_OrderMatchResult">order_placement::OrderMatchResult</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_placement.md#0x7_order_placement_number_of_fills">number_of_fills</a>(self: &<a href="order_placement.md#0x7_order_placement_OrderMatchResult">OrderMatchResult</a>): u64 {
    self.fill_sizes.length()
}
</code></pre>



</details>

<a id="0x7_order_placement_number_of_matches"></a>

## Function `number_of_matches`

Includes fills and cancels


<pre><code><b>public</b> <b>fun</b> <a href="order_placement.md#0x7_order_placement_number_of_matches">number_of_matches</a>(self: &<a href="order_placement.md#0x7_order_placement_OrderMatchResult">order_placement::OrderMatchResult</a>): u32
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_placement.md#0x7_order_placement_number_of_matches">number_of_matches</a>(self: &<a href="order_placement.md#0x7_order_placement_OrderMatchResult">OrderMatchResult</a>): u32 {
    self.match_count
}
</code></pre>



</details>

<a id="0x7_order_placement_total_fill_size"></a>

## Function `total_fill_size`



<pre><code><b>public</b> <b>fun</b> <a href="order_placement.md#0x7_order_placement_total_fill_size">total_fill_size</a>(self: &<a href="order_placement.md#0x7_order_placement_OrderMatchResult">order_placement::OrderMatchResult</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_placement.md#0x7_order_placement_total_fill_size">total_fill_size</a>(self: &<a href="order_placement.md#0x7_order_placement_OrderMatchResult">OrderMatchResult</a>): u64 {
    self.fill_sizes.fold(0, |acc, fill_size| acc + fill_size)
}
</code></pre>



</details>

<a id="0x7_order_placement_get_cancel_reason"></a>

## Function `get_cancel_reason`



<pre><code><b>public</b> <b>fun</b> <a href="order_placement.md#0x7_order_placement_get_cancel_reason">get_cancel_reason</a>(self: &<a href="order_placement.md#0x7_order_placement_OrderMatchResult">order_placement::OrderMatchResult</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="order_placement.md#0x7_order_placement_OrderCancellationReason">order_placement::OrderCancellationReason</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_placement.md#0x7_order_placement_get_cancel_reason">get_cancel_reason</a>(self: &<a href="order_placement.md#0x7_order_placement_OrderMatchResult">OrderMatchResult</a>): Option&lt;<a href="order_placement.md#0x7_order_placement_OrderCancellationReason">OrderCancellationReason</a>&gt; {
    self.cancel_reason
}
</code></pre>



</details>

<a id="0x7_order_placement_get_remaining_size_from_result"></a>

## Function `get_remaining_size_from_result`



<pre><code><b>public</b> <b>fun</b> <a href="order_placement.md#0x7_order_placement_get_remaining_size_from_result">get_remaining_size_from_result</a>(self: &<a href="order_placement.md#0x7_order_placement_OrderMatchResult">order_placement::OrderMatchResult</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_placement.md#0x7_order_placement_get_remaining_size_from_result">get_remaining_size_from_result</a>(self: &<a href="order_placement.md#0x7_order_placement_OrderMatchResult">OrderMatchResult</a>): u64 {
    self.remaining_size
}
</code></pre>



</details>

<a id="0x7_order_placement_is_ioc_violation"></a>

## Function `is_ioc_violation`



<pre><code><b>public</b> <b>fun</b> <a href="order_placement.md#0x7_order_placement_is_ioc_violation">is_ioc_violation</a>(self: <a href="order_placement.md#0x7_order_placement_OrderCancellationReason">order_placement::OrderCancellationReason</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_placement.md#0x7_order_placement_is_ioc_violation">is_ioc_violation</a>(self: <a href="order_placement.md#0x7_order_placement_OrderCancellationReason">OrderCancellationReason</a>): bool {
    <b>return</b> self == OrderCancellationReason::IOCViolation
}
</code></pre>



</details>

<a id="0x7_order_placement_is_fill_limit_violation"></a>

## Function `is_fill_limit_violation`



<pre><code><b>public</b> <b>fun</b> <a href="order_placement.md#0x7_order_placement_is_fill_limit_violation">is_fill_limit_violation</a>(cancel_reason: <a href="order_placement.md#0x7_order_placement_OrderCancellationReason">order_placement::OrderCancellationReason</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_placement.md#0x7_order_placement_is_fill_limit_violation">is_fill_limit_violation</a>(
    cancel_reason: <a href="order_placement.md#0x7_order_placement_OrderCancellationReason">OrderCancellationReason</a>
): bool {
    <b>return</b> cancel_reason == OrderCancellationReason::MaxFillLimitViolation
}
</code></pre>



</details>

<a id="0x7_order_placement_get_order_id"></a>

## Function `get_order_id`



<pre><code><b>public</b> <b>fun</b> <a href="order_placement.md#0x7_order_placement_get_order_id">get_order_id</a>(self: <a href="order_placement.md#0x7_order_placement_OrderMatchResult">order_placement::OrderMatchResult</a>): <a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_placement.md#0x7_order_placement_get_order_id">get_order_id</a>(self: <a href="order_placement.md#0x7_order_placement_OrderMatchResult">OrderMatchResult</a>): OrderIdType {
    self.order_id
}
</code></pre>



</details>

<a id="0x7_order_placement_place_limit_order"></a>

## Function `place_limit_order`

Places a limt order - If its a taker order, it will be matched immediately and if its a maker order, it will simply
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
This is used ful as the caller might not want to cancel the order when the limit is reached and can continue
that order in a separate transaction.
- callbacks: The callbacks for the market clearinghouse. This is a struct that implements the MarketClearinghouseCallbacks
interface. This is used to validate the order and settle the trade.
Returns the order id, remaining size, cancel reason and number of fills for the order.


<pre><code><b>public</b> <b>fun</b> <a href="order_placement.md#0x7_order_placement_place_limit_order">place_limit_order</a>&lt;M: <b>copy</b>, drop, store&gt;(market: &<b>mut</b> <a href="market_types.md#0x7_market_types_Market">market_types::Market</a>&lt;M&gt;, user: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, limit_price: u64, orig_size: u64, is_bid: bool, time_in_force: <a href="order_book_types.md#0x7_order_book_types_TimeInForce">order_book_types::TimeInForce</a>, trigger_condition: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="order_book_types.md#0x7_order_book_types_TriggerCondition">order_book_types::TriggerCondition</a>&gt;, metadata: M, client_order_id: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;, max_match_limit: u32, cancel_on_match_limit: bool, callbacks: &<a href="market_types.md#0x7_market_types_MarketClearinghouseCallbacks">market_types::MarketClearinghouseCallbacks</a>&lt;M&gt;): <a href="order_placement.md#0x7_order_placement_OrderMatchResult">order_placement::OrderMatchResult</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_placement.md#0x7_order_placement_place_limit_order">place_limit_order</a>&lt;M: store + <b>copy</b> + drop&gt;(
    market: &<b>mut</b> Market&lt;M&gt;,
    user: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    limit_price: u64,
    orig_size: u64,
    is_bid: bool,
    time_in_force: TimeInForce,
    trigger_condition: Option&lt;TriggerCondition&gt;,
    metadata: M,
    client_order_id: Option&lt;u64&gt;,
    max_match_limit: u32,
    cancel_on_match_limit: bool,
    callbacks: &MarketClearinghouseCallbacks&lt;M&gt;
): <a href="order_placement.md#0x7_order_placement_OrderMatchResult">OrderMatchResult</a> {
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


<pre><code><b>public</b> <b>fun</b> <a href="order_placement.md#0x7_order_placement_place_market_order">place_market_order</a>&lt;M: <b>copy</b>, drop, store&gt;(market: &<b>mut</b> <a href="market_types.md#0x7_market_types_Market">market_types::Market</a>&lt;M&gt;, user: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, orig_size: u64, is_bid: bool, metadata: M, client_order_id: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;, max_match_limit: u32, cancel_on_match_limit: bool, callbacks: &<a href="market_types.md#0x7_market_types_MarketClearinghouseCallbacks">market_types::MarketClearinghouseCallbacks</a>&lt;M&gt;): <a href="order_placement.md#0x7_order_placement_OrderMatchResult">order_placement::OrderMatchResult</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_placement.md#0x7_order_placement_place_market_order">place_market_order</a>&lt;M: store + <b>copy</b> + drop&gt;(
    market: &<b>mut</b> Market&lt;M&gt;,
    user: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    orig_size: u64,
    is_bid: bool,
    metadata: M,
    client_order_id: Option&lt;u64&gt;,
    max_match_limit: u32,
    cancel_on_match_limit: bool,
    callbacks: &MarketClearinghouseCallbacks&lt;M&gt;
): <a href="order_placement.md#0x7_order_placement_OrderMatchResult">OrderMatchResult</a> {
    <a href="order_placement.md#0x7_order_placement_place_order_with_order_id">place_order_with_order_id</a>(
        market,
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(user),
        <b>if</b> (is_bid) { <a href="order_placement.md#0x7_order_placement_U64_MAX">U64_MAX</a> } <b>else</b> { 1 },
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



<pre><code><b>fun</b> <a href="order_placement.md#0x7_order_placement_place_maker_order_internal">place_maker_order_internal</a>&lt;M: <b>copy</b>, drop, store&gt;(market: &<b>mut</b> <a href="market_types.md#0x7_market_types_Market">market_types::Market</a>&lt;M&gt;, user_addr: <b>address</b>, limit_price: u64, orig_size: u64, remaining_size: u64, fill_sizes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, match_count: u32, is_bid: bool, time_in_force: <a href="order_book_types.md#0x7_order_book_types_TimeInForce">order_book_types::TimeInForce</a>, trigger_condition: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="order_book_types.md#0x7_order_book_types_TriggerCondition">order_book_types::TriggerCondition</a>&gt;, metadata: M, order_id: <a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>, client_order_id: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;, emit_order_open: bool, callbacks: &<a href="market_types.md#0x7_market_types_MarketClearinghouseCallbacks">market_types::MarketClearinghouseCallbacks</a>&lt;M&gt;): <a href="order_placement.md#0x7_order_placement_OrderMatchResult">order_placement::OrderMatchResult</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="order_placement.md#0x7_order_placement_place_maker_order_internal">place_maker_order_internal</a>&lt;M: store + <b>copy</b> + drop&gt;(
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
    order_id: OrderIdType,
    client_order_id: Option&lt;u64&gt;,
    emit_order_open: bool,
    callbacks: &MarketClearinghouseCallbacks&lt;M&gt;
): <a href="order_placement.md#0x7_order_placement_OrderMatchResult">OrderMatchResult</a> {
    <b>if</b> (time_in_force == immediate_or_cancel() && trigger_condition.is_none()) {
        <b>return</b> <a href="order_placement.md#0x7_order_placement_cancel_single_order_internal">cancel_single_order_internal</a>(
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
            <b>false</b>, // is_taker
            OrderCancellationReason::IOCViolation,
            std::string::utf8(b"IOC Violation"),
            metadata,
            time_in_force,
            callbacks
        );
    };

    <b>if</b> (emit_order_open) {
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
            <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(metadata),
            trigger_condition,
            time_in_force,
            callbacks
        );
    };

    callbacks.place_maker_order(
        user_addr,
        order_id,
        is_bid,
        limit_price,
        remaining_size,
        metadata
    );
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
    <b>return</b> <a href="order_placement.md#0x7_order_placement_OrderMatchResult">OrderMatchResult</a> {
        order_id,
        remaining_size,
        cancel_reason: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(),
        fill_sizes,
        match_count
    }
}
</code></pre>



</details>

<a id="0x7_order_placement_cancel_maker_order_internal"></a>

## Function `cancel_maker_order_internal`



<pre><code><b>fun</b> <a href="order_placement.md#0x7_order_placement_cancel_maker_order_internal">cancel_maker_order_internal</a>&lt;M: <b>copy</b>, drop, store&gt;(market: &<b>mut</b> <a href="market_types.md#0x7_market_types_Market">market_types::Market</a>&lt;M&gt;, maker_order: &<a href="order_book_types.md#0x7_order_book_types_OrderMatchDetails">order_book_types::OrderMatchDetails</a>&lt;M&gt;, client_order_id: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;, maker_address: <b>address</b>, order_id: <a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>, maker_cancellation_reason: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, unsettled_size: u64, metadata: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;M&gt;, time_in_force: <a href="order_book_types.md#0x7_order_book_types_TimeInForce">order_book_types::TimeInForce</a>, callbacks: &<a href="market_types.md#0x7_market_types_MarketClearinghouseCallbacks">market_types::MarketClearinghouseCallbacks</a>&lt;M&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="order_placement.md#0x7_order_placement_cancel_maker_order_internal">cancel_maker_order_internal</a>&lt;M: store + <b>copy</b> + drop&gt;(
    market: &<b>mut</b> Market&lt;M&gt;,
    maker_order: &OrderMatchDetails&lt;M&gt;,
    client_order_id: Option&lt;u64&gt;,
    maker_address: <b>address</b>,
    order_id: OrderIdType,
    maker_cancellation_reason: String,
    unsettled_size: u64,
    metadata: Option&lt;M&gt;,
    time_in_force: TimeInForce,
    callbacks: &MarketClearinghouseCallbacks&lt;M&gt;
) {
    <b>let</b> maker_cancel_size = unsettled_size + maker_order.get_remaining_size_from_match_details();
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
        callbacks
    );
    // If the maker is invalid cancel the maker order and <b>continue</b> <b>to</b> the next maker order
    <b>if</b> (maker_order.get_remaining_size_from_match_details() != 0) {
        <b>let</b> order_book_type = maker_order.get_book_type_from_match_details();
        <b>if</b> (order_book_type == single_order_book_type()) {
            market.get_order_book_mut().cancel_order(maker_address, order_id);
        } <b>else</b> {
            market.get_order_book_mut().cancel_bulk_order(maker_address);
        }
    };
    <a href="order_placement.md#0x7_order_placement_cleanup_order_internal">cleanup_order_internal</a>(
        maker_address,
        order_id,
        maker_order.get_book_type_from_match_details(),
        maker_order.is_bid_from_match_details(),
        maker_cancel_size,
        metadata,
        callbacks
    );
}
</code></pre>



</details>

<a id="0x7_order_placement_cancel_single_order_internal"></a>

## Function `cancel_single_order_internal`



<pre><code><b>fun</b> <a href="order_placement.md#0x7_order_placement_cancel_single_order_internal">cancel_single_order_internal</a>&lt;M: <b>copy</b>, drop, store&gt;(market: &<b>mut</b> <a href="market_types.md#0x7_market_types_Market">market_types::Market</a>&lt;M&gt;, user_addr: <b>address</b>, limit_price: u64, order_id: <a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>, client_order_id: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;, orig_size: u64, size_delta: u64, fill_sizes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, match_count: u32, is_bid: bool, is_taker: bool, cancel_reason: <a href="order_placement.md#0x7_order_placement_OrderCancellationReason">order_placement::OrderCancellationReason</a>, cancel_details: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, metadata: M, time_in_force: <a href="order_book_types.md#0x7_order_book_types_TimeInForce">order_book_types::TimeInForce</a>, callbacks: &<a href="market_types.md#0x7_market_types_MarketClearinghouseCallbacks">market_types::MarketClearinghouseCallbacks</a>&lt;M&gt;): <a href="order_placement.md#0x7_order_placement_OrderMatchResult">order_placement::OrderMatchResult</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="order_placement.md#0x7_order_placement_cancel_single_order_internal">cancel_single_order_internal</a>&lt;M: store + <b>copy</b> + drop&gt;(
    market: &<b>mut</b> Market&lt;M&gt;,
    user_addr: <b>address</b>,
    limit_price: u64,
    order_id: OrderIdType,
    client_order_id: Option&lt;u64&gt;,
    orig_size: u64,
    size_delta: u64,
    fill_sizes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    match_count: u32,
    is_bid: bool,
    is_taker: bool,
    cancel_reason: <a href="order_placement.md#0x7_order_placement_OrderCancellationReason">OrderCancellationReason</a>,
    cancel_details: String,
    metadata: M,
    time_in_force: TimeInForce,
    callbacks: &MarketClearinghouseCallbacks&lt;M&gt;
): <a href="order_placement.md#0x7_order_placement_OrderMatchResult">OrderMatchResult</a> {
    market.emit_event_for_order(
        order_id,
        client_order_id,
        user_addr,
        orig_size,
        0,
        size_delta,
        limit_price,
        is_bid,
        is_taker,
        <a href="market_types.md#0x7_market_types_order_status_cancelled">market_types::order_status_cancelled</a>(),
        cancel_details,
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(metadata),
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(), // trigger_condition
        time_in_force,
        callbacks
    );
    callbacks.cleanup_order(
        user_addr, order_id, is_bid, size_delta, metadata
    );
    <b>return</b> <a href="order_placement.md#0x7_order_placement_OrderMatchResult">OrderMatchResult</a> {
        order_id,
        remaining_size: 0,
        cancel_reason: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(cancel_reason),
        fill_sizes,
        match_count
    }
}
</code></pre>



</details>

<a id="0x7_order_placement_cleanup_order_internal"></a>

## Function `cleanup_order_internal`



<pre><code><b>public</b> <b>fun</b> <a href="order_placement.md#0x7_order_placement_cleanup_order_internal">cleanup_order_internal</a>&lt;M: <b>copy</b>, drop, store&gt;(user_addr: <b>address</b>, order_id: <a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>, book_type: <a href="order_book_types.md#0x7_order_book_types_OrderBookType">order_book_types::OrderBookType</a>, is_bid: bool, remaining_size: u64, metadata: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;M&gt;, callbacks: &<a href="market_types.md#0x7_market_types_MarketClearinghouseCallbacks">market_types::MarketClearinghouseCallbacks</a>&lt;M&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_placement.md#0x7_order_placement_cleanup_order_internal">cleanup_order_internal</a>&lt;M: store + <b>copy</b> + drop&gt;(
    user_addr: <b>address</b>,
    order_id: OrderIdType,
    book_type: OrderBookType,
    is_bid: bool,
    remaining_size: u64,
    metadata: Option&lt;M&gt;,
    callbacks: &MarketClearinghouseCallbacks&lt;M&gt;
) {
    <b>if</b> (book_type == single_order_book_type()) {
        callbacks.cleanup_order(
            user_addr, order_id, is_bid, remaining_size, metadata.destroy_some()
        );
    } <b>else</b> {
        callbacks.cleanup_bulk_orders(
            user_addr, is_bid, remaining_size
        );
    }
}
</code></pre>



</details>

<a id="0x7_order_placement_settle_single_trade"></a>

## Function `settle_single_trade`



<pre><code><b>fun</b> <a href="order_placement.md#0x7_order_placement_settle_single_trade">settle_single_trade</a>&lt;M: <b>copy</b>, drop, store&gt;(market: &<b>mut</b> <a href="market_types.md#0x7_market_types_Market">market_types::Market</a>&lt;M&gt;, user_addr: <b>address</b>, price: u64, orig_size: u64, remaining_size: &<b>mut</b> u64, is_bid: bool, metadata: M, order_id: <a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>, client_order_id: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;, callbacks: &<a href="market_types.md#0x7_market_types_MarketClearinghouseCallbacks">market_types::MarketClearinghouseCallbacks</a>&lt;M&gt;, time_in_force: <a href="order_book_types.md#0x7_order_book_types_TimeInForce">order_book_types::TimeInForce</a>, fill_sizes: &<b>mut</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="order_placement.md#0x7_order_placement_OrderCancellationReason">order_placement::OrderCancellationReason</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="order_placement.md#0x7_order_placement_settle_single_trade">settle_single_trade</a>&lt;M: store + <b>copy</b> + drop&gt;(
    market: &<b>mut</b> Market&lt;M&gt;,
    user_addr: <b>address</b>,
    price: u64,
    orig_size: u64,
    remaining_size: &<b>mut</b> u64,
    is_bid: bool,
    metadata: M,
    order_id: OrderIdType,
    client_order_id: Option&lt;u64&gt;,
    callbacks: &MarketClearinghouseCallbacks&lt;M&gt;,
    time_in_force: TimeInForce,
    fill_sizes: &<b>mut</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
): Option&lt;<a href="order_placement.md#0x7_order_placement_OrderCancellationReason">OrderCancellationReason</a>&gt; {
    <b>let</b> result =
        market.get_order_book_mut()
            .get_single_match_for_taker(price, *remaining_size, is_bid);
    <b>let</b> (maker_order, maker_matched_size) = result.destroy_order_match();
    <b>if</b> (!market.is_allowed_self_trade() && maker_order.get_account_from_match_details() == user_addr) {
        <a href="order_placement.md#0x7_order_placement_cancel_maker_order_internal">cancel_maker_order_internal</a>(
            market,
            &maker_order,
            maker_order.get_client_order_id_from_match_details(),
            maker_order.get_account_from_match_details(),
            maker_order.get_order_id_from_match_details(),
            std::string::utf8(b"Disallowed self trading"),
            maker_matched_size,
            maker_order.get_metadata_from_match_details(),
            maker_order.get_time_in_force_from_match_details(),
            callbacks
        );
        <b>return</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>();
    };
    <b>let</b> fill_id = market.next_fill_id();
    <b>let</b> settle_result = callbacks.settle_trade(
        market,
        user_addr,
        order_id,
        maker_order.get_account_from_match_details(),
        maker_order.get_order_id_from_match_details(),
        fill_id,
        is_bid,
        price,
        maker_order.get_price_from_match_details(), // Order is usually matched at the price of the maker
        maker_matched_size,
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(metadata),
        // TODO(skedia) fix this <b>to</b> pass <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">option</a> <b>to</b> the callbacks
        maker_order.get_metadata_from_match_details()
    );

    <b>let</b> settled_price = settle_result.get_settled_price();
    <b>if</b> (is_bid) {
        <b>assert</b>!(settled_price &lt;= price, <a href="order_placement.md#0x7_order_placement_EINVALID_SETTLE_PRICE">EINVALID_SETTLE_PRICE</a>);
        <b>assert</b>!(settled_price &gt;= maker_order.get_price_from_match_details(), <a href="order_placement.md#0x7_order_placement_EINVALID_SETTLE_PRICE">EINVALID_SETTLE_PRICE</a>);
    } <b>else</b> {
        <b>assert</b>!(settled_price &gt;= price, <a href="order_placement.md#0x7_order_placement_EINVALID_SETTLE_PRICE">EINVALID_SETTLE_PRICE</a>);
        <b>assert</b>!(settled_price &lt;= maker_order.get_price_from_match_details(), <a href="order_placement.md#0x7_order_placement_EINVALID_SETTLE_PRICE">EINVALID_SETTLE_PRICE</a>);
    };

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
            settled_price,
            is_bid,
            <b>true</b>,
            <a href="market_types.md#0x7_market_types_order_status_filled">market_types::order_status_filled</a>(),
            std::string::utf8(b""),
            <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(metadata),
            <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(),
            time_in_force,
            callbacks
        );
        // Event for maker fill
        market.emit_event_for_order(
            maker_order.get_order_id_from_match_details(),
            maker_order.get_client_order_id_from_match_details(),
            maker_order.get_account_from_match_details(),
            maker_order.get_orig_size_from_match_details(),
            maker_order.get_remaining_size_from_match_details() + unsettled_maker_size,
            settled_size,
            settled_price,
            !is_bid,
            <b>false</b>,
            <a href="market_types.md#0x7_market_types_order_status_filled">market_types::order_status_filled</a>(),
            std::string::utf8(b""),
            maker_order.get_metadata_from_match_details(),
            <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(),
            maker_order.get_time_in_force_from_match_details(),
            callbacks
        );
    };

    <b>let</b> maker_cancellation_reason = settle_result.get_maker_cancellation_reason();

    <b>let</b> taker_cancellation_reason = settle_result.get_taker_cancellation_reason();
    <b>if</b> (taker_cancellation_reason.is_some()) {
        <a href="order_placement.md#0x7_order_placement_cancel_single_order_internal">cancel_single_order_internal</a>(
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
            <b>true</b>, // is_taker
            OrderCancellationReason::ClearinghouseSettleViolation,
            taker_cancellation_reason.destroy_some(),
            metadata,
            time_in_force,
            callbacks
        );
        <b>if</b> (maker_cancellation_reason.is_none() && unsettled_maker_size &gt; 0) {
            // If the taker is cancelled but the maker is not cancelled, then we need <b>to</b> re-insert
            // the maker order back into the order book
            <b>let</b> reinsertion_request = maker_order.new_order_match_details_with_modified_size(unsettled_maker_size);
            market.get_order_book_mut().reinsert_order(
                reinsertion_request,
                &maker_order
            );
        };
        <b>return</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(OrderCancellationReason::ClearinghouseSettleViolation);
    };
    <b>if</b> (maker_cancellation_reason.is_some()) {
        <a href="order_placement.md#0x7_order_placement_cancel_maker_order_internal">cancel_maker_order_internal</a>(
            market,
            &maker_order,
            maker_order.get_client_order_id_from_match_details(),
            maker_order.get_account_from_match_details(),
            maker_order.get_order_id_from_match_details(),
            maker_cancellation_reason.destroy_some(),
            unsettled_maker_size,
            maker_order.get_metadata_from_match_details(),
            maker_order.get_time_in_force_from_match_details(),
            callbacks
        );
    } <b>else</b> <b>if</b> (maker_order.get_remaining_size_from_match_details() == 0) {
        <a href="order_placement.md#0x7_order_placement_cleanup_order_internal">cleanup_order_internal</a>(
            maker_order.get_account_from_match_details(),
            maker_order.get_order_id_from_match_details(),
            maker_order.get_book_type_from_match_details(),
            !is_bid, // is_bid is inverted for maker orders
            0, // 0 because the order is fully filled
            maker_order.get_metadata_from_match_details(),
            callbacks
        );
    };
    <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
}
</code></pre>



</details>

<a id="0x7_order_placement_place_order_with_order_id"></a>

## Function `place_order_with_order_id`

Similar to <code>place_order</code> API but allows few extra parameters as follows
- order_id: The order id for the order - this is needed because for orders with trigger conditions, the order
id is generated when the order is placed and when they are triggered, the same order id is used to match the order.
- emit_taker_order_open: bool: Whether to emit an order open event for the taker order - this is used when
the caller do not wants to emit an open order event for a taker in case the taker order was intterrupted because
of fill limit violation  in the previous transaction and the order is just a continuation of the previous order.


<pre><code><b>public</b> <b>fun</b> <a href="order_placement.md#0x7_order_placement_place_order_with_order_id">place_order_with_order_id</a>&lt;M: <b>copy</b>, drop, store&gt;(market: &<b>mut</b> <a href="market_types.md#0x7_market_types_Market">market_types::Market</a>&lt;M&gt;, user_addr: <b>address</b>, limit_price: u64, orig_size: u64, remaining_size: u64, is_bid: bool, time_in_force: <a href="order_book_types.md#0x7_order_book_types_TimeInForce">order_book_types::TimeInForce</a>, trigger_condition: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="order_book_types.md#0x7_order_book_types_TriggerCondition">order_book_types::TriggerCondition</a>&gt;, metadata: M, order_id: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>&gt;, client_order_id: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;, max_match_limit: u32, cancel_on_match_limit: bool, emit_taker_order_open: bool, callbacks: &<a href="market_types.md#0x7_market_types_MarketClearinghouseCallbacks">market_types::MarketClearinghouseCallbacks</a>&lt;M&gt;): <a href="order_placement.md#0x7_order_placement_OrderMatchResult">order_placement::OrderMatchResult</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_placement.md#0x7_order_placement_place_order_with_order_id">place_order_with_order_id</a>&lt;M: store + <b>copy</b> + drop&gt;(
    market: &<b>mut</b> Market&lt;M&gt;,
    user_addr: <b>address</b>,
    limit_price: u64,
    orig_size: u64,
    remaining_size: u64,
    is_bid: bool,
    time_in_force: TimeInForce,
    trigger_condition: Option&lt;TriggerCondition&gt;,
    metadata: M,
    order_id: Option&lt;OrderIdType&gt;,
    client_order_id: Option&lt;u64&gt;,
    max_match_limit: u32,
    cancel_on_match_limit: bool,
    emit_taker_order_open: bool,
    callbacks: &MarketClearinghouseCallbacks&lt;M&gt;
): <a href="order_placement.md#0x7_order_placement_OrderMatchResult">OrderMatchResult</a> {
    <b>assert</b>!(
        orig_size &gt; 0 && remaining_size &gt; 0,
        <a href="order_placement.md#0x7_order_placement_EINVALID_ORDER">EINVALID_ORDER</a>
    );
    <b>if</b> (order_id.is_none()) {
        // If order id is not provided, generate a new order id
        order_id = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(market.next_order_id());
    };
    <b>let</b> order_id = order_id.destroy_some();
    // TODO(skedia) is_taker_order API can actually <b>return</b> <b>false</b> positive <b>as</b> the maker orders might not be valid.
    // Changes are needed <b>to</b> ensure the maker order is valid for this order <b>to</b> be a valid taker order.
    // TODO(skedia) reconsile the semantics around <b>global</b> order id vs <a href="../../aptos-framework/doc/account.md#0x1_account">account</a> <b>local</b> id.
    <b>let</b> is_taker_order =
        market.get_order_book().is_taker_order(limit_price, is_bid, trigger_condition);

    <b>if</b> (emit_taker_order_open && trigger_condition.is_none()) {
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
            <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(metadata),
            trigger_condition,
            time_in_force,
            callbacks
        );
    };

    <b>if</b> (
        !callbacks.validate_order_placement(
            user_addr,
            order_id,
            is_taker_order, // is_taker
            is_bid,
            limit_price,
            time_in_force,
            remaining_size,
            metadata
        )) {
        <b>return</b> <a href="order_placement.md#0x7_order_placement_cancel_single_order_internal">cancel_single_order_internal</a>(
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
            is_taker_order, // is_taker
            OrderCancellationReason::PositionUpdateViolation,
            std::string::utf8(b"Position Update violation"),
            metadata,
            time_in_force,
            callbacks
        );
    };

    <b>if</b> (client_order_id.is_some()) {
        <b>if</b> (market.get_order_book().client_order_id_exists(user_addr, client_order_id.destroy_some())) {
            // Client provided a client order id that already <b>exists</b> in the order book
            <b>return</b> <a href="order_placement.md#0x7_order_placement_cancel_single_order_internal">cancel_single_order_internal</a>(
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
                is_taker_order, // is_taker
                OrderCancellationReason::DuplicateClientOrderIdViolation,
                std::string::utf8(b"Duplicate client order id"),
                metadata,
                time_in_force,
                callbacks
            );
        };

        <b>if</b> (is_pre_cancelled(
            market.get_pre_cancellation_tracker_mut(),
            user_addr,
            client_order_id.destroy_some()
        )) {
            <b>return</b> <a href="order_placement.md#0x7_order_placement_cancel_single_order_internal">cancel_single_order_internal</a>(
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
                is_taker_order, // is_taker
                OrderCancellationReason::OrderPreCancelled,
                std::string::utf8(b"Order pre cancelled"),
                metadata,
                time_in_force,
                callbacks
            );
        };
    };

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
            <b>false</b>,
            callbacks
        );
    };

    // NOTE: We should always <b>use</b> is_taker: <b>true</b> for this order past this
    // point so that indexer can consistently track the order's status
    <b>if</b> (time_in_force == post_only()) {
        <b>return</b> <a href="order_placement.md#0x7_order_placement_cancel_single_order_internal">cancel_single_order_internal</a>(
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
            <b>true</b>, // is_taker
            OrderCancellationReason::PostOnlyViolation,
            std::string::utf8(b"Post Only violation"),
            metadata,
            time_in_force,
            callbacks
        );
    };
    <b>let</b> fill_sizes = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>();
    <b>let</b> match_count = 0;
    <b>loop</b> {
        match_count += 1;
        <b>let</b> taker_cancellation_reason =
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
        <b>if</b> (taker_cancellation_reason.is_some()) {
            <b>return</b> <a href="order_placement.md#0x7_order_placement_OrderMatchResult">OrderMatchResult</a> {
                order_id,
                remaining_size: 0, // 0 because the order is cancelled
                cancel_reason: taker_cancellation_reason,
                fill_sizes,
                match_count
            }
        };
        <b>if</b> (remaining_size == 0) {
            <a href="order_placement.md#0x7_order_placement_cleanup_order_internal">cleanup_order_internal</a>(
                user_addr, order_id, single_order_book_type(), is_bid, 0, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(metadata), callbacks
            );
            <b>break</b>;
        };

        // Check <b>if</b> the next iteration will still match
        <b>let</b> is_taker_order =
            market.is_taker_order(limit_price, is_bid, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>());
        <b>if</b> (!is_taker_order) {
            <b>if</b> (time_in_force == immediate_or_cancel()) {
                <b>return</b> <a href="order_placement.md#0x7_order_placement_cancel_single_order_internal">cancel_single_order_internal</a>(
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
                    <b>true</b>, // is_taker
                    OrderCancellationReason::IOCViolation,
                    std::string::utf8(b"IOC_VIOLATION"),
                    metadata,
                    time_in_force,
                    callbacks
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
                    trigger_condition,
                    metadata,
                    order_id,
                    client_order_id,
                    <b>true</b>, // emit_order_open
                    callbacks
                );
            };
        };

        <b>if</b> (match_count &gt;= max_match_limit) {
            <b>if</b> (cancel_on_match_limit) {
                <b>return</b> <a href="order_placement.md#0x7_order_placement_cancel_single_order_internal">cancel_single_order_internal</a>(
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
                    <b>true</b>, // is_taker
                    OrderCancellationReason::MaxFillLimitViolation,
                    std::string::utf8(b"Max fill limit reached"),
                    metadata,
                    time_in_force,
                    callbacks
                );
            } <b>else</b> {
                <b>return</b> <a href="order_placement.md#0x7_order_placement_OrderMatchResult">OrderMatchResult</a> {
                    order_id,
                    remaining_size,
                    cancel_reason: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(
                        OrderCancellationReason::MaxFillLimitViolation
                    ),
                    fill_sizes,
                    match_count
                }
            };
        };
    };
    <a href="order_placement.md#0x7_order_placement_OrderMatchResult">OrderMatchResult</a> {
        order_id,
        remaining_size,
        cancel_reason: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(),
        fill_sizes,
        match_count
    }
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
