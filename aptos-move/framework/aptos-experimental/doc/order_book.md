
<a id="0x7_order_book"></a>

# Module `0x7::order_book`



-  [Enum `OrderBook`](#0x7_order_book_OrderBook)
-  [Function `new_order_book`](#0x7_order_book_new_order_book)
-  [Function `new_order_request`](#0x7_order_book_new_order_request)
-  [Function `cancel_order`](#0x7_order_book_cancel_order)
-  [Function `try_cancel_order_with_client_order_id`](#0x7_order_book_try_cancel_order_with_client_order_id)
-  [Function `client_order_id_exists`](#0x7_order_book_client_order_id_exists)
-  [Function `is_taker_order`](#0x7_order_book_is_taker_order)
-  [Function `get_single_match_for_taker`](#0x7_order_book_get_single_match_for_taker)
-  [Function `reinsert_maker_order`](#0x7_order_book_reinsert_maker_order)
-  [Function `place_maker_order`](#0x7_order_book_place_maker_order)
-  [Function `decrease_order_size`](#0x7_order_book_decrease_order_size)
-  [Function `get_order_id_by_client_id`](#0x7_order_book_get_order_id_by_client_id)
-  [Function `get_order_metadata`](#0x7_order_book_get_order_metadata)
-  [Function `set_order_metadata`](#0x7_order_book_set_order_metadata)
-  [Function `is_active_order`](#0x7_order_book_is_active_order)
-  [Function `get_order`](#0x7_order_book_get_order)
-  [Function `get_remaining_size`](#0x7_order_book_get_remaining_size)
-  [Function `take_ready_price_based_orders`](#0x7_order_book_take_ready_price_based_orders)
-  [Function `take_ready_time_based_orders`](#0x7_order_book_take_ready_time_based_orders)
-  [Function `best_bid_price`](#0x7_order_book_best_bid_price)
-  [Function `best_ask_price`](#0x7_order_book_best_ask_price)
-  [Function `get_slippage_price`](#0x7_order_book_get_slippage_price)


<pre><code><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="order_book_types.md#0x7_order_book_types">0x7::order_book_types</a>;
<b>use</b> <a href="price_time_index.md#0x7_price_time_index">0x7::price_time_index</a>;
<b>use</b> <a href="single_order_book.md#0x7_single_order_book">0x7::single_order_book</a>;
<b>use</b> <a href="single_order_types.md#0x7_single_order_types">0x7::single_order_types</a>;
</code></pre>



<a id="0x7_order_book_OrderBook"></a>

## Enum `OrderBook`



<pre><code>enum <a href="order_book.md#0x7_order_book_OrderBook">OrderBook</a>&lt;M: <b>copy</b>, drop, store&gt; <b>has</b> store
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>UnifiedV1</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>retail_order_book: <a href="single_order_book.md#0x7_single_order_book_RetailOrderBook">single_order_book::RetailOrderBook</a>&lt;M&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>price_time_idx: <a href="price_time_index.md#0x7_price_time_index_PriceTimeIndex">price_time_index::PriceTimeIndex</a></code>
</dt>
<dd>

</dd>
<dt>
<code>ascending_id_generator: <a href="order_book_types.md#0x7_order_book_types_AscendingIdGenerator">order_book_types::AscendingIdGenerator</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

</details>

<a id="0x7_order_book_new_order_book"></a>

## Function `new_order_book`



<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_new_order_book">new_order_book</a>&lt;M: <b>copy</b>, drop, store&gt;(): <a href="order_book.md#0x7_order_book_OrderBook">order_book::OrderBook</a>&lt;M&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_new_order_book">new_order_book</a>&lt;M: store + <b>copy</b> + drop&gt;(): <a href="order_book.md#0x7_order_book_OrderBook">OrderBook</a>&lt;M&gt; {
    OrderBook::UnifiedV1 {
        retail_order_book: new_single_order_book(),
        price_time_idx: new_price_time_idx(),
        ascending_id_generator: new_ascending_id_generator(),
    }
}
</code></pre>



</details>

<a id="0x7_order_book_new_order_request"></a>

## Function `new_order_request`



<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_new_order_request">new_order_request</a>&lt;M: <b>copy</b>, drop, store&gt;(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>, order_id: <a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>, client_order_id: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;, price: u64, orig_size: u64, remaining_size: u64, is_bid: bool, trigger_condition: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="order_book_types.md#0x7_order_book_types_TriggerCondition">order_book_types::TriggerCondition</a>&gt;, time_in_force: <a href="order_book_types.md#0x7_order_book_types_TimeInForce">order_book_types::TimeInForce</a>, metadata: M): <a href="single_order_book.md#0x7_single_order_book_OrderRequest">single_order_book::OrderRequest</a>&lt;M&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_new_order_request">new_order_request</a>&lt;M: store + <b>copy</b> + drop&gt;(
    <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>,
    order_id: OrderIdType,
    client_order_id: Option&lt;u64&gt;,
    price: u64,
    orig_size: u64,
    remaining_size: u64,
    is_bid: bool,
    trigger_condition: Option&lt;TriggerCondition&gt;,
    time_in_force: TimeInForce,
    metadata: M
): OrderRequest&lt;M&gt; {
    aptos_experimental::single_order_book::new_order_request(
        <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>,
        order_id,
        client_order_id,
        price,
        orig_size,
        remaining_size,
        is_bid,
        trigger_condition,
        time_in_force,
        metadata
    )
}
</code></pre>



</details>

<a id="0x7_order_book_cancel_order"></a>

## Function `cancel_order`



<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_cancel_order">cancel_order</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="order_book.md#0x7_order_book_OrderBook">order_book::OrderBook</a>&lt;M&gt;, order_creator: <b>address</b>, order_id: <a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>): <a href="single_order_types.md#0x7_single_order_types_Order">single_order_types::Order</a>&lt;M&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_cancel_order">cancel_order</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="order_book.md#0x7_order_book_OrderBook">OrderBook</a>&lt;M&gt;, order_creator: <b>address</b>, order_id: OrderIdType
): Order&lt;M&gt; {
    self.retail_order_book.<a href="order_book.md#0x7_order_book_cancel_order">cancel_order</a>(&<b>mut</b> self.price_time_idx, order_creator, order_id)
}
</code></pre>



</details>

<a id="0x7_order_book_try_cancel_order_with_client_order_id"></a>

## Function `try_cancel_order_with_client_order_id`



<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_try_cancel_order_with_client_order_id">try_cancel_order_with_client_order_id</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="order_book.md#0x7_order_book_OrderBook">order_book::OrderBook</a>&lt;M&gt;, order_creator: <b>address</b>, client_order_id: u64): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="single_order_types.md#0x7_single_order_types_Order">single_order_types::Order</a>&lt;M&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_try_cancel_order_with_client_order_id">try_cancel_order_with_client_order_id</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="order_book.md#0x7_order_book_OrderBook">OrderBook</a>&lt;M&gt;, order_creator: <b>address</b>, client_order_id: u64
): Option&lt;Order&lt;M&gt;&gt; {
    self.retail_order_book.<a href="order_book.md#0x7_order_book_try_cancel_order_with_client_order_id">try_cancel_order_with_client_order_id</a>(&<b>mut</b> self.price_time_idx, order_creator, client_order_id)
}
</code></pre>



</details>

<a id="0x7_order_book_client_order_id_exists"></a>

## Function `client_order_id_exists`



<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_client_order_id_exists">client_order_id_exists</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="order_book.md#0x7_order_book_OrderBook">order_book::OrderBook</a>&lt;M&gt;, order_creator: <b>address</b>, client_order_id: u64): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_client_order_id_exists">client_order_id_exists</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="order_book.md#0x7_order_book_OrderBook">OrderBook</a>&lt;M&gt;, order_creator: <b>address</b>, client_order_id: u64
): bool {
    self.retail_order_book.<a href="order_book.md#0x7_order_book_client_order_id_exists">client_order_id_exists</a>(order_creator, client_order_id)
}
</code></pre>



</details>

<a id="0x7_order_book_is_taker_order"></a>

## Function `is_taker_order`

Checks if the order is a taker order i.e., matched immediatedly with the active order book.


<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_is_taker_order">is_taker_order</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="order_book.md#0x7_order_book_OrderBook">order_book::OrderBook</a>&lt;M&gt;, price: u64, is_bid: bool, trigger_condition: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="order_book_types.md#0x7_order_book_types_TriggerCondition">order_book_types::TriggerCondition</a>&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_is_taker_order">is_taker_order</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="order_book.md#0x7_order_book_OrderBook">OrderBook</a>&lt;M&gt;,
    price: u64,
    is_bid: bool,
    trigger_condition: Option&lt;TriggerCondition&gt;
): bool {
    <b>if</b> (trigger_condition.is_some()) {
        <b>return</b> <b>false</b>;
    };
    <b>return</b> self.price_time_idx.<a href="order_book.md#0x7_order_book_is_taker_order">is_taker_order</a>(price, is_bid)
}
</code></pre>



</details>

<a id="0x7_order_book_get_single_match_for_taker"></a>

## Function `get_single_match_for_taker`



<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_get_single_match_for_taker">get_single_match_for_taker</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="order_book.md#0x7_order_book_OrderBook">order_book::OrderBook</a>&lt;M&gt;, price: u64, size: u64, is_bid: bool): <a href="single_order_types.md#0x7_single_order_types_SingleOrderMatch">single_order_types::SingleOrderMatch</a>&lt;M&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_get_single_match_for_taker">get_single_match_for_taker</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="order_book.md#0x7_order_book_OrderBook">OrderBook</a>&lt;M&gt;,
    price: u64,
    size: u64,
    is_bid: bool
): SingleOrderMatch&lt;M&gt; {
    self.retail_order_book.<a href="order_book.md#0x7_order_book_get_single_match_for_taker">get_single_match_for_taker</a>(
        &<b>mut</b> self.price_time_idx, price, size, is_bid
    )
}
</code></pre>



</details>

<a id="0x7_order_book_reinsert_maker_order"></a>

## Function `reinsert_maker_order`



<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_reinsert_maker_order">reinsert_maker_order</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="order_book.md#0x7_order_book_OrderBook">order_book::OrderBook</a>&lt;M&gt;, order_req: <a href="single_order_book.md#0x7_single_order_book_OrderRequest">single_order_book::OrderRequest</a>&lt;M&gt;, original_order: <a href="single_order_types.md#0x7_single_order_types_Order">single_order_types::Order</a>&lt;M&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_reinsert_maker_order">reinsert_maker_order</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="order_book.md#0x7_order_book_OrderBook">OrderBook</a>&lt;M&gt;, order_req: OrderRequest&lt;M&gt;, original_order: Order&lt;M&gt;
) {
    self.retail_order_book.<a href="order_book.md#0x7_order_book_reinsert_maker_order">reinsert_maker_order</a>(
        &<b>mut</b> self.price_time_idx, &<b>mut</b> self.ascending_id_generator, order_req, original_order
    );
}
</code></pre>



</details>

<a id="0x7_order_book_place_maker_order"></a>

## Function `place_maker_order`



<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_place_maker_order">place_maker_order</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="order_book.md#0x7_order_book_OrderBook">order_book::OrderBook</a>&lt;M&gt;, order_req: <a href="single_order_book.md#0x7_single_order_book_OrderRequest">single_order_book::OrderRequest</a>&lt;M&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_place_maker_order">place_maker_order</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="order_book.md#0x7_order_book_OrderBook">OrderBook</a>&lt;M&gt;, order_req: OrderRequest&lt;M&gt;
) {
    self.retail_order_book.<a href="order_book.md#0x7_order_book_place_maker_order">place_maker_order</a>(
        &<b>mut</b> self.price_time_idx,
        &<b>mut</b> self.ascending_id_generator,
        order_req
    );
}
</code></pre>



</details>

<a id="0x7_order_book_decrease_order_size"></a>

## Function `decrease_order_size`



<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_decrease_order_size">decrease_order_size</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="order_book.md#0x7_order_book_OrderBook">order_book::OrderBook</a>&lt;M&gt;, order_creator: <b>address</b>, order_id: <a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>, size_delta: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_decrease_order_size">decrease_order_size</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="order_book.md#0x7_order_book_OrderBook">OrderBook</a>&lt;M&gt;, order_creator: <b>address</b>, order_id: OrderIdType, size_delta: u64
) {
    self.retail_order_book.<a href="order_book.md#0x7_order_book_decrease_order_size">decrease_order_size</a>(&<b>mut</b> self.price_time_idx, order_creator, order_id, size_delta)
}
</code></pre>



</details>

<a id="0x7_order_book_get_order_id_by_client_id"></a>

## Function `get_order_id_by_client_id`



<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_get_order_id_by_client_id">get_order_id_by_client_id</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="order_book.md#0x7_order_book_OrderBook">order_book::OrderBook</a>&lt;M&gt;, order_creator: <b>address</b>, client_order_id: u64): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_get_order_id_by_client_id">get_order_id_by_client_id</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="order_book.md#0x7_order_book_OrderBook">OrderBook</a>&lt;M&gt;, order_creator: <b>address</b>, client_order_id: u64
): Option&lt;OrderIdType&gt; {
    self.retail_order_book.<a href="order_book.md#0x7_order_book_get_order_id_by_client_id">get_order_id_by_client_id</a>(order_creator, client_order_id)
}
</code></pre>



</details>

<a id="0x7_order_book_get_order_metadata"></a>

## Function `get_order_metadata`



<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_get_order_metadata">get_order_metadata</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="order_book.md#0x7_order_book_OrderBook">order_book::OrderBook</a>&lt;M&gt;, order_id: <a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;M&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_get_order_metadata">get_order_metadata</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="order_book.md#0x7_order_book_OrderBook">OrderBook</a>&lt;M&gt;, order_id: OrderIdType
): Option&lt;M&gt; {
    self.retail_order_book.<a href="order_book.md#0x7_order_book_get_order_metadata">get_order_metadata</a>(order_id)
}
</code></pre>



</details>

<a id="0x7_order_book_set_order_metadata"></a>

## Function `set_order_metadata`



<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_set_order_metadata">set_order_metadata</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="order_book.md#0x7_order_book_OrderBook">order_book::OrderBook</a>&lt;M&gt;, order_id: <a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>, metadata: M)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_set_order_metadata">set_order_metadata</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="order_book.md#0x7_order_book_OrderBook">OrderBook</a>&lt;M&gt;, order_id: OrderIdType, metadata: M
) {
    self.retail_order_book.<a href="order_book.md#0x7_order_book_set_order_metadata">set_order_metadata</a>(order_id, metadata)
}
</code></pre>



</details>

<a id="0x7_order_book_is_active_order"></a>

## Function `is_active_order`



<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_is_active_order">is_active_order</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="order_book.md#0x7_order_book_OrderBook">order_book::OrderBook</a>&lt;M&gt;, order_id: <a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_is_active_order">is_active_order</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="order_book.md#0x7_order_book_OrderBook">OrderBook</a>&lt;M&gt;, order_id: OrderIdType
): bool {
    self.retail_order_book.<a href="order_book.md#0x7_order_book_is_active_order">is_active_order</a>(order_id)
}
</code></pre>



</details>

<a id="0x7_order_book_get_order"></a>

## Function `get_order`



<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_get_order">get_order</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="order_book.md#0x7_order_book_OrderBook">order_book::OrderBook</a>&lt;M&gt;, order_id: <a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="single_order_types.md#0x7_single_order_types_OrderWithState">single_order_types::OrderWithState</a>&lt;M&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_get_order">get_order</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="order_book.md#0x7_order_book_OrderBook">OrderBook</a>&lt;M&gt;, order_id: OrderIdType
): Option&lt;aptos_experimental::single_order_types::OrderWithState&lt;M&gt;&gt; {
    self.retail_order_book.<a href="order_book.md#0x7_order_book_get_order">get_order</a>(order_id)
}
</code></pre>



</details>

<a id="0x7_order_book_get_remaining_size"></a>

## Function `get_remaining_size`



<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_get_remaining_size">get_remaining_size</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="order_book.md#0x7_order_book_OrderBook">order_book::OrderBook</a>&lt;M&gt;, order_id: <a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_get_remaining_size">get_remaining_size</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="order_book.md#0x7_order_book_OrderBook">OrderBook</a>&lt;M&gt;, order_id: OrderIdType
): u64 {
    self.retail_order_book.<a href="order_book.md#0x7_order_book_get_remaining_size">get_remaining_size</a>(order_id)
}
</code></pre>



</details>

<a id="0x7_order_book_take_ready_price_based_orders"></a>

## Function `take_ready_price_based_orders`



<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_take_ready_price_based_orders">take_ready_price_based_orders</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="order_book.md#0x7_order_book_OrderBook">order_book::OrderBook</a>&lt;M&gt;, oracle_price: u64, order_limit: u64): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="single_order_types.md#0x7_single_order_types_Order">single_order_types::Order</a>&lt;M&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_take_ready_price_based_orders">take_ready_price_based_orders</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="order_book.md#0x7_order_book_OrderBook">OrderBook</a>&lt;M&gt;, oracle_price: u64, order_limit: u64
): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Order&lt;M&gt;&gt; {
    self.retail_order_book.<a href="order_book.md#0x7_order_book_take_ready_price_based_orders">take_ready_price_based_orders</a>(oracle_price, order_limit)
}
</code></pre>



</details>

<a id="0x7_order_book_take_ready_time_based_orders"></a>

## Function `take_ready_time_based_orders`



<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_take_ready_time_based_orders">take_ready_time_based_orders</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="order_book.md#0x7_order_book_OrderBook">order_book::OrderBook</a>&lt;M&gt;, order_limit: u64): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="single_order_types.md#0x7_single_order_types_Order">single_order_types::Order</a>&lt;M&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_take_ready_time_based_orders">take_ready_time_based_orders</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="order_book.md#0x7_order_book_OrderBook">OrderBook</a>&lt;M&gt;, order_limit: u64
): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Order&lt;M&gt;&gt; {
    self.retail_order_book.<a href="order_book.md#0x7_order_book_take_ready_time_based_orders">take_ready_time_based_orders</a>(order_limit)
}
</code></pre>



</details>

<a id="0x7_order_book_best_bid_price"></a>

## Function `best_bid_price`



<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_best_bid_price">best_bid_price</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="order_book.md#0x7_order_book_OrderBook">order_book::OrderBook</a>&lt;M&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_best_bid_price">best_bid_price</a>&lt;M: store + <b>copy</b> + drop&gt;(self: &<a href="order_book.md#0x7_order_book_OrderBook">OrderBook</a>&lt;M&gt;): Option&lt;u64&gt; {
    self.price_time_idx.<a href="order_book.md#0x7_order_book_best_bid_price">best_bid_price</a>()
}
</code></pre>



</details>

<a id="0x7_order_book_best_ask_price"></a>

## Function `best_ask_price`



<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_best_ask_price">best_ask_price</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="order_book.md#0x7_order_book_OrderBook">order_book::OrderBook</a>&lt;M&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_best_ask_price">best_ask_price</a>&lt;M: store + <b>copy</b> + drop&gt;(self: &<a href="order_book.md#0x7_order_book_OrderBook">OrderBook</a>&lt;M&gt;): Option&lt;u64&gt; {
    self.price_time_idx.<a href="order_book.md#0x7_order_book_best_ask_price">best_ask_price</a>()
}
</code></pre>



</details>

<a id="0x7_order_book_get_slippage_price"></a>

## Function `get_slippage_price`



<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_get_slippage_price">get_slippage_price</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="order_book.md#0x7_order_book_OrderBook">order_book::OrderBook</a>&lt;M&gt;, is_bid: bool, slippage_pct: u64): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_get_slippage_price">get_slippage_price</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="order_book.md#0x7_order_book_OrderBook">OrderBook</a>&lt;M&gt;, is_bid: bool, slippage_pct: u64
): Option&lt;u64&gt; {
    self.price_time_idx.<a href="order_book.md#0x7_order_book_get_slippage_price">get_slippage_price</a>(is_bid, slippage_pct)
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
