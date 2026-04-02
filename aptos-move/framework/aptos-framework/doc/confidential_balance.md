
<a id="0x1_confidential_balance"></a>

# Module `0x1::confidential_balance`

Balance types for the confidential asset protocol.

A balances $a$ is chunked into vectors of $b$-bit chunks $[a_0, a_1, \ldots, a_{n-1}]$ such that $a = \sum_{i = 0}^{n-1} a_i B^i$
where $B = 2^b$.
Then, each chunk $i$ is encrypted as a tuple $(P_i = a_i G + r_i H, R_i = r_i \mathsf{ek})$ under an encryption key $\mathsf{ek}$.

The pending balance has $n$ chunks while the available balance has $\ell$ such chunks.
For Aptos, we need $b n = 64$ and $b \ell = 128$.

<code><a href="confidential_balance.md#0x1_confidential_balance_CompressedBalance">CompressedBalance</a>&lt;T&gt;</code> and <code><a href="confidential_balance.md#0x1_confidential_balance_Balance">Balance</a>&lt;T&gt;</code> are parameterized by phantom markers <code><a href="confidential_balance.md#0x1_confidential_balance_Pending">Pending</a></code> and <code><a href="confidential_balance.md#0x1_confidential_balance_Available">Available</a></code>.


-  [Struct `Pending`](#0x1_confidential_balance_Pending)
-  [Struct `Available`](#0x1_confidential_balance_Available)
-  [Enum `CompressedBalance`](#0x1_confidential_balance_CompressedBalance)
-  [Enum `Balance`](#0x1_confidential_balance_Balance)
-  [Constants](#@Constants_0)
-  [Function `get_P`](#0x1_confidential_balance_get_P)
-  [Function `get_R`](#0x1_confidential_balance_get_R)
-  [Function `get_R_aud`](#0x1_confidential_balance_get_R_aud)
-  [Function `get_compressed_P`](#0x1_confidential_balance_get_compressed_P)
-  [Function `get_compressed_R`](#0x1_confidential_balance_get_compressed_R)
-  [Function `get_compressed_R_aud`](#0x1_confidential_balance_get_compressed_R_aud)
-  [Function `compress`](#0x1_confidential_balance_compress)
-  [Function `decompress`](#0x1_confidential_balance_decompress)
-  [Function `is_zero`](#0x1_confidential_balance_is_zero)
-  [Function `add_mut_base`](#0x1_confidential_balance_add_mut_base)
-  [Function `new_balance`](#0x1_confidential_balance_new_balance)
-  [Function `new_compressed_balance`](#0x1_confidential_balance_new_compressed_balance)
-  [Function `new_zero_compressed`](#0x1_confidential_balance_new_zero_compressed)
-  [Function `new_pending_from_p_and_r`](#0x1_confidential_balance_new_pending_from_p_and_r)
-  [Function `new_zero_pending_compressed`](#0x1_confidential_balance_new_zero_pending_compressed)
-  [Function `new_pending_u64_no_randomness`](#0x1_confidential_balance_new_pending_u64_no_randomness)
-  [Function `add_assign_pending`](#0x1_confidential_balance_add_assign_pending)
-  [Function `get_num_pending_chunks`](#0x1_confidential_balance_get_num_pending_chunks)
-  [Function `new_available_from_p_r_r_aud`](#0x1_confidential_balance_new_available_from_p_r_r_aud)
-  [Function `new_compressed_available_from_p_r_r_aud`](#0x1_confidential_balance_new_compressed_available_from_p_r_r_aud)
-  [Function `new_zero_available_compressed`](#0x1_confidential_balance_new_zero_available_compressed)
-  [Function `new_compressed_available_from_bytes`](#0x1_confidential_balance_new_compressed_available_from_bytes)
-  [Function `set_available_R`](#0x1_confidential_balance_set_available_R)
-  [Function `add_assign_available_excluding_auditor`](#0x1_confidential_balance_add_assign_available_excluding_auditor)
-  [Function `get_num_available_chunks`](#0x1_confidential_balance_get_num_available_chunks)
-  [Function `get_chunk_size_bits`](#0x1_confidential_balance_get_chunk_size_bits)
-  [Function `get_chunk_upper_bound`](#0x1_confidential_balance_get_chunk_upper_bound)
-  [Function `split_into_chunks`](#0x1_confidential_balance_split_into_chunks)
-  [Function `get_b_powers`](#0x1_confidential_balance_get_b_powers)
-  [Function `get_encryption_key_basepoint_compressed`](#0x1_confidential_balance_get_encryption_key_basepoint_compressed)
-  [Function `assert_correct_num_chunks`](#0x1_confidential_balance_assert_correct_num_chunks)


<pre><code><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255">0x1::ristretto255</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
</code></pre>



<a id="0x1_confidential_balance_Pending"></a>

## Struct `Pending`



<pre><code><b>struct</b> <a href="confidential_balance.md#0x1_confidential_balance_Pending">Pending</a> <b>has</b> drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>dummy_field: bool</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_confidential_balance_Available"></a>

## Struct `Available`



<pre><code><b>struct</b> <a href="confidential_balance.md#0x1_confidential_balance_Available">Available</a> <b>has</b> drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>dummy_field: bool</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_confidential_balance_CompressedBalance"></a>

## Enum `CompressedBalance`



<pre><code>enum <a href="confidential_balance.md#0x1_confidential_balance_CompressedBalance">CompressedBalance</a>&lt;T&gt; <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>V1</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>P: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>R: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>R_aud: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

</details>

<a id="0x1_confidential_balance_Balance"></a>

## Enum `Balance`



<pre><code>enum <a href="confidential_balance.md#0x1_confidential_balance_Balance">Balance</a>&lt;T&gt; <b>has</b> drop
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>V1</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>P: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>R: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>R_aud: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_confidential_balance_AVAILABLE_BALANCE_CHUNKS"></a>

The number of chunks $\ell$ in an available balance.


<pre><code><b>const</b> <a href="confidential_balance.md#0x1_confidential_balance_AVAILABLE_BALANCE_CHUNKS">AVAILABLE_BALANCE_CHUNKS</a>: u64 = 8;
</code></pre>



<a id="0x1_confidential_balance_CHUNK_SIZE_BITS"></a>

The number of bits $b$ in a single chunk.


<pre><code><b>const</b> <a href="confidential_balance.md#0x1_confidential_balance_CHUNK_SIZE_BITS">CHUNK_SIZE_BITS</a>: u64 = 16;
</code></pre>



<a id="0x1_confidential_balance_CHUNK_UPPER_BOUND"></a>

All chunks are < than this value


<pre><code><b>const</b> <a href="confidential_balance.md#0x1_confidential_balance_CHUNK_UPPER_BOUND">CHUNK_UPPER_BOUND</a>: u64 = 65536;
</code></pre>



<a id="0x1_confidential_balance_E_WRONG_NUM_CHUNKS"></a>

Expected the P or R components to have the wrong number of chunks.


<pre><code><b>const</b> <a href="confidential_balance.md#0x1_confidential_balance_E_WRONG_NUM_CHUNKS">E_WRONG_NUM_CHUNKS</a>: u64 = 1;
</code></pre>



<a id="0x1_confidential_balance_E_WRONG_NUM_CHUNKS_FOR_AUDITOR"></a>

Expected the auditor R-component to be either empty or have the correct number of chunks.


<pre><code><b>const</b> <a href="confidential_balance.md#0x1_confidential_balance_E_WRONG_NUM_CHUNKS_FOR_AUDITOR">E_WRONG_NUM_CHUNKS_FOR_AUDITOR</a>: u64 = 2;
</code></pre>



<a id="0x1_confidential_balance_PENDING_BALANCE_CHUNKS"></a>

The number of chunks $n$ in a pending balance.


<pre><code><b>const</b> <a href="confidential_balance.md#0x1_confidential_balance_PENDING_BALANCE_CHUNKS">PENDING_BALANCE_CHUNKS</a>: u64 = 4;
</code></pre>



<a id="0x1_confidential_balance_get_P"></a>

## Function `get_P`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="confidential_balance.md#0x1_confidential_balance_get_P">get_P</a>&lt;T&gt;(self: &<a href="confidential_balance.md#0x1_confidential_balance_Balance">confidential_balance::Balance</a>&lt;T&gt;): &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="confidential_balance.md#0x1_confidential_balance_get_P">get_P</a>&lt;T&gt;(self: &<a href="confidential_balance.md#0x1_confidential_balance_Balance">Balance</a>&lt;T&gt;): &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;RistrettoPoint&gt; { &self.P }
</code></pre>



</details>

<a id="0x1_confidential_balance_get_R"></a>

## Function `get_R`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="confidential_balance.md#0x1_confidential_balance_get_R">get_R</a>&lt;T&gt;(self: &<a href="confidential_balance.md#0x1_confidential_balance_Balance">confidential_balance::Balance</a>&lt;T&gt;): &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="confidential_balance.md#0x1_confidential_balance_get_R">get_R</a>&lt;T&gt;(self: &<a href="confidential_balance.md#0x1_confidential_balance_Balance">Balance</a>&lt;T&gt;): &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;RistrettoPoint&gt; { &self.R }
</code></pre>



</details>

<a id="0x1_confidential_balance_get_R_aud"></a>

## Function `get_R_aud`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="confidential_balance.md#0x1_confidential_balance_get_R_aud">get_R_aud</a>&lt;T&gt;(self: &<a href="confidential_balance.md#0x1_confidential_balance_Balance">confidential_balance::Balance</a>&lt;T&gt;): &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="confidential_balance.md#0x1_confidential_balance_get_R_aud">get_R_aud</a>&lt;T&gt;(self: &<a href="confidential_balance.md#0x1_confidential_balance_Balance">Balance</a>&lt;T&gt;): &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;RistrettoPoint&gt; { &self.R_aud }
</code></pre>



</details>

<a id="0x1_confidential_balance_get_compressed_P"></a>

## Function `get_compressed_P`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="confidential_balance.md#0x1_confidential_balance_get_compressed_P">get_compressed_P</a>&lt;T&gt;(self: &<a href="confidential_balance.md#0x1_confidential_balance_CompressedBalance">confidential_balance::CompressedBalance</a>&lt;T&gt;): &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="confidential_balance.md#0x1_confidential_balance_get_compressed_P">get_compressed_P</a>&lt;T&gt;(self: &<a href="confidential_balance.md#0x1_confidential_balance_CompressedBalance">CompressedBalance</a>&lt;T&gt;): &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;CompressedRistretto&gt; { &self.P }
</code></pre>



</details>

<a id="0x1_confidential_balance_get_compressed_R"></a>

## Function `get_compressed_R`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="confidential_balance.md#0x1_confidential_balance_get_compressed_R">get_compressed_R</a>&lt;T&gt;(self: &<a href="confidential_balance.md#0x1_confidential_balance_CompressedBalance">confidential_balance::CompressedBalance</a>&lt;T&gt;): &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="confidential_balance.md#0x1_confidential_balance_get_compressed_R">get_compressed_R</a>&lt;T&gt;(self: &<a href="confidential_balance.md#0x1_confidential_balance_CompressedBalance">CompressedBalance</a>&lt;T&gt;): &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;CompressedRistretto&gt; { &self.R }
</code></pre>



</details>

<a id="0x1_confidential_balance_get_compressed_R_aud"></a>

## Function `get_compressed_R_aud`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="confidential_balance.md#0x1_confidential_balance_get_compressed_R_aud">get_compressed_R_aud</a>&lt;T&gt;(self: &<a href="confidential_balance.md#0x1_confidential_balance_CompressedBalance">confidential_balance::CompressedBalance</a>&lt;T&gt;): &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="confidential_balance.md#0x1_confidential_balance_get_compressed_R_aud">get_compressed_R_aud</a>&lt;T&gt;(self: &<a href="confidential_balance.md#0x1_confidential_balance_CompressedBalance">CompressedBalance</a>&lt;T&gt;): &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;CompressedRistretto&gt; { &self.R_aud }
</code></pre>



</details>

<a id="0x1_confidential_balance_compress"></a>

## Function `compress`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="confidential_balance.md#0x1_confidential_balance_compress">compress</a>&lt;T&gt;(self: &<a href="confidential_balance.md#0x1_confidential_balance_Balance">confidential_balance::Balance</a>&lt;T&gt;): <a href="confidential_balance.md#0x1_confidential_balance_CompressedBalance">confidential_balance::CompressedBalance</a>&lt;T&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="confidential_balance.md#0x1_confidential_balance_compress">compress</a>&lt;T&gt;(self: &<a href="confidential_balance.md#0x1_confidential_balance_Balance">Balance</a>&lt;T&gt;): <a href="confidential_balance.md#0x1_confidential_balance_CompressedBalance">CompressedBalance</a>&lt;T&gt; {
    CompressedBalance::V1 {
        P: self.P.map_ref(|p| p.point_compress()),
        R: self.R.map_ref(|r| r.point_compress()),
        R_aud: self.R_aud.map_ref(|r| r.point_compress()),
    }
}
</code></pre>



</details>

<a id="0x1_confidential_balance_decompress"></a>

## Function `decompress`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="confidential_balance.md#0x1_confidential_balance_decompress">decompress</a>&lt;T&gt;(self: &<a href="confidential_balance.md#0x1_confidential_balance_CompressedBalance">confidential_balance::CompressedBalance</a>&lt;T&gt;): <a href="confidential_balance.md#0x1_confidential_balance_Balance">confidential_balance::Balance</a>&lt;T&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="confidential_balance.md#0x1_confidential_balance_decompress">decompress</a>&lt;T&gt;(self: &<a href="confidential_balance.md#0x1_confidential_balance_CompressedBalance">CompressedBalance</a>&lt;T&gt;): <a href="confidential_balance.md#0x1_confidential_balance_Balance">Balance</a>&lt;T&gt; {
    Balance::V1 {
        P: self.P.map_ref(|p| p.point_decompress()),
        R: self.R.map_ref(|r| r.point_decompress()),
        R_aud: self.R_aud.map_ref(|r| r.point_decompress()),
    }
}
</code></pre>



</details>

<a id="0x1_confidential_balance_is_zero"></a>

## Function `is_zero`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="confidential_balance.md#0x1_confidential_balance_is_zero">is_zero</a>&lt;T&gt;(self: &<a href="confidential_balance.md#0x1_confidential_balance_CompressedBalance">confidential_balance::CompressedBalance</a>&lt;T&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="confidential_balance.md#0x1_confidential_balance_is_zero">is_zero</a>&lt;T&gt;(self: &<a href="confidential_balance.md#0x1_confidential_balance_CompressedBalance">CompressedBalance</a>&lt;T&gt;): bool {
    self.P.all(|p| p.is_identity()) &&
    self.R.all(|r| r.is_identity())
}
</code></pre>



</details>

<a id="0x1_confidential_balance_add_mut_base"></a>

## Function `add_mut_base`

Element-wise P and R addition. R_aud is NOT touched.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="confidential_balance.md#0x1_confidential_balance_add_mut_base">add_mut_base</a>&lt;T&gt;(self: &<b>mut</b> <a href="confidential_balance.md#0x1_confidential_balance_Balance">confidential_balance::Balance</a>&lt;T&gt;, rhs_P: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;, rhs_R: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="confidential_balance.md#0x1_confidential_balance_add_mut_base">add_mut_base</a>&lt;T&gt;(self: &<b>mut</b> <a href="confidential_balance.md#0x1_confidential_balance_Balance">Balance</a>&lt;T&gt;, rhs_P: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;RistrettoPoint&gt;, rhs_R: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;RistrettoPoint&gt;) {
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, rhs_P.length()).for_each(|i| {
        self.P[i].point_add_assign(&rhs_P[i]);
        self.R[i].point_add_assign(&rhs_R[i]);
    });
}
</code></pre>



</details>

<a id="0x1_confidential_balance_new_balance"></a>

## Function `new_balance`



<pre><code><b>fun</b> <a href="confidential_balance.md#0x1_confidential_balance_new_balance">new_balance</a>&lt;T&gt;(p: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;, r: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;, r_aud: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;, expected_chunks: u64): <a href="confidential_balance.md#0x1_confidential_balance_Balance">confidential_balance::Balance</a>&lt;T&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="confidential_balance.md#0x1_confidential_balance_new_balance">new_balance</a>&lt;T&gt;(
    p: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;RistrettoPoint&gt;, r: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;RistrettoPoint&gt;, r_aud: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;RistrettoPoint&gt;,
    expected_chunks: u64,
): <a href="confidential_balance.md#0x1_confidential_balance_Balance">Balance</a>&lt;T&gt; {
    <a href="confidential_balance.md#0x1_confidential_balance_assert_correct_num_chunks">assert_correct_num_chunks</a>(&p, &r, &r_aud, expected_chunks);
    Balance::V1 { P: p, R: r, R_aud: r_aud }
}
</code></pre>



</details>

<a id="0x1_confidential_balance_new_compressed_balance"></a>

## Function `new_compressed_balance`



<pre><code><b>fun</b> <a href="confidential_balance.md#0x1_confidential_balance_new_compressed_balance">new_compressed_balance</a>&lt;T&gt;(p: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;, r: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;, r_aud: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;, expected_chunks: u64): <a href="confidential_balance.md#0x1_confidential_balance_CompressedBalance">confidential_balance::CompressedBalance</a>&lt;T&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="confidential_balance.md#0x1_confidential_balance_new_compressed_balance">new_compressed_balance</a>&lt;T&gt;(
    p: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;CompressedRistretto&gt;, r: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;CompressedRistretto&gt;, r_aud: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;CompressedRistretto&gt;,
    expected_chunks: u64,
): <a href="confidential_balance.md#0x1_confidential_balance_CompressedBalance">CompressedBalance</a>&lt;T&gt; {
    <a href="confidential_balance.md#0x1_confidential_balance_assert_correct_num_chunks">assert_correct_num_chunks</a>(&p, &r, &r_aud, expected_chunks);
    CompressedBalance::V1 { P: p, R: r, R_aud: r_aud }
}
</code></pre>



</details>

<a id="0x1_confidential_balance_new_zero_compressed"></a>

## Function `new_zero_compressed`



<pre><code><b>fun</b> <a href="confidential_balance.md#0x1_confidential_balance_new_zero_compressed">new_zero_compressed</a>&lt;T&gt;(num_chunks: u64): <a href="confidential_balance.md#0x1_confidential_balance_CompressedBalance">confidential_balance::CompressedBalance</a>&lt;T&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="confidential_balance.md#0x1_confidential_balance_new_zero_compressed">new_zero_compressed</a>&lt;T&gt;(num_chunks: u64): <a href="confidential_balance.md#0x1_confidential_balance_CompressedBalance">CompressedBalance</a>&lt;T&gt; {
    <b>let</b> identity = point_identity_compressed();
    <a href="confidential_balance.md#0x1_confidential_balance_new_compressed_balance">new_compressed_balance</a>&lt;T&gt;(
        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, num_chunks).map(|_| identity),
        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, num_chunks).map(|_| identity),
        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[],
        num_chunks
    )
}
</code></pre>



</details>

<a id="0x1_confidential_balance_new_pending_from_p_and_r"></a>

## Function `new_pending_from_p_and_r`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="confidential_balance.md#0x1_confidential_balance_new_pending_from_p_and_r">new_pending_from_p_and_r</a>(p: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;, r: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;): <a href="confidential_balance.md#0x1_confidential_balance_Balance">confidential_balance::Balance</a>&lt;<a href="confidential_balance.md#0x1_confidential_balance_Pending">confidential_balance::Pending</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="confidential_balance.md#0x1_confidential_balance_new_pending_from_p_and_r">new_pending_from_p_and_r</a>(p: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;RistrettoPoint&gt;, r: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;RistrettoPoint&gt;): <a href="confidential_balance.md#0x1_confidential_balance_Balance">Balance</a>&lt;<a href="confidential_balance.md#0x1_confidential_balance_Pending">Pending</a>&gt; {
    <a href="confidential_balance.md#0x1_confidential_balance_new_balance">new_balance</a>(p, r, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[], <a href="confidential_balance.md#0x1_confidential_balance_PENDING_BALANCE_CHUNKS">PENDING_BALANCE_CHUNKS</a>)
}
</code></pre>



</details>

<a id="0x1_confidential_balance_new_zero_pending_compressed"></a>

## Function `new_zero_pending_compressed`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="confidential_balance.md#0x1_confidential_balance_new_zero_pending_compressed">new_zero_pending_compressed</a>(): <a href="confidential_balance.md#0x1_confidential_balance_CompressedBalance">confidential_balance::CompressedBalance</a>&lt;<a href="confidential_balance.md#0x1_confidential_balance_Pending">confidential_balance::Pending</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="confidential_balance.md#0x1_confidential_balance_new_zero_pending_compressed">new_zero_pending_compressed</a>(): <a href="confidential_balance.md#0x1_confidential_balance_CompressedBalance">CompressedBalance</a>&lt;<a href="confidential_balance.md#0x1_confidential_balance_Pending">Pending</a>&gt; {
    <a href="confidential_balance.md#0x1_confidential_balance_new_zero_compressed">new_zero_compressed</a>(<a href="confidential_balance.md#0x1_confidential_balance_PENDING_BALANCE_CHUNKS">PENDING_BALANCE_CHUNKS</a>)
}
</code></pre>



</details>

<a id="0x1_confidential_balance_new_pending_u64_no_randomness"></a>

## Function `new_pending_u64_no_randomness`

Creates a pending balance from a 64-bit amount with no randomness (R = identity).


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="confidential_balance.md#0x1_confidential_balance_new_pending_u64_no_randomness">new_pending_u64_no_randomness</a>(amount: u64): <a href="confidential_balance.md#0x1_confidential_balance_Balance">confidential_balance::Balance</a>&lt;<a href="confidential_balance.md#0x1_confidential_balance_Pending">confidential_balance::Pending</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="confidential_balance.md#0x1_confidential_balance_new_pending_u64_no_randomness">new_pending_u64_no_randomness</a>(amount: u64): <a href="confidential_balance.md#0x1_confidential_balance_Balance">Balance</a>&lt;<a href="confidential_balance.md#0x1_confidential_balance_Pending">Pending</a>&gt; {
    <b>let</b> identity = point_identity();
    <a href="confidential_balance.md#0x1_confidential_balance_new_pending_from_p_and_r">new_pending_from_p_and_r</a>(
        <a href="confidential_balance.md#0x1_confidential_balance_split_into_chunks">split_into_chunks</a>((amount <b>as</b> u128), <a href="confidential_balance.md#0x1_confidential_balance_PENDING_BALANCE_CHUNKS">PENDING_BALANCE_CHUNKS</a>).map(|chunk| chunk.basepoint_mul()),
        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, <a href="confidential_balance.md#0x1_confidential_balance_PENDING_BALANCE_CHUNKS">PENDING_BALANCE_CHUNKS</a>).map(|_| identity.point_clone()),
    )
}
</code></pre>



</details>

<a id="0x1_confidential_balance_add_assign_pending"></a>

## Function `add_assign_pending`

Adds a pending balance to a compressed pending balance in place.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="confidential_balance.md#0x1_confidential_balance_add_assign_pending">add_assign_pending</a>(balance: &<b>mut</b> <a href="confidential_balance.md#0x1_confidential_balance_CompressedBalance">confidential_balance::CompressedBalance</a>&lt;<a href="confidential_balance.md#0x1_confidential_balance_Pending">confidential_balance::Pending</a>&gt;, rhs: &<a href="confidential_balance.md#0x1_confidential_balance_Balance">confidential_balance::Balance</a>&lt;<a href="confidential_balance.md#0x1_confidential_balance_Pending">confidential_balance::Pending</a>&gt;): <a href="confidential_balance.md#0x1_confidential_balance_CompressedBalance">confidential_balance::CompressedBalance</a>&lt;<a href="confidential_balance.md#0x1_confidential_balance_Pending">confidential_balance::Pending</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="confidential_balance.md#0x1_confidential_balance_add_assign_pending">add_assign_pending</a>(balance: &<b>mut</b> <a href="confidential_balance.md#0x1_confidential_balance_CompressedBalance">CompressedBalance</a>&lt;<a href="confidential_balance.md#0x1_confidential_balance_Pending">Pending</a>&gt;, rhs: &<a href="confidential_balance.md#0x1_confidential_balance_Balance">Balance</a>&lt;<a href="confidential_balance.md#0x1_confidential_balance_Pending">Pending</a>&gt;): <a href="confidential_balance.md#0x1_confidential_balance_CompressedBalance">CompressedBalance</a>&lt;<a href="confidential_balance.md#0x1_confidential_balance_Pending">Pending</a>&gt; {
    <b>let</b> decompressed = balance.<a href="confidential_balance.md#0x1_confidential_balance_decompress">decompress</a>();
    decompressed.<a href="confidential_balance.md#0x1_confidential_balance_add_mut_base">add_mut_base</a>(rhs.<a href="confidential_balance.md#0x1_confidential_balance_get_P">get_P</a>(), rhs.<a href="confidential_balance.md#0x1_confidential_balance_get_R">get_R</a>());
    *balance = decompressed.<a href="confidential_balance.md#0x1_confidential_balance_compress">compress</a>();
    *balance
}
</code></pre>



</details>

<a id="0x1_confidential_balance_get_num_pending_chunks"></a>

## Function `get_num_pending_chunks`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="confidential_balance.md#0x1_confidential_balance_get_num_pending_chunks">get_num_pending_chunks</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_balance.md#0x1_confidential_balance_get_num_pending_chunks">get_num_pending_chunks</a>(): u64 { <a href="confidential_balance.md#0x1_confidential_balance_PENDING_BALANCE_CHUNKS">PENDING_BALANCE_CHUNKS</a> }
</code></pre>



</details>

<a id="0x1_confidential_balance_new_available_from_p_r_r_aud"></a>

## Function `new_available_from_p_r_r_aud`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="confidential_balance.md#0x1_confidential_balance_new_available_from_p_r_r_aud">new_available_from_p_r_r_aud</a>(p: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;, r: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;, r_aud: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;): <a href="confidential_balance.md#0x1_confidential_balance_Balance">confidential_balance::Balance</a>&lt;<a href="confidential_balance.md#0x1_confidential_balance_Available">confidential_balance::Available</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="confidential_balance.md#0x1_confidential_balance_new_available_from_p_r_r_aud">new_available_from_p_r_r_aud</a>(
    p: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;RistrettoPoint&gt;, r: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;RistrettoPoint&gt;, r_aud: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;RistrettoPoint&gt;
): <a href="confidential_balance.md#0x1_confidential_balance_Balance">Balance</a>&lt;<a href="confidential_balance.md#0x1_confidential_balance_Available">Available</a>&gt; {
    <a href="confidential_balance.md#0x1_confidential_balance_new_balance">new_balance</a>(p, r, r_aud, <a href="confidential_balance.md#0x1_confidential_balance_AVAILABLE_BALANCE_CHUNKS">AVAILABLE_BALANCE_CHUNKS</a>)
}
</code></pre>



</details>

<a id="0x1_confidential_balance_new_compressed_available_from_p_r_r_aud"></a>

## Function `new_compressed_available_from_p_r_r_aud`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="confidential_balance.md#0x1_confidential_balance_new_compressed_available_from_p_r_r_aud">new_compressed_available_from_p_r_r_aud</a>(p: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;, r: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;, r_aud: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;): <a href="confidential_balance.md#0x1_confidential_balance_CompressedBalance">confidential_balance::CompressedBalance</a>&lt;<a href="confidential_balance.md#0x1_confidential_balance_Available">confidential_balance::Available</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="confidential_balance.md#0x1_confidential_balance_new_compressed_available_from_p_r_r_aud">new_compressed_available_from_p_r_r_aud</a>(
    p: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;CompressedRistretto&gt;, r: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;CompressedRistretto&gt;, r_aud: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;CompressedRistretto&gt;
): <a href="confidential_balance.md#0x1_confidential_balance_CompressedBalance">CompressedBalance</a>&lt;<a href="confidential_balance.md#0x1_confidential_balance_Available">Available</a>&gt; {
    <a href="confidential_balance.md#0x1_confidential_balance_new_compressed_balance">new_compressed_balance</a>(p, r, r_aud, <a href="confidential_balance.md#0x1_confidential_balance_AVAILABLE_BALANCE_CHUNKS">AVAILABLE_BALANCE_CHUNKS</a>)
}
</code></pre>



</details>

<a id="0x1_confidential_balance_new_zero_available_compressed"></a>

## Function `new_zero_available_compressed`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="confidential_balance.md#0x1_confidential_balance_new_zero_available_compressed">new_zero_available_compressed</a>(): <a href="confidential_balance.md#0x1_confidential_balance_CompressedBalance">confidential_balance::CompressedBalance</a>&lt;<a href="confidential_balance.md#0x1_confidential_balance_Available">confidential_balance::Available</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="confidential_balance.md#0x1_confidential_balance_new_zero_available_compressed">new_zero_available_compressed</a>(): <a href="confidential_balance.md#0x1_confidential_balance_CompressedBalance">CompressedBalance</a>&lt;<a href="confidential_balance.md#0x1_confidential_balance_Available">Available</a>&gt; {
    <a href="confidential_balance.md#0x1_confidential_balance_new_zero_compressed">new_zero_compressed</a>(<a href="confidential_balance.md#0x1_confidential_balance_AVAILABLE_BALANCE_CHUNKS">AVAILABLE_BALANCE_CHUNKS</a>)
}
</code></pre>



</details>

<a id="0x1_confidential_balance_new_compressed_available_from_bytes"></a>

## Function `new_compressed_available_from_bytes`

Deserializes raw byte vectors into a CompressedBalance<Available> (without decompressing).


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="confidential_balance.md#0x1_confidential_balance_new_compressed_available_from_bytes">new_compressed_available_from_bytes</a>(p_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, r_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, r_aud_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;): <a href="confidential_balance.md#0x1_confidential_balance_CompressedBalance">confidential_balance::CompressedBalance</a>&lt;<a href="confidential_balance.md#0x1_confidential_balance_Available">confidential_balance::Available</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="confidential_balance.md#0x1_confidential_balance_new_compressed_available_from_bytes">new_compressed_available_from_bytes</a>(
    p_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    r_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    r_aud_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
): <a href="confidential_balance.md#0x1_confidential_balance_CompressedBalance">CompressedBalance</a>&lt;<a href="confidential_balance.md#0x1_confidential_balance_Available">Available</a>&gt; {
    <a href="confidential_balance.md#0x1_confidential_balance_new_compressed_available_from_p_r_r_aud">new_compressed_available_from_p_r_r_aud</a>(
        deserialize_compressed_points(p_bytes),
        deserialize_compressed_points(r_bytes),
        deserialize_compressed_points(r_aud_bytes),
    )
}
</code></pre>



</details>

<a id="0x1_confidential_balance_set_available_R"></a>

## Function `set_available_R`

Sets only the R component (EK component) of a compressed available balance.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="confidential_balance.md#0x1_confidential_balance_set_available_R">set_available_R</a>(balance: &<b>mut</b> <a href="confidential_balance.md#0x1_confidential_balance_CompressedBalance">confidential_balance::CompressedBalance</a>&lt;<a href="confidential_balance.md#0x1_confidential_balance_Available">confidential_balance::Available</a>&gt;, new_R: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="confidential_balance.md#0x1_confidential_balance_set_available_R">set_available_R</a>(balance: &<b>mut</b> <a href="confidential_balance.md#0x1_confidential_balance_CompressedBalance">CompressedBalance</a>&lt;<a href="confidential_balance.md#0x1_confidential_balance_Available">Available</a>&gt;, new_R: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;CompressedRistretto&gt;) {
    <b>assert</b>!(new_R.length() == <a href="confidential_balance.md#0x1_confidential_balance_AVAILABLE_BALANCE_CHUNKS">AVAILABLE_BALANCE_CHUNKS</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="confidential_balance.md#0x1_confidential_balance_E_WRONG_NUM_CHUNKS">E_WRONG_NUM_CHUNKS</a>));
    balance.R = new_R;
}
</code></pre>



</details>

<a id="0x1_confidential_balance_add_assign_available_excluding_auditor"></a>

## Function `add_assign_available_excluding_auditor`

Adds a pending balance to an available balance in place. R_aud is NOT touched (stale after rollover).


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="confidential_balance.md#0x1_confidential_balance_add_assign_available_excluding_auditor">add_assign_available_excluding_auditor</a>(self: &<b>mut</b> <a href="confidential_balance.md#0x1_confidential_balance_CompressedBalance">confidential_balance::CompressedBalance</a>&lt;<a href="confidential_balance.md#0x1_confidential_balance_Available">confidential_balance::Available</a>&gt;, rhs: &<a href="confidential_balance.md#0x1_confidential_balance_CompressedBalance">confidential_balance::CompressedBalance</a>&lt;<a href="confidential_balance.md#0x1_confidential_balance_Pending">confidential_balance::Pending</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="confidential_balance.md#0x1_confidential_balance_add_assign_available_excluding_auditor">add_assign_available_excluding_auditor</a>(self: &<b>mut</b> <a href="confidential_balance.md#0x1_confidential_balance_CompressedBalance">CompressedBalance</a>&lt;<a href="confidential_balance.md#0x1_confidential_balance_Available">Available</a>&gt;, rhs: &<a href="confidential_balance.md#0x1_confidential_balance_CompressedBalance">CompressedBalance</a>&lt;<a href="confidential_balance.md#0x1_confidential_balance_Pending">Pending</a>&gt;) {
    <b>let</b> lhs_P = self.P.map_ref(|p| p.point_decompress());
    <b>let</b> lhs_R = self.R.map_ref(|r| r.point_decompress());
    <b>let</b> rhs_P = rhs.P.map_ref(|p| p.point_decompress());
    <b>let</b> rhs_R = rhs.R.map_ref(|r| r.point_decompress());

    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, rhs_P.length()).for_each(|i| {
        lhs_P[i].point_add_assign(&rhs_P[i]);
        lhs_R[i].point_add_assign(&rhs_R[i]);
    });

    self.P = lhs_P.map_ref(|p| p.point_compress());
    self.R = lhs_R.map_ref(|r| r.point_compress());
}
</code></pre>



</details>

<a id="0x1_confidential_balance_get_num_available_chunks"></a>

## Function `get_num_available_chunks`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="confidential_balance.md#0x1_confidential_balance_get_num_available_chunks">get_num_available_chunks</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_balance.md#0x1_confidential_balance_get_num_available_chunks">get_num_available_chunks</a>(): u64 { <a href="confidential_balance.md#0x1_confidential_balance_AVAILABLE_BALANCE_CHUNKS">AVAILABLE_BALANCE_CHUNKS</a> }
</code></pre>



</details>

<a id="0x1_confidential_balance_get_chunk_size_bits"></a>

## Function `get_chunk_size_bits`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="confidential_balance.md#0x1_confidential_balance_get_chunk_size_bits">get_chunk_size_bits</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="confidential_balance.md#0x1_confidential_balance_get_chunk_size_bits">get_chunk_size_bits</a>(): u64 { <a href="confidential_balance.md#0x1_confidential_balance_CHUNK_SIZE_BITS">CHUNK_SIZE_BITS</a> }
</code></pre>



</details>

<a id="0x1_confidential_balance_get_chunk_upper_bound"></a>

## Function `get_chunk_upper_bound`

Every balance chunk is $<$ than this bound (i.e., $< 2^{16}$).


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="confidential_balance.md#0x1_confidential_balance_get_chunk_upper_bound">get_chunk_upper_bound</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="confidential_balance.md#0x1_confidential_balance_get_chunk_upper_bound">get_chunk_upper_bound</a>(): u64 { <a href="confidential_balance.md#0x1_confidential_balance_CHUNK_UPPER_BOUND">CHUNK_UPPER_BOUND</a> }
</code></pre>



</details>

<a id="0x1_confidential_balance_split_into_chunks"></a>

## Function `split_into_chunks`

Splits <code>amount</code> into <code>num_chunks</code> 16-bit chunks as <code>Scalar</code> values.


<pre><code><b>fun</b> <a href="confidential_balance.md#0x1_confidential_balance_split_into_chunks">split_into_chunks</a>(amount: u128, num_chunks: u64): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="confidential_balance.md#0x1_confidential_balance_split_into_chunks">split_into_chunks</a>(amount: u128, num_chunks: u64): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Scalar&gt; {
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, num_chunks).map(|i| {
        new_scalar_from_u128(amount &gt;&gt; (i * <a href="confidential_balance.md#0x1_confidential_balance_CHUNK_SIZE_BITS">CHUNK_SIZE_BITS</a> <b>as</b> u8) & 0xffff)
    })
}
</code></pre>



</details>

<a id="0x1_confidential_balance_get_b_powers"></a>

## Function `get_b_powers`

Returns [B^0, B^1, ..., B^{count-1}] where B = 2^chunk_size_bits.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="confidential_balance.md#0x1_confidential_balance_get_b_powers">get_b_powers</a>(count: u64): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="confidential_balance.md#0x1_confidential_balance_get_b_powers">get_b_powers</a>(count: u64): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Scalar&gt; {
    <b>let</b> b = new_scalar_from_u128((<a href="confidential_balance.md#0x1_confidential_balance_CHUNK_UPPER_BOUND">CHUNK_UPPER_BOUND</a> <b>as</b> u128));
    <b>let</b> powers = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[scalar_one()];
    <b>let</b> prev = scalar_one();
    for (i in 1..count) {
        prev = prev.scalar_mul(&b);
        powers.push_back(prev);
    };
    powers
}
</code></pre>



</details>

<a id="0x1_confidential_balance_get_encryption_key_basepoint_compressed"></a>

## Function `get_encryption_key_basepoint_compressed`

Returns the compressed generator H used to derive the encryption key as EK = DK^(-1) * H.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="confidential_balance.md#0x1_confidential_balance_get_encryption_key_basepoint_compressed">get_encryption_key_basepoint_compressed</a>(): <a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="confidential_balance.md#0x1_confidential_balance_get_encryption_key_basepoint_compressed">get_encryption_key_basepoint_compressed</a>(): CompressedRistretto {
    <a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_basepoint_H_compressed">ristretto255::basepoint_H_compressed</a>()
}
</code></pre>



</details>

<a id="0x1_confidential_balance_assert_correct_num_chunks"></a>

## Function `assert_correct_num_chunks`



<pre><code><b>fun</b> <a href="confidential_balance.md#0x1_confidential_balance_assert_correct_num_chunks">assert_correct_num_chunks</a>&lt;T&gt;(p: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;T&gt;, r: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;T&gt;, r_aud: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;T&gt;, expected_chunks: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="confidential_balance.md#0x1_confidential_balance_assert_correct_num_chunks">assert_correct_num_chunks</a>&lt;T&gt;(p: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;T&gt;, r: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;T&gt;, r_aud: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;T&gt;, expected_chunks: u64) {
    <b>assert</b>!(p.length() == expected_chunks, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="confidential_balance.md#0x1_confidential_balance_E_WRONG_NUM_CHUNKS">E_WRONG_NUM_CHUNKS</a>));
    <b>assert</b>!(r.length() == expected_chunks, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="confidential_balance.md#0x1_confidential_balance_E_WRONG_NUM_CHUNKS">E_WRONG_NUM_CHUNKS</a>));
    <b>assert</b>!(r_aud.is_empty() || r_aud.length() == expected_chunks, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(
        <a href="confidential_balance.md#0x1_confidential_balance_E_WRONG_NUM_CHUNKS_FOR_AUDITOR">E_WRONG_NUM_CHUNKS_FOR_AUDITOR</a>
    ));
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
