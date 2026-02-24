
<a id="0x7_order_id_generation"></a>

# Module `0x7::order_id_generation`



-  [Function `next_order_id`](#0x7_order_id_generation_next_order_id)
-  [Function `reverse_bits`](#0x7_order_id_generation_reverse_bits)


<pre><code><b>use</b> <a href="../../aptos-framework/doc/transaction_context.md#0x1_transaction_context">0x1::transaction_context</a>;
<b>use</b> <a href="">0x5::order_book_types</a>;
</code></pre>



<a id="0x7_order_id_generation_next_order_id"></a>

## Function `next_order_id`



<pre><code><b>public</b> <b>fun</b> <a href="order_id_generation.md#0x7_order_id_generation_next_order_id">next_order_id</a>(): <a href="_OrderId">order_book_types::OrderId</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="order_id_generation.md#0x7_order_id_generation_next_order_id">next_order_id</a>(): OrderId {
    // reverse bits <b>to</b> make order ids random, so indices on top of them are shuffled.
    new_order_id_type(<a href="order_id_generation.md#0x7_order_id_generation_reverse_bits">reverse_bits</a>(
        <a href="../../aptos-framework/doc/transaction_context.md#0x1_transaction_context_monotonically_increasing_counter">transaction_context::monotonically_increasing_counter</a>()
    ))
}
</code></pre>



</details>

<a id="0x7_order_id_generation_reverse_bits"></a>

## Function `reverse_bits`

Reverse the bits in a u128 value using divide and conquer approach
This is more efficient than the bit-by-bit approach, reducing from O(n) to O(log n)


<pre><code><b>fun</b> <a href="order_id_generation.md#0x7_order_id_generation_reverse_bits">reverse_bits</a>(value: u128): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="order_id_generation.md#0x7_order_id_generation_reverse_bits">reverse_bits</a>(value: u128): u128 {
    <b>let</b> v = value;

    // Swap odd and even bits
    v =
        ((v & 0x55555555555555555555555555555555) &lt;&lt; 1)
            | ((v &gt;&gt; 1) & 0x55555555555555555555555555555555);

    // Swap consecutive pairs
    v =
        ((v & 0x33333333333333333333333333333333) &lt;&lt; 2)
            | ((v &gt;&gt; 2) & 0x33333333333333333333333333333333);

    // Swap nibbles
    v =
        ((v & 0x0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f) &lt;&lt; 4)
            | ((v &gt;&gt; 4) & 0x0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f);

    // Swap bytes
    v =
        ((v & 0x00ff00ff00ff00ff00ff00ff00ff00ff) &lt;&lt; 8)
            | ((v &gt;&gt; 8) & 0x00ff00ff00ff00ff00ff00ff00ff00ff);

    // Swap 2-byte chunks
    v =
        ((v & 0x0000ffff0000ffff0000ffff0000ffff) &lt;&lt; 16)
            | ((v &gt;&gt; 16) & 0x0000ffff0000ffff0000ffff0000ffff);

    // Swap 4-byte chunks
    v =
        ((v & 0x00000000ffffffff00000000ffffffff) &lt;&lt; 32)
            | ((v &gt;&gt; 32) & 0x00000000ffffffff00000000ffffffff);

    // Swap 8-byte chunks
    v = (v &lt;&lt; 64) | (v &gt;&gt; 64);

    v
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
