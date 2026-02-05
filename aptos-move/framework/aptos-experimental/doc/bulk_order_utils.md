
<a id="0x7_bulk_order_utils"></a>

# Module `0x7::bulk_order_utils`



-  [Constants](#@Constants_0)
-  [Function `new_bulk_order_with_sanitization`](#0x7_bulk_order_utils_new_bulk_order_with_sanitization)
    -  [Arguments:](#@Arguments:_1)
    -  [Returns:](#@Returns:_2)
-  [Function `trim_start`](#0x7_bulk_order_utils_trim_start)
-  [Function `discard_price_crossing_levels`](#0x7_bulk_order_utils_discard_price_crossing_levels)
-  [Function `reinsert_order_into_bulk_order`](#0x7_bulk_order_utils_reinsert_order_into_bulk_order)
    -  [Arguments:](#@Arguments:_3)
-  [Function `match_order_and_get_next_from_bulk_order`](#0x7_bulk_order_utils_match_order_and_get_next_from_bulk_order)
    -  [Arguments:](#@Arguments:_4)
    -  [Returns:](#@Returns:_5)
    -  [Aborts:](#@Aborts:_6)
-  [Function `cancel_at_price_level`](#0x7_bulk_order_utils_cancel_at_price_level)
    -  [Arguments:](#@Arguments:_7)
    -  [Returns:](#@Returns:_8)


<pre><code><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../aptos-framework/doc/timestamp.md#0x1_timestamp">0x1::timestamp</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
<b>use</b> <a href="">0x5::bulk_order_types</a>;
<b>use</b> <a href="">0x5::order_book_types</a>;
<b>use</b> <a href="">0x5::order_match_types</a>;
</code></pre>



<a id="@Constants_0"></a>

## Constants


<a id="0x7_bulk_order_utils_EUNEXPECTED_MATCH_SIZE"></a>



<pre><code><b>const</b> <a href="bulk_order_utils.md#0x7_bulk_order_utils_EUNEXPECTED_MATCH_SIZE">EUNEXPECTED_MATCH_SIZE</a>: u64 = 2;
</code></pre>



<a id="0x7_bulk_order_utils_new_bulk_order_with_sanitization"></a>

## Function `new_bulk_order_with_sanitization`

Creates a new bulk order with the specified parameters.


<a id="@Arguments:_1"></a>

### Arguments:

- <code>order_id</code>: Unique identifier for the order
- <code>unique_priority_idx</code>: Priority index for time-based ordering
- <code>order_req</code>: The bulk order request containing all order details
- <code>best_bid_price</code>: Current best bid price in the market
- <code>best_ask_price</code>: Current best ask price in the market


<a id="@Returns:_2"></a>

### Returns:

A tuple containing:
- <code>BulkOrder&lt;M&gt;</code>: The created bulk order with non-crossing levels
- <code><a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;</code>: Cancelled bid prices (levels that crossed the spread)
- <code><a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;</code>: Cancelled bid sizes corresponding to cancelled prices
- <code><a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;</code>: Cancelled ask prices (levels that crossed the spread)
- <code><a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;</code>: Cancelled ask sizes corresponding to cancelled prices


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="bulk_order_utils.md#0x7_bulk_order_utils_new_bulk_order_with_sanitization">new_bulk_order_with_sanitization</a>&lt;M: <b>copy</b>, drop, store&gt;(order_id: <a href="_OrderId">order_book_types::OrderId</a>, unique_priority_idx: <a href="_IncreasingIdx">order_book_types::IncreasingIdx</a>, order_req: <a href="_BulkOrderRequest">bulk_order_types::BulkOrderRequest</a>&lt;M&gt;, best_bid_price: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;, best_ask_price: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;): (<a href="_BulkOrder">bulk_order_types::BulkOrder</a>&lt;M&gt;, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="bulk_order_utils.md#0x7_bulk_order_utils_new_bulk_order_with_sanitization">new_bulk_order_with_sanitization</a>&lt;M: store + <b>copy</b> + drop&gt;(
    order_id: OrderId,
    unique_priority_idx: IncreasingIdx,
    order_req: BulkOrderRequest&lt;M&gt;,
    best_bid_price: Option&lt;u64&gt;,
    best_ask_price: Option&lt;u64&gt;
): (BulkOrder&lt;M&gt;, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;) {
    <b>let</b> creation_time_micros = <a href="../../aptos-framework/doc/timestamp.md#0x1_timestamp_now_microseconds">timestamp::now_microseconds</a>();
    <b>let</b> bid_price_crossing_idx =
        <a href="bulk_order_utils.md#0x7_bulk_order_utils_discard_price_crossing_levels">discard_price_crossing_levels</a>(
            &order_req.get_all_prices(<b>true</b>), best_ask_price, <b>true</b>
        );
    <b>let</b> ask_price_crossing_idx =
        <a href="bulk_order_utils.md#0x7_bulk_order_utils_discard_price_crossing_levels">discard_price_crossing_levels</a>(
            &order_req.get_all_prices(<b>false</b>), best_bid_price, <b>false</b>
        );

    // Extract cancelled levels (the ones that were discarded)
    <b>let</b> (cancelled_bid_prices, cancelled_bid_sizes) =
        <b>if</b> (bid_price_crossing_idx &gt; 0) {
            <b>let</b> cancelled_bid_prices =
                <a href="bulk_order_utils.md#0x7_bulk_order_utils_trim_start">trim_start</a>(
                    order_req.get_all_prices_mut(<b>true</b>), bid_price_crossing_idx
                );
            <b>let</b> cancelled_bid_sizes =
                <a href="bulk_order_utils.md#0x7_bulk_order_utils_trim_start">trim_start</a>(
                    order_req.get_all_sizes_mut(<b>true</b>), bid_price_crossing_idx
                );
            (cancelled_bid_prices, cancelled_bid_sizes)
        } <b>else</b> {
            (<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>&lt;u64&gt;(), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>&lt;u64&gt;())
        };
    <b>let</b> (cancelled_ask_prices, cancelled_ask_sizes) =
        <b>if</b> (ask_price_crossing_idx &gt; 0) {
            <b>let</b> cancelled_ask_prices =
                <a href="bulk_order_utils.md#0x7_bulk_order_utils_trim_start">trim_start</a>(
                    order_req.get_all_prices_mut(<b>false</b>), ask_price_crossing_idx
                );
            <b>let</b> cancelled_ask_sizes =
                <a href="bulk_order_utils.md#0x7_bulk_order_utils_trim_start">trim_start</a>(
                    order_req.get_all_sizes_mut(<b>false</b>), ask_price_crossing_idx
                );
            (cancelled_ask_prices, cancelled_ask_sizes)
        } <b>else</b> {
            (<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>&lt;u64&gt;(), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>&lt;u64&gt;())
        };
    <b>let</b> bulk_order =
        <a href="_new_bulk_order">bulk_order_types::new_bulk_order</a>(
            order_req,
            order_id,
            unique_priority_idx,
            creation_time_micros
        );
    (
        bulk_order,
        cancelled_bid_prices,
        cancelled_bid_sizes,
        cancelled_ask_prices,
        cancelled_ask_sizes
    )
}
</code></pre>



</details>

<a id="0x7_bulk_order_utils_trim_start"></a>

## Function `trim_start`



<pre><code><b>fun</b> <a href="bulk_order_utils.md#0x7_bulk_order_utils_trim_start">trim_start</a>&lt;Element&gt;(v: &<b>mut</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Element&gt;, new_start: u64): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Element&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="bulk_order_utils.md#0x7_bulk_order_utils_trim_start">trim_start</a>&lt;Element&gt;(v: &<b>mut</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Element&gt;, new_start: u64): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Element&gt; {
    <b>let</b> other = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>();
    <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_move_range">vector::move_range</a>(v, 0, new_start, &<b>mut</b> other, 0);
    other
}
</code></pre>



</details>

<a id="0x7_bulk_order_utils_discard_price_crossing_levels"></a>

## Function `discard_price_crossing_levels`



<pre><code><b>fun</b> <a href="bulk_order_utils.md#0x7_bulk_order_utils_discard_price_crossing_levels">discard_price_crossing_levels</a>(prices: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, best_price: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;, is_bid: bool): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="bulk_order_utils.md#0x7_bulk_order_utils_discard_price_crossing_levels">discard_price_crossing_levels</a>(
    prices: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, best_price: Option&lt;u64&gt;, is_bid: bool
): u64 {
    // Discard bid levels that are &gt;= best ask price
    <b>let</b> i = 0;
    <b>if</b> (best_price.is_some()) {
        <b>let</b> best_price = best_price.destroy_some();
        <b>while</b> (i &lt; prices.length()) {
            <b>if</b> (is_bid && prices[i] &lt; best_price) {
                <b>break</b>;
            } <b>else</b> <b>if</b> (!is_bid && prices[i] &gt; best_price) {
                <b>break</b>;
            };
            i += 1;
        };
    };
    i // Return the index of the first non-crossing level
}
</code></pre>



</details>

<a id="0x7_bulk_order_utils_reinsert_order_into_bulk_order"></a>

## Function `reinsert_order_into_bulk_order`

Reinserts an order into a bulk order.

This function adds the reinserted order's price and size to the appropriate side
of the bulk order. If the price already exists at the first level, it increases
the size; otherwise, it inserts the new price level at the front.


<a id="@Arguments:_3"></a>

### Arguments:

- <code>self</code>: Mutable reference to the bulk order
- <code>other</code>: Reference to the order result to reinsert


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="bulk_order_utils.md#0x7_bulk_order_utils_reinsert_order_into_bulk_order">reinsert_order_into_bulk_order</a>&lt;M: <b>copy</b>, drop, store&gt;(order: &<b>mut</b> <a href="_BulkOrder">bulk_order_types::BulkOrder</a>&lt;M&gt;, other: &<a href="_OrderMatchDetails">order_match_types::OrderMatchDetails</a>&lt;M&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="bulk_order_utils.md#0x7_bulk_order_utils_reinsert_order_into_bulk_order">reinsert_order_into_bulk_order</a>&lt;M: store + <b>copy</b> + drop&gt;(
    order: &<b>mut</b> BulkOrder&lt;M&gt;, other: &OrderMatchDetails&lt;M&gt;
) {
    // Reinsert the order into the bulk order
    <b>let</b> (prices, sizes) =
        order.get_order_request_mut().get_prices_and_sizes_mut(
            other.is_bid_from_match_details()
        );
    // Reinsert the price and size at the front of the respective vectors - <b>if</b> the price already <b>exists</b>, we ensure that
    // it is same <b>as</b> the reinsertion price and we just increase the size
    // If the price does not exist, we insert it at the front.
    <b>let</b> other_price = other.get_price_from_match_details();
    <b>if</b> (prices.length() &gt; 0 && prices[0] == other_price) {
        sizes[0] += other.get_remaining_size_from_match_details(); // Increase the size at the first price level
    } <b>else</b> {
        prices.insert(0, other_price); // Insert the new price at the front
        sizes.insert(0, other.get_remaining_size_from_match_details()); // Insert the new size at the front
    }
}
</code></pre>



</details>

<a id="0x7_bulk_order_utils_match_order_and_get_next_from_bulk_order"></a>

## Function `match_order_and_get_next_from_bulk_order`

Matches an order and returns the next active price and size.

This function reduces the size at the first price level by the matched size.
If the first level becomes empty, it is removed and the next level becomes active.


<a id="@Arguments:_4"></a>

### Arguments:

- <code>self</code>: Mutable reference to the bulk order
- <code>is_bid</code>: True if matching against bid side, false for ask side
- <code>matched_size</code>: Size that was matched in this operation


<a id="@Returns:_5"></a>

### Returns:

A tuple containing the next active price and size as options.


<a id="@Aborts:_6"></a>

### Aborts:

- If the matched size exceeds the available size at the first level


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="bulk_order_utils.md#0x7_bulk_order_utils_match_order_and_get_next_from_bulk_order">match_order_and_get_next_from_bulk_order</a>&lt;M: <b>copy</b>, drop, store&gt;(order: &<b>mut</b> <a href="_BulkOrder">bulk_order_types::BulkOrder</a>&lt;M&gt;, is_bid: bool, matched_size: u64): (<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="bulk_order_utils.md#0x7_bulk_order_utils_match_order_and_get_next_from_bulk_order">match_order_and_get_next_from_bulk_order</a>&lt;M: store + <b>copy</b> + drop&gt;(
    order: &<b>mut</b> BulkOrder&lt;M&gt;, is_bid: bool, matched_size: u64
): (Option&lt;u64&gt;, Option&lt;u64&gt;) {
    <b>let</b> (prices, sizes) =
        order.get_order_request_mut().get_prices_and_sizes_mut(is_bid);
    <b>assert</b>!(matched_size &lt;= sizes[0], <a href="bulk_order_utils.md#0x7_bulk_order_utils_EUNEXPECTED_MATCH_SIZE">EUNEXPECTED_MATCH_SIZE</a>); // Ensure the remaining size is not more than the size at the first price level
    sizes[0] -= matched_size; // Decrease the size at the first price level by the matched size
    <b>if</b> (sizes[0] == 0) {
        // If the size at the first price level is now 0, remove this price level
        prices.remove(0);
        sizes.remove(0);
    };
    <b>if</b> (sizes.length() == 0) {
        (<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()) // No active price or size left
    } <b>else</b> {
        (<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(prices[0]), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(sizes[0])) // Return the next active price and size
    }
}
</code></pre>



</details>

<a id="0x7_bulk_order_utils_cancel_at_price_level"></a>

## Function `cancel_at_price_level`

Cancels a specific price level in a bulk order by setting its size to 0 and removing it.

This function finds the price level matching the specified price and removes it from
the order, keeping other price levels intact.


<a id="@Arguments:_7"></a>

### Arguments:

- <code>order</code>: Mutable reference to the bulk order
- <code>price</code>: The price level to cancel
- <code>is_bid</code>: True to cancel from bid side, false for ask side


<a id="@Returns:_8"></a>

### Returns:

The size that was cancelled at that price level, or 0 if the price wasn't found


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="bulk_order_utils.md#0x7_bulk_order_utils_cancel_at_price_level">cancel_at_price_level</a>&lt;M: <b>copy</b>, drop, store&gt;(order: &<b>mut</b> <a href="_BulkOrder">bulk_order_types::BulkOrder</a>&lt;M&gt;, price: u64, is_bid: bool): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="bulk_order_utils.md#0x7_bulk_order_utils_cancel_at_price_level">cancel_at_price_level</a>&lt;M: store + <b>copy</b> + drop&gt;(
    order: &<b>mut</b> BulkOrder&lt;M&gt;, price: u64, is_bid: bool
): u64 {
    <b>let</b> (prices, sizes) =
        order.get_order_request_mut().get_prices_and_sizes_mut(is_bid);
    <b>let</b> i = 0;
    <b>while</b> (i &lt; prices.length()) {
        <b>if</b> (prices[i] == price) {
            // Found the price level, remove it
            <b>let</b> cancelled_size = sizes[i];
            prices.remove(i);
            sizes.remove(i);
            <b>return</b> cancelled_size
        };
        i = i + 1;
    };
    0 // Price not found
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
