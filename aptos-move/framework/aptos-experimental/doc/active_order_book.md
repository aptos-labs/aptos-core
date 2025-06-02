
<a id="0x7_active_order_book"></a>

# Module `0x7::active_order_book`

(work in progress)


-  [Struct `ActiveBidKey`](#0x7_active_order_book_ActiveBidKey)
-  [Struct `ActiveBidData`](#0x7_active_order_book_ActiveBidData)
-  [Enum `ActiveOrderBook`](#0x7_active_order_book_ActiveOrderBook)
-  [Constants](#@Constants_0)
-  [Function `new_active_order_book`](#0x7_active_order_book_new_active_order_book)
-  [Function `best_bid_price`](#0x7_active_order_book_best_bid_price)
-  [Function `best_ask_price`](#0x7_active_order_book_best_ask_price)
-  [Function `get_mid_price`](#0x7_active_order_book_get_mid_price)
-  [Function `get_slippage_price`](#0x7_active_order_book_get_slippage_price)
-  [Function `get_impact_bid_price`](#0x7_active_order_book_get_impact_bid_price)
-  [Function `get_impact_ask_price`](#0x7_active_order_book_get_impact_ask_price)
-  [Function `get_tie_breaker`](#0x7_active_order_book_get_tie_breaker)
-  [Function `cancel_active_order`](#0x7_active_order_book_cancel_active_order)
-  [Function `is_active_order`](#0x7_active_order_book_is_active_order)
-  [Function `is_taker_order`](#0x7_active_order_book_is_taker_order)
-  [Function `single_match_with_current_active_order`](#0x7_active_order_book_single_match_with_current_active_order)
-  [Function `get_single_match_for_buy_order`](#0x7_active_order_book_get_single_match_for_buy_order)
-  [Function `get_single_match_for_sell_order`](#0x7_active_order_book_get_single_match_for_sell_order)
-  [Function `get_single_match_result`](#0x7_active_order_book_get_single_match_result)
-  [Function `increase_order_size`](#0x7_active_order_book_increase_order_size)
-  [Function `decrease_order_size`](#0x7_active_order_book_decrease_order_size)
-  [Function `place_maker_order`](#0x7_active_order_book_place_maker_order)


<pre><code><b>use</b> <a href="../../aptos-framework/doc/big_ordered_map.md#0x1_big_ordered_map">0x1::big_ordered_map</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="order_book_types.md#0x7_order_book_types">0x7::order_book_types</a>;
</code></pre>



<a id="0x7_active_order_book_ActiveBidKey"></a>

## Struct `ActiveBidKey`



<pre><code><b>struct</b> <a href="active_order_book.md#0x7_active_order_book_ActiveBidKey">ActiveBidKey</a> <b>has</b> <b>copy</b>, drop, store
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
<code>tie_breaker: <a href="order_book_types.md#0x7_order_book_types_UniqueIdxType">order_book_types::UniqueIdxType</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x7_active_order_book_ActiveBidData"></a>

## Struct `ActiveBidData`



<pre><code><b>struct</b> <a href="active_order_book.md#0x7_active_order_book_ActiveBidData">ActiveBidData</a> <b>has</b> <b>copy</b>, drop, store
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
<code>size: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x7_active_order_book_ActiveOrderBook"></a>

## Enum `ActiveOrderBook`

OrderBook tracking active (i.e. unconditional, immediately executable) limit orders.

- invariant - all buys are smaller than sells, at all times.
- tie_breaker in sells is U256_MAX-value, to make sure largest value in the book
that is taken first, is the one inserted first, amongst those with same bid price.


<pre><code>enum <a href="active_order_book.md#0x7_active_order_book_ActiveOrderBook">ActiveOrderBook</a> <b>has</b> store
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>V1</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>buys: <a href="../../aptos-framework/doc/big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">big_ordered_map::BigOrderedMap</a>&lt;<a href="active_order_book.md#0x7_active_order_book_ActiveBidKey">active_order_book::ActiveBidKey</a>, <a href="active_order_book.md#0x7_active_order_book_ActiveBidData">active_order_book::ActiveBidData</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>sells: <a href="../../aptos-framework/doc/big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">big_ordered_map::BigOrderedMap</a>&lt;<a href="active_order_book.md#0x7_active_order_book_ActiveBidKey">active_order_book::ActiveBidKey</a>, <a href="active_order_book.md#0x7_active_order_book_ActiveBidData">active_order_book::ActiveBidData</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x7_active_order_book_EINTERNAL_INVARIANT_BROKEN"></a>

There is a code bug that breaks internal invariant


<pre><code><b>const</b> <a href="active_order_book.md#0x7_active_order_book_EINTERNAL_INVARIANT_BROKEN">EINTERNAL_INVARIANT_BROKEN</a>: u64 = 2;
</code></pre>



<a id="0x7_active_order_book_U256_MAX"></a>



<pre><code><b>const</b> <a href="active_order_book.md#0x7_active_order_book_U256_MAX">U256_MAX</a>: u256 = 115792089237316195423570985008687907853269984665640564039457584007913129639935;
</code></pre>



<a id="0x7_active_order_book_EINVALID_MAKER_ORDER"></a>



<pre><code><b>const</b> <a href="active_order_book.md#0x7_active_order_book_EINVALID_MAKER_ORDER">EINVALID_MAKER_ORDER</a>: u64 = 1;
</code></pre>



<a id="0x7_active_order_book_U64_MAX"></a>

========= Active OrderBook ===========


<pre><code><b>const</b> <a href="active_order_book.md#0x7_active_order_book_U64_MAX">U64_MAX</a>: u64 = 18446744073709551615;
</code></pre>



<a id="0x7_active_order_book_new_active_order_book"></a>

## Function `new_active_order_book`



<pre><code><b>public</b> <b>fun</b> <a href="active_order_book.md#0x7_active_order_book_new_active_order_book">new_active_order_book</a>(): <a href="active_order_book.md#0x7_active_order_book_ActiveOrderBook">active_order_book::ActiveOrderBook</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="active_order_book.md#0x7_active_order_book_new_active_order_book">new_active_order_book</a>(): <a href="active_order_book.md#0x7_active_order_book_ActiveOrderBook">ActiveOrderBook</a> {
    // potentially add max value <b>to</b> both sides (that will be skipped),
    // so that max_key never changes, and doesn't create conflict.
    ActiveOrderBook::V1 {
        buys: new_default_big_ordered_map(),
        sells: new_default_big_ordered_map()
    }
}
</code></pre>



</details>

<a id="0x7_active_order_book_best_bid_price"></a>

## Function `best_bid_price`

Picks the best (i.e. highest) bid (i.e. buy) price from the active order book.
aborts if there are no buys


<pre><code><b>public</b> <b>fun</b> <a href="active_order_book.md#0x7_active_order_book_best_bid_price">best_bid_price</a>(self: &<a href="active_order_book.md#0x7_active_order_book_ActiveOrderBook">active_order_book::ActiveOrderBook</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="active_order_book.md#0x7_active_order_book_best_bid_price">best_bid_price</a>(self: &<a href="active_order_book.md#0x7_active_order_book_ActiveOrderBook">ActiveOrderBook</a>): Option&lt;u64&gt; {
    <b>if</b> (self.buys.is_empty()) {
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
    } <b>else</b> {
        <b>let</b> (back_key, _back_value) = self.buys.borrow_back();
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(back_key.price)
    }
}
</code></pre>



</details>

<a id="0x7_active_order_book_best_ask_price"></a>

## Function `best_ask_price`

Picks the best (i.e. lowest) ask (i.e. sell) price from the active order book.
aborts if there are no sells


<pre><code><b>public</b> <b>fun</b> <a href="active_order_book.md#0x7_active_order_book_best_ask_price">best_ask_price</a>(self: &<a href="active_order_book.md#0x7_active_order_book_ActiveOrderBook">active_order_book::ActiveOrderBook</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="active_order_book.md#0x7_active_order_book_best_ask_price">best_ask_price</a>(self: &<a href="active_order_book.md#0x7_active_order_book_ActiveOrderBook">ActiveOrderBook</a>): Option&lt;u64&gt; {
    <b>if</b> (self.sells.is_empty()) {
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
    } <b>else</b> {
        <b>let</b> (front_key, _front_value) = self.sells.borrow_front();
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(front_key.price)
    }
}
</code></pre>



</details>

<a id="0x7_active_order_book_get_mid_price"></a>

## Function `get_mid_price`



<pre><code><b>public</b> <b>fun</b> <a href="active_order_book.md#0x7_active_order_book_get_mid_price">get_mid_price</a>(self: &<a href="active_order_book.md#0x7_active_order_book_ActiveOrderBook">active_order_book::ActiveOrderBook</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="active_order_book.md#0x7_active_order_book_get_mid_price">get_mid_price</a>(self: &<a href="active_order_book.md#0x7_active_order_book_ActiveOrderBook">ActiveOrderBook</a>): Option&lt;u64&gt; {
    <b>let</b> best_bid = self.<a href="active_order_book.md#0x7_active_order_book_best_bid_price">best_bid_price</a>();
    <b>let</b> best_ask = self.<a href="active_order_book.md#0x7_active_order_book_best_ask_price">best_ask_price</a>();
    <b>if</b> (best_bid.is_none() || best_ask.is_none()) {
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
    } <b>else</b> {
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(
            (best_bid.destroy_some() + best_ask.destroy_some()) / 2
        )
    }
}
</code></pre>



</details>

<a id="0x7_active_order_book_get_slippage_price"></a>

## Function `get_slippage_price`



<pre><code><b>public</b> <b>fun</b> <a href="active_order_book.md#0x7_active_order_book_get_slippage_price">get_slippage_price</a>(self: &<a href="active_order_book.md#0x7_active_order_book_ActiveOrderBook">active_order_book::ActiveOrderBook</a>, is_buy: bool, slippage_pct: u64): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="active_order_book.md#0x7_active_order_book_get_slippage_price">get_slippage_price</a>(
    self: &<a href="active_order_book.md#0x7_active_order_book_ActiveOrderBook">ActiveOrderBook</a>, is_buy: bool, slippage_pct: u64
): Option&lt;u64&gt; {
    <b>let</b> mid_price = self.<a href="active_order_book.md#0x7_active_order_book_get_mid_price">get_mid_price</a>();
    <b>if</b> (mid_price.is_none()) {
        <b>return</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>();
    };
    <b>let</b> mid_price = mid_price.destroy_some();
    <b>let</b> slippage = mul_div(
        mid_price, slippage_pct, get_slippage_pct_precision() * 100
    );
    <b>if</b> (is_buy) {
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(mid_price + slippage)
    } <b>else</b> {
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(mid_price - slippage)
    }
}
</code></pre>



</details>

<a id="0x7_active_order_book_get_impact_bid_price"></a>

## Function `get_impact_bid_price`



<pre><code><b>fun</b> <a href="active_order_book.md#0x7_active_order_book_get_impact_bid_price">get_impact_bid_price</a>(self: &<a href="active_order_book.md#0x7_active_order_book_ActiveOrderBook">active_order_book::ActiveOrderBook</a>, impact_size: u64): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="active_order_book.md#0x7_active_order_book_get_impact_bid_price">get_impact_bid_price</a>(self: &<a href="active_order_book.md#0x7_active_order_book_ActiveOrderBook">ActiveOrderBook</a>, impact_size: u64): Option&lt;u64&gt; {
    <b>let</b> total_value = (0 <b>as</b> u128);
    <b>let</b> total_size = 0;
    <b>let</b> orders = &self.buys;
    <b>if</b> (orders.is_empty()) {
        <b>return</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>();
    };
    <b>let</b> (front_key, front_value) = orders.borrow_back();
    <b>while</b> (total_size &lt; impact_size) {
        <b>let</b> matched_size =
            <b>if</b> (total_size + front_value.size &gt; impact_size) {
                impact_size - total_size
            } <b>else</b> {
                front_value.size
            };
        total_value = total_value
            + (matched_size <b>as</b> u128) * (front_key.price <b>as</b> u128);
        total_size = total_size + matched_size;
        <b>let</b> next_key = orders.prev_key(&front_key);
        <b>if</b> (next_key.is_none()) {
            // TODO maybe we should <b>return</b> none <b>if</b> there is not enough depth?
            <b>break</b>;
        };
        front_key = next_key.destroy_some();
        front_value = orders.borrow(&front_key);
    };
    <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>((total_value / (total_size <b>as</b> u128)) <b>as</b> u64)
}
</code></pre>



</details>

<a id="0x7_active_order_book_get_impact_ask_price"></a>

## Function `get_impact_ask_price`



<pre><code><b>fun</b> <a href="active_order_book.md#0x7_active_order_book_get_impact_ask_price">get_impact_ask_price</a>(self: &<a href="active_order_book.md#0x7_active_order_book_ActiveOrderBook">active_order_book::ActiveOrderBook</a>, impact_size: u64): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="active_order_book.md#0x7_active_order_book_get_impact_ask_price">get_impact_ask_price</a>(self: &<a href="active_order_book.md#0x7_active_order_book_ActiveOrderBook">ActiveOrderBook</a>, impact_size: u64): Option&lt;u64&gt; {
    <b>let</b> total_value = 0 <b>as</b> u128;
    <b>let</b> total_size = 0;
    <b>let</b> orders = &self.sells;
    <b>if</b> (orders.is_empty()) {
        <b>return</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>();
    };
    <b>let</b> (front_key, front_value) = orders.borrow_front();
    <b>while</b> (total_size &lt; impact_size) {
        <b>let</b> matched_size =
            <b>if</b> (total_size + front_value.size &gt; impact_size) {
                impact_size - total_size
            } <b>else</b> {
                front_value.size
            };
        total_value = total_value
            + (matched_size <b>as</b> u128) * (front_key.price <b>as</b> u128);
        total_size = total_size + matched_size;
        <b>let</b> next_key = orders.next_key(&front_key);
        <b>if</b> (next_key.is_none()) {
            <b>break</b>;
        };
        front_key = next_key.destroy_some();
        front_value = orders.borrow(&front_key);
    };
    <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>((total_value / (total_size <b>as</b> u128)) <b>as</b> u64)
}
</code></pre>



</details>

<a id="0x7_active_order_book_get_tie_breaker"></a>

## Function `get_tie_breaker`



<pre><code><b>fun</b> <a href="active_order_book.md#0x7_active_order_book_get_tie_breaker">get_tie_breaker</a>(unique_priority_idx: <a href="order_book_types.md#0x7_order_book_types_UniqueIdxType">order_book_types::UniqueIdxType</a>, is_buy: bool): <a href="order_book_types.md#0x7_order_book_types_UniqueIdxType">order_book_types::UniqueIdxType</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="active_order_book.md#0x7_active_order_book_get_tie_breaker">get_tie_breaker</a>(
    unique_priority_idx: UniqueIdxType, is_buy: bool
): UniqueIdxType {
    <b>if</b> (is_buy) {
        unique_priority_idx
    } <b>else</b> {
        unique_priority_idx.descending_idx()
    }
}
</code></pre>



</details>

<a id="0x7_active_order_book_cancel_active_order"></a>

## Function `cancel_active_order`



<pre><code><b>public</b> <b>fun</b> <a href="active_order_book.md#0x7_active_order_book_cancel_active_order">cancel_active_order</a>(self: &<b>mut</b> <a href="active_order_book.md#0x7_active_order_book_ActiveOrderBook">active_order_book::ActiveOrderBook</a>, price: u64, unique_priority_idx: <a href="order_book_types.md#0x7_order_book_types_UniqueIdxType">order_book_types::UniqueIdxType</a>, is_buy: bool): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="active_order_book.md#0x7_active_order_book_cancel_active_order">cancel_active_order</a>(
    self: &<b>mut</b> <a href="active_order_book.md#0x7_active_order_book_ActiveOrderBook">ActiveOrderBook</a>,
    price: u64,
    unique_priority_idx: UniqueIdxType,
    is_buy: bool
): u64 {
    <b>let</b> tie_breaker = <a href="active_order_book.md#0x7_active_order_book_get_tie_breaker">get_tie_breaker</a>(unique_priority_idx, is_buy);
    <b>let</b> key = <a href="active_order_book.md#0x7_active_order_book_ActiveBidKey">ActiveBidKey</a> { price: price, tie_breaker };
    <b>let</b> value =
        <b>if</b> (is_buy) {
            self.buys.remove(&key)
        } <b>else</b> {
            self.sells.remove(&key)
        };
    value.size
}
</code></pre>



</details>

<a id="0x7_active_order_book_is_active_order"></a>

## Function `is_active_order`



<pre><code><b>public</b> <b>fun</b> <a href="active_order_book.md#0x7_active_order_book_is_active_order">is_active_order</a>(self: &<a href="active_order_book.md#0x7_active_order_book_ActiveOrderBook">active_order_book::ActiveOrderBook</a>, price: u64, unique_priority_idx: <a href="order_book_types.md#0x7_order_book_types_UniqueIdxType">order_book_types::UniqueIdxType</a>, is_buy: bool): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="active_order_book.md#0x7_active_order_book_is_active_order">is_active_order</a>(
    self: &<a href="active_order_book.md#0x7_active_order_book_ActiveOrderBook">ActiveOrderBook</a>,
    price: u64,
    unique_priority_idx: UniqueIdxType,
    is_buy: bool
): bool {
    <b>let</b> tie_breaker = <a href="active_order_book.md#0x7_active_order_book_get_tie_breaker">get_tie_breaker</a>(unique_priority_idx, is_buy);
    <b>let</b> key = <a href="active_order_book.md#0x7_active_order_book_ActiveBidKey">ActiveBidKey</a> { price: price, tie_breaker };
    <b>if</b> (is_buy) {
        self.buys.contains(&key)
    } <b>else</b> {
        self.sells.contains(&key)
    }
}
</code></pre>



</details>

<a id="0x7_active_order_book_is_taker_order"></a>

## Function `is_taker_order`

Check if the order is a taker order - i.e. if it can be immediately matched with the order book fully or partially.


<pre><code><b>public</b> <b>fun</b> <a href="active_order_book.md#0x7_active_order_book_is_taker_order">is_taker_order</a>(self: &<a href="active_order_book.md#0x7_active_order_book_ActiveOrderBook">active_order_book::ActiveOrderBook</a>, price: u64, is_buy: bool): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="active_order_book.md#0x7_active_order_book_is_taker_order">is_taker_order</a>(
    self: &<a href="active_order_book.md#0x7_active_order_book_ActiveOrderBook">ActiveOrderBook</a>, price: u64, is_buy: bool
): bool {
    <b>if</b> (is_buy) {
        <b>let</b> best_ask_price = self.<a href="active_order_book.md#0x7_active_order_book_best_ask_price">best_ask_price</a>();
        best_ask_price.is_some() && price &gt;= best_ask_price.destroy_some()
    } <b>else</b> {
        <b>let</b> best_bid_price = self.<a href="active_order_book.md#0x7_active_order_book_best_bid_price">best_bid_price</a>();
        best_bid_price.is_some() && price &lt;= best_bid_price.destroy_some()
    }
}
</code></pre>



</details>

<a id="0x7_active_order_book_single_match_with_current_active_order"></a>

## Function `single_match_with_current_active_order`



<pre><code><b>fun</b> <a href="active_order_book.md#0x7_active_order_book_single_match_with_current_active_order">single_match_with_current_active_order</a>(remaining_size: u64, cur_key: <a href="active_order_book.md#0x7_active_order_book_ActiveBidKey">active_order_book::ActiveBidKey</a>, cur_value: <a href="active_order_book.md#0x7_active_order_book_ActiveBidData">active_order_book::ActiveBidData</a>, orders: &<b>mut</b> <a href="../../aptos-framework/doc/big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">big_ordered_map::BigOrderedMap</a>&lt;<a href="active_order_book.md#0x7_active_order_book_ActiveBidKey">active_order_book::ActiveBidKey</a>, <a href="active_order_book.md#0x7_active_order_book_ActiveBidData">active_order_book::ActiveBidData</a>&gt;): <a href="order_book_types.md#0x7_order_book_types_ActiveMatchedOrder">order_book_types::ActiveMatchedOrder</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="active_order_book.md#0x7_active_order_book_single_match_with_current_active_order">single_match_with_current_active_order</a>(
    remaining_size: u64,
    cur_key: <a href="active_order_book.md#0x7_active_order_book_ActiveBidKey">ActiveBidKey</a>,
    cur_value: <a href="active_order_book.md#0x7_active_order_book_ActiveBidData">ActiveBidData</a>,
    orders: &<b>mut</b> BigOrderedMap&lt;<a href="active_order_book.md#0x7_active_order_book_ActiveBidKey">ActiveBidKey</a>, <a href="active_order_book.md#0x7_active_order_book_ActiveBidData">ActiveBidData</a>&gt;
): ActiveMatchedOrder {
    <b>let</b> is_cur_match_fully_consumed = cur_value.size &lt;= remaining_size;

    <b>let</b> matched_size_for_this_order =
        <b>if</b> (is_cur_match_fully_consumed) {
            cur_value.size
        } <b>else</b> {
            remaining_size
        };

    <b>let</b> result =
        new_active_matched_order(
            cur_value.order_id,
            matched_size_for_this_order, // Matched size on the maker order
            cur_value.size - matched_size_for_this_order // Remaining size on the maker order
        );

    <b>if</b> (is_cur_match_fully_consumed) {
        orders.remove(&cur_key);
    } <b>else</b> {
        orders.borrow_mut(&cur_key).size -= matched_size_for_this_order;
    };
    result
}
</code></pre>



</details>

<a id="0x7_active_order_book_get_single_match_for_buy_order"></a>

## Function `get_single_match_for_buy_order`



<pre><code><b>fun</b> <a href="active_order_book.md#0x7_active_order_book_get_single_match_for_buy_order">get_single_match_for_buy_order</a>(self: &<b>mut</b> <a href="active_order_book.md#0x7_active_order_book_ActiveOrderBook">active_order_book::ActiveOrderBook</a>, price: u64, size: u64): <a href="order_book_types.md#0x7_order_book_types_ActiveMatchedOrder">order_book_types::ActiveMatchedOrder</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="active_order_book.md#0x7_active_order_book_get_single_match_for_buy_order">get_single_match_for_buy_order</a>(
    self: &<b>mut</b> <a href="active_order_book.md#0x7_active_order_book_ActiveOrderBook">ActiveOrderBook</a>, price: u64, size: u64
): ActiveMatchedOrder {
    <b>let</b> (smallest_key, smallest_value) = self.sells.borrow_front();
    <b>assert</b>!(price &gt;= smallest_key.price, <a href="active_order_book.md#0x7_active_order_book_EINTERNAL_INVARIANT_BROKEN">EINTERNAL_INVARIANT_BROKEN</a>);
    <a href="active_order_book.md#0x7_active_order_book_single_match_with_current_active_order">single_match_with_current_active_order</a>(
        size,
        smallest_key,
        *smallest_value,
        &<b>mut</b> self.sells
    )
}
</code></pre>



</details>

<a id="0x7_active_order_book_get_single_match_for_sell_order"></a>

## Function `get_single_match_for_sell_order`



<pre><code><b>fun</b> <a href="active_order_book.md#0x7_active_order_book_get_single_match_for_sell_order">get_single_match_for_sell_order</a>(self: &<b>mut</b> <a href="active_order_book.md#0x7_active_order_book_ActiveOrderBook">active_order_book::ActiveOrderBook</a>, price: u64, size: u64): <a href="order_book_types.md#0x7_order_book_types_ActiveMatchedOrder">order_book_types::ActiveMatchedOrder</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="active_order_book.md#0x7_active_order_book_get_single_match_for_sell_order">get_single_match_for_sell_order</a>(
    self: &<b>mut</b> <a href="active_order_book.md#0x7_active_order_book_ActiveOrderBook">ActiveOrderBook</a>, price: u64, size: u64
): ActiveMatchedOrder {
    <b>let</b> (largest_key, largest_value) = self.buys.borrow_back();
    <b>assert</b>!(price &lt;= largest_key.price, <a href="active_order_book.md#0x7_active_order_book_EINTERNAL_INVARIANT_BROKEN">EINTERNAL_INVARIANT_BROKEN</a>);
    <a href="active_order_book.md#0x7_active_order_book_single_match_with_current_active_order">single_match_with_current_active_order</a>(
        size,
        largest_key,
        *largest_value,
        &<b>mut</b> self.buys
    )
}
</code></pre>



</details>

<a id="0x7_active_order_book_get_single_match_result"></a>

## Function `get_single_match_result`



<pre><code><b>public</b> <b>fun</b> <a href="active_order_book.md#0x7_active_order_book_get_single_match_result">get_single_match_result</a>(self: &<b>mut</b> <a href="active_order_book.md#0x7_active_order_book_ActiveOrderBook">active_order_book::ActiveOrderBook</a>, price: u64, size: u64, is_buy: bool): <a href="order_book_types.md#0x7_order_book_types_ActiveMatchedOrder">order_book_types::ActiveMatchedOrder</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="active_order_book.md#0x7_active_order_book_get_single_match_result">get_single_match_result</a>(
    self: &<b>mut</b> <a href="active_order_book.md#0x7_active_order_book_ActiveOrderBook">ActiveOrderBook</a>,
    price: u64,
    size: u64,
    is_buy: bool
): ActiveMatchedOrder {
    <b>if</b> (is_buy) {
        self.<a href="active_order_book.md#0x7_active_order_book_get_single_match_for_buy_order">get_single_match_for_buy_order</a>(price, size)
    } <b>else</b> {
        self.<a href="active_order_book.md#0x7_active_order_book_get_single_match_for_sell_order">get_single_match_for_sell_order</a>(price, size)
    }
}
</code></pre>



</details>

<a id="0x7_active_order_book_increase_order_size"></a>

## Function `increase_order_size`

Increase the size of the order in the orderbook without altering its position in the price-time priority.


<pre><code><b>public</b> <b>fun</b> <a href="active_order_book.md#0x7_active_order_book_increase_order_size">increase_order_size</a>(self: &<b>mut</b> <a href="active_order_book.md#0x7_active_order_book_ActiveOrderBook">active_order_book::ActiveOrderBook</a>, price: u64, unique_priority_idx: <a href="order_book_types.md#0x7_order_book_types_UniqueIdxType">order_book_types::UniqueIdxType</a>, size_delta: u64, is_buy: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="active_order_book.md#0x7_active_order_book_increase_order_size">increase_order_size</a>(
    self: &<b>mut</b> <a href="active_order_book.md#0x7_active_order_book_ActiveOrderBook">ActiveOrderBook</a>,
    price: u64,
    unique_priority_idx: UniqueIdxType,
    size_delta: u64,
    is_buy: bool
) {
    <b>let</b> tie_breaker = <a href="active_order_book.md#0x7_active_order_book_get_tie_breaker">get_tie_breaker</a>(unique_priority_idx, is_buy);
    <b>let</b> key = <a href="active_order_book.md#0x7_active_order_book_ActiveBidKey">ActiveBidKey</a> { price, tie_breaker };
    <b>if</b> (is_buy) {
        self.buys.borrow_mut(&key).size += size_delta;
    } <b>else</b> {
        self.sells.borrow_mut(&key).size += size_delta;
    };
}
</code></pre>



</details>

<a id="0x7_active_order_book_decrease_order_size"></a>

## Function `decrease_order_size`

Decrease the size of the order in the order book without altering its position in the price-time priority.


<pre><code><b>public</b> <b>fun</b> <a href="active_order_book.md#0x7_active_order_book_decrease_order_size">decrease_order_size</a>(self: &<b>mut</b> <a href="active_order_book.md#0x7_active_order_book_ActiveOrderBook">active_order_book::ActiveOrderBook</a>, price: u64, unique_priority_idx: <a href="order_book_types.md#0x7_order_book_types_UniqueIdxType">order_book_types::UniqueIdxType</a>, size_delta: u64, is_buy: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="active_order_book.md#0x7_active_order_book_decrease_order_size">decrease_order_size</a>(
    self: &<b>mut</b> <a href="active_order_book.md#0x7_active_order_book_ActiveOrderBook">ActiveOrderBook</a>,
    price: u64,
    unique_priority_idx: UniqueIdxType,
    size_delta: u64,
    is_buy: bool
) {
    <b>let</b> tie_breaker = <a href="active_order_book.md#0x7_active_order_book_get_tie_breaker">get_tie_breaker</a>(unique_priority_idx, is_buy);
    <b>let</b> key = <a href="active_order_book.md#0x7_active_order_book_ActiveBidKey">ActiveBidKey</a> { price, tie_breaker };
    <b>if</b> (is_buy) {
        self.buys.borrow_mut(&key).size -= size_delta;
    } <b>else</b> {
        self.sells.borrow_mut(&key).size -= size_delta;
    };
}
</code></pre>



</details>

<a id="0x7_active_order_book_place_maker_order"></a>

## Function `place_maker_order`



<pre><code><b>public</b> <b>fun</b> <a href="active_order_book.md#0x7_active_order_book_place_maker_order">place_maker_order</a>(self: &<b>mut</b> <a href="active_order_book.md#0x7_active_order_book_ActiveOrderBook">active_order_book::ActiveOrderBook</a>, order_id: <a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>, price: u64, unique_priority_idx: <a href="order_book_types.md#0x7_order_book_types_UniqueIdxType">order_book_types::UniqueIdxType</a>, size: u64, is_buy: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="active_order_book.md#0x7_active_order_book_place_maker_order">place_maker_order</a>(
    self: &<b>mut</b> <a href="active_order_book.md#0x7_active_order_book_ActiveOrderBook">ActiveOrderBook</a>,
    order_id: OrderIdType,
    price: u64,
    unique_priority_idx: UniqueIdxType,
    size: u64,
    is_buy: bool
) {
    <b>let</b> tie_breaker = <a href="active_order_book.md#0x7_active_order_book_get_tie_breaker">get_tie_breaker</a>(unique_priority_idx, is_buy);
    <b>let</b> key = <a href="active_order_book.md#0x7_active_order_book_ActiveBidKey">ActiveBidKey</a> { price, tie_breaker };
    <b>let</b> value = <a href="active_order_book.md#0x7_active_order_book_ActiveBidData">ActiveBidData</a> { order_id, size };
    // Assert that this is not a taker order
    <b>assert</b>!(!self.<a href="active_order_book.md#0x7_active_order_book_is_taker_order">is_taker_order</a>(price, is_buy), <a href="active_order_book.md#0x7_active_order_book_EINVALID_MAKER_ORDER">EINVALID_MAKER_ORDER</a>);
    <b>if</b> (is_buy) {
        self.buys.add(key, value);
    } <b>else</b> {
        self.sells.add(key, value);
    };
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
