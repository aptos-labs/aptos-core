
<a id="0x7_confidential_available_balance"></a>

# Module `0x7::confidential_available_balance`

Available balance: 8 Twisted ElGamal ciphertext triples (P_i, R_i, R_aud_i), supporting 128-bit values.
P_i = a_i*G + r_i*H, R_i = r_i*EK, R_aud_i = r_i*EK_auditor (empty if no auditor).


-  [Constants](#@Constants_0)
-  [Function `new_from_p_r_r_aud`](#0x7_confidential_available_balance_new_from_p_r_r_aud)
-  [Function `new_compressed_from_p_r_r_aud`](#0x7_confidential_available_balance_new_compressed_from_p_r_r_aud)
-  [Function `new_zero_compressed`](#0x7_confidential_available_balance_new_zero_compressed)
-  [Function `new_compressed_from_bytes`](#0x7_confidential_available_balance_new_compressed_from_bytes)
-  [Function `split_into_chunks`](#0x7_confidential_available_balance_split_into_chunks)
-  [Function `get_num_chunks`](#0x7_confidential_available_balance_get_num_chunks)


<pre><code><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255">0x1::ristretto255</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
<b>use</b> <a href="confidential_balance.md#0x7_confidential_balance">0x7::confidential_balance</a>;
</code></pre>



<a id="@Constants_0"></a>

## Constants


<a id="0x7_confidential_available_balance_AVAILABLE_BALANCE_CHUNKS"></a>

The number of chunks $\ell$ in an available balance.


<pre><code><b>const</b> <a href="confidential_available_balance.md#0x7_confidential_available_balance_AVAILABLE_BALANCE_CHUNKS">AVAILABLE_BALANCE_CHUNKS</a>: u64 = 8;
</code></pre>



<a id="0x7_confidential_available_balance_new_from_p_r_r_aud"></a>

## Function `new_from_p_r_r_aud`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="confidential_available_balance.md#0x7_confidential_available_balance_new_from_p_r_r_aud">new_from_p_r_r_aud</a>(p: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;, r: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;, r_aud: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;): <a href="confidential_balance.md#0x7_confidential_balance_Balance">confidential_balance::Balance</a>&lt;<a href="confidential_balance.md#0x7_confidential_balance_Available">confidential_balance::Available</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="confidential_available_balance.md#0x7_confidential_available_balance_new_from_p_r_r_aud">new_from_p_r_r_aud</a>(
    p: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;RistrettoPoint&gt;, r: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;RistrettoPoint&gt;, r_aud: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;RistrettoPoint&gt;
): Balance&lt;Available&gt; {
    <a href="confidential_balance.md#0x7_confidential_balance_new_balance">confidential_balance::new_balance</a>(p, r, r_aud, <a href="confidential_available_balance.md#0x7_confidential_available_balance_AVAILABLE_BALANCE_CHUNKS">AVAILABLE_BALANCE_CHUNKS</a>)
}
</code></pre>



</details>

<a id="0x7_confidential_available_balance_new_compressed_from_p_r_r_aud"></a>

## Function `new_compressed_from_p_r_r_aud`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="confidential_available_balance.md#0x7_confidential_available_balance_new_compressed_from_p_r_r_aud">new_compressed_from_p_r_r_aud</a>(p: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;, r: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;, r_aud: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;): <a href="confidential_balance.md#0x7_confidential_balance_CompressedBalance">confidential_balance::CompressedBalance</a>&lt;<a href="confidential_balance.md#0x7_confidential_balance_Available">confidential_balance::Available</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="confidential_available_balance.md#0x7_confidential_available_balance_new_compressed_from_p_r_r_aud">new_compressed_from_p_r_r_aud</a>(
    p: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;CompressedRistretto&gt;, r: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;CompressedRistretto&gt;, r_aud: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;CompressedRistretto&gt;
): CompressedBalance&lt;Available&gt; {
    <a href="confidential_balance.md#0x7_confidential_balance_new_compressed_balance">confidential_balance::new_compressed_balance</a>(p, r, r_aud, <a href="confidential_available_balance.md#0x7_confidential_available_balance_AVAILABLE_BALANCE_CHUNKS">AVAILABLE_BALANCE_CHUNKS</a>)
}
</code></pre>



</details>

<a id="0x7_confidential_available_balance_new_zero_compressed"></a>

## Function `new_zero_compressed`



<pre><code><b>public</b> <b>fun</b> <a href="confidential_available_balance.md#0x7_confidential_available_balance_new_zero_compressed">new_zero_compressed</a>(): <a href="confidential_balance.md#0x7_confidential_balance_CompressedBalance">confidential_balance::CompressedBalance</a>&lt;<a href="confidential_balance.md#0x7_confidential_balance_Available">confidential_balance::Available</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_available_balance.md#0x7_confidential_available_balance_new_zero_compressed">new_zero_compressed</a>(): CompressedBalance&lt;Available&gt; {
    <a href="confidential_balance.md#0x7_confidential_balance_new_zero_compressed">confidential_balance::new_zero_compressed</a>(<a href="confidential_available_balance.md#0x7_confidential_available_balance_AVAILABLE_BALANCE_CHUNKS">AVAILABLE_BALANCE_CHUNKS</a>)
}
</code></pre>



</details>

<a id="0x7_confidential_available_balance_new_compressed_from_bytes"></a>

## Function `new_compressed_from_bytes`

Deserializes raw byte vectors into a CompressedBalance<Available> (without decompressing).


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="confidential_available_balance.md#0x7_confidential_available_balance_new_compressed_from_bytes">new_compressed_from_bytes</a>(p_bytes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, r_bytes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, r_aud_bytes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;): <a href="confidential_balance.md#0x7_confidential_balance_CompressedBalance">confidential_balance::CompressedBalance</a>&lt;<a href="confidential_balance.md#0x7_confidential_balance_Available">confidential_balance::Available</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="confidential_available_balance.md#0x7_confidential_available_balance_new_compressed_from_bytes">new_compressed_from_bytes</a>(
    p_bytes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    r_bytes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    r_aud_bytes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
): CompressedBalance&lt;Available&gt; {
    <a href="confidential_available_balance.md#0x7_confidential_available_balance_new_compressed_from_p_r_r_aud">new_compressed_from_p_r_r_aud</a>(
        deserialize_compressed_points(p_bytes),
        deserialize_compressed_points(r_bytes),
        deserialize_compressed_points(r_aud_bytes),
    )
}
</code></pre>



</details>

<a id="0x7_confidential_available_balance_split_into_chunks"></a>

## Function `split_into_chunks`

Splits an integer amount into <code><a href="confidential_available_balance.md#0x7_confidential_available_balance_AVAILABLE_BALANCE_CHUNKS">AVAILABLE_BALANCE_CHUNKS</a></code> 16-bit chunks, represented as <code>Scalar</code> values.


<pre><code><b>public</b> <b>fun</b> <a href="confidential_available_balance.md#0x7_confidential_available_balance_split_into_chunks">split_into_chunks</a>(amount: u128): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_available_balance.md#0x7_confidential_available_balance_split_into_chunks">split_into_chunks</a>(amount: u128): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Scalar&gt; {
    <a href="confidential_balance.md#0x7_confidential_balance_split_into_chunks">confidential_balance::split_into_chunks</a>(amount, <a href="confidential_available_balance.md#0x7_confidential_available_balance_AVAILABLE_BALANCE_CHUNKS">AVAILABLE_BALANCE_CHUNKS</a>)
}
</code></pre>



</details>

<a id="0x7_confidential_available_balance_get_num_chunks"></a>

## Function `get_num_chunks`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="confidential_available_balance.md#0x7_confidential_available_balance_get_num_chunks">get_num_chunks</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_available_balance.md#0x7_confidential_available_balance_get_num_chunks">get_num_chunks</a>(): u64 {
    <a href="confidential_available_balance.md#0x7_confidential_available_balance_AVAILABLE_BALANCE_CHUNKS">AVAILABLE_BALANCE_CHUNKS</a>
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
