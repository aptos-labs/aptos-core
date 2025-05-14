
<a id="0x7_market"></a>

# Module `0x7::market`

This module provides a generic trading engine implementation for a market. On a high level, its a data structure,
that stores an order book and provides APIs to place orders, cancel orders, and match orders. The market also acts
as a wrapper around the order book and pluggable clearinghouse implementation.
A clearing house implementation is expected to implement the following APIs
- settle_trade(taker, maker, is_taker_long, price, size): SettleTradeResult -> Called by the market when there
is an match between taker and maker. The clearinghouse is expected to settle the trade and return the result. Please
note that the clearing house settlment size might not be the same as the order match size and the settlement might
also fail.
- validate_order_placement(account, is_taker, is_long, price, size): bool -> Called by the market to validate
an order when its placed. The clearinghouse is expected to validate the order and return true if the order is valid.
Checkout clearinghouse_test as an example of the simplest form of clearing house implementation that just tracks
the position size of the user and does not do any validation.

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


-  [Struct `Market`](#0x7_market_Market)
-  [Struct `OrderEvent`](#0x7_market_OrderEvent)
-  [Enum `OrderCancellationReason`](#0x7_market_OrderCancellationReason)
-  [Struct `OrderMatchResult`](#0x7_market_OrderMatchResult)
-  [Constants](#@Constants_0)
-  [Function `good_till_cancelled`](#0x7_market_good_till_cancelled)
-  [Function `post_only`](#0x7_market_post_only)
-  [Function `immediate_or_cancel`](#0x7_market_immediate_or_cancel)
-  [Function `order_status_open`](#0x7_market_order_status_open)
-  [Function `order_status_filled`](#0x7_market_order_status_filled)
-  [Function `order_status_cancelled`](#0x7_market_order_status_cancelled)
-  [Function `order_status_rejected`](#0x7_market_order_status_rejected)
-  [Function `destroy_order_match_result`](#0x7_market_destroy_order_match_result)
-  [Function `number_of_fills`](#0x7_market_number_of_fills)
-  [Function `get_cancel_reason`](#0x7_market_get_cancel_reason)
-  [Function `get_remaining_size_from_result`](#0x7_market_get_remaining_size_from_result)
-  [Function `is_ioc_violation`](#0x7_market_is_ioc_violation)
-  [Function `is_fill_limit_violation`](#0x7_market_is_fill_limit_violation)
-  [Function `get_order_id`](#0x7_market_get_order_id)
-  [Function `new_market`](#0x7_market_new_market)
-  [Function `get_market`](#0x7_market_get_market)
-  [Function `get_order_book`](#0x7_market_get_order_book)
-  [Function `get_order_book_mut`](#0x7_market_get_order_book_mut)
-  [Function `best_bid_price`](#0x7_market_best_bid_price)
-  [Function `best_ask_price`](#0x7_market_best_ask_price)
-  [Function `is_taker_order`](#0x7_market_is_taker_order)
-  [Function `place_order`](#0x7_market_place_order)
-  [Function `next_order_id`](#0x7_market_next_order_id)
-  [Function `place_order_with_user_addr`](#0x7_market_place_order_with_user_addr)
-  [Function `place_maker_order`](#0x7_market_place_maker_order)
-  [Function `place_order_with_order_id`](#0x7_market_place_order_with_order_id)
-  [Function `cancel_order`](#0x7_market_cancel_order)
-  [Function `get_remaining_size`](#0x7_market_get_remaining_size)
-  [Function `take_ready_price_based_orders`](#0x7_market_take_ready_price_based_orders)
-  [Function `take_ready_time_based_orders`](#0x7_market_take_ready_time_based_orders)


<pre><code><b>use</b> <a href="../../aptos-framework/doc/event.md#0x1_event">0x1::event</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;
<b>use</b> <a href="market_types.md#0x7_market_types">0x7::market_types</a>;
<b>use</b> <a href="order_book.md#0x7_order_book">0x7::order_book</a>;
<b>use</b> <a href="order_book_types.md#0x7_order_book_types">0x7::order_book_types</a>;
</code></pre>



<a id="0x7_market_Market"></a>

## Struct `Market`



<pre><code><b>struct</b> <a href="market.md#0x7_market_Market">Market</a>&lt;M: <b>copy</b>, drop, store&gt; <b>has</b> store
</code></pre>



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
<code><a href="market.md#0x7_market">market</a>: <b>address</b></code>
</dt>
<dd>
 Address of the market object of this market.
</dd>
<dt>
<code>last_order_id: u64</code>
</dt>
<dd>

</dd>
<dt>
<code><a href="order_book.md#0x7_order_book">order_book</a>: <a href="order_book.md#0x7_order_book_OrderBook">order_book::OrderBook</a>&lt;M&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x7_market_OrderEvent"></a>

## Struct `OrderEvent`



<pre><code>#[<a href="../../aptos-framework/doc/event.md#0x1_event">event</a>]
<b>struct</b> <a href="market.md#0x7_market_OrderEvent">OrderEvent</a> <b>has</b> <b>copy</b>, drop, store
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
<code><a href="market.md#0x7_market">market</a>: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>order_id: u64</code>
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
<code>is_buy: bool</code>
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
<code>status: u8</code>
</dt>
<dd>

</dd>
<dt>
<code>details: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x7_market_OrderCancellationReason"></a>

## Enum `OrderCancellationReason`



<pre><code>enum <a href="market.md#0x7_market_OrderCancellationReason">OrderCancellationReason</a> <b>has</b> <b>copy</b>, drop
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

</details>

<a id="0x7_market_OrderMatchResult"></a>

## Struct `OrderMatchResult`



<pre><code><b>struct</b> <a href="market.md#0x7_market_OrderMatchResult">OrderMatchResult</a> <b>has</b> drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>order_id: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>remaining_size: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>cancel_reason: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="market.md#0x7_market_OrderCancellationReason">market::OrderCancellationReason</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>num_fills: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x7_market_ENOT_ADMIN"></a>



<pre><code><b>const</b> <a href="market.md#0x7_market_ENOT_ADMIN">ENOT_ADMIN</a>: u64 = 4;
</code></pre>



<a id="0x7_market_EINVALID_FEE_TIER"></a>



<pre><code><b>const</b> <a href="market.md#0x7_market_EINVALID_FEE_TIER">EINVALID_FEE_TIER</a>: u64 = 5;
</code></pre>



<a id="0x7_market_EINVALID_LIQUIDATION"></a>



<pre><code><b>const</b> <a href="market.md#0x7_market_EINVALID_LIQUIDATION">EINVALID_LIQUIDATION</a>: u64 = 11;
</code></pre>



<a id="0x7_market_EINVALID_MATCHING_FOR_MAKER_REINSERT"></a>



<pre><code><b>const</b> <a href="market.md#0x7_market_EINVALID_MATCHING_FOR_MAKER_REINSERT">EINVALID_MATCHING_FOR_MAKER_REINSERT</a>: u64 = 9;
</code></pre>



<a id="0x7_market_EINVALID_ORDER"></a>



<pre><code><b>const</b> <a href="market.md#0x7_market_EINVALID_ORDER">EINVALID_ORDER</a>: u64 = 1;
</code></pre>



<a id="0x7_market_EINVALID_TAKER_POSITION_UPDATE"></a>



<pre><code><b>const</b> <a href="market.md#0x7_market_EINVALID_TAKER_POSITION_UPDATE">EINVALID_TAKER_POSITION_UPDATE</a>: u64 = 10;
</code></pre>



<a id="0x7_market_EINVALID_TIME_IN_FORCE_FOR_MAKER"></a>



<pre><code><b>const</b> <a href="market.md#0x7_market_EINVALID_TIME_IN_FORCE_FOR_MAKER">EINVALID_TIME_IN_FORCE_FOR_MAKER</a>: u64 = 7;
</code></pre>



<a id="0x7_market_EINVALID_TIME_IN_FORCE_FOR_TAKER"></a>



<pre><code><b>const</b> <a href="market.md#0x7_market_EINVALID_TIME_IN_FORCE_FOR_TAKER">EINVALID_TIME_IN_FORCE_FOR_TAKER</a>: u64 = 8;
</code></pre>



<a id="0x7_market_EMARKET_NOT_FOUND"></a>



<pre><code><b>const</b> <a href="market.md#0x7_market_EMARKET_NOT_FOUND">EMARKET_NOT_FOUND</a>: u64 = 3;
</code></pre>



<a id="0x7_market_EORDER_BOOK_FULL"></a>



<pre><code><b>const</b> <a href="market.md#0x7_market_EORDER_BOOK_FULL">EORDER_BOOK_FULL</a>: u64 = 2;
</code></pre>



<a id="0x7_market_EORDER_DOES_NOT_EXIST"></a>



<pre><code><b>const</b> <a href="market.md#0x7_market_EORDER_DOES_NOT_EXIST">EORDER_DOES_NOT_EXIST</a>: u64 = 6;
</code></pre>



<a id="0x7_market_ORDER_STATUS_CANCELLED"></a>

Order has been cancelled by the user or engine.


<pre><code><b>const</b> <a href="market.md#0x7_market_ORDER_STATUS_CANCELLED">ORDER_STATUS_CANCELLED</a>: u8 = 2;
</code></pre>



<a id="0x7_market_ORDER_STATUS_FILLED"></a>

Order has been fully or partially filled.


<pre><code><b>const</b> <a href="market.md#0x7_market_ORDER_STATUS_FILLED">ORDER_STATUS_FILLED</a>: u8 = 1;
</code></pre>



<a id="0x7_market_ORDER_STATUS_OPEN"></a>

Order has been accepted by the engine.


<pre><code><b>const</b> <a href="market.md#0x7_market_ORDER_STATUS_OPEN">ORDER_STATUS_OPEN</a>: u8 = 0;
</code></pre>



<a id="0x7_market_ORDER_STATUS_REJECTED"></a>

Order has been rejected by the engine. Unlike cancelled orders, rejected
orders are invalid orders. Rejection reasons:
1. Insufficient margin
2. Order is reduce_only but does not reduce


<pre><code><b>const</b> <a href="market.md#0x7_market_ORDER_STATUS_REJECTED">ORDER_STATUS_REJECTED</a>: u8 = 3;
</code></pre>



<a id="0x7_market_TIME_IN_FORCE_GTC"></a>

Good till cancelled order type


<pre><code><b>const</b> <a href="market.md#0x7_market_TIME_IN_FORCE_GTC">TIME_IN_FORCE_GTC</a>: u8 = 0;
</code></pre>



<a id="0x7_market_TIME_IN_FORCE_IOC"></a>

Immediate or Cancel order type - ensures that the order is a taker order. Try to match as much of the
order as possible as taker order and cancel the rest.


<pre><code><b>const</b> <a href="market.md#0x7_market_TIME_IN_FORCE_IOC">TIME_IN_FORCE_IOC</a>: u8 = 2;
</code></pre>



<a id="0x7_market_TIME_IN_FORCE_POST_ONLY"></a>

Post Only order type - ensures that the order is not a taker order


<pre><code><b>const</b> <a href="market.md#0x7_market_TIME_IN_FORCE_POST_ONLY">TIME_IN_FORCE_POST_ONLY</a>: u8 = 1;
</code></pre>



<a id="0x7_market_good_till_cancelled"></a>

## Function `good_till_cancelled`



<pre><code><b>public</b> <b>fun</b> <a href="market.md#0x7_market_good_till_cancelled">good_till_cancelled</a>(): u8
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market.md#0x7_market_good_till_cancelled">good_till_cancelled</a>(): u8 {
    <a href="market.md#0x7_market_TIME_IN_FORCE_GTC">TIME_IN_FORCE_GTC</a>
}
</code></pre>



</details>

<a id="0x7_market_post_only"></a>

## Function `post_only`



<pre><code><b>public</b> <b>fun</b> <a href="market.md#0x7_market_post_only">post_only</a>(): u8
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market.md#0x7_market_post_only">post_only</a>(): u8 {
    <a href="market.md#0x7_market_TIME_IN_FORCE_POST_ONLY">TIME_IN_FORCE_POST_ONLY</a>
}
</code></pre>



</details>

<a id="0x7_market_immediate_or_cancel"></a>

## Function `immediate_or_cancel`



<pre><code><b>public</b> <b>fun</b> <a href="market.md#0x7_market_immediate_or_cancel">immediate_or_cancel</a>(): u8
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market.md#0x7_market_immediate_or_cancel">immediate_or_cancel</a>(): u8 {
    <a href="market.md#0x7_market_TIME_IN_FORCE_IOC">TIME_IN_FORCE_IOC</a>
}
</code></pre>



</details>

<a id="0x7_market_order_status_open"></a>

## Function `order_status_open`



<pre><code><b>public</b> <b>fun</b> <a href="market.md#0x7_market_order_status_open">order_status_open</a>(): u8
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market.md#0x7_market_order_status_open">order_status_open</a>(): u8 {
    <a href="market.md#0x7_market_ORDER_STATUS_OPEN">ORDER_STATUS_OPEN</a>
}
</code></pre>



</details>

<a id="0x7_market_order_status_filled"></a>

## Function `order_status_filled`



<pre><code><b>public</b> <b>fun</b> <a href="market.md#0x7_market_order_status_filled">order_status_filled</a>(): u8
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market.md#0x7_market_order_status_filled">order_status_filled</a>(): u8 {
    <a href="market.md#0x7_market_ORDER_STATUS_FILLED">ORDER_STATUS_FILLED</a>
}
</code></pre>



</details>

<a id="0x7_market_order_status_cancelled"></a>

## Function `order_status_cancelled`



<pre><code><b>public</b> <b>fun</b> <a href="market.md#0x7_market_order_status_cancelled">order_status_cancelled</a>(): u8
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market.md#0x7_market_order_status_cancelled">order_status_cancelled</a>(): u8 {
    <a href="market.md#0x7_market_ORDER_STATUS_CANCELLED">ORDER_STATUS_CANCELLED</a>
}
</code></pre>



</details>

<a id="0x7_market_order_status_rejected"></a>

## Function `order_status_rejected`



<pre><code><b>public</b> <b>fun</b> <a href="market.md#0x7_market_order_status_rejected">order_status_rejected</a>(): u8
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market.md#0x7_market_order_status_rejected">order_status_rejected</a>(): u8 {
    <a href="market.md#0x7_market_ORDER_STATUS_REJECTED">ORDER_STATUS_REJECTED</a>
}
</code></pre>



</details>

<a id="0x7_market_destroy_order_match_result"></a>

## Function `destroy_order_match_result`



<pre><code><b>public</b> <b>fun</b> <a href="market.md#0x7_market_destroy_order_match_result">destroy_order_match_result</a>(self: <a href="market.md#0x7_market_OrderMatchResult">market::OrderMatchResult</a>): (u64, u64, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="market.md#0x7_market_OrderCancellationReason">market::OrderCancellationReason</a>&gt;, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market.md#0x7_market_destroy_order_match_result">destroy_order_match_result</a>(
    self: <a href="market.md#0x7_market_OrderMatchResult">OrderMatchResult</a>
): (u64, u64, Option&lt;<a href="market.md#0x7_market_OrderCancellationReason">OrderCancellationReason</a>&gt;, u64) {
    <b>let</b> <a href="market.md#0x7_market_OrderMatchResult">OrderMatchResult</a> { order_id, remaining_size, cancel_reason, num_fills } =
        self;
    (order_id, remaining_size, cancel_reason, num_fills)
}
</code></pre>



</details>

<a id="0x7_market_number_of_fills"></a>

## Function `number_of_fills`



<pre><code><b>public</b> <b>fun</b> <a href="market.md#0x7_market_number_of_fills">number_of_fills</a>(self: &<a href="market.md#0x7_market_OrderMatchResult">market::OrderMatchResult</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market.md#0x7_market_number_of_fills">number_of_fills</a>(self: &<a href="market.md#0x7_market_OrderMatchResult">OrderMatchResult</a>): u64 {
    self.num_fills
}
</code></pre>



</details>

<a id="0x7_market_get_cancel_reason"></a>

## Function `get_cancel_reason`



<pre><code><b>public</b> <b>fun</b> <a href="market.md#0x7_market_get_cancel_reason">get_cancel_reason</a>(self: &<a href="market.md#0x7_market_OrderMatchResult">market::OrderMatchResult</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="market.md#0x7_market_OrderCancellationReason">market::OrderCancellationReason</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market.md#0x7_market_get_cancel_reason">get_cancel_reason</a>(self: &<a href="market.md#0x7_market_OrderMatchResult">OrderMatchResult</a>): Option&lt;<a href="market.md#0x7_market_OrderCancellationReason">OrderCancellationReason</a>&gt; {
    self.cancel_reason
}
</code></pre>



</details>

<a id="0x7_market_get_remaining_size_from_result"></a>

## Function `get_remaining_size_from_result`



<pre><code><b>public</b> <b>fun</b> <a href="market.md#0x7_market_get_remaining_size_from_result">get_remaining_size_from_result</a>(self: &<a href="market.md#0x7_market_OrderMatchResult">market::OrderMatchResult</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market.md#0x7_market_get_remaining_size_from_result">get_remaining_size_from_result</a>(self: &<a href="market.md#0x7_market_OrderMatchResult">OrderMatchResult</a>): u64 {
    self.remaining_size
}
</code></pre>



</details>

<a id="0x7_market_is_ioc_violation"></a>

## Function `is_ioc_violation`



<pre><code><b>public</b> <b>fun</b> <a href="market.md#0x7_market_is_ioc_violation">is_ioc_violation</a>(self: <a href="market.md#0x7_market_OrderCancellationReason">market::OrderCancellationReason</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market.md#0x7_market_is_ioc_violation">is_ioc_violation</a>(self: <a href="market.md#0x7_market_OrderCancellationReason">OrderCancellationReason</a>): bool {
    <b>return</b> self == OrderCancellationReason::IOCViolation
}
</code></pre>



</details>

<a id="0x7_market_is_fill_limit_violation"></a>

## Function `is_fill_limit_violation`



<pre><code><b>public</b> <b>fun</b> <a href="market.md#0x7_market_is_fill_limit_violation">is_fill_limit_violation</a>(cancel_reason: <a href="market.md#0x7_market_OrderCancellationReason">market::OrderCancellationReason</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market.md#0x7_market_is_fill_limit_violation">is_fill_limit_violation</a>(
    cancel_reason: <a href="market.md#0x7_market_OrderCancellationReason">OrderCancellationReason</a>
): bool {
    <b>return</b> cancel_reason == OrderCancellationReason::MaxFillLimitViolation
}
</code></pre>



</details>

<a id="0x7_market_get_order_id"></a>

## Function `get_order_id`



<pre><code><b>public</b> <b>fun</b> <a href="market.md#0x7_market_get_order_id">get_order_id</a>(self: <a href="market.md#0x7_market_OrderMatchResult">market::OrderMatchResult</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market.md#0x7_market_get_order_id">get_order_id</a>(self: <a href="market.md#0x7_market_OrderMatchResult">OrderMatchResult</a>): u64 {
    self.order_id
}
</code></pre>



</details>

<a id="0x7_market_new_market"></a>

## Function `new_market`



<pre><code><b>public</b> <b>fun</b> <a href="market.md#0x7_market_new_market">new_market</a>&lt;M: <b>copy</b>, drop, store&gt;(parent: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="market.md#0x7_market">market</a>: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>): <a href="market.md#0x7_market_Market">market::Market</a>&lt;M&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market.md#0x7_market_new_market">new_market</a>&lt;M: store + <b>copy</b> + drop&gt;(
    parent: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="market.md#0x7_market">market</a>: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>
): <a href="market.md#0x7_market_Market">Market</a>&lt;M&gt; {
    // requiring signers, and not addresses, purely <b>to</b> guarantee different dexes
    // cannot polute events <b>to</b> each other, accidentally or maliciously.
    <a href="market.md#0x7_market_Market">Market</a> {
        parent: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(parent),
        <a href="market.md#0x7_market">market</a>: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="market.md#0x7_market">market</a>),
        last_order_id: 0,
        <a href="order_book.md#0x7_order_book">order_book</a>: new_order_book()
    }
}
</code></pre>



</details>

<a id="0x7_market_get_market"></a>

## Function `get_market`



<pre><code><b>public</b> <b>fun</b> <a href="market.md#0x7_market_get_market">get_market</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="market.md#0x7_market_Market">market::Market</a>&lt;M&gt;): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market.md#0x7_market_get_market">get_market</a>&lt;M: store + <b>copy</b> + drop&gt;(self: &<a href="market.md#0x7_market_Market">Market</a>&lt;M&gt;): <b>address</b> {
    self.<a href="market.md#0x7_market">market</a>
}
</code></pre>



</details>

<a id="0x7_market_get_order_book"></a>

## Function `get_order_book`



<pre><code><b>public</b> <b>fun</b> <a href="market.md#0x7_market_get_order_book">get_order_book</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="market.md#0x7_market_Market">market::Market</a>&lt;M&gt;): &<a href="order_book.md#0x7_order_book_OrderBook">order_book::OrderBook</a>&lt;M&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market.md#0x7_market_get_order_book">get_order_book</a>&lt;M: store + <b>copy</b> + drop&gt;(self: &<a href="market.md#0x7_market_Market">Market</a>&lt;M&gt;): &OrderBook&lt;M&gt; {
    &self.<a href="order_book.md#0x7_order_book">order_book</a>
}
</code></pre>



</details>

<a id="0x7_market_get_order_book_mut"></a>

## Function `get_order_book_mut`



<pre><code><b>public</b> <b>fun</b> <a href="market.md#0x7_market_get_order_book_mut">get_order_book_mut</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="market.md#0x7_market_Market">market::Market</a>&lt;M&gt;): &<b>mut</b> <a href="order_book.md#0x7_order_book_OrderBook">order_book::OrderBook</a>&lt;M&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market.md#0x7_market_get_order_book_mut">get_order_book_mut</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="market.md#0x7_market_Market">Market</a>&lt;M&gt;
): &<b>mut</b> OrderBook&lt;M&gt; {
    &<b>mut</b> self.<a href="order_book.md#0x7_order_book">order_book</a>
}
</code></pre>



</details>

<a id="0x7_market_best_bid_price"></a>

## Function `best_bid_price`



<pre><code><b>public</b> <b>fun</b> <a href="market.md#0x7_market_best_bid_price">best_bid_price</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="market.md#0x7_market_Market">market::Market</a>&lt;M&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market.md#0x7_market_best_bid_price">best_bid_price</a>&lt;M: store + <b>copy</b> + drop&gt;(self: &<a href="market.md#0x7_market_Market">Market</a>&lt;M&gt;): Option&lt;u64&gt; {
    self.<a href="order_book.md#0x7_order_book">order_book</a>.<a href="market.md#0x7_market_best_bid_price">best_bid_price</a>()
}
</code></pre>



</details>

<a id="0x7_market_best_ask_price"></a>

## Function `best_ask_price`



<pre><code><b>public</b> <b>fun</b> <a href="market.md#0x7_market_best_ask_price">best_ask_price</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="market.md#0x7_market_Market">market::Market</a>&lt;M&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market.md#0x7_market_best_ask_price">best_ask_price</a>&lt;M: store + <b>copy</b> + drop&gt;(self: &<a href="market.md#0x7_market_Market">Market</a>&lt;M&gt;): Option&lt;u64&gt; {
    self.<a href="order_book.md#0x7_order_book">order_book</a>.<a href="market.md#0x7_market_best_ask_price">best_ask_price</a>()
}
</code></pre>



</details>

<a id="0x7_market_is_taker_order"></a>

## Function `is_taker_order`



<pre><code><b>public</b> <b>fun</b> <a href="market.md#0x7_market_is_taker_order">is_taker_order</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="market.md#0x7_market_Market">market::Market</a>&lt;M&gt;, price: u64, is_buy: bool, trigger_condition: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="order_book_types.md#0x7_order_book_types_TriggerCondition">order_book_types::TriggerCondition</a>&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market.md#0x7_market_is_taker_order">is_taker_order</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="market.md#0x7_market_Market">Market</a>&lt;M&gt;, price: u64, is_buy: bool, trigger_condition: Option&lt;TriggerCondition&gt;
): bool {
    self.<a href="order_book.md#0x7_order_book">order_book</a>.<a href="market.md#0x7_market_is_taker_order">is_taker_order</a>(price, is_buy, trigger_condition)
}
</code></pre>



</details>

<a id="0x7_market_place_order"></a>

## Function `place_order`

Places an order - If its a taker order, it will be matched immediately and if its a maker order, it will simply
be placed in the order book. An order id is generated when the order is placed and this id can be used to
uniquely identify the order for this market and can also be used to get the status of the order or cancel the order.
The order is placed with the following parameters:
- user: The user who is placing the order
- price: The price at which the order is placed
- orig_size: The original size of the order
- is_buy: Whether the order is a buy order or a sell order
- time_in_force: The time in force for the order. This can be one of the following:
- TIME_IN_FORCE_GTC: Good till cancelled order type
- TIME_IN_FORCE_POST_ONLY: Post Only order type - ensures that the order is not a taker order
- TIME_IN_FORCE_IOC: Immediate or Cancel order type - ensures that the order is a taker order. Try to match as much of the
order as possible as taker order and cancel the rest.
- trigger_condition: The trigger condition
- metadata: The metadata for the order. This can be any type that the clearing house implementation supports.
- max_fill_limit: The maximum fill limit for the order. This is the maximum number of fills to trigger for this order.
This knob is present to configure maximum amount of gas any order placement transaction might consume and avoid
hitting the maximum has limit of the blockchain.
- emit_cancel_on_fill_limit: bool,: Whether to emit an order cancellation event when the fill limit is reached.
This is used ful as the caller might not want to cancel the order when the limit is reached and can continue
that order in a separate transaction.
- callbacks: The callbacks for the market clearinghouse. This is a struct that implements the MarketClearinghouseCallbacks
interface. This is used to validate the order and settle the trade.
Returns the order id, remaining size, cancel reason and number of fills for the order.


<pre><code><b>public</b> <b>fun</b> <a href="market.md#0x7_market_place_order">place_order</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="market.md#0x7_market_Market">market::Market</a>&lt;M&gt;, user: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, price: u64, orig_size: u64, is_buy: bool, time_in_force: u8, trigger_condition: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="order_book_types.md#0x7_order_book_types_TriggerCondition">order_book_types::TriggerCondition</a>&gt;, metadata: M, max_fill_limit: u64, emit_cancel_on_fill_limit: bool, callbacks: &<a href="market_types.md#0x7_market_types_MarketClearinghouseCallbacks">market_types::MarketClearinghouseCallbacks</a>&lt;M&gt;): <a href="market.md#0x7_market_OrderMatchResult">market::OrderMatchResult</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market.md#0x7_market_place_order">place_order</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="market.md#0x7_market_Market">Market</a>&lt;M&gt;,
    user: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    price: u64,
    orig_size: u64,
    is_buy: bool,
    time_in_force: u8,
    trigger_condition: Option&lt;TriggerCondition&gt;,
    metadata: M,
    max_fill_limit: u64,
    emit_cancel_on_fill_limit: bool,
    callbacks: &MarketClearinghouseCallbacks&lt;M&gt;
): <a href="market.md#0x7_market_OrderMatchResult">OrderMatchResult</a> {
    <b>let</b> order_id = self.<a href="market.md#0x7_market_next_order_id">next_order_id</a>();
    self.<a href="market.md#0x7_market_place_order_with_order_id">place_order_with_order_id</a>(
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(user),
        price,
        orig_size,
        orig_size,
        is_buy,
        time_in_force,
        trigger_condition,
        metadata,
        order_id,
        max_fill_limit,
        emit_cancel_on_fill_limit,
        <b>true</b>,
        callbacks
    )
}
</code></pre>



</details>

<a id="0x7_market_next_order_id"></a>

## Function `next_order_id`



<pre><code><b>public</b> <b>fun</b> <a href="market.md#0x7_market_next_order_id">next_order_id</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="market.md#0x7_market_Market">market::Market</a>&lt;M&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market.md#0x7_market_next_order_id">next_order_id</a>&lt;M: store + <b>copy</b> + drop&gt;(self: &<b>mut</b> <a href="market.md#0x7_market_Market">Market</a>&lt;M&gt;): u64 {
    self.last_order_id += 1;
    self.last_order_id
}
</code></pre>



</details>

<a id="0x7_market_place_order_with_user_addr"></a>

## Function `place_order_with_user_addr`

Similar to <code>place_order</code> API but instead of a signer, it takes a user address - can be used in case trading
functionality is delegated to a different address. Please note that it is the responsibility of the caller
to verify that the transaction signer is authorized to place orders on behalf of the user.


<pre><code><b>public</b> <b>fun</b> <a href="market.md#0x7_market_place_order_with_user_addr">place_order_with_user_addr</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="market.md#0x7_market_Market">market::Market</a>&lt;M&gt;, user_addr: <b>address</b>, price: u64, orig_size: u64, is_buy: bool, time_in_force: u8, trigger_condition: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="order_book_types.md#0x7_order_book_types_TriggerCondition">order_book_types::TriggerCondition</a>&gt;, metadata: M, max_fill_limit: u64, emit_cancel_on_fill_limit: bool, callbacks: &<a href="market_types.md#0x7_market_types_MarketClearinghouseCallbacks">market_types::MarketClearinghouseCallbacks</a>&lt;M&gt;): <a href="market.md#0x7_market_OrderMatchResult">market::OrderMatchResult</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market.md#0x7_market_place_order_with_user_addr">place_order_with_user_addr</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="market.md#0x7_market_Market">Market</a>&lt;M&gt;,
    user_addr: <b>address</b>,
    price: u64,
    orig_size: u64,
    is_buy: bool,
    time_in_force: u8,
    trigger_condition: Option&lt;TriggerCondition&gt;,
    metadata: M,
    max_fill_limit: u64,
    emit_cancel_on_fill_limit: bool,
    callbacks: &MarketClearinghouseCallbacks&lt;M&gt;
): <a href="market.md#0x7_market_OrderMatchResult">OrderMatchResult</a> {
    <b>let</b> order_id = self.<a href="market.md#0x7_market_next_order_id">next_order_id</a>();
    self.<a href="market.md#0x7_market_place_order_with_order_id">place_order_with_order_id</a>(
        user_addr,
        price,
        orig_size,
        orig_size,
        is_buy,
        time_in_force,
        trigger_condition,
        metadata,
        order_id,
        max_fill_limit,
        emit_cancel_on_fill_limit,
        <b>true</b>,
        callbacks
    )
}
</code></pre>



</details>

<a id="0x7_market_place_maker_order"></a>

## Function `place_maker_order`



<pre><code><b>fun</b> <a href="market.md#0x7_market_place_maker_order">place_maker_order</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="market.md#0x7_market_Market">market::Market</a>&lt;M&gt;, user_addr: <b>address</b>, price: u64, orig_size: u64, remaining_size: u64, is_buy: bool, time_in_force: u8, trigger_condition: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="order_book_types.md#0x7_order_book_types_TriggerCondition">order_book_types::TriggerCondition</a>&gt;, metadata: M, order_id: u64, callbacks: &<a href="market_types.md#0x7_market_types_MarketClearinghouseCallbacks">market_types::MarketClearinghouseCallbacks</a>&lt;M&gt;): <a href="market.md#0x7_market_OrderMatchResult">market::OrderMatchResult</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="market.md#0x7_market_place_maker_order">place_maker_order</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="market.md#0x7_market_Market">Market</a>&lt;M&gt;,
    user_addr: <b>address</b>,
    price: u64,
    orig_size: u64,
    remaining_size: u64,
    is_buy: bool,
    time_in_force: u8,
    trigger_condition: Option&lt;TriggerCondition&gt;,
    metadata: M,
    order_id: u64,
    callbacks: &MarketClearinghouseCallbacks&lt;M&gt;
): <a href="market.md#0x7_market_OrderMatchResult">OrderMatchResult</a> {
    // Validate that the order is valid from position management perspective
    <b>if</b> (time_in_force == <a href="market.md#0x7_market_TIME_IN_FORCE_IOC">TIME_IN_FORCE_IOC</a>) {
        <a href="../../aptos-framework/doc/event.md#0x1_event_emit">event::emit</a>(
            <a href="market.md#0x7_market_OrderEvent">OrderEvent</a> {
                parent: self.parent,
                <a href="market.md#0x7_market">market</a>: self.<a href="market.md#0x7_market">market</a>,
                order_id,
                user: user_addr,
                orig_size,
                remaining_size: orig_size,
                size_delta: orig_size,
                price,
                is_buy,
                is_taker: <b>false</b>,
                status: <a href="market.md#0x7_market_ORDER_STATUS_OPEN">ORDER_STATUS_OPEN</a>,
                details: std::string::utf8(b"")
            }
        );
        <a href="../../aptos-framework/doc/event.md#0x1_event_emit">event::emit</a>(
            <a href="market.md#0x7_market_OrderEvent">OrderEvent</a> {
                parent: self.parent,
                <a href="market.md#0x7_market">market</a>: self.<a href="market.md#0x7_market">market</a>,
                order_id,
                user: user_addr,
                orig_size,
                remaining_size: orig_size,
                size_delta: orig_size,
                price,
                is_buy,
                is_taker: <b>false</b>,
                status: <a href="market.md#0x7_market_ORDER_STATUS_CANCELLED">ORDER_STATUS_CANCELLED</a>,
                details: std::string::utf8(b"IOC_VIOLATION")
            }
        );
        <b>return</b> <a href="market.md#0x7_market_OrderMatchResult">OrderMatchResult</a> {
            order_id,
            remaining_size,
            cancel_reason: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(OrderCancellationReason::IOCViolation),
            num_fills: 0
        };
    };

    <b>if</b> (
        !callbacks.validate_order_placement(
            user_addr, <b>false</b>, // is_taker
            is_buy, price, orig_size, metadata
        )) {
        <a href="../../aptos-framework/doc/event.md#0x1_event_emit">event::emit</a>(
            <a href="market.md#0x7_market_OrderEvent">OrderEvent</a> {
                parent: self.parent,
                <a href="market.md#0x7_market">market</a>: self.<a href="market.md#0x7_market">market</a>,
                order_id,
                user: user_addr,
                orig_size,
                remaining_size,
                size_delta: 0, // 0 because order was never placed
                price,
                is_buy,
                is_taker: <b>false</b>,
                status: <a href="market.md#0x7_market_ORDER_STATUS_REJECTED">ORDER_STATUS_REJECTED</a>,
                details: std::string::utf8(b"Position Update violation")
            }
        );
        <b>return</b> <a href="market.md#0x7_market_OrderMatchResult">OrderMatchResult</a> {
            order_id,
            remaining_size,
            cancel_reason: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(
                OrderCancellationReason::PositionUpdateViolation
            ),
            num_fills: 0
        };
    };
    self.<a href="order_book.md#0x7_order_book">order_book</a>.<a href="market.md#0x7_market_place_maker_order">place_maker_order</a>(
        new_order_request(
            user_addr,
            order_id,
            <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(),
            price,
            orig_size,
            remaining_size,
            is_buy,
            trigger_condition,
            metadata
        )
    );
    // Order was successfully placed
    <a href="../../aptos-framework/doc/event.md#0x1_event_emit">event::emit</a>(
        <a href="market.md#0x7_market_OrderEvent">OrderEvent</a> {
            parent: self.parent,
            <a href="market.md#0x7_market">market</a>: self.<a href="market.md#0x7_market">market</a>,
            order_id,
            user: user_addr,
            orig_size,
            remaining_size,
            size_delta: orig_size,
            price,
            is_buy,
            is_taker: <b>false</b>,
            status: <a href="market.md#0x7_market_ORDER_STATUS_OPEN">ORDER_STATUS_OPEN</a>,
            details: std::string::utf8(b"")
        }
    );
    <b>return</b> <a href="market.md#0x7_market_OrderMatchResult">OrderMatchResult</a> {
        order_id,
        remaining_size,
        cancel_reason: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(),
        num_fills: 0
    }
}
</code></pre>



</details>

<a id="0x7_market_place_order_with_order_id"></a>

## Function `place_order_with_order_id`

Similar to <code>place_order</code> API but allows few extra parameters as follows
- order_id: The order id for the order - this is needed because for orders with trigger conditions, the order
id is generated when the order is placed and when they are triggered, the same order id is used to match the order.
- emit_taker_order_open: bool: Whether to emit an order open event for the taker order - this is used when
the caller do not wants to emit an open order event for a taker in case the taker order was intterrupted because
of fill limit violation  in the previous transaction and the order is just a continuation of the previous order.


<pre><code><b>public</b> <b>fun</b> <a href="market.md#0x7_market_place_order_with_order_id">place_order_with_order_id</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="market.md#0x7_market_Market">market::Market</a>&lt;M&gt;, user_addr: <b>address</b>, price: u64, orig_size: u64, remaining_size: u64, is_buy: bool, time_in_force: u8, trigger_condition: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="order_book_types.md#0x7_order_book_types_TriggerCondition">order_book_types::TriggerCondition</a>&gt;, metadata: M, order_id: u64, max_fill_limit: u64, emit_cancel_on_fill_limit: bool, emit_taker_order_open: bool, callbacks: &<a href="market_types.md#0x7_market_types_MarketClearinghouseCallbacks">market_types::MarketClearinghouseCallbacks</a>&lt;M&gt;): <a href="market.md#0x7_market_OrderMatchResult">market::OrderMatchResult</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market.md#0x7_market_place_order_with_order_id">place_order_with_order_id</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="market.md#0x7_market_Market">Market</a>&lt;M&gt;,
    user_addr: <b>address</b>,
    price: u64,
    orig_size: u64,
    remaining_size: u64,
    is_buy: bool,
    time_in_force: u8,
    trigger_condition: Option&lt;TriggerCondition&gt;,
    metadata: M,
    order_id: u64,
    max_fill_limit: u64,
    emit_cancel_on_fill_limit: bool,
    emit_taker_order_open: bool,
    callbacks: &MarketClearinghouseCallbacks&lt;M&gt;
): <a href="market.md#0x7_market_OrderMatchResult">OrderMatchResult</a> {
    <b>assert</b>!(orig_size &gt; 0 && remaining_size &gt; 0, <a href="market.md#0x7_market_EINVALID_ORDER">EINVALID_ORDER</a>);
    // TODO(skedia) is_taker_order API can actually <b>return</b> <b>false</b> positive <b>as</b> the maker orders might not be valid.
    // Changes are needed <b>to</b> ensure the maker order is valid for this order <b>to</b> be a valid taker order.
    // TODO(skedia) reconsile the semantics around <b>global</b> order id vs <a href="../../aptos-framework/doc/account.md#0x1_account">account</a> <b>local</b> id.
    <b>if</b> (
        !callbacks.validate_order_placement(
            user_addr,
            <b>true</b>, // is_taker
            is_buy,
            price,
            remaining_size,
            metadata
        )) {
        <a href="../../aptos-framework/doc/event.md#0x1_event_emit">event::emit</a>(
            <a href="market.md#0x7_market_OrderEvent">OrderEvent</a> {
                parent: self.parent,
                <a href="market.md#0x7_market">market</a>: self.<a href="market.md#0x7_market">market</a>,
                order_id,
                user: user_addr,
                orig_size,
                remaining_size,
                size_delta: 0, // 0 because order was never placed
                price,
                is_buy,
                is_taker: <b>false</b>,
                status: <a href="market.md#0x7_market_ORDER_STATUS_REJECTED">ORDER_STATUS_REJECTED</a>,
                details: std::string::utf8(b"Position Update violation")
            }
        );
        <b>return</b> <a href="market.md#0x7_market_OrderMatchResult">OrderMatchResult</a> {
            order_id,
            remaining_size: orig_size,
            cancel_reason: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(
                OrderCancellationReason::PositionUpdateViolation
            ),
            num_fills: 0
        };
    };

    <b>let</b> is_taker_order = self.<a href="order_book.md#0x7_order_book">order_book</a>.<a href="market.md#0x7_market_is_taker_order">is_taker_order</a>(price, is_buy, trigger_condition);
    <b>if</b> (!is_taker_order) {
        <b>return</b> self.<a href="market.md#0x7_market_place_maker_order">place_maker_order</a>(
            user_addr,
            price,
            orig_size,
            remaining_size,
            is_buy,
            time_in_force,
            trigger_condition,
            metadata,
            order_id,
            callbacks
        );
    };

    // NOTE: We should always <b>use</b> is_taker: <b>true</b> for this order past this
    // point so that indexer can consistently track the order's status
    <b>if</b> (emit_taker_order_open) {
        <a href="../../aptos-framework/doc/event.md#0x1_event_emit">event::emit</a>(
            <a href="market.md#0x7_market_OrderEvent">OrderEvent</a> {
                parent: self.parent,
                <a href="market.md#0x7_market">market</a>: self.<a href="market.md#0x7_market">market</a>,
                order_id,
                user: user_addr,
                orig_size,
                remaining_size,
                size_delta: orig_size,
                price,
                is_buy,
                is_taker: <b>true</b>,
                status: <a href="market.md#0x7_market_ORDER_STATUS_OPEN">ORDER_STATUS_OPEN</a>,
                details: std::string::utf8(b"")
            }
        );
    };
    <b>if</b> (time_in_force == <a href="market.md#0x7_market_TIME_IN_FORCE_POST_ONLY">TIME_IN_FORCE_POST_ONLY</a>) {
        <a href="../../aptos-framework/doc/event.md#0x1_event_emit">event::emit</a>(
            <a href="market.md#0x7_market_OrderEvent">OrderEvent</a> {
                parent: self.parent,
                <a href="market.md#0x7_market">market</a>: self.<a href="market.md#0x7_market">market</a>,
                order_id,
                user: user_addr,
                orig_size,
                remaining_size,
                size_delta: remaining_size,
                price,
                is_buy,
                is_taker: <b>true</b>,
                status: <a href="market.md#0x7_market_ORDER_STATUS_CANCELLED">ORDER_STATUS_CANCELLED</a>,
                details: std::string::utf8(b"Post only violation")
            }
        );
        <b>return</b> <a href="market.md#0x7_market_OrderMatchResult">OrderMatchResult</a> {
            order_id,
            remaining_size: orig_size,
            cancel_reason: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(OrderCancellationReason::PostOnlyViolation),
            num_fills: 0
        };
    };
    <b>let</b> num_fills = 0;
    <b>loop</b> {
        <b>let</b> result =
            self.<a href="order_book.md#0x7_order_book">order_book</a>.get_single_match_for_taker(price, remaining_size, is_buy);
        <b>let</b> (maker_order, maker_matched_size) = result.destroy_single_order_match();
        <b>let</b> (maker_address, maker_order_id) =
            maker_order.<a href="market.md#0x7_market_get_order_id">get_order_id</a>().destroy_order_id_type();
        <b>let</b> settle_result =
            callbacks.settle_trade(
                user_addr,
                maker_address,
                is_buy,
                maker_order.get_price(), // Order is always matched at the price of the maker
                maker_matched_size,
                metadata,
                maker_order.get_metadata_from_order()
            );

        <b>let</b> maker_remaining_settled_size = maker_matched_size;
        <b>let</b> settled_size = settle_result.get_settled_size();
        <b>if</b> (settled_size &gt; 0) {
            remaining_size -= settled_size;
            maker_remaining_settled_size -= settled_size;
            num_fills += 1;
            // Event for taker fill
            <a href="../../aptos-framework/doc/event.md#0x1_event_emit">event::emit</a>(
                <a href="market.md#0x7_market_OrderEvent">OrderEvent</a> {
                    parent: self.parent,
                    <a href="market.md#0x7_market">market</a>: self.<a href="market.md#0x7_market">market</a>,
                    order_id,
                    user: user_addr,
                    orig_size,
                    remaining_size,
                    size_delta: settled_size,
                    price: maker_order.get_price(),
                    is_buy,
                    is_taker: <b>true</b>,
                    status: <a href="market.md#0x7_market_ORDER_STATUS_FILLED">ORDER_STATUS_FILLED</a>,
                    details: std::string::utf8(b"")
                }
            );
            // Event for maker fill
            <a href="../../aptos-framework/doc/event.md#0x1_event_emit">event::emit</a>(
                <a href="market.md#0x7_market_OrderEvent">OrderEvent</a> {
                    parent: self.parent,
                    <a href="market.md#0x7_market">market</a>: self.<a href="market.md#0x7_market">market</a>,
                    order_id: maker_order_id,
                    user: maker_address,
                    orig_size: maker_order.get_orig_size(),
                    remaining_size: maker_order.<a href="market.md#0x7_market_get_remaining_size">get_remaining_size</a>() + maker_remaining_settled_size,
                    size_delta: settled_size,
                    price: maker_order.get_price(),
                    is_buy: !is_buy,
                    is_taker: <b>false</b>,
                    status: <a href="market.md#0x7_market_ORDER_STATUS_FILLED">ORDER_STATUS_FILLED</a>,
                    details: std::string::utf8(b"")
                }
            );
        };

        <b>let</b> maker_cancellation_reason = settle_result.get_maker_cancellation_reason();
        <b>if</b> (maker_cancellation_reason.is_some()) {
            <b>let</b> maker_cancel_size =
                maker_remaining_settled_size + maker_order.<a href="market.md#0x7_market_get_remaining_size">get_remaining_size</a>();
            <a href="../../aptos-framework/doc/event.md#0x1_event_emit">event::emit</a>(
                <a href="market.md#0x7_market_OrderEvent">OrderEvent</a> {
                    parent: self.parent,
                    <a href="market.md#0x7_market">market</a>: self.<a href="market.md#0x7_market">market</a>,
                    order_id: maker_order_id,
                    user: maker_address,
                    orig_size: maker_order.get_orig_size(),
                    remaining_size: 0,
                    size_delta: maker_cancel_size,
                    price: maker_order.get_price(),
                    is_buy: !is_buy,
                    is_taker: <b>false</b>,
                    status: <a href="market.md#0x7_market_ORDER_STATUS_CANCELLED">ORDER_STATUS_CANCELLED</a>,
                    details: maker_cancellation_reason.destroy_some()
                }
            );
            // If the maker is invalid cancel the maker order and <b>continue</b> <b>to</b> the next maker order
            <b>if</b> (maker_order.<a href="market.md#0x7_market_get_remaining_size">get_remaining_size</a>() != 0) {
                self.<a href="order_book.md#0x7_order_book">order_book</a>.<a href="market.md#0x7_market_cancel_order">cancel_order</a>(maker_address, maker_order_id);
            }
        };

        <b>let</b> taker_cancellation_reason = settle_result.get_taker_cancellation_reason();
        <b>if</b> (taker_cancellation_reason.is_some()) {
            <a href="../../aptos-framework/doc/event.md#0x1_event_emit">event::emit</a>(
                <a href="market.md#0x7_market_OrderEvent">OrderEvent</a> {
                    parent: self.parent,
                    <a href="market.md#0x7_market">market</a>: self.<a href="market.md#0x7_market">market</a>,
                    order_id,
                    user: user_addr,
                    orig_size,
                    remaining_size,
                    size_delta: remaining_size,
                    price,
                    is_buy,
                    is_taker: <b>true</b>,
                    status: <a href="market.md#0x7_market_ORDER_STATUS_CANCELLED">ORDER_STATUS_CANCELLED</a>,
                    details: taker_cancellation_reason.destroy_some()
                }
            );
            <b>if</b> (maker_cancellation_reason.is_none() && maker_remaining_settled_size &gt; 0) {
                // If the taker is cancelled but the maker is not cancelled, then we need <b>to</b> re-insert
                // the maker order back into the order book
                self.<a href="order_book.md#0x7_order_book">order_book</a>.reinsert_maker_order(
                    new_order_request(
                        maker_address,
                        maker_order_id,
                        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(maker_order.get_unique_priority_idx()),
                        maker_order.get_price(),
                        maker_order.get_orig_size(),
                        maker_remaining_settled_size,
                        !is_buy,
                        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(),
                        maker_order.get_metadata_from_order()
                    )
                );

            };
            <b>return</b> <a href="market.md#0x7_market_OrderMatchResult">OrderMatchResult</a> {
                order_id,
                remaining_size,
                cancel_reason: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(
                    OrderCancellationReason::ClearinghouseSettleViolation
                ),
                num_fills
            };
        };

        <b>if</b> (remaining_size == 0) {
            <b>break</b>;
        };

        // Check <b>if</b> the next iteration will still match
        <b>let</b> is_taker_order =
            self.<a href="order_book.md#0x7_order_book">order_book</a>.<a href="market.md#0x7_market_is_taker_order">is_taker_order</a>(price, is_buy, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>());
        <b>if</b> (!is_taker_order) {
            <b>if</b> (time_in_force == <a href="market.md#0x7_market_TIME_IN_FORCE_IOC">TIME_IN_FORCE_IOC</a>) {
                <a href="../../aptos-framework/doc/event.md#0x1_event_emit">event::emit</a>(
                    <a href="market.md#0x7_market_OrderEvent">OrderEvent</a> {
                        parent: self.parent,
                        <a href="market.md#0x7_market">market</a>: self.<a href="market.md#0x7_market">market</a>,
                        order_id,
                        user: user_addr,
                        orig_size,
                        remaining_size,
                        size_delta: remaining_size,
                        price,
                        is_buy,
                        // NOTE: Keep consistent <b>with</b> all the logs we've
                        // emitted for this taker order
                        is_taker: <b>true</b>,
                        status: <a href="market.md#0x7_market_ORDER_STATUS_CANCELLED">ORDER_STATUS_CANCELLED</a>,
                        details: std::string::utf8(b"IOC_VIOLATION")
                    }
                );
                <b>return</b> <a href="market.md#0x7_market_OrderMatchResult">OrderMatchResult</a> {
                    order_id,
                    remaining_size,
                    cancel_reason: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(OrderCancellationReason::IOCViolation),
                    num_fills
                };
            };
            <a href="../../aptos-framework/doc/event.md#0x1_event_emit">event::emit</a>(
                <a href="market.md#0x7_market_OrderEvent">OrderEvent</a> {
                    parent: self.parent,
                    <a href="market.md#0x7_market">market</a>: self.<a href="market.md#0x7_market">market</a>,
                    order_id,
                    user: user_addr,
                    orig_size,
                    remaining_size,
                    size_delta: orig_size,
                    price,
                    is_buy,
                    is_taker: <b>false</b>,
                    status: <a href="market.md#0x7_market_ORDER_STATUS_OPEN">ORDER_STATUS_OPEN</a>,
                    details: std::string::utf8(b"")
                }
            );
            self.<a href="order_book.md#0x7_order_book">order_book</a>.<a href="market.md#0x7_market_place_maker_order">place_maker_order</a>(
                new_order_request(
                    user_addr,
                    order_id,
                    <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(),
                    price,
                    orig_size,
                    remaining_size,
                    is_buy,
                    trigger_condition,
                    metadata
                )
            );
            <b>break</b>;
        };

        <b>if</b> (num_fills &gt;= max_fill_limit) {
            <b>if</b> (emit_cancel_on_fill_limit) {
                <a href="../../aptos-framework/doc/event.md#0x1_event_emit">event::emit</a>(
                    <a href="market.md#0x7_market_OrderEvent">OrderEvent</a> {
                        parent: self.parent,
                        <a href="market.md#0x7_market">market</a>: self.<a href="market.md#0x7_market">market</a>,
                        order_id,
                        user: user_addr,
                        orig_size,
                        remaining_size,
                        size_delta: remaining_size,
                        price,
                        is_buy,
                        is_taker: <b>true</b>,
                        status: <a href="market.md#0x7_market_ORDER_STATUS_CANCELLED">ORDER_STATUS_CANCELLED</a>,
                        details: std::string::utf8(b"Fill limit reached")
                    }
                );
            };
            <b>return</b> <a href="market.md#0x7_market_OrderMatchResult">OrderMatchResult</a> {
                order_id,
                remaining_size,
                cancel_reason: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(
                    OrderCancellationReason::MaxFillLimitViolation
                ),
                num_fills
            };
        };
    };
    <a href="market.md#0x7_market_OrderMatchResult">OrderMatchResult</a> {
        order_id,
        remaining_size,
        cancel_reason: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(),
        num_fills
    }
}
</code></pre>



</details>

<a id="0x7_market_cancel_order"></a>

## Function `cancel_order`

Cancels an order - this will cancel the order and emit an event for the order cancellation.


<pre><code><b>public</b> <b>fun</b> <a href="market.md#0x7_market_cancel_order">cancel_order</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="market.md#0x7_market_Market">market::Market</a>&lt;M&gt;, user: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, order_id: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market.md#0x7_market_cancel_order">cancel_order</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="market.md#0x7_market_Market">Market</a>&lt;M&gt;, user: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, order_id: u64
) {
    <b>let</b> <a href="../../aptos-framework/doc/account.md#0x1_account">account</a> = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(user);
    <b>let</b> maybe_order = self.<a href="order_book.md#0x7_order_book">order_book</a>.<a href="market.md#0x7_market_cancel_order">cancel_order</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>, order_id);
    <b>if</b> (maybe_order.is_some()) {
        <b>let</b> order = maybe_order.destroy_some();
        <b>let</b> (
            order_id_type,
            _unique_priority_idx,
            price,
            orig_size,
            remaining_size,
            is_buy,
            _trigger_condition,
            _metadata
        ) = order.destroy_order();
        <b>let</b> (user, order_id) = order_id_type.destroy_order_id_type();
        <a href="../../aptos-framework/doc/event.md#0x1_event_emit">event::emit</a>(
            <a href="market.md#0x7_market_OrderEvent">OrderEvent</a> {
                parent: self.parent,
                <a href="market.md#0x7_market">market</a>: self.<a href="market.md#0x7_market">market</a>,
                order_id,
                user,
                orig_size,
                remaining_size,
                size_delta: remaining_size,
                price,
                is_buy,
                is_taker: <b>false</b>,
                status: <a href="market.md#0x7_market_ORDER_STATUS_CANCELLED">ORDER_STATUS_CANCELLED</a>,
                details: std::string::utf8(b"Order cancelled")
            }
        )
    }
}
</code></pre>



</details>

<a id="0x7_market_get_remaining_size"></a>

## Function `get_remaining_size`

Remaining size of the order in the order book.


<pre><code><b>public</b> <b>fun</b> <a href="market.md#0x7_market_get_remaining_size">get_remaining_size</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="market.md#0x7_market_Market">market::Market</a>&lt;M&gt;, user: <b>address</b>, order_id: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market.md#0x7_market_get_remaining_size">get_remaining_size</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="market.md#0x7_market_Market">Market</a>&lt;M&gt;, user: <b>address</b>, order_id: u64
): u64 {
    self.<a href="order_book.md#0x7_order_book">order_book</a>.<a href="market.md#0x7_market_get_remaining_size">get_remaining_size</a>(user, order_id)
}
</code></pre>



</details>

<a id="0x7_market_take_ready_price_based_orders"></a>

## Function `take_ready_price_based_orders`

Returns all the pending order ready to be executed based on the oracle price. The caller is responsible to
call the <code>place_order_with_order_id</code> API to place the order with the order id returned from this API.


<pre><code><b>public</b> <b>fun</b> <a href="market.md#0x7_market_take_ready_price_based_orders">take_ready_price_based_orders</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="market.md#0x7_market_Market">market::Market</a>&lt;M&gt;, oracle_price: u64): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="order_book_types.md#0x7_order_book_types_Order">order_book_types::Order</a>&lt;M&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market.md#0x7_market_take_ready_price_based_orders">take_ready_price_based_orders</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="market.md#0x7_market_Market">Market</a>&lt;M&gt;, oracle_price: u64
): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Order&lt;M&gt;&gt; {
    self.<a href="order_book.md#0x7_order_book">order_book</a>.<a href="market.md#0x7_market_take_ready_price_based_orders">take_ready_price_based_orders</a>(oracle_price)
}
</code></pre>



</details>

<a id="0x7_market_take_ready_time_based_orders"></a>

## Function `take_ready_time_based_orders`

Returns all the pending order that are ready to be executed based on current time stamp. The caller is responsible to
call the <code>place_order_with_order_id</code> API to place the order with the order id returned from this API.


<pre><code><b>public</b> <b>fun</b> <a href="market.md#0x7_market_take_ready_time_based_orders">take_ready_time_based_orders</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="market.md#0x7_market_Market">market::Market</a>&lt;M&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="order_book_types.md#0x7_order_book_types_Order">order_book_types::Order</a>&lt;M&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="market.md#0x7_market_take_ready_time_based_orders">take_ready_time_based_orders</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="market.md#0x7_market_Market">Market</a>&lt;M&gt;
): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Order&lt;M&gt;&gt; {
    self.<a href="order_book.md#0x7_order_book">order_book</a>.<a href="market.md#0x7_market_take_ready_time_based_orders">take_ready_time_based_orders</a>()
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
