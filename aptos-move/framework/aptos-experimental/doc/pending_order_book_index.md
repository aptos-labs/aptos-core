
<a id="0x7_pending_order_book_index"></a>

# Module `0x7::pending_order_book_index`

(work in progress)


-  [Struct `PendingUpOrderKey`](#0x7_pending_order_book_index_PendingUpOrderKey)
-  [Struct `PendingDownOrderKey`](#0x7_pending_order_book_index_PendingDownOrderKey)
-  [Struct `PendingTimeKey`](#0x7_pending_order_book_index_PendingTimeKey)
-  [Enum `PendingOrderBookIndex`](#0x7_pending_order_book_index_PendingOrderBookIndex)
-  [Function `new_pending_order_book_index`](#0x7_pending_order_book_index_new_pending_order_book_index)
-  [Function `cancel_pending_order`](#0x7_pending_order_book_index_cancel_pending_order)
-  [Function `place_pending_order`](#0x7_pending_order_book_index_place_pending_order)
-  [Function `take_ready_price_move_up_orders`](#0x7_pending_order_book_index_take_ready_price_move_up_orders)
-  [Function `take_ready_price_move_down_orders`](#0x7_pending_order_book_index_take_ready_price_move_down_orders)
-  [Function `take_ready_price_based_orders`](#0x7_pending_order_book_index_take_ready_price_based_orders)
-  [Function `take_ready_time_based_orders`](#0x7_pending_order_book_index_take_ready_time_based_orders)


<pre><code><b>use</b> <a href="../../aptos-framework/doc/big_ordered_map.md#0x1_big_ordered_map">0x1::big_ordered_map</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../aptos-framework/doc/timestamp.md#0x1_timestamp">0x1::timestamp</a>;
<b>use</b> <a href="">0x5::order_book_types</a>;
</code></pre>



<a id="0x7_pending_order_book_index_PendingUpOrderKey"></a>

## Struct `PendingUpOrderKey`



<pre><code><b>struct</b> <a href="pending_order_book_index.md#0x7_pending_order_book_index_PendingUpOrderKey">PendingUpOrderKey</a> <b>has</b> <b>copy</b>, drop, store
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

<a id="0x7_pending_order_book_index_PendingDownOrderKey"></a>

## Struct `PendingDownOrderKey`



<pre><code><b>struct</b> <a href="pending_order_book_index.md#0x7_pending_order_book_index_PendingDownOrderKey">PendingDownOrderKey</a> <b>has</b> <b>copy</b>, drop, store
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

<a id="0x7_pending_order_book_index_PendingTimeKey"></a>

## Struct `PendingTimeKey`



<pre><code><b>struct</b> <a href="pending_order_book_index.md#0x7_pending_order_book_index_PendingTimeKey">PendingTimeKey</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>time: u64</code>
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

<a id="0x7_pending_order_book_index_PendingOrderBookIndex"></a>

## Enum `PendingOrderBookIndex`



<pre><code>enum <a href="pending_order_book_index.md#0x7_pending_order_book_index_PendingOrderBookIndex">PendingOrderBookIndex</a> <b>has</b> store
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>V1</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>price_move_down_index: <a href="../../aptos-framework/doc/big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">big_ordered_map::BigOrderedMap</a>&lt;<a href="pending_order_book_index.md#0x7_pending_order_book_index_PendingDownOrderKey">pending_order_book_index::PendingDownOrderKey</a>, <a href="_OrderId">order_book_types::OrderId</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>price_move_up_index: <a href="../../aptos-framework/doc/big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">big_ordered_map::BigOrderedMap</a>&lt;<a href="pending_order_book_index.md#0x7_pending_order_book_index_PendingUpOrderKey">pending_order_book_index::PendingUpOrderKey</a>, <a href="_OrderId">order_book_types::OrderId</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>time_based_index: <a href="../../aptos-framework/doc/big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">big_ordered_map::BigOrderedMap</a>&lt;<a href="pending_order_book_index.md#0x7_pending_order_book_index_PendingTimeKey">pending_order_book_index::PendingTimeKey</a>, <a href="_OrderId">order_book_types::OrderId</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

</details>

<a id="0x7_pending_order_book_index_new_pending_order_book_index"></a>

## Function `new_pending_order_book_index`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="pending_order_book_index.md#0x7_pending_order_book_index_new_pending_order_book_index">new_pending_order_book_index</a>(): <a href="pending_order_book_index.md#0x7_pending_order_book_index_PendingOrderBookIndex">pending_order_book_index::PendingOrderBookIndex</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="pending_order_book_index.md#0x7_pending_order_book_index_new_pending_order_book_index">new_pending_order_book_index</a>(): <a href="pending_order_book_index.md#0x7_pending_order_book_index_PendingOrderBookIndex">PendingOrderBookIndex</a> {
    PendingOrderBookIndex::V1 {
        price_move_up_index: <a href="order_book_utils.md#0x7_order_book_utils_new_default_big_ordered_map">order_book_utils::new_default_big_ordered_map</a>(),
        price_move_down_index: <a href="order_book_utils.md#0x7_order_book_utils_new_default_big_ordered_map">order_book_utils::new_default_big_ordered_map</a>(),
        time_based_index: <a href="order_book_utils.md#0x7_order_book_utils_new_default_big_ordered_map">order_book_utils::new_default_big_ordered_map</a>()
    }
}
</code></pre>



</details>

<a id="0x7_pending_order_book_index_cancel_pending_order"></a>

## Function `cancel_pending_order`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="pending_order_book_index.md#0x7_pending_order_book_index_cancel_pending_order">cancel_pending_order</a>(self: &<b>mut</b> <a href="pending_order_book_index.md#0x7_pending_order_book_index_PendingOrderBookIndex">pending_order_book_index::PendingOrderBookIndex</a>, trigger_condition: <a href="_TriggerCondition">order_book_types::TriggerCondition</a>, unique_priority_idx: <a href="_IncreasingIdx">order_book_types::IncreasingIdx</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="pending_order_book_index.md#0x7_pending_order_book_index_cancel_pending_order">cancel_pending_order</a>(
    self: &<b>mut</b> <a href="pending_order_book_index.md#0x7_pending_order_book_index_PendingOrderBookIndex">PendingOrderBookIndex</a>,
    trigger_condition: TriggerCondition,
    unique_priority_idx: IncreasingIdx
) {
    <b>let</b> (price_move_down_index, price_move_up_index, time_based_index) =
        trigger_condition.get_trigger_condition_indices();
    <b>if</b> (price_move_up_index.is_some()) {
        self.price_move_up_index.remove(
            &<a href="pending_order_book_index.md#0x7_pending_order_book_index_PendingUpOrderKey">PendingUpOrderKey</a> {
                price: price_move_up_index.destroy_some(),
                tie_breaker: unique_priority_idx
            }
        );
    };
    <b>if</b> (price_move_down_index.is_some()) {
        self.price_move_down_index.remove(
            &<a href="pending_order_book_index.md#0x7_pending_order_book_index_PendingDownOrderKey">PendingDownOrderKey</a> {
                price: price_move_down_index.destroy_some(),
                tie_breaker: unique_priority_idx.into_decreasing_idx_type()
            }
        );
    };
    <b>if</b> (time_based_index.is_some()) {
        self.time_based_index.remove(
            &<a href="pending_order_book_index.md#0x7_pending_order_book_index_PendingTimeKey">PendingTimeKey</a> {
                time: time_based_index.destroy_some(),
                tie_breaker: unique_priority_idx
            }
        );
    };
}
</code></pre>



</details>

<a id="0x7_pending_order_book_index_place_pending_order"></a>

## Function `place_pending_order`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="pending_order_book_index.md#0x7_pending_order_book_index_place_pending_order">place_pending_order</a>(self: &<b>mut</b> <a href="pending_order_book_index.md#0x7_pending_order_book_index_PendingOrderBookIndex">pending_order_book_index::PendingOrderBookIndex</a>, order_id: <a href="_OrderId">order_book_types::OrderId</a>, trigger_condition: <a href="_TriggerCondition">order_book_types::TriggerCondition</a>, unique_priority_idx: <a href="_IncreasingIdx">order_book_types::IncreasingIdx</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="pending_order_book_index.md#0x7_pending_order_book_index_place_pending_order">place_pending_order</a>(
    self: &<b>mut</b> <a href="pending_order_book_index.md#0x7_pending_order_book_index_PendingOrderBookIndex">PendingOrderBookIndex</a>,
    order_id: OrderId,
    trigger_condition: TriggerCondition,
    unique_priority_idx: IncreasingIdx
) {
    // Add this order <b>to</b> the pending order book index
    <b>let</b> (price_move_down_index, price_move_up_index, time_based_index) =
        trigger_condition.get_trigger_condition_indices();
    <b>if</b> (price_move_up_index.is_some()) {
        self.price_move_up_index.add(
            <a href="pending_order_book_index.md#0x7_pending_order_book_index_PendingUpOrderKey">PendingUpOrderKey</a> {
                price: price_move_up_index.destroy_some(),
                tie_breaker: unique_priority_idx
            },
            order_id
        );
    } <b>else</b> <b>if</b> (price_move_down_index.is_some()) {
        self.price_move_down_index.add(
            <a href="pending_order_book_index.md#0x7_pending_order_book_index_PendingDownOrderKey">PendingDownOrderKey</a> {
                price: price_move_down_index.destroy_some(),
                // Use a descending tie breaker <b>to</b> ensure that for price <b>move</b> down orders,
                // orders <b>with</b> the same price are processed in FIFO order
                tie_breaker: unique_priority_idx.into_decreasing_idx_type()
            },
            order_id
        );
    } <b>else</b> <b>if</b> (time_based_index.is_some()) {
        self.time_based_index.add(
            <a href="pending_order_book_index.md#0x7_pending_order_book_index_PendingTimeKey">PendingTimeKey</a> {
                time: time_based_index.destroy_some(),
                tie_breaker: unique_priority_idx
            },
            order_id
        );
    };
}
</code></pre>



</details>

<a id="0x7_pending_order_book_index_take_ready_price_move_up_orders"></a>

## Function `take_ready_price_move_up_orders`



<pre><code><b>fun</b> <a href="pending_order_book_index.md#0x7_pending_order_book_index_take_ready_price_move_up_orders">take_ready_price_move_up_orders</a>(self: &<b>mut</b> <a href="pending_order_book_index.md#0x7_pending_order_book_index_PendingOrderBookIndex">pending_order_book_index::PendingOrderBookIndex</a>, current_price: u64, orders: &<b>mut</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="_OrderId">order_book_types::OrderId</a>&gt;, limit: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="pending_order_book_index.md#0x7_pending_order_book_index_take_ready_price_move_up_orders">take_ready_price_move_up_orders</a>(
    self: &<b>mut</b> <a href="pending_order_book_index.md#0x7_pending_order_book_index_PendingOrderBookIndex">PendingOrderBookIndex</a>,
    current_price: u64,
    orders: &<b>mut</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;OrderId&gt;,
    limit: u64
) {
    <b>while</b> (!self.price_move_up_index.is_empty() && orders.length() &lt; limit) {
        <b>let</b> (key, order_id) = self.price_move_up_index.borrow_front();
        <b>if</b> (current_price &gt;= key.price) {
            orders.push_back(*order_id);
            self.price_move_up_index.remove(&key);
        } <b>else</b> {
            <b>break</b>;
        }
    };
}
</code></pre>



</details>

<a id="0x7_pending_order_book_index_take_ready_price_move_down_orders"></a>

## Function `take_ready_price_move_down_orders`



<pre><code><b>fun</b> <a href="pending_order_book_index.md#0x7_pending_order_book_index_take_ready_price_move_down_orders">take_ready_price_move_down_orders</a>(self: &<b>mut</b> <a href="pending_order_book_index.md#0x7_pending_order_book_index_PendingOrderBookIndex">pending_order_book_index::PendingOrderBookIndex</a>, current_price: u64, orders: &<b>mut</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="_OrderId">order_book_types::OrderId</a>&gt;, limit: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="pending_order_book_index.md#0x7_pending_order_book_index_take_ready_price_move_down_orders">take_ready_price_move_down_orders</a>(
    self: &<b>mut</b> <a href="pending_order_book_index.md#0x7_pending_order_book_index_PendingOrderBookIndex">PendingOrderBookIndex</a>,
    current_price: u64,
    orders: &<b>mut</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;OrderId&gt;,
    limit: u64
) {
    <b>while</b> (!self.price_move_down_index.is_empty() && orders.length() &lt; limit) {
        <b>let</b> (key, order_id) = self.price_move_down_index.borrow_back();
        <b>if</b> (current_price &lt;= key.price) {
            orders.push_back(*order_id);
            self.price_move_down_index.remove(&key);
        } <b>else</b> {
            <b>break</b>;
        }
    };
}
</code></pre>



</details>

<a id="0x7_pending_order_book_index_take_ready_price_based_orders"></a>

## Function `take_ready_price_based_orders`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="pending_order_book_index.md#0x7_pending_order_book_index_take_ready_price_based_orders">take_ready_price_based_orders</a>(self: &<b>mut</b> <a href="pending_order_book_index.md#0x7_pending_order_book_index_PendingOrderBookIndex">pending_order_book_index::PendingOrderBookIndex</a>, current_price: u64, order_limit: u64): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="_OrderId">order_book_types::OrderId</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="pending_order_book_index.md#0x7_pending_order_book_index_take_ready_price_based_orders">take_ready_price_based_orders</a>(
    self: &<b>mut</b> <a href="pending_order_book_index.md#0x7_pending_order_book_index_PendingOrderBookIndex">PendingOrderBookIndex</a>, current_price: u64, order_limit: u64
): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;OrderId&gt; {
    <b>let</b> orders = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>();
    self.<a href="pending_order_book_index.md#0x7_pending_order_book_index_take_ready_price_move_up_orders">take_ready_price_move_up_orders</a>(
        current_price,
        &<b>mut</b> orders,
        <a href="../../aptos-framework/../aptos-stdlib/doc/math64.md#0x1_math64_ceil_div">math64::ceil_div</a>(order_limit, 2)
    );
    self.<a href="pending_order_book_index.md#0x7_pending_order_book_index_take_ready_price_move_down_orders">take_ready_price_move_down_orders</a>(current_price, &<b>mut</b> orders, order_limit);
    // Try <b>to</b> fill the rest of the space <b>if</b> available.
    self.<a href="pending_order_book_index.md#0x7_pending_order_book_index_take_ready_price_move_up_orders">take_ready_price_move_up_orders</a>(current_price, &<b>mut</b> orders, order_limit);
    orders
}
</code></pre>



</details>

<a id="0x7_pending_order_book_index_take_ready_time_based_orders"></a>

## Function `take_ready_time_based_orders`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="pending_order_book_index.md#0x7_pending_order_book_index_take_ready_time_based_orders">take_ready_time_based_orders</a>(self: &<b>mut</b> <a href="pending_order_book_index.md#0x7_pending_order_book_index_PendingOrderBookIndex">pending_order_book_index::PendingOrderBookIndex</a>, order_limit: u64): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="_OrderId">order_book_types::OrderId</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="pending_order_book_index.md#0x7_pending_order_book_index_take_ready_time_based_orders">take_ready_time_based_orders</a>(
    self: &<b>mut</b> <a href="pending_order_book_index.md#0x7_pending_order_book_index_PendingOrderBookIndex">PendingOrderBookIndex</a>, order_limit: u64
): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;OrderId&gt; {
    <b>let</b> orders = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>();
    <b>while</b> (!self.time_based_index.is_empty() && orders.length() &lt; order_limit) {
        <b>let</b> current_time = <a href="../../aptos-framework/doc/timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>();
        <b>let</b> (time, order_id) = self.time_based_index.borrow_front();
        <b>if</b> (current_time &gt;= time.time) {
            orders.push_back(*order_id);
            self.time_based_index.remove(&time);
        } <b>else</b> {
            <b>break</b>;
        }
    };
    orders
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
