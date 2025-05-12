
<a id="0x7_pending_order_book_index"></a>

# Module `0x7::pending_order_book_index`

(work in progress)


-  [Struct `PendingOrderKey`](#0x7_pending_order_book_index_PendingOrderKey)
-  [Enum `PendingOrderBookIndex`](#0x7_pending_order_book_index_PendingOrderBookIndex)
-  [Function `new_pending_order_book_index`](#0x7_pending_order_book_index_new_pending_order_book_index)
-  [Function `cancel_pending_order`](#0x7_pending_order_book_index_cancel_pending_order)
-  [Function `place_pending_maker_order`](#0x7_pending_order_book_index_place_pending_maker_order)
-  [Function `take_ready_price_based_orders`](#0x7_pending_order_book_index_take_ready_price_based_orders)
-  [Function `take_time_time_based_orders`](#0x7_pending_order_book_index_take_time_time_based_orders)


<pre><code><b>use</b> <a href="../../aptos-framework/doc/big_ordered_map.md#0x1_big_ordered_map">0x1::big_ordered_map</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../aptos-framework/doc/timestamp.md#0x1_timestamp">0x1::timestamp</a>;
<b>use</b> <a href="order_book_types.md#0x7_order_book_types">0x7::order_book_types</a>;
</code></pre>



<a id="0x7_pending_order_book_index_PendingOrderKey"></a>

## Struct `PendingOrderKey`



<pre><code><b>struct</b> <a href="pending_order_book_index.md#0x7_pending_order_book_index_PendingOrderKey">PendingOrderKey</a> <b>has</b> <b>copy</b>, drop, store
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
<code>price_move_down_index: <a href="../../aptos-framework/doc/big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">big_ordered_map::BigOrderedMap</a>&lt;<a href="pending_order_book_index.md#0x7_pending_order_book_index_PendingOrderKey">pending_order_book_index::PendingOrderKey</a>, <a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>price_move_up_index: <a href="../../aptos-framework/doc/big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">big_ordered_map::BigOrderedMap</a>&lt;<a href="pending_order_book_index.md#0x7_pending_order_book_index_PendingOrderKey">pending_order_book_index::PendingOrderKey</a>, <a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>time_based_index: <a href="../../aptos-framework/doc/big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">big_ordered_map::BigOrderedMap</a>&lt;u64, <a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>&gt;</code>
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
        price_move_up_index: new_default_big_ordered_map(),
        price_move_down_index: new_default_big_ordered_map(),
        time_based_index: new_default_big_ordered_map()
    }
}
</code></pre>



</details>

<a id="0x7_pending_order_book_index_cancel_pending_order"></a>

## Function `cancel_pending_order`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="pending_order_book_index.md#0x7_pending_order_book_index_cancel_pending_order">cancel_pending_order</a>(self: &<b>mut</b> <a href="pending_order_book_index.md#0x7_pending_order_book_index_PendingOrderBookIndex">pending_order_book_index::PendingOrderBookIndex</a>, trigger_condition: <a href="order_book_types.md#0x7_order_book_types_TriggerCondition">order_book_types::TriggerCondition</a>, unique_priority_idx: <a href="order_book_types.md#0x7_order_book_types_UniqueIdxType">order_book_types::UniqueIdxType</a>, is_buy: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="pending_order_book_index.md#0x7_pending_order_book_index_cancel_pending_order">cancel_pending_order</a>(
    self: &<b>mut</b> <a href="pending_order_book_index.md#0x7_pending_order_book_index_PendingOrderBookIndex">PendingOrderBookIndex</a>,
    trigger_condition: TriggerCondition,
    unique_priority_idx: UniqueIdxType,
    is_buy: bool
) {
    <b>let</b> (price_move_up_index, price_move_down_index, time_based_index) =
        trigger_condition.index(is_buy);
    <b>if</b> (price_move_up_index.is_some()) {
        self.price_move_up_index.remove(
            &<a href="pending_order_book_index.md#0x7_pending_order_book_index_PendingOrderKey">PendingOrderKey</a> {
                price: price_move_up_index.destroy_some(),
                tie_breaker: unique_priority_idx
            }
        );
    };
    <b>if</b> (price_move_down_index.is_some()) {
        self.price_move_down_index.remove(
            &<a href="pending_order_book_index.md#0x7_pending_order_book_index_PendingOrderKey">PendingOrderKey</a> {
                price: price_move_down_index.destroy_some(),
                tie_breaker: unique_priority_idx
            }
        );
    };
    <b>if</b> (time_based_index.is_some()) {
        self.time_based_index.remove(&time_based_index.destroy_some());
    };
}
</code></pre>



</details>

<a id="0x7_pending_order_book_index_place_pending_maker_order"></a>

## Function `place_pending_maker_order`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="pending_order_book_index.md#0x7_pending_order_book_index_place_pending_maker_order">place_pending_maker_order</a>(self: &<b>mut</b> <a href="pending_order_book_index.md#0x7_pending_order_book_index_PendingOrderBookIndex">pending_order_book_index::PendingOrderBookIndex</a>, order_id: <a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>, trigger_condition: <a href="order_book_types.md#0x7_order_book_types_TriggerCondition">order_book_types::TriggerCondition</a>, unique_priority_idx: <a href="order_book_types.md#0x7_order_book_types_UniqueIdxType">order_book_types::UniqueIdxType</a>, is_buy: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="pending_order_book_index.md#0x7_pending_order_book_index_place_pending_maker_order">place_pending_maker_order</a>(
    self: &<b>mut</b> <a href="pending_order_book_index.md#0x7_pending_order_book_index_PendingOrderBookIndex">PendingOrderBookIndex</a>,
    order_id: OrderIdType,
    trigger_condition: TriggerCondition,
    unique_priority_idx: UniqueIdxType,
    is_buy: bool
) {
    // Add this order <b>to</b> the pending order book index
    <b>let</b> (price_move_down_index, price_move_up_index, time_based_index) =
        trigger_condition.index(is_buy);

    <b>if</b> (price_move_up_index.is_some()) {
        self.price_move_up_index.add(
            <a href="pending_order_book_index.md#0x7_pending_order_book_index_PendingOrderKey">PendingOrderKey</a> {
                price: price_move_up_index.destroy_some(),
                tie_breaker: unique_priority_idx
            },
            order_id
        );
    } <b>else</b> <b>if</b> (price_move_down_index.is_some()) {
        self.price_move_down_index.add(
            <a href="pending_order_book_index.md#0x7_pending_order_book_index_PendingOrderKey">PendingOrderKey</a> {
                price: price_move_down_index.destroy_some(),
                tie_breaker: unique_priority_idx
            },
            order_id
        );
    } <b>else</b> <b>if</b> (time_based_index.is_some()) {
        self.time_based_index.add(time_based_index.destroy_some(), order_id);
    };
}
</code></pre>



</details>

<a id="0x7_pending_order_book_index_take_ready_price_based_orders"></a>

## Function `take_ready_price_based_orders`



<pre><code><b>public</b> <b>fun</b> <a href="pending_order_book_index.md#0x7_pending_order_book_index_take_ready_price_based_orders">take_ready_price_based_orders</a>(self: &<b>mut</b> <a href="pending_order_book_index.md#0x7_pending_order_book_index_PendingOrderBookIndex">pending_order_book_index::PendingOrderBookIndex</a>, current_price: u64): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="pending_order_book_index.md#0x7_pending_order_book_index_take_ready_price_based_orders">take_ready_price_based_orders</a>(
    self: &<b>mut</b> <a href="pending_order_book_index.md#0x7_pending_order_book_index_PendingOrderBookIndex">PendingOrderBookIndex</a>, current_price: u64
): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;OrderIdType&gt; {
    <b>let</b> orders = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>();
    <b>while</b> (!self.price_move_up_index.is_empty()) {
        <b>let</b> (key, order_id) = self.price_move_up_index.borrow_front();
        <b>if</b> (current_price &gt;= key.price) {
            orders.push_back(*order_id);
            self.price_move_up_index.remove(&key);
        } <b>else</b> {
            <b>break</b>;
        }
    };
    <b>while</b> (!self.price_move_down_index.is_empty()) {
        <b>let</b> (key, order_id) = self.price_move_down_index.borrow_back();
        <b>if</b> (current_price &lt;= key.price) {
            orders.push_back(*order_id);
            self.price_move_down_index.remove(&key);
        } <b>else</b> {
            <b>break</b>;
        }
    };
    orders
}
</code></pre>



</details>

<a id="0x7_pending_order_book_index_take_time_time_based_orders"></a>

## Function `take_time_time_based_orders`



<pre><code><b>public</b> <b>fun</b> <a href="pending_order_book_index.md#0x7_pending_order_book_index_take_time_time_based_orders">take_time_time_based_orders</a>(self: &<b>mut</b> <a href="pending_order_book_index.md#0x7_pending_order_book_index_PendingOrderBookIndex">pending_order_book_index::PendingOrderBookIndex</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="pending_order_book_index.md#0x7_pending_order_book_index_take_time_time_based_orders">take_time_time_based_orders</a>(
    self: &<b>mut</b> <a href="pending_order_book_index.md#0x7_pending_order_book_index_PendingOrderBookIndex">PendingOrderBookIndex</a>
): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;OrderIdType&gt; {
    <b>let</b> orders = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>();
    <b>while</b> (!self.time_based_index.is_empty()) {
        <b>let</b> current_time = <a href="../../aptos-framework/doc/timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>();
        <b>let</b> (time, order_id) = self.time_based_index.borrow_front();
        <b>if</b> (current_time &gt;= time) {
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
