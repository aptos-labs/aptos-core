
<a id="0x7_price_time_index"></a>

# Module `0x7::price_time_index`

ActiveOrderBook: This is the main order book that keeps track of active orders and their states. The active order
book is backed by a BigOrderedMap, which is a data structure that allows for efficient insertion, deletion, and matching of the order
The orders are matched based on price-time priority.

This is internal module, which cannot be used directly, use OrderBook instead.

Supports two variants:
- V1: Pure Move implementation backed by BigOrderedMap
- NativeV2: Native Rust implementation using in-memory overlay BTreeMaps.
The native index is a derived view kept in validator memory — never stored on-chain.
Operations dispatch to native Rust functions via market_addr.
OrderBook.ensure_native_index_ready() must be called before any operation.


-  [Struct `PriceAscTime`](#0x7_price_time_index_PriceAscTime)
-  [Struct `PriceDescTime`](#0x7_price_time_index_PriceDescTime)
-  [Struct `OrderData`](#0x7_price_time_index_OrderData)
-  [Enum `PriceTimeIndex`](#0x7_price_time_index_PriceTimeIndex)
-  [Constants](#@Constants_0)
-  [Function `get_slippage_pct_precision`](#0x7_price_time_index_get_slippage_pct_precision)
-  [Function `new_price_time_idx`](#0x7_price_time_index_new_price_time_idx)
-  [Function `new_native_price_time_idx`](#0x7_price_time_index_new_native_price_time_idx)
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
-  [Function `destroy_v1_for_migration`](#0x7_price_time_index_destroy_v1_for_migration)
-  [Function `get_native_market_addr`](#0x7_price_time_index_get_native_market_addr)
-  [Function `native_best_bid_price`](#0x7_price_time_index_native_best_bid_price)
-  [Function `native_best_ask_price`](#0x7_price_time_index_native_best_ask_price)
-  [Function `native_get_mid_price`](#0x7_price_time_index_native_get_mid_price)
-  [Function `native_get_slippage_price`](#0x7_price_time_index_native_get_slippage_price)
-  [Function `native_is_taker_order`](#0x7_price_time_index_native_is_taker_order)
-  [Function `native_place_maker_order`](#0x7_price_time_index_native_place_maker_order)
-  [Function `native_cancel_active_order`](#0x7_price_time_index_native_cancel_active_order)
-  [Function `native_get_single_match_result`](#0x7_price_time_index_native_get_single_match_result)
-  [Function `native_increase_order_size`](#0x7_price_time_index_native_increase_order_size)
-  [Function `native_decrease_order_size`](#0x7_price_time_index_native_decrease_order_size)
-  [Function `native_timing_start`](#0x7_price_time_index_native_timing_start)
-  [Function `native_timing_end`](#0x7_price_time_index_native_timing_end)
-  [Function `native_set_timing_context`](#0x7_price_time_index_native_set_timing_context)


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

V1: Pure Move implementation with BigOrderedMap.
NativeV2: Native Rust overlay. <code>market_addr</code> identifies the native overlay.
The overlay is acquired/released by OrderBook lifecycle natives.

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

<details>
<summary>NativeV2</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>market_addr: <b>address</b></code>
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



<pre><code><b>public</b> <b>fun</b> <a href="price_time_index.md#0x7_price_time_index_new_price_time_idx">new_price_time_idx</a>(): <a href="price_time_index.md#0x7_price_time_index_PriceTimeIndex">price_time_index::PriceTimeIndex</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="price_time_index.md#0x7_price_time_index_new_price_time_idx">new_price_time_idx</a>(): <a href="price_time_index.md#0x7_price_time_index_PriceTimeIndex">PriceTimeIndex</a> {
    // potentially add max value <b>to</b> both sides (that will be skipped),
    // so that max_key never changes, and doesn't create conflict.
    PriceTimeIndex::V1 {
        buys: <a href="order_book_utils.md#0x7_order_book_utils_new_default_big_ordered_map">order_book_utils::new_default_big_ordered_map</a>(),
        sells: <a href="order_book_utils.md#0x7_order_book_utils_new_default_big_ordered_map">order_book_utils::new_default_big_ordered_map</a>()
    }
}
</code></pre>



</details>

<a id="0x7_price_time_index_new_native_price_time_idx"></a>

## Function `new_native_price_time_idx`

Creates a new native (Rust-backed) price-time index for the given market.
The native overlay is acquired by OrderBook.ensure_native_index_ready().


<pre><code><b>public</b> <b>fun</b> <a href="price_time_index.md#0x7_price_time_index_new_native_price_time_idx">new_native_price_time_idx</a>(market_addr: <b>address</b>): <a href="price_time_index.md#0x7_price_time_index_PriceTimeIndex">price_time_index::PriceTimeIndex</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="price_time_index.md#0x7_price_time_index_new_native_price_time_idx">new_native_price_time_idx</a>(market_addr: <b>address</b>): <a href="price_time_index.md#0x7_price_time_index_PriceTimeIndex">PriceTimeIndex</a> {
    PriceTimeIndex::NativeV2 { market_addr }
}
</code></pre>



</details>

<a id="0x7_price_time_index_best_bid_price"></a>

## Function `best_bid_price`

Picks the best (i.e. highest) bid (i.e. buy) price from the active order book.
Returns None if there are no buys


<pre><code><b>public</b> <b>fun</b> <a href="price_time_index.md#0x7_price_time_index_best_bid_price">best_bid_price</a>(self: &<a href="price_time_index.md#0x7_price_time_index_PriceTimeIndex">price_time_index::PriceTimeIndex</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="price_time_index.md#0x7_price_time_index_best_bid_price">best_bid_price</a>(self: &<a href="price_time_index.md#0x7_price_time_index_PriceTimeIndex">PriceTimeIndex</a>): Option&lt;u64&gt; {
    match (self) {
        V1 { buys, sells: _ } =&gt; {
            <b>let</b> _t = <a href="price_time_index.md#0x7_price_time_index_native_timing_start">native_timing_start</a>();
            <b>let</b> result = <b>if</b> (buys.is_empty()) {
                <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
            } <b>else</b> {
                <b>let</b> (back_key, _) = buys.borrow_back();
                <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(back_key.price)
            };
            <a href="price_time_index.md#0x7_price_time_index_native_timing_end">native_timing_end</a>(0, &<b>mut</b> _t);
            result
        },
        NativeV2 { market_addr } =&gt; {
            <b>let</b> (has_value, price) = <a href="price_time_index.md#0x7_price_time_index_native_best_bid_price">native_best_bid_price</a>(*market_addr);
            <b>if</b> (has_value) { <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(price) } <b>else</b> { <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>() }
        }
    }
}
</code></pre>



</details>

<a id="0x7_price_time_index_best_ask_price"></a>

## Function `best_ask_price`

Picks the best (i.e. lowest) ask (i.e. sell) price from the active order book.
Returns None if there are no sells


<pre><code><b>public</b> <b>fun</b> <a href="price_time_index.md#0x7_price_time_index_best_ask_price">best_ask_price</a>(self: &<a href="price_time_index.md#0x7_price_time_index_PriceTimeIndex">price_time_index::PriceTimeIndex</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="price_time_index.md#0x7_price_time_index_best_ask_price">best_ask_price</a>(self: &<a href="price_time_index.md#0x7_price_time_index_PriceTimeIndex">PriceTimeIndex</a>): Option&lt;u64&gt; {
    match (self) {
        V1 { buys: _, sells } =&gt; {
            <b>let</b> _t = <a href="price_time_index.md#0x7_price_time_index_native_timing_start">native_timing_start</a>();
            <b>let</b> result = <b>if</b> (sells.is_empty()) {
                <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
            } <b>else</b> {
                <b>let</b> (front_key, _) = sells.borrow_front();
                <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(front_key.price)
            };
            <a href="price_time_index.md#0x7_price_time_index_native_timing_end">native_timing_end</a>(1, &<b>mut</b> _t);
            result
        },
        NativeV2 { market_addr } =&gt; {
            <b>let</b> (has_value, price) = <a href="price_time_index.md#0x7_price_time_index_native_best_ask_price">native_best_ask_price</a>(*market_addr);
            <b>if</b> (has_value) { <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(price) } <b>else</b> { <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>() }
        }
    }
}
</code></pre>



</details>

<a id="0x7_price_time_index_get_mid_price"></a>

## Function `get_mid_price`

Returns the mid price (i.e. the average of the highest bid (buy) price and the lowest ask (sell) price. If
there are no buys / sells, returns None.


<pre><code><b>public</b> <b>fun</b> <a href="price_time_index.md#0x7_price_time_index_get_mid_price">get_mid_price</a>(self: &<a href="price_time_index.md#0x7_price_time_index_PriceTimeIndex">price_time_index::PriceTimeIndex</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="price_time_index.md#0x7_price_time_index_get_mid_price">get_mid_price</a>(self: &<a href="price_time_index.md#0x7_price_time_index_PriceTimeIndex">PriceTimeIndex</a>): Option&lt;u64&gt; {
    match (self) {
        V1 { buys, sells } =&gt; {
            <b>if</b> (sells.is_empty() || buys.is_empty()) {
                <b>return</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>();
            };
            <b>let</b> (front_key, _) = sells.borrow_front();
            <b>let</b> best_ask = front_key.price;
            <b>let</b> (back_key, _) = buys.borrow_back();
            <b>let</b> best_bid = back_key.price;
            <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>((best_bid + best_ask) / 2)
        },
        NativeV2 { market_addr } =&gt; {
            <b>let</b> (has_value, mid) = <a href="price_time_index.md#0x7_price_time_index_native_get_mid_price">native_get_mid_price</a>(*market_addr);
            <b>if</b> (has_value) { <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(mid) } <b>else</b> { <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>() }
        }
    }
}
</code></pre>



</details>

<a id="0x7_price_time_index_get_slippage_price"></a>

## Function `get_slippage_price`



<pre><code><b>public</b> <b>fun</b> <a href="price_time_index.md#0x7_price_time_index_get_slippage_price">get_slippage_price</a>(self: &<a href="price_time_index.md#0x7_price_time_index_PriceTimeIndex">price_time_index::PriceTimeIndex</a>, is_bid: bool, slippage_bps: u64): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="price_time_index.md#0x7_price_time_index_get_slippage_price">get_slippage_price</a>(
    self: &<a href="price_time_index.md#0x7_price_time_index_PriceTimeIndex">PriceTimeIndex</a>, is_bid: bool, slippage_bps: u64
): Option&lt;u64&gt; {
    match (self) {
        V1 { buys: _, sells: _ } =&gt; {
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
        },
        NativeV2 { market_addr } =&gt; {
            <b>let</b> (has_value, price) = <a href="price_time_index.md#0x7_price_time_index_native_get_slippage_price">native_get_slippage_price</a>(*market_addr, is_bid, slippage_bps);
            <b>if</b> (has_value) { <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(price) } <b>else</b> { <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>() }
        }
    }
}
</code></pre>



</details>

<a id="0x7_price_time_index_cancel_active_order"></a>

## Function `cancel_active_order`



<pre><code><b>public</b> <b>fun</b> <a href="price_time_index.md#0x7_price_time_index_cancel_active_order">cancel_active_order</a>(self: &<b>mut</b> <a href="price_time_index.md#0x7_price_time_index_PriceTimeIndex">price_time_index::PriceTimeIndex</a>, price: u64, unique_priority_idx: <a href="_IncreasingIdx">order_book_types::IncreasingIdx</a>, is_bid: bool): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="price_time_index.md#0x7_price_time_index_cancel_active_order">cancel_active_order</a>(
    self: &<b>mut</b> <a href="price_time_index.md#0x7_price_time_index_PriceTimeIndex">PriceTimeIndex</a>,
    price: u64,
    unique_priority_idx: IncreasingIdx,
    is_bid: bool
): u64 {
    match (self) {
        V1 { buys, sells } =&gt; {
            <b>let</b> _t = <a href="price_time_index.md#0x7_price_time_index_native_timing_start">native_timing_start</a>();
            <b>let</b> result = <b>if</b> (is_bid) {
                <b>let</b> key = <a href="price_time_index.md#0x7_price_time_index_PriceDescTime">PriceDescTime</a> {
                    price,
                    tie_breaker: unique_priority_idx.into_decreasing_idx_type()
                };
                buys.remove(&key).size
            } <b>else</b> {
                <b>let</b> key = <a href="price_time_index.md#0x7_price_time_index_PriceAscTime">PriceAscTime</a> { price, tie_breaker: unique_priority_idx };
                sells.remove(&key).size
            };
            <a href="price_time_index.md#0x7_price_time_index_native_timing_end">native_timing_end</a>(3, &<b>mut</b> _t);
            result
        },
        NativeV2 { market_addr } =&gt; {
            <a href="price_time_index.md#0x7_price_time_index_native_cancel_active_order">native_cancel_active_order</a>(
                *market_addr, price, unique_priority_idx.get_increasing_idx_value(), is_bid
            )
        }
    }
}
</code></pre>



</details>

<a id="0x7_price_time_index_is_taker_order"></a>

## Function `is_taker_order`

Check if the order is a taker order - i.e. if it can be immediately matched with the order book fully or partially.


<pre><code><b>public</b> <b>fun</b> <a href="price_time_index.md#0x7_price_time_index_is_taker_order">is_taker_order</a>(self: &<a href="price_time_index.md#0x7_price_time_index_PriceTimeIndex">price_time_index::PriceTimeIndex</a>, price: u64, is_bid: bool): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="price_time_index.md#0x7_price_time_index_is_taker_order">is_taker_order</a>(
    self: &<a href="price_time_index.md#0x7_price_time_index_PriceTimeIndex">PriceTimeIndex</a>, price: u64, is_bid: bool
): bool {
    match (self) {
        V1 { buys: _, sells: _ } =&gt; {
            <b>let</b> _t = <a href="price_time_index.md#0x7_price_time_index_native_timing_start">native_timing_start</a>();
            <b>let</b> result = <b>if</b> (is_bid) {
                <b>let</b> best_ask_price = self.<a href="price_time_index.md#0x7_price_time_index_best_ask_price">best_ask_price</a>();
                best_ask_price.is_some() && price &gt;= best_ask_price.destroy_some()
            } <b>else</b> {
                <b>let</b> best_bid_price = self.<a href="price_time_index.md#0x7_price_time_index_best_bid_price">best_bid_price</a>();
                best_bid_price.is_some() && price &lt;= best_bid_price.destroy_some()
            };
            <a href="price_time_index.md#0x7_price_time_index_native_timing_end">native_timing_end</a>(4, &<b>mut</b> _t);
            result
        },
        NativeV2 { market_addr } =&gt; {
            <a href="price_time_index.md#0x7_price_time_index_native_is_taker_order">native_is_taker_order</a>(*market_addr, price, is_bid)
        }
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



<pre><code><b>public</b> <b>fun</b> <a href="price_time_index.md#0x7_price_time_index_get_single_match_result">get_single_match_result</a>(self: &<b>mut</b> <a href="price_time_index.md#0x7_price_time_index_PriceTimeIndex">price_time_index::PriceTimeIndex</a>, price: u64, size: u64, is_bid: bool): <a href="_ActiveMatchedOrder">order_match_types::ActiveMatchedOrder</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="price_time_index.md#0x7_price_time_index_get_single_match_result">get_single_match_result</a>(
    self: &<b>mut</b> <a href="price_time_index.md#0x7_price_time_index_PriceTimeIndex">PriceTimeIndex</a>,
    price: u64,
    size: u64,
    is_bid: bool
): ActiveMatchedOrder {
    match (self) {
        V1 { buys: _, sells: _ } =&gt; {
            <b>let</b> _t = <a href="price_time_index.md#0x7_price_time_index_native_timing_start">native_timing_start</a>();
            <b>let</b> result = <b>if</b> (is_bid) {
                self.<a href="price_time_index.md#0x7_price_time_index_get_single_match_for_buy_order">get_single_match_for_buy_order</a>(price, size)
            } <b>else</b> {
                self.<a href="price_time_index.md#0x7_price_time_index_get_single_match_for_sell_order">get_single_match_for_sell_order</a>(price, size)
            };
            <a href="price_time_index.md#0x7_price_time_index_native_timing_end">native_timing_end</a>(5, &<b>mut</b> _t);
            result
        },
        NativeV2 { market_addr } =&gt; {
            <b>let</b> (order_id, matched_size, remaining_size, order_type) =
                <a href="price_time_index.md#0x7_price_time_index_native_get_single_match_result">native_get_single_match_result</a>(*market_addr, price, size, is_bid);
            new_active_matched_order(
                aptos_trading::order_book_types::new_order_id_type(order_id),
                matched_size,
                remaining_size,
                <b>if</b> (order_type == 0) {
                    aptos_trading::order_book_types::single_order_type()
                } <b>else</b> {
                    aptos_trading::order_book_types::bulk_order_type()
                }
            )
        }
    }
}
</code></pre>



</details>

<a id="0x7_price_time_index_increase_order_size"></a>

## Function `increase_order_size`

Increase the size of the order in the orderbook without altering its position in the price-time priority.


<pre><code><b>public</b> <b>fun</b> <a href="price_time_index.md#0x7_price_time_index_increase_order_size">increase_order_size</a>(self: &<b>mut</b> <a href="price_time_index.md#0x7_price_time_index_PriceTimeIndex">price_time_index::PriceTimeIndex</a>, price: u64, unique_priority_idx: <a href="_IncreasingIdx">order_book_types::IncreasingIdx</a>, size_delta: u64, is_bid: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="price_time_index.md#0x7_price_time_index_increase_order_size">increase_order_size</a>(
    self: &<b>mut</b> <a href="price_time_index.md#0x7_price_time_index_PriceTimeIndex">PriceTimeIndex</a>,
    price: u64,
    unique_priority_idx: IncreasingIdx,
    size_delta: u64,
    is_bid: bool
) {
    match (self) {
        V1 { buys, sells } =&gt; {
            <b>if</b> (is_bid) {
                <b>let</b> key = <a href="price_time_index.md#0x7_price_time_index_PriceDescTime">PriceDescTime</a> {
                    price,
                    tie_breaker: unique_priority_idx.into_decreasing_idx_type()
                };
                <a href="price_time_index.md#0x7_price_time_index_modify_order_data">modify_order_data</a>(
                    buys,
                    &key,
                    |order_data| {
                        order_data.size += size_delta;
                    }
                );
            } <b>else</b> {
                <b>let</b> key = <a href="price_time_index.md#0x7_price_time_index_PriceAscTime">PriceAscTime</a> { price, tie_breaker: unique_priority_idx };
                <a href="price_time_index.md#0x7_price_time_index_modify_order_data">modify_order_data</a>(
                    sells,
                    &key,
                    |order_data| {
                        order_data.size += size_delta;
                    }
                );
            };
        },
        NativeV2 { market_addr } =&gt; {
            <a href="price_time_index.md#0x7_price_time_index_native_increase_order_size">native_increase_order_size</a>(
                *market_addr, price, unique_priority_idx.get_increasing_idx_value(),
                size_delta, is_bid
            );
        }
    }
}
</code></pre>



</details>

<a id="0x7_price_time_index_decrease_order_size"></a>

## Function `decrease_order_size`

Decrease the size of the order in the order book without altering its position in the price-time priority.


<pre><code><b>public</b> <b>fun</b> <a href="price_time_index.md#0x7_price_time_index_decrease_order_size">decrease_order_size</a>(self: &<b>mut</b> <a href="price_time_index.md#0x7_price_time_index_PriceTimeIndex">price_time_index::PriceTimeIndex</a>, price: u64, unique_priority_idx: <a href="_IncreasingIdx">order_book_types::IncreasingIdx</a>, size_delta: u64, is_bid: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="price_time_index.md#0x7_price_time_index_decrease_order_size">decrease_order_size</a>(
    self: &<b>mut</b> <a href="price_time_index.md#0x7_price_time_index_PriceTimeIndex">PriceTimeIndex</a>,
    price: u64,
    unique_priority_idx: IncreasingIdx,
    size_delta: u64,
    is_bid: bool
) {
    match (self) {
        V1 { buys, sells } =&gt; {
            <b>if</b> (is_bid) {
                <b>let</b> key = <a href="price_time_index.md#0x7_price_time_index_PriceDescTime">PriceDescTime</a> {
                    price,
                    tie_breaker: unique_priority_idx.into_decreasing_idx_type()
                };
                <a href="price_time_index.md#0x7_price_time_index_modify_order_data">modify_order_data</a>(
                    buys,
                    &key,
                    |order_data| {
                        order_data.size -= size_delta;
                    }
                );
            } <b>else</b> {
                <b>let</b> key = <a href="price_time_index.md#0x7_price_time_index_PriceAscTime">PriceAscTime</a> { price, tie_breaker: unique_priority_idx };
                <a href="price_time_index.md#0x7_price_time_index_modify_order_data">modify_order_data</a>(
                    sells,
                    &key,
                    |order_data| {
                        order_data.size -= size_delta;
                    }
                );
            };
        },
        NativeV2 { market_addr } =&gt; {
            <a href="price_time_index.md#0x7_price_time_index_native_decrease_order_size">native_decrease_order_size</a>(
                *market_addr, price, unique_priority_idx.get_increasing_idx_value(),
                size_delta, is_bid
            );
        }
    }
}
</code></pre>



</details>

<a id="0x7_price_time_index_place_maker_order"></a>

## Function `place_maker_order`



<pre><code><b>public</b> <b>fun</b> <a href="price_time_index.md#0x7_price_time_index_place_maker_order">place_maker_order</a>(self: &<b>mut</b> <a href="price_time_index.md#0x7_price_time_index_PriceTimeIndex">price_time_index::PriceTimeIndex</a>, order_id: <a href="_OrderId">order_book_types::OrderId</a>, order_book_type: <a href="_OrderType">order_book_types::OrderType</a>, price: u64, unique_priority_idx: <a href="_IncreasingIdx">order_book_types::IncreasingIdx</a>, size: u64, is_bid: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="price_time_index.md#0x7_price_time_index_place_maker_order">place_maker_order</a>(
    self: &<b>mut</b> <a href="price_time_index.md#0x7_price_time_index_PriceTimeIndex">PriceTimeIndex</a>,
    order_id: OrderId,
    order_book_type: OrderType,
    price: u64,
    unique_priority_idx: IncreasingIdx,
    size: u64,
    is_bid: bool
) {
    match (self) {
        V1 { buys, sells } =&gt; {
            <b>let</b> _t = <a href="price_time_index.md#0x7_price_time_index_native_timing_start">native_timing_start</a>();
            <b>let</b> value = <a href="price_time_index.md#0x7_price_time_index_OrderData">OrderData</a> { order_id, order_book_type, size };
            <b>let</b> is_taker = <b>if</b> (is_bid) {
                !sells.is_empty() && {
                    <b>let</b> (front_key, _) = sells.borrow_front();
                    price &gt;= front_key.price
                }
            } <b>else</b> {
                !buys.is_empty() && {
                    <b>let</b> (back_key, _) = buys.borrow_back();
                    price &lt;= back_key.price
                }
            };
            <b>assert</b>!(!is_taker, <a href="price_time_index.md#0x7_price_time_index_EINVALID_MAKER_ORDER">EINVALID_MAKER_ORDER</a>);
            <b>if</b> (is_bid) {
                <b>let</b> key = <a href="price_time_index.md#0x7_price_time_index_PriceDescTime">PriceDescTime</a> {
                    price,
                    tie_breaker: unique_priority_idx.into_decreasing_idx_type()
                };
                buys.add(key, value);
            } <b>else</b> {
                <b>let</b> key = <a href="price_time_index.md#0x7_price_time_index_PriceAscTime">PriceAscTime</a> { price, tie_breaker: unique_priority_idx };
                sells.add(key, value);
            };
            <a href="price_time_index.md#0x7_price_time_index_native_timing_end">native_timing_end</a>(2, &<b>mut</b> _t);
        },
        NativeV2 { market_addr } =&gt; {
            <b>let</b> order_type_val: u64 =
                <b>if</b> (aptos_trading::order_book_types::is_single_order_type(&order_book_type)) { 0 } <b>else</b> { 1 };
            <a href="price_time_index.md#0x7_price_time_index_native_place_maker_order">native_place_maker_order</a>(
                *market_addr,
                order_id.get_order_id_value(),
                order_type_val,
                price,
                unique_priority_idx.get_increasing_idx_value(),
                size,
                is_bid
            );
        }
    }
}
</code></pre>



</details>

<a id="0x7_price_time_index_destroy_v1_for_migration"></a>

## Function `destroy_v1_for_migration`

Destroy a V1 PriceTimeIndex during migration to NativeV2.
The BigOrderedMap data is redundant (orders are in SingleOrderBook + BulkOrderBook).


<pre><code><b>public</b> <b>fun</b> <a href="price_time_index.md#0x7_price_time_index_destroy_v1_for_migration">destroy_v1_for_migration</a>(self: <a href="price_time_index.md#0x7_price_time_index_PriceTimeIndex">price_time_index::PriceTimeIndex</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="price_time_index.md#0x7_price_time_index_destroy_v1_for_migration">destroy_v1_for_migration</a>(self: <a href="price_time_index.md#0x7_price_time_index_PriceTimeIndex">PriceTimeIndex</a>) {
    match (self) {
        V1 { buys, sells } =&gt; {
            buys.destroy(|_v| {});
            sells.destroy(|_v| {});
        },
        NativeV2 { market_addr: _ } =&gt; {
            <b>abort</b> 0 // Should never be called on NativeV2
        }
    }
}
</code></pre>



</details>

<a id="0x7_price_time_index_get_native_market_addr"></a>

## Function `get_native_market_addr`

Returns the market address if this is a NativeV2 index, None otherwise.


<pre><code><b>public</b> <b>fun</b> <a href="price_time_index.md#0x7_price_time_index_get_native_market_addr">get_native_market_addr</a>(self: &<a href="price_time_index.md#0x7_price_time_index_PriceTimeIndex">price_time_index::PriceTimeIndex</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<b>address</b>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="price_time_index.md#0x7_price_time_index_get_native_market_addr">get_native_market_addr</a>(self: &<a href="price_time_index.md#0x7_price_time_index_PriceTimeIndex">PriceTimeIndex</a>): Option&lt;<b>address</b>&gt; {
    match (self) {
        NativeV2 { market_addr } =&gt; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(*market_addr),
        _ =&gt; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
    }
}
</code></pre>



</details>

<a id="0x7_price_time_index_native_best_bid_price"></a>

## Function `native_best_bid_price`



<pre><code><b>fun</b> <a href="price_time_index.md#0x7_price_time_index_native_best_bid_price">native_best_bid_price</a>(market_addr: <b>address</b>): (bool, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="price_time_index.md#0x7_price_time_index_native_best_bid_price">native_best_bid_price</a>(market_addr: <b>address</b>): (bool, u64);
</code></pre>



</details>

<a id="0x7_price_time_index_native_best_ask_price"></a>

## Function `native_best_ask_price`



<pre><code><b>fun</b> <a href="price_time_index.md#0x7_price_time_index_native_best_ask_price">native_best_ask_price</a>(market_addr: <b>address</b>): (bool, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="price_time_index.md#0x7_price_time_index_native_best_ask_price">native_best_ask_price</a>(market_addr: <b>address</b>): (bool, u64);
</code></pre>



</details>

<a id="0x7_price_time_index_native_get_mid_price"></a>

## Function `native_get_mid_price`



<pre><code><b>fun</b> <a href="price_time_index.md#0x7_price_time_index_native_get_mid_price">native_get_mid_price</a>(market_addr: <b>address</b>): (bool, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="price_time_index.md#0x7_price_time_index_native_get_mid_price">native_get_mid_price</a>(market_addr: <b>address</b>): (bool, u64);
</code></pre>



</details>

<a id="0x7_price_time_index_native_get_slippage_price"></a>

## Function `native_get_slippage_price`



<pre><code><b>fun</b> <a href="price_time_index.md#0x7_price_time_index_native_get_slippage_price">native_get_slippage_price</a>(market_addr: <b>address</b>, is_bid: bool, slippage_bps: u64): (bool, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="price_time_index.md#0x7_price_time_index_native_get_slippage_price">native_get_slippage_price</a>(market_addr: <b>address</b>, is_bid: bool, slippage_bps: u64): (bool, u64);
</code></pre>



</details>

<a id="0x7_price_time_index_native_is_taker_order"></a>

## Function `native_is_taker_order`



<pre><code><b>fun</b> <a href="price_time_index.md#0x7_price_time_index_native_is_taker_order">native_is_taker_order</a>(market_addr: <b>address</b>, price: u64, is_bid: bool): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="price_time_index.md#0x7_price_time_index_native_is_taker_order">native_is_taker_order</a>(market_addr: <b>address</b>, price: u64, is_bid: bool): bool;
</code></pre>



</details>

<a id="0x7_price_time_index_native_place_maker_order"></a>

## Function `native_place_maker_order`



<pre><code><b>fun</b> <a href="price_time_index.md#0x7_price_time_index_native_place_maker_order">native_place_maker_order</a>(market_addr: <b>address</b>, order_id: u128, order_type: u64, price: u64, unique_priority_idx: u128, size: u64, is_bid: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="price_time_index.md#0x7_price_time_index_native_place_maker_order">native_place_maker_order</a>(
    market_addr: <b>address</b>, order_id: u128, order_type: u64, price: u64,
    unique_priority_idx: u128, size: u64, is_bid: bool
);
</code></pre>



</details>

<a id="0x7_price_time_index_native_cancel_active_order"></a>

## Function `native_cancel_active_order`



<pre><code><b>fun</b> <a href="price_time_index.md#0x7_price_time_index_native_cancel_active_order">native_cancel_active_order</a>(market_addr: <b>address</b>, price: u64, unique_priority_idx: u128, is_bid: bool): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="price_time_index.md#0x7_price_time_index_native_cancel_active_order">native_cancel_active_order</a>(
    market_addr: <b>address</b>, price: u64, unique_priority_idx: u128, is_bid: bool
): u64;
</code></pre>



</details>

<a id="0x7_price_time_index_native_get_single_match_result"></a>

## Function `native_get_single_match_result`



<pre><code><b>fun</b> <a href="price_time_index.md#0x7_price_time_index_native_get_single_match_result">native_get_single_match_result</a>(market_addr: <b>address</b>, price: u64, size: u64, is_bid: bool): (u128, u64, u64, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="price_time_index.md#0x7_price_time_index_native_get_single_match_result">native_get_single_match_result</a>(
    market_addr: <b>address</b>, price: u64, size: u64, is_bid: bool
): (u128, u64, u64, u64);
</code></pre>



</details>

<a id="0x7_price_time_index_native_increase_order_size"></a>

## Function `native_increase_order_size`



<pre><code><b>fun</b> <a href="price_time_index.md#0x7_price_time_index_native_increase_order_size">native_increase_order_size</a>(market_addr: <b>address</b>, price: u64, unique_priority_idx: u128, size_delta: u64, is_bid: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="price_time_index.md#0x7_price_time_index_native_increase_order_size">native_increase_order_size</a>(
    market_addr: <b>address</b>, price: u64, unique_priority_idx: u128, size_delta: u64, is_bid: bool
);
</code></pre>



</details>

<a id="0x7_price_time_index_native_decrease_order_size"></a>

## Function `native_decrease_order_size`



<pre><code><b>fun</b> <a href="price_time_index.md#0x7_price_time_index_native_decrease_order_size">native_decrease_order_size</a>(market_addr: <b>address</b>, price: u64, unique_priority_idx: u128, size_delta: u64, is_bid: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="price_time_index.md#0x7_price_time_index_native_decrease_order_size">native_decrease_order_size</a>(
    market_addr: <b>address</b>, price: u64, unique_priority_idx: u128, size_delta: u64, is_bid: bool
);
</code></pre>



</details>

<a id="0x7_price_time_index_native_timing_start"></a>

## Function `native_timing_start`



<pre><code><b>public</b> <b>fun</b> <a href="price_time_index.md#0x7_price_time_index_native_timing_start">native_timing_start</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>native</b> <b>fun</b> <a href="price_time_index.md#0x7_price_time_index_native_timing_start">native_timing_start</a>(): u64;
</code></pre>



</details>

<a id="0x7_price_time_index_native_timing_end"></a>

## Function `native_timing_end`



<pre><code><b>public</b> <b>fun</b> <a href="price_time_index.md#0x7_price_time_index_native_timing_end">native_timing_end</a>(label: u64, start_token: &<b>mut</b> u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>native</b> <b>fun</b> <a href="price_time_index.md#0x7_price_time_index_native_timing_end">native_timing_end</a>(label: u64, start_token: &<b>mut</b> u64);
</code></pre>



</details>

<a id="0x7_price_time_index_native_set_timing_context"></a>

## Function `native_set_timing_context`



<pre><code><b>public</b> <b>fun</b> <a href="price_time_index.md#0x7_price_time_index_native_set_timing_context">native_set_timing_context</a>(ctx: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>native</b> <b>fun</b> <a href="price_time_index.md#0x7_price_time_index_native_set_timing_context">native_set_timing_context</a>(ctx: u64);
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
