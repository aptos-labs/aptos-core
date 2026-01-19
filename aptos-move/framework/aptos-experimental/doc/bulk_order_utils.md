
<a id="0x7_bulk_order_utils"></a>

# Module `0x7::bulk_order_utils`



-  [Constants](#@Constants_0)
-  [Function `reinsert_order_into_bulk_order`](#0x7_bulk_order_utils_reinsert_order_into_bulk_order)
    -  [Arguments:](#@Arguments:_1)
-  [Function `match_order_and_get_next_from_bulk_order`](#0x7_bulk_order_utils_match_order_and_get_next_from_bulk_order)
    -  [Arguments:](#@Arguments:_2)
    -  [Returns:](#@Returns:_3)
    -  [Aborts:](#@Aborts:_4)
-  [Function `cancel_at_price_level`](#0x7_bulk_order_utils_cancel_at_price_level)
    -  [Arguments:](#@Arguments:_5)
    -  [Returns:](#@Returns:_6)


<pre><code><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
<b>use</b> <a href="">0x5::bulk_order_types</a>;
<b>use</b> <a href="">0x5::order_match_types</a>;
</code></pre>



<a id="@Constants_0"></a>

## Constants


<a id="0x7_bulk_order_utils_EUNEXPECTED_MATCH_SIZE"></a>



<pre><code><b>const</b> <a href="bulk_order_utils.md#0x7_bulk_order_utils_EUNEXPECTED_MATCH_SIZE">EUNEXPECTED_MATCH_SIZE</a>: u64 = 2;
</code></pre>



<a id="0x7_bulk_order_utils_reinsert_order_into_bulk_order"></a>

## Function `reinsert_order_into_bulk_order`

Reinserts an order into a bulk order.

This function adds the reinserted order's price and size to the appropriate side
of the bulk order. If the price already exists at the first level, it increases
the size; otherwise, it inserts the new price level at the front.


<a id="@Arguments:_1"></a>

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
    <b>let</b> (prices, sizes) = order.get_order_request_mut().get_prices_and_sizes_mut(other.is_bid_from_match_details());
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


<a id="@Arguments:_2"></a>

### Arguments:

- <code>self</code>: Mutable reference to the bulk order
- <code>is_bid</code>: True if matching against bid side, false for ask side
- <code>matched_size</code>: Size that was matched in this operation


<a id="@Returns:_3"></a>

### Returns:

A tuple containing the next active price and size as options.


<a id="@Aborts:_4"></a>

### Aborts:

- If the matched size exceeds the available size at the first level


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="bulk_order_utils.md#0x7_bulk_order_utils_match_order_and_get_next_from_bulk_order">match_order_and_get_next_from_bulk_order</a>&lt;M: <b>copy</b>, drop, store&gt;(order: &<b>mut</b> <a href="_BulkOrder">bulk_order_types::BulkOrder</a>&lt;M&gt;, is_bid: bool, matched_size: u64): (<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="bulk_order_utils.md#0x7_bulk_order_utils_match_order_and_get_next_from_bulk_order">match_order_and_get_next_from_bulk_order</a>&lt;M: store + <b>copy</b> + drop&gt;(
    order: &<b>mut</b> BulkOrder&lt;M&gt;, is_bid: bool, matched_size: u64
): (Option&lt;u64&gt;, Option&lt;u64&gt;) {
    <b>let</b> (prices, sizes) = order.get_order_request_mut().get_prices_and_sizes_mut(is_bid);
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


<a id="@Arguments:_5"></a>

### Arguments:

- <code>order</code>: Mutable reference to the bulk order
- <code>price</code>: The price level to cancel
- <code>is_bid</code>: True to cancel from bid side, false for ask side


<a id="@Returns:_6"></a>

### Returns:

The size that was cancelled at that price level, or 0 if the price wasn't found


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="bulk_order_utils.md#0x7_bulk_order_utils_cancel_at_price_level">cancel_at_price_level</a>&lt;M: <b>copy</b>, drop, store&gt;(order: &<b>mut</b> <a href="_BulkOrder">bulk_order_types::BulkOrder</a>&lt;M&gt;, price: u64, is_bid: bool): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="bulk_order_utils.md#0x7_bulk_order_utils_cancel_at_price_level">cancel_at_price_level</a>&lt;M: store + <b>copy</b> + drop&gt;(
    order: &<b>mut</b> BulkOrder&lt;M&gt;,
    price: u64,
    is_bid: bool
): u64 {
    <b>let</b> (prices, sizes) = order.get_order_request_mut().get_prices_and_sizes_mut(is_bid);
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
