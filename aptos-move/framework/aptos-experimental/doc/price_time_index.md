
<a id="0x7_price_time_index"></a>

# Module `0x7::price_time_index`

ActiveOrderBook: This is the main order book that keeps track of active orders and their states. The active order
book is backed by a BigOrderedMap, which is a data structure that allows for efficient insertion, deletion, and matching of the order
The orders are matched based on price-time priority.

This is internal module, which cannot be used directly, use OrderBook instead.


-  [Struct `PriceAscTime`](#0x7_price_time_index_PriceAscTime)
-  [Struct `PriceDescTime`](#0x7_price_time_index_PriceDescTime)
-  [Struct `OrderData`](#0x7_price_time_index_OrderData)
-  [Enum `PriceTimeIndex`](#0x7_price_time_index_PriceTimeIndex)
-  [Constants](#@Constants_0)
-  [Function `get_slippage_pct_precision`](#0x7_price_time_index_get_slippage_pct_precision)
-  [Function `new_price_time_idx`](#0x7_price_time_index_new_price_time_idx)
-  [Function `best_bid_price`](#0x7_price_time_index_best_bid_price)
-  [Function `best_ask_price`](#0x7_price_time_index_best_ask_price)
-  [Function `get_mid_price`](#0x7_price_time_index_get_mid_price)
-  [Function `get_slippage_price`](#0x7_price_time_index_get_slippage_price)
-  [Function `cancel_active_order`](#0x7_price_time_index_cancel_active_order)
-  [Function `is_taker_order`](#0x7_price_time_index_is_taker_order)
-  [Function `single_match_with_current_active_order`](#0x7_price_time_index_single_match_with_current_active_order)
-  [Function `get_single_match_for_buy_order`](#0x7_price_time_index_get_single_match_for_buy_order)
-  [Function `get_single_match_for_sell_order`](#0x7_price_time_index_get_single_match_for_sell_order)
-  [Function `modify_order_data`](#0x7_price_time_index_modify_order_data)
-  [Function `get_single_match_result`](#0x7_price_time_index_get_single_match_result)
-  [Function `increase_order_size`](#0x7_price_time_index_increase_order_size)
-  [Function `decrease_order_size`](#0x7_price_time_index_decrease_order_size)
-  [Function `place_maker_order`](#0x7_price_time_index_place_maker_order)


<pre><code><b>use</b> <a href="../../aptos-framework/doc/big_ordered_map.md#0x1_big_ordered_map">0x1::big_ordered_map</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="">0x5::order_book_types</a>;
<b>use</b> <a href="">0x5::order_match_types</a>;
</code></pre>



<a id="0x7_price_time_index_PriceAscTime"></a>

## Struct `PriceAscTime`



<pre><code><b>struct</b> <a href="price_time_index.md#0x7_price_time_index_PriceAscTime">PriceAscTime</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>price: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>tie_breaker: <a href="_IncreasingIdx">order_book_types::IncreasingIdx</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x7_price_time_index_PriceDescTime"></a>

## Struct `PriceDescTime`



<pre><code><b>struct</b> <a href="price_time_index.md#0x7_price_time_index_PriceDescTime">PriceDescTime</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>price: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>tie_breaker: <a href="_DecreasingIdx">order_book_types::DecreasingIdx</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x7_price_time_index_OrderData"></a>

## Struct `OrderData`



<pre><code><b>struct</b> <a href="price_time_index.md#0x7_price_time_index_OrderData">OrderData</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>order_id: <a href="_OrderId">order_book_types::OrderId</a></code>
</dt>
<dd>

</dd>
<dt>
<code>order_book_type: <a href="_OrderType">order_book_types::OrderType</a></code>
</dt>
<dd>

</dd>
<dt>
<code>size: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x7_price_time_index_PriceTimeIndex"></a>

## Enum `PriceTimeIndex`

OrderBook tracking active (i.e. unconditional, immediately executable) limit orders.

- invariant - all buys are smaller than sells, at all times.
- tie_breaker in sells is U128_MAX-value, to make sure largest value in the book
that is taken first, is the one inserted first, amongst those with same bid price.


<pre><code>enum <a href="price_time_index.md#0x7_price_time_index_PriceTimeIndex">PriceTimeIndex</a> <b>has</b> store
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>V1</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>buys: <a href="../../aptos-framework/doc/big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">big_ordered_map::BigOrderedMap</a>&lt;<a href="price_time_index.md#0x7_price_time_index_PriceDescTime">price_time_index::PriceDescTime</a>, <a href="price_time_index.md#0x7_price_time_index_OrderData">price_time_index::OrderData</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>sells: <a href="../../aptos-framework/doc/big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">big_ordered_map::BigOrderedMap</a>&lt;<a href="price_time_index.md#0x7_price_time_index_PriceAscTime">price_time_index::PriceAscTime</a>, <a href="price_time_index.md#0x7_price_time_index_OrderData">price_time_index::OrderData</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x7_price_time_index_EINTERNAL_INVARIANT_BROKEN"></a>

There is a code bug that breaks internal invariant


<pre><code><b>const</b> <a href="price_time_index.md#0x7_price_time_index_EINTERNAL_INVARIANT_BROKEN">EINTERNAL_INVARIANT_BROKEN</a>: u64 = 2;
</code></pre>



<a id="0x7_price_time_index_EINVALID_MAKER_ORDER"></a>



<pre><code><b>const</b> <a href="price_time_index.md#0x7_price_time_index_EINVALID_MAKER_ORDER">EINVALID_MAKER_ORDER</a>: u64 = 1;
</code></pre>



<a id="0x7_price_time_index_EINVALID_SLIPPAGE_BPS"></a>



<pre><code><b>const</b> <a href="price_time_index.md#0x7_price_time_index_EINVALID_SLIPPAGE_BPS">EINVALID_SLIPPAGE_BPS</a>: u64 = 3;
</code></pre>



<a id="0x7_price_time_index_SLIPPAGE_PCT_PRECISION"></a>



<pre><code><b>const</b> <a href="price_time_index.md#0x7_price_time_index_SLIPPAGE_PCT_PRECISION">SLIPPAGE_PCT_PRECISION</a>: u64 = 100;
</code></pre>



<a id="0x7_price_time_index_U64_MAX"></a>

========= Active OrderBook ===========


<pre><code><b>const</b> <a href="price_time_index.md#0x7_price_time_index_U64_MAX">U64_MAX</a>: u64 = 18446744073709551615;
</code></pre>



<a id="0x7_price_time_index_get_slippage_pct_precision"></a>

## Function `get_slippage_pct_precision`



<pre><code><b>public</b> <b>fun</b> <a href="price_time_index.md#0x7_price_time_index_get_slippage_pct_precision">get_slippage_pct_precision</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="price_time_index.md#0x7_price_time_index_get_slippage_pct_precision">get_slippage_pct_precision</a>(): u64 {
    <a href="price_time_index.md#0x7_price_time_index_SLIPPAGE_PCT_PRECISION">SLIPPAGE_PCT_PRECISION</a>
}
</code></pre>



</details>

<a id="0x7_price_time_index_new_price_time_idx"></a>

## Function `new_price_time_idx`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="price_time_index.md#0x7_price_time_index_new_price_time_idx">new_price_time_idx</a>(): <a href="price_time_index.md#0x7_price_time_index_PriceTimeIndex">price_time_index::PriceTimeIndex</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="price_time_index.md#0x7_price_time_index_new_price_time_idx">new_price_time_idx</a>(): <a href="price_time_index.md#0x7_price_time_index_PriceTimeIndex">PriceTimeIndex</a> {
    // potentially add max value <b>to</b> both sides (that will be skipped),
    // so that max_key never changes, and doesn't create conflict.
    PriceTimeIndex::V1 {
        buys: <a href="order_book_utils.md#0x7_order_book_utils_new_default_big_ordered_map">order_book_utils::new_default_big_ordered_map</a>(),
        sells: <a href="order_book_utils.md#0x7_order_book_utils_new_default_big_ordered_map">order_book_utils::new_default_big_ordered_map</a>()
    }
}
</code></pre>



</details>

<a id="0x7_price_time_index_best_bid_price"></a>

## Function `best_bid_price`

Picks the best (i.e. highest) bid (i.e. buy) price from the active order book.
Returns None if there are no buys


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="price_time_index.md#0x7_price_time_index_best_bid_price">best_bid_price</a>(self: &<a href="price_time_index.md#0x7_price_time_index_PriceTimeIndex">price_time_index::PriceTimeIndex</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="price_time_index.md#0x7_price_time_index_best_bid_price">best_bid_price</a>(self: &<a href="price_time_index.md#0x7_price_time_index_PriceTimeIndex">PriceTimeIndex</a>): Option&lt;u64&gt; {
    <b>if</b> (self.buys.is_empty()) {
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
    } <b>else</b> {
        <b>let</b> (back_key, _) = self.buys.borrow_back();
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(back_key.price)
    }
}
</code></pre>



</details>

<a id="0x7_price_time_index_best_ask_price"></a>

## Function `best_ask_price`

Picks the best (i.e. lowest) ask (i.e. sell) price from the active order book.
Returns None if there are no sells


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="price_time_index.md#0x7_price_time_index_best_ask_price">best_ask_price</a>(self: &<a href="price_time_index.md#0x7_price_time_index_PriceTimeIndex">price_time_index::PriceTimeIndex</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="price_time_index.md#0x7_price_time_index_best_ask_price">best_ask_price</a>(self: &<a href="price_time_index.md#0x7_price_time_index_PriceTimeIndex">PriceTimeIndex</a>): Option&lt;u64&gt; {
    <b>if</b> (self.sells.is_empty()) {
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
    } <b>else</b> {
        <b>let</b> (front_key, _) = self.sells.borrow_front();
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(front_key.price)
    }
}
</code></pre>



</details>

<a id="0x7_price_time_index_get_mid_price"></a>

## Function `get_mid_price`

Returns the mid price (i.e. the average of the highest bid (buy) price and the lowest ask (sell) price. If
there are o buys / sells, returns None.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="price_time_index.md#0x7_price_time_index_get_mid_price">get_mid_price</a>(self: &<a href="price_time_index.md#0x7_price_time_index_PriceTimeIndex">price_time_index::PriceTimeIndex</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="price_time_index.md#0x7_price_time_index_get_mid_price">get_mid_price</a>(self: &<a href="price_time_index.md#0x7_price_time_index_PriceTimeIndex">PriceTimeIndex</a>): Option&lt;u64&gt; {
    <b>if</b> (self.sells.is_empty() || self.buys.is_empty()) {
        <b>return</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>();
    };

    <b>let</b> (front_key, _) = self.sells.borrow_front();
    <b>let</b> best_ask = front_key.price;
    <b>let</b> (back_key, _) = self.buys.borrow_back();
    <b>let</b> best_bid = back_key.price;
    <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>((best_bid + best_ask) / 2)
}
</code></pre>



</details>

<a id="0x7_price_time_index_get_slippage_price"></a>

## Function `get_slippage_price`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="price_time_index.md#0x7_price_time_index_get_slippage_price">get_slippage_price</a>(self: &<a href="price_time_index.md#0x7_price_time_index_PriceTimeIndex">price_time_index::PriceTimeIndex</a>, is_bid: bool, slippage_bps: u64): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="price_time_index.md#0x7_price_time_index_get_slippage_price">get_slippage_price</a>(
    self: &<a href="price_time_index.md#0x7_price_time_index_PriceTimeIndex">PriceTimeIndex</a>, is_bid: bool, slippage_bps: u64
): Option&lt;u64&gt; {
    <b>if</b> (!is_bid) {
        <b>assert</b>!(
            slippage_bps &lt;= <a href="price_time_index.md#0x7_price_time_index_get_slippage_pct_precision">get_slippage_pct_precision</a>() * 100,
            <a href="price_time_index.md#0x7_price_time_index_EINVALID_SLIPPAGE_BPS">EINVALID_SLIPPAGE_BPS</a>
        );
    };
    <b>let</b> mid_price = self.<a href="price_time_index.md#0x7_price_time_index_get_mid_price">get_mid_price</a>();
    <b>if</b> (mid_price.is_none()) {
        <b>return</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>();
    };
    <b>let</b> mid_price = mid_price.destroy_some();
    <b>let</b> slippage = mul_div(
        mid_price, slippage_bps, <a href="price_time_index.md#0x7_price_time_index_get_slippage_pct_precision">get_slippage_pct_precision</a>() * 100
    );
    <b>if</b> (is_bid) {
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(mid_price + slippage)
    } <b>else</b> {
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(mid_price - slippage)
    }
}
</code></pre>



</details>

<a id="0x7_price_time_index_cancel_active_order"></a>

## Function `cancel_active_order`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="price_time_index.md#0x7_price_time_index_cancel_active_order">cancel_active_order</a>(self: &<b>mut</b> <a href="price_time_index.md#0x7_price_time_index_PriceTimeIndex">price_time_index::PriceTimeIndex</a>, price: u64, unique_priority_idx: <a href="_IncreasingIdx">order_book_types::IncreasingIdx</a>, is_bid: bool): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="price_time_index.md#0x7_price_time_index_cancel_active_order">cancel_active_order</a>(
    self: &<b>mut</b> <a href="price_time_index.md#0x7_price_time_index_PriceTimeIndex">PriceTimeIndex</a>,
    price: u64,
    unique_priority_idx: IncreasingIdx,
    is_bid: bool
): u64 {
    <b>if</b> (is_bid) {
        <b>let</b> key = <a href="price_time_index.md#0x7_price_time_index_PriceDescTime">PriceDescTime</a> {
            price,
            tie_breaker: unique_priority_idx.into_decreasing_idx_type()
        };
        self.buys.remove(&key).size
    } <b>else</b> {
        <b>let</b> key = <a href="price_time_index.md#0x7_price_time_index_PriceAscTime">PriceAscTime</a> { price, tie_breaker: unique_priority_idx };
        self.sells.remove(&key).size
    }
}
</code></pre>



</details>

<a id="0x7_price_time_index_is_taker_order"></a>

## Function `is_taker_order`

Check if the order is a taker order - i.e. if it can be immediately matched with the order book fully or partially.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="price_time_index.md#0x7_price_time_index_is_taker_order">is_taker_order</a>(self: &<a href="price_time_index.md#0x7_price_time_index_PriceTimeIndex">price_time_index::PriceTimeIndex</a>, price: u64, is_bid: bool): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="price_time_index.md#0x7_price_time_index_is_taker_order">is_taker_order</a>(
    self: &<a href="price_time_index.md#0x7_price_time_index_PriceTimeIndex">PriceTimeIndex</a>, price: u64, is_bid: bool
): bool {
    <b>if</b> (is_bid) {
        <b>let</b> best_ask_price = self.<a href="price_time_index.md#0x7_price_time_index_best_ask_price">best_ask_price</a>();
        best_ask_price.is_some() && price &gt;= best_ask_price.destroy_some()
    } <b>else</b> {
        <b>let</b> best_bid_price = self.<a href="price_time_index.md#0x7_price_time_index_best_bid_price">best_bid_price</a>();
        best_bid_price.is_some() && price &lt;= best_bid_price.destroy_some()
    }
}
</code></pre>



</details>

<a id="0x7_price_time_index_single_match_with_current_active_order"></a>

## Function `single_match_with_current_active_order`



<pre><code><b>fun</b> <a href="price_time_index.md#0x7_price_time_index_single_match_with_current_active_order">single_match_with_current_active_order</a>&lt;K: <b>copy</b>, drop, store&gt;(remaining_size: u64, cur_key: K, cur_value: <a href="price_time_index.md#0x7_price_time_index_OrderData">price_time_index::OrderData</a>, orders: &<b>mut</b> <a href="../../aptos-framework/doc/big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">big_ordered_map::BigOrderedMap</a>&lt;K, <a href="price_time_index.md#0x7_price_time_index_OrderData">price_time_index::OrderData</a>&gt;): <a href="_ActiveMatchedOrder">order_match_types::ActiveMatchedOrder</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="price_time_index.md#0x7_price_time_index_single_match_with_current_active_order">single_match_with_current_active_order</a>&lt;K: drop + <b>copy</b> + store&gt;(
    remaining_size: u64,
    cur_key: K,
    cur_value: <a href="price_time_index.md#0x7_price_time_index_OrderData">OrderData</a>,
    orders: &<b>mut</b> BigOrderedMap&lt;K, <a href="price_time_index.md#0x7_price_time_index_OrderData">OrderData</a>&gt;
): ActiveMatchedOrder {
    <b>let</b> is_cur_match_fully_consumed = cur_value.size &lt;= remaining_size;

    <b>let</b> matched_size_for_this_order =
        <b>if</b> (is_cur_match_fully_consumed) {
            orders.remove(&cur_key);
            cur_value.size
        } <b>else</b> {
            <a href="price_time_index.md#0x7_price_time_index_modify_order_data">modify_order_data</a>(
                orders,
                &cur_key,
                |order_data| {
                    order_data.size -= remaining_size;
                }
            );
            remaining_size
        };

    new_active_matched_order(
        cur_value.order_id,
        matched_size_for_this_order, // Matched size on the maker order
        cur_value.size - matched_size_for_this_order, // Remaining size on the maker order
        cur_value.order_book_type
    )
}
</code></pre>



</details>

<a id="0x7_price_time_index_get_single_match_for_buy_order"></a>

## Function `get_single_match_for_buy_order`



<pre><code><b>fun</b> <a href="price_time_index.md#0x7_price_time_index_get_single_match_for_buy_order">get_single_match_for_buy_order</a>(self: &<b>mut</b> <a href="price_time_index.md#0x7_price_time_index_PriceTimeIndex">price_time_index::PriceTimeIndex</a>, price: u64, size: u64): <a href="_ActiveMatchedOrder">order_match_types::ActiveMatchedOrder</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="price_time_index.md#0x7_price_time_index_get_single_match_for_buy_order">get_single_match_for_buy_order</a>(
    self: &<b>mut</b> <a href="price_time_index.md#0x7_price_time_index_PriceTimeIndex">PriceTimeIndex</a>, price: u64, size: u64
): ActiveMatchedOrder {
    <b>let</b> (smallest_key, smallest_value) = self.sells.borrow_front();
    <b>assert</b>!(price &gt;= smallest_key.price, <a href="price_time_index.md#0x7_price_time_index_EINTERNAL_INVARIANT_BROKEN">EINTERNAL_INVARIANT_BROKEN</a>);
    <a href="price_time_index.md#0x7_price_time_index_single_match_with_current_active_order">single_match_with_current_active_order</a>(
        size,
        smallest_key,
        *smallest_value,
        &<b>mut</b> self.sells
    )
}
</code></pre>



</details>

<a id="0x7_price_time_index_get_single_match_for_sell_order"></a>

## Function `get_single_match_for_sell_order`



<pre><code><b>fun</b> <a href="price_time_index.md#0x7_price_time_index_get_single_match_for_sell_order">get_single_match_for_sell_order</a>(self: &<b>mut</b> <a href="price_time_index.md#0x7_price_time_index_PriceTimeIndex">price_time_index::PriceTimeIndex</a>, price: u64, size: u64): <a href="_ActiveMatchedOrder">order_match_types::ActiveMatchedOrder</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="price_time_index.md#0x7_price_time_index_get_single_match_for_sell_order">get_single_match_for_sell_order</a>(
    self: &<b>mut</b> <a href="price_time_index.md#0x7_price_time_index_PriceTimeIndex">PriceTimeIndex</a>, price: u64, size: u64
): ActiveMatchedOrder {
    <b>let</b> (largest_key, largest_value) = self.buys.borrow_back();
    <b>assert</b>!(price &lt;= largest_key.price, <a href="price_time_index.md#0x7_price_time_index_EINTERNAL_INVARIANT_BROKEN">EINTERNAL_INVARIANT_BROKEN</a>);
    <a href="price_time_index.md#0x7_price_time_index_single_match_with_current_active_order">single_match_with_current_active_order</a>(
        size,
        largest_key,
        *largest_value,
        &<b>mut</b> self.buys
    )
}
</code></pre>



</details>

<a id="0x7_price_time_index_modify_order_data"></a>

## Function `modify_order_data`



<pre><code><b>fun</b> <a href="price_time_index.md#0x7_price_time_index_modify_order_data">modify_order_data</a>&lt;K: <b>copy</b>, drop, store&gt;(orders: &<b>mut</b> <a href="../../aptos-framework/doc/big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">big_ordered_map::BigOrderedMap</a>&lt;K, <a href="price_time_index.md#0x7_price_time_index_OrderData">price_time_index::OrderData</a>&gt;, key: &K, modify_fn: |&<b>mut</b> <a href="price_time_index.md#0x7_price_time_index_OrderData">price_time_index::OrderData</a>|)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="price_time_index.md#0x7_price_time_index_modify_order_data">modify_order_data</a>&lt;K: drop + <b>copy</b> + store&gt;(
    orders: &<b>mut</b> BigOrderedMap&lt;K, <a href="price_time_index.md#0x7_price_time_index_OrderData">OrderData</a>&gt;,
    key: &K,
    modify_fn: |&<b>mut</b> <a href="price_time_index.md#0x7_price_time_index_OrderData">OrderData</a>|
) {
    <b>let</b> order = orders.borrow_mut(key);
    modify_fn(order);
}
</code></pre>



</details>

<a id="0x7_price_time_index_get_single_match_result"></a>

## Function `get_single_match_result`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="price_time_index.md#0x7_price_time_index_get_single_match_result">get_single_match_result</a>(self: &<b>mut</b> <a href="price_time_index.md#0x7_price_time_index_PriceTimeIndex">price_time_index::PriceTimeIndex</a>, price: u64, size: u64, is_bid: bool): <a href="_ActiveMatchedOrder">order_match_types::ActiveMatchedOrder</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="price_time_index.md#0x7_price_time_index_get_single_match_result">get_single_match_result</a>(
    self: &<b>mut</b> <a href="price_time_index.md#0x7_price_time_index_PriceTimeIndex">PriceTimeIndex</a>,
    price: u64,
    size: u64,
    is_bid: bool
): ActiveMatchedOrder {
    <b>if</b> (is_bid) {
        self.<a href="price_time_index.md#0x7_price_time_index_get_single_match_for_buy_order">get_single_match_for_buy_order</a>(price, size)
    } <b>else</b> {
        self.<a href="price_time_index.md#0x7_price_time_index_get_single_match_for_sell_order">get_single_match_for_sell_order</a>(price, size)
    }
}
</code></pre>



</details>

<a id="0x7_price_time_index_increase_order_size"></a>

## Function `increase_order_size`

Increase the size of the order in the orderbook without altering its position in the price-time priority.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="price_time_index.md#0x7_price_time_index_increase_order_size">increase_order_size</a>(self: &<b>mut</b> <a href="price_time_index.md#0x7_price_time_index_PriceTimeIndex">price_time_index::PriceTimeIndex</a>, price: u64, unique_priority_idx: <a href="_IncreasingIdx">order_book_types::IncreasingIdx</a>, size_delta: u64, is_bid: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="price_time_index.md#0x7_price_time_index_increase_order_size">increase_order_size</a>(
    self: &<b>mut</b> <a href="price_time_index.md#0x7_price_time_index_PriceTimeIndex">PriceTimeIndex</a>,
    price: u64,
    unique_priority_idx: IncreasingIdx,
    size_delta: u64,
    is_bid: bool
) {
    <b>if</b> (is_bid) {
        <b>let</b> key = <a href="price_time_index.md#0x7_price_time_index_PriceDescTime">PriceDescTime</a> {
            price,
            tie_breaker: unique_priority_idx.into_decreasing_idx_type()
        };
        <a href="price_time_index.md#0x7_price_time_index_modify_order_data">modify_order_data</a>(
            &<b>mut</b> self.buys,
            &key,
            |order_data| {
                order_data.size += size_delta;
            }
        );
    } <b>else</b> {
        <b>let</b> key = <a href="price_time_index.md#0x7_price_time_index_PriceAscTime">PriceAscTime</a> { price, tie_breaker: unique_priority_idx };
        <a href="price_time_index.md#0x7_price_time_index_modify_order_data">modify_order_data</a>(
            &<b>mut</b> self.sells,
            &key,
            |order_data| {
                order_data.size += size_delta;
            }
        );
    };
}
</code></pre>



</details>

<a id="0x7_price_time_index_decrease_order_size"></a>

## Function `decrease_order_size`

Decrease the size of the order in the order book without altering its position in the price-time priority.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="price_time_index.md#0x7_price_time_index_decrease_order_size">decrease_order_size</a>(self: &<b>mut</b> <a href="price_time_index.md#0x7_price_time_index_PriceTimeIndex">price_time_index::PriceTimeIndex</a>, price: u64, unique_priority_idx: <a href="_IncreasingIdx">order_book_types::IncreasingIdx</a>, size_delta: u64, is_bid: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="price_time_index.md#0x7_price_time_index_decrease_order_size">decrease_order_size</a>(
    self: &<b>mut</b> <a href="price_time_index.md#0x7_price_time_index_PriceTimeIndex">PriceTimeIndex</a>,
    price: u64,
    unique_priority_idx: IncreasingIdx,
    size_delta: u64,
    is_bid: bool
) {
    <b>if</b> (is_bid) {
        <b>let</b> key = <a href="price_time_index.md#0x7_price_time_index_PriceDescTime">PriceDescTime</a> {
            price,
            tie_breaker: unique_priority_idx.into_decreasing_idx_type()
        };
        <a href="price_time_index.md#0x7_price_time_index_modify_order_data">modify_order_data</a>(
            &<b>mut</b> self.buys,
            &key,
            |order_data| {
                order_data.size -= size_delta;
            }
        );
    } <b>else</b> {
        <b>let</b> key = <a href="price_time_index.md#0x7_price_time_index_PriceAscTime">PriceAscTime</a> { price, tie_breaker: unique_priority_idx };
        <a href="price_time_index.md#0x7_price_time_index_modify_order_data">modify_order_data</a>(
            &<b>mut</b> self.sells,
            &key,
            |order_data| {
                order_data.size -= size_delta;
            }
        );
    };
}
</code></pre>



</details>

<a id="0x7_price_time_index_place_maker_order"></a>

## Function `place_maker_order`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="price_time_index.md#0x7_price_time_index_place_maker_order">place_maker_order</a>(self: &<b>mut</b> <a href="price_time_index.md#0x7_price_time_index_PriceTimeIndex">price_time_index::PriceTimeIndex</a>, order_id: <a href="_OrderId">order_book_types::OrderId</a>, order_book_type: <a href="_OrderType">order_book_types::OrderType</a>, price: u64, unique_priority_idx: <a href="_IncreasingIdx">order_book_types::IncreasingIdx</a>, size: u64, is_bid: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="price_time_index.md#0x7_price_time_index_place_maker_order">place_maker_order</a>(
    self: &<b>mut</b> <a href="price_time_index.md#0x7_price_time_index_PriceTimeIndex">PriceTimeIndex</a>,
    order_id: OrderId,
    order_book_type: OrderType,
    price: u64,
    unique_priority_idx: IncreasingIdx,
    size: u64,
    is_bid: bool
) {
    <b>let</b> value = <a href="price_time_index.md#0x7_price_time_index_OrderData">OrderData</a> { order_id, order_book_type, size };
    // Assert that this is not a taker order
    <b>assert</b>!(!self.<a href="price_time_index.md#0x7_price_time_index_is_taker_order">is_taker_order</a>(price, is_bid), <a href="price_time_index.md#0x7_price_time_index_EINVALID_MAKER_ORDER">EINVALID_MAKER_ORDER</a>);
    <b>if</b> (is_bid) {
        <b>let</b> key = <a href="price_time_index.md#0x7_price_time_index_PriceDescTime">PriceDescTime</a> {
            price,
            tie_breaker: unique_priority_idx.into_decreasing_idx_type()
        };
        self.buys.add(key, value);
    } <b>else</b> {
        <b>let</b> key = <a href="price_time_index.md#0x7_price_time_index_PriceAscTime">PriceAscTime</a> { price, tie_breaker: unique_priority_idx };
        self.sells.add(key, value);
    };
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
