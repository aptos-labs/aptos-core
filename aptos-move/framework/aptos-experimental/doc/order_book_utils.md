
<a id="0x7_order_book_utils"></a>

# Module `0x7::order_book_utils`



-  [Constants](#@Constants_0)
-  [Function `new_default_big_ordered_map`](#0x7_order_book_utils_new_default_big_ordered_map)


<pre><code><b>use</b> <a href="../../aptos-framework/doc/big_ordered_map.md#0x1_big_ordered_map">0x1::big_ordered_map</a>;
</code></pre>



<a id="@Constants_0"></a>

## Constants


<a id="0x7_order_book_utils_BIG_MAP_INNER_DEGREE"></a>



<pre><code><b>const</b> <a href="order_book_utils.md#0x7_order_book_utils_BIG_MAP_INNER_DEGREE">BIG_MAP_INNER_DEGREE</a>: u16 = 64;
</code></pre>



<a id="0x7_order_book_utils_BIG_MAP_LEAF_DEGREE"></a>



<pre><code><b>const</b> <a href="order_book_utils.md#0x7_order_book_utils_BIG_MAP_LEAF_DEGREE">BIG_MAP_LEAF_DEGREE</a>: u16 = 32;
</code></pre>



<a id="0x7_order_book_utils_new_default_big_ordered_map"></a>

## Function `new_default_big_ordered_map`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book_utils.md#0x7_order_book_utils_new_default_big_ordered_map">new_default_big_ordered_map</a>&lt;K: store, V: store&gt;(): <a href="../../aptos-framework/doc/big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">big_ordered_map::BigOrderedMap</a>&lt;K, V&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="order_book_utils.md#0x7_order_book_utils_new_default_big_ordered_map">new_default_big_ordered_map</a>&lt;K: store, V: store&gt;()
    : BigOrderedMap&lt;K, V&gt; {
    <a href="../../aptos-framework/doc/big_ordered_map.md#0x1_big_ordered_map_new_with_config">big_ordered_map::new_with_config</a>(
        <a href="order_book_utils.md#0x7_order_book_utils_BIG_MAP_INNER_DEGREE">BIG_MAP_INNER_DEGREE</a>,
        <a href="order_book_utils.md#0x7_order_book_utils_BIG_MAP_LEAF_DEGREE">BIG_MAP_LEAF_DEGREE</a>,
        <b>true</b>
    )
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
