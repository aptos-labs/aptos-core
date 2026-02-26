
<a id="0x7_confidential_balance"></a>

# Module `0x7::confidential_balance`

This module provides the shared <code>ConfidentialBalanceRandomness</code> type used by the Confidential Balance modules
(<code><a href="confidential_pending_balance.md#0x7_confidential_pending_balance">confidential_pending_balance</a></code> and <code><a href="confidential_available_balance.md#0x7_confidential_available_balance">confidential_available_balance</a></code>) for test-only randomness generation.

The actual balance types and their specialized chunk-splitting / randomness functions live in:
- <code><a href="confidential_pending_balance.md#0x7_confidential_pending_balance">confidential_pending_balance</a></code>: PendingBalance (4 chunks, 64-bit values)
- <code><a href="confidential_available_balance.md#0x7_confidential_available_balance">confidential_available_balance</a></code>: AvailableBalance (8 chunks, 128-bit values, with auditor A component)


-  [Constants](#@Constants_0)
-  [Function `get_chunk_size_bits`](#0x7_confidential_balance_get_chunk_size_bits)


<pre><code></code></pre>



<a id="@Constants_0"></a>

## Constants


<a id="0x7_confidential_balance_CHUNK_SIZE_BITS"></a>

The number of bits $b$ in a single chunk.


<pre><code><b>const</b> <a href="confidential_balance.md#0x7_confidential_balance_CHUNK_SIZE_BITS">CHUNK_SIZE_BITS</a>: u64 = 16;
</code></pre>



<a id="0x7_confidential_balance_get_chunk_size_bits"></a>

## Function `get_chunk_size_bits`

Returns the number of bits per chunk.


<pre><code><b>public</b> <b>fun</b> <a href="confidential_balance.md#0x7_confidential_balance_get_chunk_size_bits">get_chunk_size_bits</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_balance.md#0x7_confidential_balance_get_chunk_size_bits">get_chunk_size_bits</a>(): u64 {
    <a href="confidential_balance.md#0x7_confidential_balance_CHUNK_SIZE_BITS">CHUNK_SIZE_BITS</a>
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
