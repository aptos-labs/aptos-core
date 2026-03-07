
<a id="0x7_confidential_pending_balance"></a>

# Module `0x7::confidential_pending_balance`

Pending balance: 4 Twisted ElGamal ciphertext pairs (P_i, R_i), supporting 64-bit values.
P_i = a_i*G + r_i*H, R_i = r_i*EK.


-  [Constants](#@Constants_0)
-  [Function `new_from_p_and_r`](#0x7_confidential_pending_balance_new_from_p_and_r)
-  [Function `new_compressed_from_p_and_r`](#0x7_confidential_pending_balance_new_compressed_from_p_and_r)
-  [Function `split_into_chunks`](#0x7_confidential_pending_balance_split_into_chunks)
-  [Function `new_zero_compressed`](#0x7_confidential_pending_balance_new_zero_compressed)
-  [Function `new_u64_no_randomness`](#0x7_confidential_pending_balance_new_u64_no_randomness)
-  [Function `get_num_chunks`](#0x7_confidential_pending_balance_get_num_chunks)


<pre><code><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255">0x1::ristretto255</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
<b>use</b> <a href="confidential_balance.md#0x7_confidential_balance">0x7::confidential_balance</a>;
</code></pre>



<a id="@Constants_0"></a>

## Constants


<a id="0x7_confidential_pending_balance_PENDING_BALANCE_CHUNKS"></a>

The number of chunks $n$ in a pending balance.


<pre><code><b>const</b> <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_PENDING_BALANCE_CHUNKS">PENDING_BALANCE_CHUNKS</a>: u64 = 4;
</code></pre>



<a id="0x7_confidential_pending_balance_new_from_p_and_r"></a>

## Function `new_from_p_and_r`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_new_from_p_and_r">new_from_p_and_r</a>(p: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;, r: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;): <a href="confidential_balance.md#0x7_confidential_balance_Balance">confidential_balance::Balance</a>&lt;<a href="confidential_balance.md#0x7_confidential_balance_Pending">confidential_balance::Pending</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_new_from_p_and_r">new_from_p_and_r</a>(p: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;RistrettoPoint&gt;, r: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;RistrettoPoint&gt;): Balance&lt;Pending&gt; {
    <a href="confidential_balance.md#0x7_confidential_balance_new_balance">confidential_balance::new_balance</a>(p, r, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[], <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_PENDING_BALANCE_CHUNKS">PENDING_BALANCE_CHUNKS</a>)
}
</code></pre>



</details>

<a id="0x7_confidential_pending_balance_new_compressed_from_p_and_r"></a>

## Function `new_compressed_from_p_and_r`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_new_compressed_from_p_and_r">new_compressed_from_p_and_r</a>(p: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;, r: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;): <a href="confidential_balance.md#0x7_confidential_balance_CompressedBalance">confidential_balance::CompressedBalance</a>&lt;<a href="confidential_balance.md#0x7_confidential_balance_Pending">confidential_balance::Pending</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_new_compressed_from_p_and_r">new_compressed_from_p_and_r</a>(
    p: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;CompressedRistretto&gt;, r: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;CompressedRistretto&gt;
): CompressedBalance&lt;Pending&gt; {
    <a href="confidential_balance.md#0x7_confidential_balance_new_compressed_balance">confidential_balance::new_compressed_balance</a>(p, r, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[], <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_PENDING_BALANCE_CHUNKS">PENDING_BALANCE_CHUNKS</a>)
}
</code></pre>



</details>

<a id="0x7_confidential_pending_balance_split_into_chunks"></a>

## Function `split_into_chunks`

Splits an integer amount into <code><a href="confidential_pending_balance.md#0x7_confidential_pending_balance_PENDING_BALANCE_CHUNKS">PENDING_BALANCE_CHUNKS</a></code> 16-bit chunks, represented as <code>Scalar</code> values.


<pre><code><b>public</b> <b>fun</b> <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_split_into_chunks">split_into_chunks</a>(amount: u128): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_split_into_chunks">split_into_chunks</a>(amount: u128): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Scalar&gt; {
    <a href="confidential_balance.md#0x7_confidential_balance_split_into_chunks">confidential_balance::split_into_chunks</a>(amount, <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_PENDING_BALANCE_CHUNKS">PENDING_BALANCE_CHUNKS</a>)
}
</code></pre>



</details>

<a id="0x7_confidential_pending_balance_new_zero_compressed"></a>

## Function `new_zero_compressed`



<pre><code><b>public</b> <b>fun</b> <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_new_zero_compressed">new_zero_compressed</a>(): <a href="confidential_balance.md#0x7_confidential_balance_CompressedBalance">confidential_balance::CompressedBalance</a>&lt;<a href="confidential_balance.md#0x7_confidential_balance_Pending">confidential_balance::Pending</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_new_zero_compressed">new_zero_compressed</a>(): CompressedBalance&lt;Pending&gt; {
    <a href="confidential_balance.md#0x7_confidential_balance_new_zero_compressed">confidential_balance::new_zero_compressed</a>(<a href="confidential_pending_balance.md#0x7_confidential_pending_balance_PENDING_BALANCE_CHUNKS">PENDING_BALANCE_CHUNKS</a>)
}
</code></pre>



</details>

<a id="0x7_confidential_pending_balance_new_u64_no_randomness"></a>

## Function `new_u64_no_randomness`

Creates a pending balance from a 64-bit amount with no randomness (R = identity).


<pre><code><b>public</b> <b>fun</b> <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_new_u64_no_randomness">new_u64_no_randomness</a>(amount: u64): <a href="confidential_balance.md#0x7_confidential_balance_Balance">confidential_balance::Balance</a>&lt;<a href="confidential_balance.md#0x7_confidential_balance_Pending">confidential_balance::Pending</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_new_u64_no_randomness">new_u64_no_randomness</a>(amount: u64): Balance&lt;Pending&gt; {
    <b>let</b> identity = point_identity();
    <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_new_from_p_and_r">new_from_p_and_r</a>(
        <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_split_into_chunks">split_into_chunks</a>((amount <b>as</b> u128)).map(|chunk| chunk.basepoint_mul()),
        std::vector::range(0, <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_PENDING_BALANCE_CHUNKS">PENDING_BALANCE_CHUNKS</a>).map(|_| identity.point_clone()),
    )
}
</code></pre>



</details>

<a id="0x7_confidential_pending_balance_get_num_chunks"></a>

## Function `get_num_chunks`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_get_num_chunks">get_num_chunks</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_get_num_chunks">get_num_chunks</a>(): u64 {
    <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_PENDING_BALANCE_CHUNKS">PENDING_BALANCE_CHUNKS</a>
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
