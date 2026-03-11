
<a id="0x7_order_book"></a>

# Module `0x7::order_book`



-  [Resource `OrderBookVersion`](#0x7_order_book_OrderBookVersion)
-  [Enum `OrderBook`](#0x7_order_book_OrderBook)
-  [Constants](#@Constants_0)
-  [Function `new_order_book`](#0x7_order_book_new_order_book)
-  [Function `new_native_order_book`](#0x7_order_book_new_native_order_book)
-  [Function `migrate_to_native`](#0x7_order_book_migrate_to_native)
-  [Function `ensure_native_index_ready`](#0x7_order_book_ensure_native_index_ready)
-  [Function `maybe_flush_handle`](#0x7_order_book_maybe_flush_handle)
-  [Function `get_native_market_addr`](#0x7_order_book_get_native_market_addr)
-  [Function `rebuild_native_index`](#0x7_order_book_rebuild_native_index)
-  [Function `client_order_id_exists`](#0x7_order_book_client_order_id_exists)
-  [Function `get_single_order_metadata`](#0x7_order_book_get_single_order_metadata)
-  [Function `get_order_id_by_client_id`](#0x7_order_book_get_order_id_by_client_id)
-  [Function `get_single_order`](#0x7_order_book_get_single_order)
-  [Function `get_single_remaining_size`](#0x7_order_book_get_single_remaining_size)
-  [Function `cancel_single_order`](#0x7_order_book_cancel_single_order)
-  [Function `try_cancel_single_order`](#0x7_order_book_try_cancel_single_order)
-  [Function `try_cancel_single_order_with_client_order_id`](#0x7_order_book_try_cancel_single_order_with_client_order_id)
-  [Function `place_maker_order`](#0x7_order_book_place_maker_order)
-  [Function `decrease_single_order_size`](#0x7_order_book_decrease_single_order_size)
-  [Function `set_single_order_metadata`](#0x7_order_book_set_single_order_metadata)
-  [Function `take_ready_price_based_orders`](#0x7_order_book_take_ready_price_based_orders)
-  [Function `take_ready_time_based_orders`](#0x7_order_book_take_ready_time_based_orders)
-  [Function `best_bid_price`](#0x7_order_book_best_bid_price)
-  [Function `best_ask_price`](#0x7_order_book_best_ask_price)
-  [Function `get_slippage_price`](#0x7_order_book_get_slippage_price)
-  [Function `is_taker_order`](#0x7_order_book_is_taker_order)
-  [Function `get_single_match_for_taker`](#0x7_order_book_get_single_match_for_taker)
-  [Function `reinsert_order`](#0x7_order_book_reinsert_order)
-  [Function `get_bulk_order_remaining_size`](#0x7_order_book_get_bulk_order_remaining_size)
-  [Function `place_bulk_order`](#0x7_order_book_place_bulk_order)
-  [Function `get_bulk_order`](#0x7_order_book_get_bulk_order)
-  [Function `cancel_bulk_order`](#0x7_order_book_cancel_bulk_order)
-  [Function `cancel_bulk_order_at_price`](#0x7_order_book_cancel_bulk_order_at_price)
-  [Function `native_is_acquired`](#0x7_order_book_native_is_acquired)
-  [Function `native_ensure_acquired`](#0x7_order_book_native_ensure_acquired)
-  [Function `native_flush`](#0x7_order_book_native_flush)
-  [Function `native_rebuild_complete`](#0x7_order_book_native_rebuild_complete)


<pre><code><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features">0x1::features</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/mem.md#0x1_mem">0x1::mem</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;
<b>use</b> <a href="">0x5::bulk_order_types</a>;
<b>use</b> <a href="">0x5::order_book_types</a>;
<b>use</b> <a href="">0x5::order_match_types</a>;
<b>use</b> <a href="">0x5::single_order_types</a>;
<b>use</b> <a href="bulk_order_book.md#0x7_bulk_order_book">0x7::bulk_order_book</a>;
<b>use</b> <a href="price_time_index.md#0x7_price_time_index">0x7::price_time_index</a>;
<b>use</b> <a href="single_order_book.md#0x7_single_order_book">0x7::single_order_book</a>;
</code></pre>



<a id="0x7_order_book_OrderBookVersion"></a>

## Resource `OrderBookVersion`

Version handle for the native PriceTimeIndex overlay.
Written to MVHashMap at flush, creating Block-STM dependency.


<pre><code><b>struct</b> <a href="order_book.md#0x7_order_book_OrderBookVersion">OrderBookVersion</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>handle: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

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
<code><a href="single_order_book.md#0x7_single_order_book">single_order_book</a>: <a href="single_order_book.md#0x7_single_order_book_SingleOrderBook">single_order_book::SingleOrderBook</a>&lt;M&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code><a href="bulk_order_book.md#0x7_bulk_order_book">bulk_order_book</a>: <a href="bulk_order_book.md#0x7_bulk_order_book_BulkOrderBook">bulk_order_book::BulkOrderBook</a>&lt;M&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>price_time_idx: <a href="price_time_index.md#0x7_price_time_index_PriceTimeIndex">price_time_index::PriceTimeIndex</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x7_order_book_E_REINSERT_ORDER_MISMATCH"></a>



<pre><code><b>const</b> <a href="order_book.md#0x7_order_book_E_REINSERT_ORDER_MISMATCH">E_REINSERT_ORDER_MISMATCH</a>: u64 = 8;
</code></pre>



<a id="0x7_order_book_E_ALREADY_NATIVE"></a>



<pre><code><b>const</b> <a href="order_book.md#0x7_order_book_E_ALREADY_NATIVE">E_ALREADY_NATIVE</a>: u64 = 21;
</code></pre>



<a id="0x7_order_book_E_NATIVE_ORDER_BOOK_NOT_ENABLED"></a>



<pre><code><b>const</b> <a href="order_book.md#0x7_order_book_E_NATIVE_ORDER_BOOK_NOT_ENABLED">E_NATIVE_ORDER_BOOK_NOT_ENABLED</a>: u64 = 20;
</code></pre>



<a id="0x7_order_book_E_ORDER_BOOK_VERSION_EXISTS"></a>



<pre><code><b>const</b> <a href="order_book.md#0x7_order_book_E_ORDER_BOOK_VERSION_EXISTS">E_ORDER_BOOK_VERSION_EXISTS</a>: u64 = 22;
</code></pre>



<a id="0x7_order_book_new_order_book"></a>

## Function `new_order_book`



<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_new_order_book">new_order_book</a>&lt;M: <b>copy</b>, drop, store&gt;(): <a href="order_book.md#0x7_order_book_OrderBook">order_book::OrderBook</a>&lt;M&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_new_order_book">new_order_book</a>&lt;M: store + <b>copy</b> + drop&gt;(): <a href="order_book.md#0x7_order_book_OrderBook">OrderBook</a>&lt;M&gt; {
    OrderBook::UnifiedV1 {
        <a href="single_order_book.md#0x7_single_order_book">single_order_book</a>: new_single_order_book(),
        <a href="bulk_order_book.md#0x7_bulk_order_book">bulk_order_book</a>: new_bulk_order_book(),
        price_time_idx: new_price_time_idx()
    }
}
</code></pre>



</details>

<a id="0x7_order_book_new_native_order_book"></a>

## Function `new_native_order_book`

Creates a new order book with native Rust-backed PriceTimeIndex.
The native index lives in validator memory as BTreeMap overlays.
<code>market_addr</code> identifies this market in the native layer.
Requires the NATIVE_ORDER_BOOK feature flag to be enabled.

Callers must bracket operations with:
ensure_native_index_ready() ... maybe_flush_handle()


<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_new_native_order_book">new_native_order_book</a>&lt;M: <b>copy</b>, drop, store&gt;(market_signer: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, market_addr: <b>address</b>): <a href="order_book.md#0x7_order_book_OrderBook">order_book::OrderBook</a>&lt;M&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_new_native_order_book">new_native_order_book</a>&lt;M: store + <b>copy</b> + drop&gt;(
    market_signer: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    market_addr: <b>address</b>
): <a href="order_book.md#0x7_order_book_OrderBook">OrderBook</a>&lt;M&gt; {
    <b>assert</b>!(
        std::features::is_native_order_book_enabled(),
        <a href="order_book.md#0x7_order_book_E_NATIVE_ORDER_BOOK_NOT_ENABLED">E_NATIVE_ORDER_BOOK_NOT_ENABLED</a>
    );
    <b>assert</b>!(
        !<b>exists</b>&lt;<a href="order_book.md#0x7_order_book_OrderBookVersion">OrderBookVersion</a>&gt;(market_addr),
        <a href="order_book.md#0x7_order_book_E_ORDER_BOOK_VERSION_EXISTS">E_ORDER_BOOK_VERSION_EXISTS</a>
    );
    // Create the <a href="order_book.md#0x7_order_book_OrderBookVersion">OrderBookVersion</a> resource for Block-STM handle tracking
    <b>move_to</b>(market_signer, <a href="order_book.md#0x7_order_book_OrderBookVersion">OrderBookVersion</a> { handle: 0 });
    OrderBook::UnifiedV1 {
        <a href="single_order_book.md#0x7_single_order_book">single_order_book</a>: new_single_order_book(),
        <a href="bulk_order_book.md#0x7_bulk_order_book">bulk_order_book</a>: new_bulk_order_book(),
        price_time_idx: new_native_price_time_idx(market_addr)
    }
}
</code></pre>



</details>

<a id="0x7_order_book_migrate_to_native"></a>

## Function `migrate_to_native`

Migrate an existing V1 order book to NativeV2.
The V1 PriceTimeIndex BigOrderedMaps are destroyed (data is redundant with
SingleOrderBook + BulkOrderBook orders). The native index will be rebuilt
from those orders on first access via cold start.
Requires the NATIVE_ORDER_BOOK feature flag to be enabled.


<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_migrate_to_native">migrate_to_native</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="order_book.md#0x7_order_book_OrderBook">order_book::OrderBook</a>&lt;M&gt;, market_signer: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, market_addr: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_migrate_to_native">migrate_to_native</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="order_book.md#0x7_order_book_OrderBook">OrderBook</a>&lt;M&gt;,
    market_signer: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    market_addr: <b>address</b>
) {
    <b>assert</b>!(
        std::features::is_native_order_book_enabled(),
        <a href="order_book.md#0x7_order_book_E_NATIVE_ORDER_BOOK_NOT_ENABLED">E_NATIVE_ORDER_BOOK_NOT_ENABLED</a>
    );
    // Ensure not already <b>native</b>
    <b>assert</b>!(
        self.price_time_idx.<a href="order_book.md#0x7_order_book_get_native_market_addr">get_native_market_addr</a>().is_none(),
        <a href="order_book.md#0x7_order_book_E_ALREADY_NATIVE">E_ALREADY_NATIVE</a>
    );
    <b>assert</b>!(
        !<b>exists</b>&lt;<a href="order_book.md#0x7_order_book_OrderBookVersion">OrderBookVersion</a>&gt;(market_addr),
        <a href="order_book.md#0x7_order_book_E_ORDER_BOOK_VERSION_EXISTS">E_ORDER_BOOK_VERSION_EXISTS</a>
    );

    // Swap the PriceTimeIndex: destroy V1 BigOrderedMaps, replace <b>with</b> NativeV2
    <b>let</b> old_idx = std::mem::replace(
        &<b>mut</b> self.price_time_idx,
        new_native_price_time_idx(market_addr)
    );
    old_idx.destroy_v1_for_migration();

    // Create the <a href="order_book.md#0x7_order_book_OrderBookVersion">OrderBookVersion</a> resource
    <b>move_to</b>(market_signer, <a href="order_book.md#0x7_order_book_OrderBookVersion">OrderBookVersion</a> { handle: 0 });
}
</code></pre>



</details>

<a id="0x7_order_book_ensure_native_index_ready"></a>

## Function `ensure_native_index_ready`

Acquire the native overlay for this market. Must be called before any
OrderBook operation when using NativeV2. No-op for V1.

If cold start (validator restart), triggers a rebuild from on-chain orders.


<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_ensure_native_index_ready">ensure_native_index_ready</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="order_book.md#0x7_order_book_OrderBook">order_book::OrderBook</a>&lt;M&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_ensure_native_index_ready">ensure_native_index_ready</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="order_book.md#0x7_order_book_OrderBook">OrderBook</a>&lt;M&gt;
) <b>acquires</b> <a href="order_book.md#0x7_order_book_OrderBookVersion">OrderBookVersion</a> {
    <b>let</b> addr_opt = self.price_time_idx.<a href="order_book.md#0x7_order_book_get_native_market_addr">get_native_market_addr</a>();
    <b>if</b> (addr_opt.is_some()) {
        <b>let</b> market_addr = addr_opt.destroy_some();
        // Fast path: skip <b>borrow_global</b> <b>if</b> overlay already acquired in this TX.
        <b>if</b> (<a href="order_book.md#0x7_order_book_native_is_acquired">native_is_acquired</a>(market_addr)) {
            <b>return</b>;
        };
        <b>let</b> handle = <b>borrow_global</b>&lt;<a href="order_book.md#0x7_order_book_OrderBookVersion">OrderBookVersion</a>&gt;(market_addr).handle;
        <b>let</b> needs_rebuild = <a href="order_book.md#0x7_order_book_native_ensure_acquired">native_ensure_acquired</a>(market_addr, handle);
        <b>if</b> (needs_rebuild) {
            <a href="order_book.md#0x7_order_book_rebuild_native_index">rebuild_native_index</a>(self, market_addr);
            <a href="order_book.md#0x7_order_book_native_rebuild_complete">native_rebuild_complete</a>(market_addr);
        };
    };
}
</code></pre>



</details>

<a id="0x7_order_book_maybe_flush_handle"></a>

## Function `maybe_flush_handle`

Flush the native overlay if modified. Bumps the handle in OrderBookVersion,
creating the MVHashMap WRITE for Block-STM conflict detection.
Called once per market at the end of each entry point.


<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_maybe_flush_handle">maybe_flush_handle</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="order_book.md#0x7_order_book_OrderBook">order_book::OrderBook</a>&lt;M&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_maybe_flush_handle">maybe_flush_handle</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="order_book.md#0x7_order_book_OrderBook">OrderBook</a>&lt;M&gt;
) <b>acquires</b> <a href="order_book.md#0x7_order_book_OrderBookVersion">OrderBookVersion</a> {
    <b>let</b> addr_opt = self.<a href="order_book.md#0x7_order_book_get_native_market_addr">get_native_market_addr</a>();
    <b>if</b> (addr_opt.is_some()) {
        <b>let</b> market_addr = addr_opt.destroy_some();
        <b>let</b> ver = <b>borrow_global_mut</b>&lt;<a href="order_book.md#0x7_order_book_OrderBookVersion">OrderBookVersion</a>&gt;(market_addr);
        <b>let</b> new_handle = ver.handle + 1;
        <b>let</b> modified = <a href="order_book.md#0x7_order_book_native_flush">native_flush</a>(market_addr, new_handle);
        <b>if</b> (modified) {
            ver.handle = new_handle;
        };
    };
}
</code></pre>



</details>

<a id="0x7_order_book_get_native_market_addr"></a>

## Function `get_native_market_addr`

Returns the market address if this is a NativeV2 order book, None otherwise.


<pre><code><b>fun</b> <a href="order_book.md#0x7_order_book_get_native_market_addr">get_native_market_addr</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="order_book.md#0x7_order_book_OrderBook">order_book::OrderBook</a>&lt;M&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<b>address</b>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="order_book.md#0x7_order_book_get_native_market_addr">get_native_market_addr</a>&lt;M: store + <b>copy</b> + drop&gt;(self: &<a href="order_book.md#0x7_order_book_OrderBook">OrderBook</a>&lt;M&gt;): Option&lt;<b>address</b>&gt; {
    self.price_time_idx.<a href="order_book.md#0x7_order_book_get_native_market_addr">get_native_market_addr</a>()
}
</code></pre>



</details>

<a id="0x7_order_book_rebuild_native_index"></a>

## Function `rebuild_native_index`

Rebuild the native index from all active orders (cold start).


<pre><code><b>fun</b> <a href="order_book.md#0x7_order_book_rebuild_native_index">rebuild_native_index</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="order_book.md#0x7_order_book_OrderBook">order_book::OrderBook</a>&lt;M&gt;, market_addr: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="order_book.md#0x7_order_book_rebuild_native_index">rebuild_native_index</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="order_book.md#0x7_order_book_OrderBook">OrderBook</a>&lt;M&gt;, market_addr: <b>address</b>
) {
    self.<a href="single_order_book.md#0x7_single_order_book">single_order_book</a>.<a href="order_book.md#0x7_order_book_rebuild_native_index">rebuild_native_index</a>(market_addr);
    self.<a href="bulk_order_book.md#0x7_bulk_order_book">bulk_order_book</a>.<a href="order_book.md#0x7_order_book_rebuild_native_index">rebuild_native_index</a>(market_addr);
}
</code></pre>



</details>

<a id="0x7_order_book_client_order_id_exists"></a>

## Function `client_order_id_exists`



<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_client_order_id_exists">client_order_id_exists</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="order_book.md#0x7_order_book_OrderBook">order_book::OrderBook</a>&lt;M&gt;, order_creator: <b>address</b>, client_order_id: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_client_order_id_exists">client_order_id_exists</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="order_book.md#0x7_order_book_OrderBook">OrderBook</a>&lt;M&gt;, order_creator: <b>address</b>, client_order_id: String
): bool {
    <b>let</b> _ob_t = native_timing_start();
    self.<a href="order_book.md#0x7_order_book_ensure_native_index_ready">ensure_native_index_ready</a>();
    <b>let</b> _r = self.<a href="single_order_book.md#0x7_single_order_book">single_order_book</a>.<a href="order_book.md#0x7_order_book_client_order_id_exists">client_order_id_exists</a>(order_creator, client_order_id);
    native_timing_end(24, &<b>mut</b> _ob_t);
    _r
}
</code></pre>



</details>

<a id="0x7_order_book_get_single_order_metadata"></a>

## Function `get_single_order_metadata`



<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_get_single_order_metadata">get_single_order_metadata</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="order_book.md#0x7_order_book_OrderBook">order_book::OrderBook</a>&lt;M&gt;, order_id: <a href="_OrderId">order_book_types::OrderId</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;M&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_get_single_order_metadata">get_single_order_metadata</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="order_book.md#0x7_order_book_OrderBook">OrderBook</a>&lt;M&gt;, order_id: OrderId
): Option&lt;M&gt; {
    <b>let</b> _ob_t = native_timing_start();
    self.<a href="order_book.md#0x7_order_book_ensure_native_index_ready">ensure_native_index_ready</a>();
    <b>let</b> _r = self.<a href="single_order_book.md#0x7_single_order_book">single_order_book</a>.get_order_metadata(order_id);
    native_timing_end(25, &<b>mut</b> _ob_t);
    _r
}
</code></pre>



</details>

<a id="0x7_order_book_get_order_id_by_client_id"></a>

## Function `get_order_id_by_client_id`



<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_get_order_id_by_client_id">get_order_id_by_client_id</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="order_book.md#0x7_order_book_OrderBook">order_book::OrderBook</a>&lt;M&gt;, order_creator: <b>address</b>, client_order_id: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="_OrderId">order_book_types::OrderId</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_get_order_id_by_client_id">get_order_id_by_client_id</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="order_book.md#0x7_order_book_OrderBook">OrderBook</a>&lt;M&gt;, order_creator: <b>address</b>, client_order_id: String
): Option&lt;OrderId&gt; {
    <b>let</b> _ob_t = native_timing_start();
    self.<a href="order_book.md#0x7_order_book_ensure_native_index_ready">ensure_native_index_ready</a>();
    <b>let</b> _r = self.<a href="single_order_book.md#0x7_single_order_book">single_order_book</a>.<a href="order_book.md#0x7_order_book_get_order_id_by_client_id">get_order_id_by_client_id</a>(order_creator, client_order_id);
    native_timing_end(27, &<b>mut</b> _ob_t);
    _r
}
</code></pre>



</details>

<a id="0x7_order_book_get_single_order"></a>

## Function `get_single_order`



<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_get_single_order">get_single_order</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="order_book.md#0x7_order_book_OrderBook">order_book::OrderBook</a>&lt;M&gt;, order_id: <a href="_OrderId">order_book_types::OrderId</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="_OrderWithState">single_order_types::OrderWithState</a>&lt;M&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_get_single_order">get_single_order</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="order_book.md#0x7_order_book_OrderBook">OrderBook</a>&lt;M&gt;, order_id: OrderId
): Option&lt;aptos_trading::single_order_types::OrderWithState&lt;M&gt;&gt; {
    <b>let</b> _ob_t = native_timing_start();
    self.<a href="order_book.md#0x7_order_book_ensure_native_index_ready">ensure_native_index_ready</a>();
    <b>let</b> _r = self.<a href="single_order_book.md#0x7_single_order_book">single_order_book</a>.get_order(order_id);
    native_timing_end(23, &<b>mut</b> _ob_t);
    _r
}
</code></pre>



</details>

<a id="0x7_order_book_get_single_remaining_size"></a>

## Function `get_single_remaining_size`



<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_get_single_remaining_size">get_single_remaining_size</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="order_book.md#0x7_order_book_OrderBook">order_book::OrderBook</a>&lt;M&gt;, order_id: <a href="_OrderId">order_book_types::OrderId</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_get_single_remaining_size">get_single_remaining_size</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="order_book.md#0x7_order_book_OrderBook">OrderBook</a>&lt;M&gt;, order_id: OrderId
): u64 {
    self.<a href="order_book.md#0x7_order_book_ensure_native_index_ready">ensure_native_index_ready</a>();
    self.<a href="single_order_book.md#0x7_single_order_book">single_order_book</a>.get_remaining_size(order_id)
}
</code></pre>



</details>

<a id="0x7_order_book_cancel_single_order"></a>

## Function `cancel_single_order`



<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_cancel_single_order">cancel_single_order</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="order_book.md#0x7_order_book_OrderBook">order_book::OrderBook</a>&lt;M&gt;, order_creator: <b>address</b>, order_id: <a href="_OrderId">order_book_types::OrderId</a>): <a href="_SingleOrder">single_order_types::SingleOrder</a>&lt;M&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_cancel_single_order">cancel_single_order</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="order_book.md#0x7_order_book_OrderBook">OrderBook</a>&lt;M&gt;, order_creator: <b>address</b>, order_id: OrderId
): SingleOrder&lt;M&gt; {
    self.<a href="order_book.md#0x7_order_book_ensure_native_index_ready">ensure_native_index_ready</a>();
    <b>let</b> _t = native_timing_start();
    <b>let</b> result = self.<a href="single_order_book.md#0x7_single_order_book">single_order_book</a>.cancel_order(
        &<b>mut</b> self.price_time_idx, order_creator, order_id
    );
    native_timing_end(11, &<b>mut</b> _t);
    result
}
</code></pre>



</details>

<a id="0x7_order_book_try_cancel_single_order"></a>

## Function `try_cancel_single_order`



<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_try_cancel_single_order">try_cancel_single_order</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="order_book.md#0x7_order_book_OrderBook">order_book::OrderBook</a>&lt;M&gt;, order_creator: <b>address</b>, order_id: <a href="_OrderId">order_book_types::OrderId</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="_SingleOrder">single_order_types::SingleOrder</a>&lt;M&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_try_cancel_single_order">try_cancel_single_order</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="order_book.md#0x7_order_book_OrderBook">OrderBook</a>&lt;M&gt;, order_creator: <b>address</b>, order_id: OrderId
): Option&lt;SingleOrder&lt;M&gt;&gt; {
    <b>let</b> _ob_t = native_timing_start();
    self.<a href="order_book.md#0x7_order_book_ensure_native_index_ready">ensure_native_index_ready</a>();
    self.<a href="single_order_book.md#0x7_single_order_book">single_order_book</a>.try_cancel_order(
        &<b>mut</b> self.price_time_idx, order_creator, order_id
    )
}
</code></pre>



</details>

<a id="0x7_order_book_try_cancel_single_order_with_client_order_id"></a>

## Function `try_cancel_single_order_with_client_order_id`



<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_try_cancel_single_order_with_client_order_id">try_cancel_single_order_with_client_order_id</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="order_book.md#0x7_order_book_OrderBook">order_book::OrderBook</a>&lt;M&gt;, order_creator: <b>address</b>, client_order_id: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="_SingleOrder">single_order_types::SingleOrder</a>&lt;M&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_try_cancel_single_order_with_client_order_id">try_cancel_single_order_with_client_order_id</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="order_book.md#0x7_order_book_OrderBook">OrderBook</a>&lt;M&gt;, order_creator: <b>address</b>, client_order_id: String
): Option&lt;SingleOrder&lt;M&gt;&gt; {
    <b>let</b> _ob_t = native_timing_start();
    self.<a href="order_book.md#0x7_order_book_ensure_native_index_ready">ensure_native_index_ready</a>();
    self.<a href="single_order_book.md#0x7_single_order_book">single_order_book</a>.try_cancel_order_with_client_order_id(
        &<b>mut</b> self.price_time_idx, order_creator, client_order_id
    )
}
</code></pre>



</details>

<a id="0x7_order_book_place_maker_order"></a>

## Function `place_maker_order`



<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_place_maker_order">place_maker_order</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="order_book.md#0x7_order_book_OrderBook">order_book::OrderBook</a>&lt;M&gt;, order_req: <a href="_SingleOrderRequest">single_order_types::SingleOrderRequest</a>&lt;M&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_place_maker_order">place_maker_order</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="order_book.md#0x7_order_book_OrderBook">OrderBook</a>&lt;M&gt;, order_req: SingleOrderRequest&lt;M&gt;
) {
    self.<a href="order_book.md#0x7_order_book_ensure_native_index_ready">ensure_native_index_ready</a>();
    <b>let</b> _t = native_timing_start();
    self.<a href="single_order_book.md#0x7_single_order_book">single_order_book</a>.place_maker_or_pending_order(
        &<b>mut</b> self.price_time_idx, order_req
    );
    native_timing_end(10, &<b>mut</b> _t);
}
</code></pre>



</details>

<a id="0x7_order_book_decrease_single_order_size"></a>

## Function `decrease_single_order_size`



<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_decrease_single_order_size">decrease_single_order_size</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="order_book.md#0x7_order_book_OrderBook">order_book::OrderBook</a>&lt;M&gt;, order_creator: <b>address</b>, order_id: <a href="_OrderId">order_book_types::OrderId</a>, size_delta: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_decrease_single_order_size">decrease_single_order_size</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="order_book.md#0x7_order_book_OrderBook">OrderBook</a>&lt;M&gt;,
    order_creator: <b>address</b>,
    order_id: OrderId,
    size_delta: u64
) {
    <b>let</b> _ob_t = native_timing_start();
    self.<a href="order_book.md#0x7_order_book_ensure_native_index_ready">ensure_native_index_ready</a>();
    self.<a href="single_order_book.md#0x7_single_order_book">single_order_book</a>.decrease_order_size(
        &<b>mut</b> self.price_time_idx,
        order_creator,
        order_id,
        size_delta
    )
}
</code></pre>



</details>

<a id="0x7_order_book_set_single_order_metadata"></a>

## Function `set_single_order_metadata`



<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_set_single_order_metadata">set_single_order_metadata</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="order_book.md#0x7_order_book_OrderBook">order_book::OrderBook</a>&lt;M&gt;, order_id: <a href="_OrderId">order_book_types::OrderId</a>, metadata: M)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_set_single_order_metadata">set_single_order_metadata</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="order_book.md#0x7_order_book_OrderBook">OrderBook</a>&lt;M&gt;, order_id: OrderId, metadata: M
) {
    <b>let</b> _ob_t = native_timing_start();
    self.<a href="order_book.md#0x7_order_book_ensure_native_index_ready">ensure_native_index_ready</a>();
    self.<a href="single_order_book.md#0x7_single_order_book">single_order_book</a>.set_order_metadata(order_id, metadata)
}
</code></pre>



</details>

<a id="0x7_order_book_take_ready_price_based_orders"></a>

## Function `take_ready_price_based_orders`



<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_take_ready_price_based_orders">take_ready_price_based_orders</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="order_book.md#0x7_order_book_OrderBook">order_book::OrderBook</a>&lt;M&gt;, oracle_price: u64, order_limit: u64): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="_SingleOrder">single_order_types::SingleOrder</a>&lt;M&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_take_ready_price_based_orders">take_ready_price_based_orders</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="order_book.md#0x7_order_book_OrderBook">OrderBook</a>&lt;M&gt;, oracle_price: u64, order_limit: u64
): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;SingleOrder&lt;M&gt;&gt; {
    <b>let</b> _ob_t = native_timing_start();
    self.<a href="order_book.md#0x7_order_book_ensure_native_index_ready">ensure_native_index_ready</a>();
    self.<a href="single_order_book.md#0x7_single_order_book">single_order_book</a>.<a href="order_book.md#0x7_order_book_take_ready_price_based_orders">take_ready_price_based_orders</a>(oracle_price, order_limit)
}
</code></pre>



</details>

<a id="0x7_order_book_take_ready_time_based_orders"></a>

## Function `take_ready_time_based_orders`



<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_take_ready_time_based_orders">take_ready_time_based_orders</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="order_book.md#0x7_order_book_OrderBook">order_book::OrderBook</a>&lt;M&gt;, order_limit: u64): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="_SingleOrder">single_order_types::SingleOrder</a>&lt;M&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_take_ready_time_based_orders">take_ready_time_based_orders</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="order_book.md#0x7_order_book_OrderBook">OrderBook</a>&lt;M&gt;, order_limit: u64
): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;SingleOrder&lt;M&gt;&gt; {
    <b>let</b> _ob_t = native_timing_start();
    self.<a href="order_book.md#0x7_order_book_ensure_native_index_ready">ensure_native_index_ready</a>();
    self.<a href="single_order_book.md#0x7_single_order_book">single_order_book</a>.<a href="order_book.md#0x7_order_book_take_ready_time_based_orders">take_ready_time_based_orders</a>(order_limit)
}
</code></pre>



</details>

<a id="0x7_order_book_best_bid_price"></a>

## Function `best_bid_price`



<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_best_bid_price">best_bid_price</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="order_book.md#0x7_order_book_OrderBook">order_book::OrderBook</a>&lt;M&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_best_bid_price">best_bid_price</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="order_book.md#0x7_order_book_OrderBook">OrderBook</a>&lt;M&gt;
): Option&lt;u64&gt; {
    <b>let</b> _ob_t = native_timing_start();
    self.<a href="order_book.md#0x7_order_book_ensure_native_index_ready">ensure_native_index_ready</a>();
    <b>let</b> _r = self.price_time_idx.<a href="order_book.md#0x7_order_book_best_bid_price">best_bid_price</a>();
    native_timing_end(14, &<b>mut</b> _ob_t);
    _r
}
</code></pre>



</details>

<a id="0x7_order_book_best_ask_price"></a>

## Function `best_ask_price`



<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_best_ask_price">best_ask_price</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="order_book.md#0x7_order_book_OrderBook">order_book::OrderBook</a>&lt;M&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_best_ask_price">best_ask_price</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="order_book.md#0x7_order_book_OrderBook">OrderBook</a>&lt;M&gt;
): Option&lt;u64&gt; {
    <b>let</b> _ob_t = native_timing_start();
    self.<a href="order_book.md#0x7_order_book_ensure_native_index_ready">ensure_native_index_ready</a>();
    <b>let</b> _r = self.price_time_idx.<a href="order_book.md#0x7_order_book_best_ask_price">best_ask_price</a>();
    native_timing_end(15, &<b>mut</b> _ob_t);
    _r
}
</code></pre>



</details>

<a id="0x7_order_book_get_slippage_price"></a>

## Function `get_slippage_price`



<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_get_slippage_price">get_slippage_price</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="order_book.md#0x7_order_book_OrderBook">order_book::OrderBook</a>&lt;M&gt;, is_bid: bool, slippage_bps: u64): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_get_slippage_price">get_slippage_price</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="order_book.md#0x7_order_book_OrderBook">OrderBook</a>&lt;M&gt;, is_bid: bool, slippage_bps: u64
): Option&lt;u64&gt; {
    self.<a href="order_book.md#0x7_order_book_ensure_native_index_ready">ensure_native_index_ready</a>();
    self.price_time_idx.<a href="order_book.md#0x7_order_book_get_slippage_price">get_slippage_price</a>(is_bid, slippage_bps)
}
</code></pre>



</details>

<a id="0x7_order_book_is_taker_order"></a>

## Function `is_taker_order`

Checks if the order is a taker order i.e., matched immediately with the active order book.


<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_is_taker_order">is_taker_order</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="order_book.md#0x7_order_book_OrderBook">order_book::OrderBook</a>&lt;M&gt;, price: u64, is_bid: bool, trigger_condition: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="_TriggerCondition">order_book_types::TriggerCondition</a>&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_is_taker_order">is_taker_order</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="order_book.md#0x7_order_book_OrderBook">OrderBook</a>&lt;M&gt;,
    price: u64,
    is_bid: bool,
    trigger_condition: Option&lt;TriggerCondition&gt;
): bool {
    self.<a href="order_book.md#0x7_order_book_ensure_native_index_ready">ensure_native_index_ready</a>();
    <b>let</b> _t = native_timing_start();
    <b>let</b> result = <b>if</b> (trigger_condition.is_some()) {
        <b>false</b>
    } <b>else</b> {
        self.price_time_idx.<a href="order_book.md#0x7_order_book_is_taker_order">is_taker_order</a>(price, is_bid)
    };
    native_timing_end(13, &<b>mut</b> _t);
    result
}
</code></pre>



</details>

<a id="0x7_order_book_get_single_match_for_taker"></a>

## Function `get_single_match_for_taker`



<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_get_single_match_for_taker">get_single_match_for_taker</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="order_book.md#0x7_order_book_OrderBook">order_book::OrderBook</a>&lt;M&gt;, price: u64, size: u64, is_bid: bool): <a href="_OrderMatch">order_match_types::OrderMatch</a>&lt;M&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_get_single_match_for_taker">get_single_match_for_taker</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="order_book.md#0x7_order_book_OrderBook">OrderBook</a>&lt;M&gt;,
    price: u64,
    size: u64,
    is_bid: bool
): OrderMatch&lt;M&gt; {
    self.<a href="order_book.md#0x7_order_book_ensure_native_index_ready">ensure_native_index_ready</a>();
    <b>let</b> _t = native_timing_start();
    <b>let</b> result = self.price_time_idx.get_single_match_result(price, size, is_bid);
    <b>let</b> match_result = <b>if</b> (result.is_active_matched_book_type_single_order()) {
        self.<a href="single_order_book.md#0x7_single_order_book">single_order_book</a>.<a href="order_book.md#0x7_order_book_get_single_match_for_taker">get_single_match_for_taker</a>(result)
    } <b>else</b> {
        self.<a href="bulk_order_book.md#0x7_bulk_order_book">bulk_order_book</a>.<a href="order_book.md#0x7_order_book_get_single_match_for_taker">get_single_match_for_taker</a>(
            &<b>mut</b> self.price_time_idx, result, is_bid
        )
    };
    native_timing_end(12, &<b>mut</b> _t);
    match_result
}
</code></pre>



</details>

<a id="0x7_order_book_reinsert_order"></a>

## Function `reinsert_order`



<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_reinsert_order">reinsert_order</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="order_book.md#0x7_order_book_OrderBook">order_book::OrderBook</a>&lt;M&gt;, reinsert_order: <a href="_OrderMatchDetails">order_match_types::OrderMatchDetails</a>&lt;M&gt;, original_order: &<a href="_OrderMatchDetails">order_match_types::OrderMatchDetails</a>&lt;M&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_reinsert_order">reinsert_order</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="order_book.md#0x7_order_book_OrderBook">OrderBook</a>&lt;M&gt;,
    reinsert_order: OrderMatchDetails&lt;M&gt;,
    original_order: &OrderMatchDetails&lt;M&gt;
) {
    <b>let</b> _ob_t = native_timing_start();
    self.<a href="order_book.md#0x7_order_book_ensure_native_index_ready">ensure_native_index_ready</a>();
    <b>if</b> (reinsert_order.is_single_order_from_match_details()) {
        self.<a href="single_order_book.md#0x7_single_order_book">single_order_book</a>.<a href="order_book.md#0x7_order_book_reinsert_order">reinsert_order</a>(
            &<b>mut</b> self.price_time_idx, reinsert_order, original_order
        )
    } <b>else</b> {
        self.<a href="bulk_order_book.md#0x7_bulk_order_book">bulk_order_book</a>.<a href="order_book.md#0x7_order_book_reinsert_order">reinsert_order</a>(
            &<b>mut</b> self.price_time_idx, reinsert_order, original_order
        );
    }
}
</code></pre>



</details>

<a id="0x7_order_book_get_bulk_order_remaining_size"></a>

## Function `get_bulk_order_remaining_size`



<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_get_bulk_order_remaining_size">get_bulk_order_remaining_size</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="order_book.md#0x7_order_book_OrderBook">order_book::OrderBook</a>&lt;M&gt;, order_creator: <b>address</b>, is_bid: bool): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_get_bulk_order_remaining_size">get_bulk_order_remaining_size</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="order_book.md#0x7_order_book_OrderBook">OrderBook</a>&lt;M&gt;, order_creator: <b>address</b>, is_bid: bool
): u64 {
    self.<a href="order_book.md#0x7_order_book_ensure_native_index_ready">ensure_native_index_ready</a>();
    self.<a href="bulk_order_book.md#0x7_bulk_order_book">bulk_order_book</a>.get_remaining_size(order_creator, is_bid)
}
</code></pre>



</details>

<a id="0x7_order_book_place_bulk_order"></a>

## Function `place_bulk_order`



<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_place_bulk_order">place_bulk_order</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="order_book.md#0x7_order_book_OrderBook">order_book::OrderBook</a>&lt;M&gt;, order_req: <a href="_BulkOrderRequest">bulk_order_types::BulkOrderRequest</a>&lt;M&gt;): <a href="_BulkOrderPlaceResponse">bulk_order_types::BulkOrderPlaceResponse</a>&lt;M&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_place_bulk_order">place_bulk_order</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="order_book.md#0x7_order_book_OrderBook">OrderBook</a>&lt;M&gt;, order_req: BulkOrderRequest&lt;M&gt;
): BulkOrderPlaceResponse&lt;M&gt; {
    self.<a href="order_book.md#0x7_order_book_ensure_native_index_ready">ensure_native_index_ready</a>();
    <b>let</b> _t = native_timing_start();
    <b>let</b> result = self.<a href="bulk_order_book.md#0x7_bulk_order_book">bulk_order_book</a>.<a href="order_book.md#0x7_order_book_place_bulk_order">place_bulk_order</a>(&<b>mut</b> self.price_time_idx, order_req);
    native_timing_end(16, &<b>mut</b> _t);
    result
}
</code></pre>



</details>

<a id="0x7_order_book_get_bulk_order"></a>

## Function `get_bulk_order`



<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_get_bulk_order">get_bulk_order</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<a href="order_book.md#0x7_order_book_OrderBook">order_book::OrderBook</a>&lt;M&gt;, order_creator: <b>address</b>): <a href="_BulkOrder">bulk_order_types::BulkOrder</a>&lt;M&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_get_bulk_order">get_bulk_order</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<a href="order_book.md#0x7_order_book_OrderBook">OrderBook</a>&lt;M&gt;, order_creator: <b>address</b>
): BulkOrder&lt;M&gt; {
    <b>let</b> _ob_t = native_timing_start();
    self.<a href="order_book.md#0x7_order_book_ensure_native_index_ready">ensure_native_index_ready</a>();
    <b>let</b> _r = self.<a href="bulk_order_book.md#0x7_bulk_order_book">bulk_order_book</a>.<a href="order_book.md#0x7_order_book_get_bulk_order">get_bulk_order</a>(order_creator);
    native_timing_end(21, &<b>mut</b> _ob_t);
    _r
}
</code></pre>



</details>

<a id="0x7_order_book_cancel_bulk_order"></a>

## Function `cancel_bulk_order`



<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_cancel_bulk_order">cancel_bulk_order</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="order_book.md#0x7_order_book_OrderBook">order_book::OrderBook</a>&lt;M&gt;, order_creator: <b>address</b>): <a href="_BulkOrder">bulk_order_types::BulkOrder</a>&lt;M&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_cancel_bulk_order">cancel_bulk_order</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="order_book.md#0x7_order_book_OrderBook">OrderBook</a>&lt;M&gt;, order_creator: <b>address</b>
): BulkOrder&lt;M&gt; {
    self.<a href="order_book.md#0x7_order_book_ensure_native_index_ready">ensure_native_index_ready</a>();
    self.<a href="bulk_order_book.md#0x7_bulk_order_book">bulk_order_book</a>.<a href="order_book.md#0x7_order_book_cancel_bulk_order">cancel_bulk_order</a>(&<b>mut</b> self.price_time_idx, order_creator)
}
</code></pre>



</details>

<a id="0x7_order_book_cancel_bulk_order_at_price"></a>

## Function `cancel_bulk_order_at_price`



<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_cancel_bulk_order_at_price">cancel_bulk_order_at_price</a>&lt;M: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="order_book.md#0x7_order_book_OrderBook">order_book::OrderBook</a>&lt;M&gt;, order_creator: <b>address</b>, price: u64, is_bid: bool): (u64, <a href="_BulkOrder">bulk_order_types::BulkOrder</a>&lt;M&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_book.md#0x7_order_book_cancel_bulk_order_at_price">cancel_bulk_order_at_price</a>&lt;M: store + <b>copy</b> + drop&gt;(
    self: &<b>mut</b> <a href="order_book.md#0x7_order_book_OrderBook">OrderBook</a>&lt;M&gt;,
    order_creator: <b>address</b>,
    price: u64,
    is_bid: bool
): (u64, BulkOrder&lt;M&gt;) {
    <b>let</b> _ob_t = native_timing_start();
    self.<a href="order_book.md#0x7_order_book_ensure_native_index_ready">ensure_native_index_ready</a>();
    self.<a href="bulk_order_book.md#0x7_bulk_order_book">bulk_order_book</a>.<a href="order_book.md#0x7_order_book_cancel_bulk_order_at_price">cancel_bulk_order_at_price</a>(
        &<b>mut</b> self.price_time_idx,
        order_creator,
        price,
        is_bid
    )
}
</code></pre>



</details>

<a id="0x7_order_book_native_is_acquired"></a>

## Function `native_is_acquired`



<pre><code><b>fun</b> <a href="order_book.md#0x7_order_book_native_is_acquired">native_is_acquired</a>(market_addr: <b>address</b>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="order_book.md#0x7_order_book_native_is_acquired">native_is_acquired</a>(market_addr: <b>address</b>): bool;
</code></pre>



</details>

<a id="0x7_order_book_native_ensure_acquired"></a>

## Function `native_ensure_acquired`



<pre><code><b>fun</b> <a href="order_book.md#0x7_order_book_native_ensure_acquired">native_ensure_acquired</a>(market_addr: <b>address</b>, handle: u64): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="order_book.md#0x7_order_book_native_ensure_acquired">native_ensure_acquired</a>(market_addr: <b>address</b>, handle: u64): bool;
</code></pre>



</details>

<a id="0x7_order_book_native_flush"></a>

## Function `native_flush`



<pre><code><b>fun</b> <a href="order_book.md#0x7_order_book_native_flush">native_flush</a>(market_addr: <b>address</b>, new_handle: u64): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="order_book.md#0x7_order_book_native_flush">native_flush</a>(market_addr: <b>address</b>, new_handle: u64): bool;
</code></pre>



</details>

<a id="0x7_order_book_native_rebuild_complete"></a>

## Function `native_rebuild_complete`



<pre><code><b>fun</b> <a href="order_book.md#0x7_order_book_native_rebuild_complete">native_rebuild_complete</a>(market_addr: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="order_book.md#0x7_order_book_native_rebuild_complete">native_rebuild_complete</a>(market_addr: <b>address</b>);
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
