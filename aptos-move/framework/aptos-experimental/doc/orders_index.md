
<a id="0x7_orders_index"></a>

# Module `0x7::orders_index`



-  [Enum `IndexedOrder`](#0x7_orders_index_IndexedOrder)
-  [Enum `OrdersIndex`](#0x7_orders_index_OrdersIndex)
-  [Function `new_orders_index`](#0x7_orders_index_new_orders_index)
-  [Function `place_maker_order`](#0x7_orders_index_place_maker_order)
-  [Function `is_taker_order`](#0x7_orders_index_is_taker_order)
-  [Function `get_single_match_for_taker`](#0x7_orders_index_get_single_match_for_taker)


<pre><code><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table">0x1::table</a>;
<b>use</b> <a href="../../aptos-framework/doc/transaction_context.md#0x1_transaction_context">0x1::transaction_context</a>;
<b>use</b> <a href="bulk_order_book_types.md#0x7_bulk_order_book_types">0x7::bulk_order_book_types</a>;
<b>use</b> <a href="order_book_types.md#0x7_order_book_types">0x7::order_book_types</a>;
<b>use</b> <a href="single_order_book.md#0x7_single_order_book">0x7::single_order_book</a>;
<b>use</b> <a href="single_order_types.md#0x7_single_order_types">0x7::single_order_types</a>;
</code></pre>



<a id="0x7_orders_index_IndexedOrder"></a>

## Enum `IndexedOrder`



<pre><code>enum <a href="orders_index.md#0x7_orders_index_IndexedOrder">IndexedOrder</a>&lt;M: <b>copy</b>, drop, store&gt;
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>SingleOrder</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>unique_priority_idx: <a href="order_book_types.md#0x7_order_book_types_UniqueIdxType">order_book_types::UniqueIdxType</a></code>
</dt>
<dd>

</dd>
<dt>
<code>order_request: <a href="single_order_book.md#0x7_single_order_book_SingleOrderRequest">single_order_book::SingleOrderRequest</a>&lt;M&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

<details>
<summary>BulkOrder</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>order_id: <a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a></code>
</dt>
<dd>

</dd>
<dt>
<code>unique_priority_idx: <a href="order_book_types.md#0x7_order_book_types_UniqueIdxType">order_book_types::UniqueIdxType</a></code>
</dt>
<dd>

</dd>
<dt>
<code>order_request: <a href="bulk_order_book_types.md#0x7_bulk_order_book_types_BulkOrderRequest">bulk_order_book_types::BulkOrderRequest</a>&lt;M&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

</details>

<a id="0x7_orders_index_OrdersIndex"></a>

## Enum `OrdersIndex`



<pre><code>enum <a href="orders_index.md#0x7_orders_index_OrdersIndex">OrdersIndex</a>&lt;M: <b>copy</b>, drop, store&gt; <b>has</b> store
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>V1</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>id: u128</code>
</dt>
<dd>

</dd>
<dt>
<code><a href="../../aptos-framework/doc/version.md#0x1_version">version</a>: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>data: <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_Table">table::Table</a>&lt;<a href="order_book_types.md#0x7_order_book_types_OrderIdType">order_book_types::OrderIdType</a>, <a href="single_order_types.md#0x7_single_order_types_SingleOrder">single_order_types::SingleOrder</a>&lt;M&gt;&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

</details>

<a id="0x7_orders_index_new_orders_index"></a>

## Function `new_orders_index`



<pre><code><b>public</b> <b>fun</b> <a href="orders_index.md#0x7_orders_index_new_orders_index">new_orders_index</a>&lt;M: <b>copy</b>, drop, store&gt;(): <a href="orders_index.md#0x7_orders_index_OrdersIndex">orders_index::OrdersIndex</a>&lt;M&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="orders_index.md#0x7_orders_index_new_orders_index">new_orders_index</a>&lt;M: store + <b>copy</b> + drop&gt;(): <a href="orders_index.md#0x7_orders_index_OrdersIndex">OrdersIndex</a>&lt;M&gt; {
    OrdersIndex::V1 {
        id: <a href="../../aptos-framework/doc/transaction_context.md#0x1_transaction_context_monotonically_increasing_counter">transaction_context::monotonically_increasing_counter</a>(),
        <a href="../../aptos-framework/doc/version.md#0x1_version">version</a>: 0,
        data: <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_new">table::new</a>(),
    }
}
</code></pre>



</details>

<a id="0x7_orders_index_place_maker_order"></a>

## Function `place_maker_order`



<pre><code><b>public</b> <b>fun</b> <a href="orders_index.md#0x7_orders_index_place_maker_order">place_maker_order</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="orders_index.md#0x7_orders_index_OrdersIndex">orders_index::OrdersIndex</a>&lt;M&gt;, order_req: <a href="single_order_book.md#0x7_single_order_book_SingleOrderRequest">single_order_book::SingleOrderRequest</a>&lt;M&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>native</b> <b>fun</b> <a href="orders_index.md#0x7_orders_index_place_maker_order">place_maker_order</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="orders_index.md#0x7_orders_index_OrdersIndex">OrdersIndex</a>&lt;M&gt;, order_req: SingleOrderRequest&lt;M&gt;
);
</code></pre>



</details>

<a id="0x7_orders_index_is_taker_order"></a>

## Function `is_taker_order`

Checks if the order is a taker order i.e., matched immediately with the active order book.


<pre><code><b>public</b> <b>fun</b> <a href="orders_index.md#0x7_orders_index_is_taker_order">is_taker_order</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="orders_index.md#0x7_orders_index_OrdersIndex">orders_index::OrdersIndex</a>&lt;M&gt;, price: u64, is_bid: bool, trigger_condition: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="order_book_types.md#0x7_order_book_types_TriggerCondition">order_book_types::TriggerCondition</a>&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>native</b> <b>fun</b> <a href="orders_index.md#0x7_orders_index_is_taker_order">is_taker_order</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="orders_index.md#0x7_orders_index_OrdersIndex">OrdersIndex</a>&lt;M&gt;,
    price: u64,
    is_bid: bool,
    trigger_condition: Option&lt;TriggerCondition&gt;
): bool;
</code></pre>



</details>

<a id="0x7_orders_index_get_single_match_for_taker"></a>

## Function `get_single_match_for_taker`



<pre><code><b>public</b> <b>fun</b> <a href="orders_index.md#0x7_orders_index_get_single_match_for_taker">get_single_match_for_taker</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="orders_index.md#0x7_orders_index_OrdersIndex">orders_index::OrdersIndex</a>&lt;M&gt;, price: u64, size: u64, is_bid: bool): <a href="order_book_types.md#0x7_order_book_types_OrderMatch">order_book_types::OrderMatch</a>&lt;M&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>native</b> <b>fun</b> <a href="orders_index.md#0x7_orders_index_get_single_match_for_taker">get_single_match_for_taker</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="orders_index.md#0x7_orders_index_OrdersIndex">OrdersIndex</a>&lt;M&gt;,
    price: u64,
    size: u64,
    is_bid: bool
): OrderMatch&lt;M&gt;;
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
