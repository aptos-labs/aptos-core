
<a id="0x7_confidential_pending_balance"></a>

# Module `0x7::confidential_pending_balance`

This module implements a Confidential Pending Balance abstraction, built on top of Twisted ElGamal encryption,
over the Ristretto255 curve.

A pending balance stores encrypted representations of incoming transfers, split into chunks and stored as pairs of
ciphertext components <code>(P_i, R_i)</code> under basepoints <code>G</code> and <code>H</code> and an encryption key <code>EK = dk^(-1) * H</code>, where <code>dk</code>
is the corresponding decryption key. Each pair represents an encrypted value <code>a_i</code> - the <code>i</code>-th 16-bit portion of
the total encrypted amount - and its associated randomness <code>r_i</code>, such that <code>P_i = a_i * G + r_i * H</code> and <code>R_i = r_i * EK</code>.

Pending balances are represented by four ciphertext pairs <code>(P_i, R_i), i = 1..4</code>, suitable for 64-bit values.


-  [Struct `CompressedPendingBalance`](#0x7_confidential_pending_balance_CompressedPendingBalance)
-  [Struct `PendingBalance`](#0x7_confidential_pending_balance_PendingBalance)
-  [Constants](#@Constants_0)
-  [Function `get_P`](#0x7_confidential_pending_balance_get_P)
-  [Function `get_R`](#0x7_confidential_pending_balance_get_R)
-  [Function `get_compressed_P`](#0x7_confidential_pending_balance_get_compressed_P)
-  [Function `get_compressed_R`](#0x7_confidential_pending_balance_get_compressed_R)
-  [Function `new_from_p_and_r`](#0x7_confidential_pending_balance_new_from_p_and_r)
-  [Function `new_compressed_from_p_and_r`](#0x7_confidential_pending_balance_new_compressed_from_p_and_r)
-  [Function `into_p_and_r`](#0x7_confidential_pending_balance_into_p_and_r)
-  [Function `split_into_chunks`](#0x7_confidential_pending_balance_split_into_chunks)
-  [Function `new_zero_compressed`](#0x7_confidential_pending_balance_new_zero_compressed)
-  [Function `new_u64_no_randomness`](#0x7_confidential_pending_balance_new_u64_no_randomness)
-  [Function `new_from_byte_vectors`](#0x7_confidential_pending_balance_new_from_byte_vectors)
-  [Function `compress`](#0x7_confidential_pending_balance_compress)
-  [Function `add_assign`](#0x7_confidential_pending_balance_add_assign)
-  [Function `decompress`](#0x7_confidential_pending_balance_decompress)
-  [Function `add_mut`](#0x7_confidential_pending_balance_add_mut)
-  [Function `is_zero`](#0x7_confidential_pending_balance_is_zero)
-  [Function `get_num_chunks`](#0x7_confidential_pending_balance_get_num_chunks)


<pre><code><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255">0x1::ristretto255</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
</code></pre>



<a id="0x7_confidential_pending_balance_CompressedPendingBalance"></a>

## Struct `CompressedPendingBalance`

Represents a compressed pending balance.
- <code>P[i]</code> is the value component: <code>chunk_i * G + r_i * H</code>
- <code>R[i]</code> is the EK component: <code>r_i * EK</code>


<pre><code><b>struct</b> <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_CompressedPendingBalance">CompressedPendingBalance</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>P: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>R: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x7_confidential_pending_balance_PendingBalance"></a>

## Struct `PendingBalance`

Represents an uncompressed pending balance.
- <code>P[i]</code> is the value component: <code>chunk_i * G + r_i * H</code>
- <code>R[i]</code> is the EK component: <code>r_i * EK</code>


<pre><code><b>struct</b> <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_PendingBalance">PendingBalance</a> <b>has</b> drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>P: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>R: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x7_confidential_pending_balance_EINTERNAL_ERROR"></a>

An internal error occurred, indicating unexpected behavior.


<pre><code><b>const</b> <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_EINTERNAL_ERROR">EINTERNAL_ERROR</a>: u64 = 1;
</code></pre>



<a id="0x7_confidential_pending_balance_PENDING_BALANCE_CHUNKS"></a>

The number of chunks $n$ in a pending balance.


<pre><code><b>const</b> <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_PENDING_BALANCE_CHUNKS">PENDING_BALANCE_CHUNKS</a>: u64 = 4;
</code></pre>



<a id="0x7_confidential_pending_balance_get_P"></a>

## Function `get_P`

Returns a reference to the P components (value components) of a pending balance.


<pre><code><b>public</b> <b>fun</b> <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_get_P">get_P</a>(self: &<a href="confidential_pending_balance.md#0x7_confidential_pending_balance_PendingBalance">confidential_pending_balance::PendingBalance</a>): &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_get_P">get_P</a>(self: &<a href="confidential_pending_balance.md#0x7_confidential_pending_balance_PendingBalance">PendingBalance</a>): &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;RistrettoPoint&gt; {
    &self.P
}
</code></pre>



</details>

<a id="0x7_confidential_pending_balance_get_R"></a>

## Function `get_R`

Returns a reference to the R components (EK components) of a pending balance.


<pre><code><b>public</b> <b>fun</b> <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_get_R">get_R</a>(self: &<a href="confidential_pending_balance.md#0x7_confidential_pending_balance_PendingBalance">confidential_pending_balance::PendingBalance</a>): &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_get_R">get_R</a>(self: &<a href="confidential_pending_balance.md#0x7_confidential_pending_balance_PendingBalance">PendingBalance</a>): &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;RistrettoPoint&gt; {
    &self.R
}
</code></pre>



</details>

<a id="0x7_confidential_pending_balance_get_compressed_P"></a>

## Function `get_compressed_P`

Returns a reference to the P components (value components) of a compressed pending balance.


<pre><code><b>public</b> <b>fun</b> <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_get_compressed_P">get_compressed_P</a>(self: &<a href="confidential_pending_balance.md#0x7_confidential_pending_balance_CompressedPendingBalance">confidential_pending_balance::CompressedPendingBalance</a>): &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_get_compressed_P">get_compressed_P</a>(self: &<a href="confidential_pending_balance.md#0x7_confidential_pending_balance_CompressedPendingBalance">CompressedPendingBalance</a>): &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;CompressedRistretto&gt; {
    &self.P
}
</code></pre>



</details>

<a id="0x7_confidential_pending_balance_get_compressed_R"></a>

## Function `get_compressed_R`

Returns a reference to the R components (EK components) of a compressed pending balance.


<pre><code><b>public</b> <b>fun</b> <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_get_compressed_R">get_compressed_R</a>(self: &<a href="confidential_pending_balance.md#0x7_confidential_pending_balance_CompressedPendingBalance">confidential_pending_balance::CompressedPendingBalance</a>): &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_get_compressed_R">get_compressed_R</a>(self: &<a href="confidential_pending_balance.md#0x7_confidential_pending_balance_CompressedPendingBalance">CompressedPendingBalance</a>): &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;CompressedRistretto&gt; {
    &self.R
}
</code></pre>



</details>

<a id="0x7_confidential_pending_balance_new_from_p_and_r"></a>

## Function `new_from_p_and_r`

Creates a PendingBalance from separate P and R component vectors.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_new_from_p_and_r">new_from_p_and_r</a>(p: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;, r: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;): <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_PendingBalance">confidential_pending_balance::PendingBalance</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_new_from_p_and_r">new_from_p_and_r</a>(p: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;RistrettoPoint&gt;, r: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;RistrettoPoint&gt;): <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_PendingBalance">PendingBalance</a> {
    <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_PendingBalance">PendingBalance</a> { P: p, R: r }
}
</code></pre>



</details>

<a id="0x7_confidential_pending_balance_new_compressed_from_p_and_r"></a>

## Function `new_compressed_from_p_and_r`

Creates a CompressedPendingBalance from separate compressed P and R component vectors.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_new_compressed_from_p_and_r">new_compressed_from_p_and_r</a>(p: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;, r: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;): <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_CompressedPendingBalance">confidential_pending_balance::CompressedPendingBalance</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_new_compressed_from_p_and_r">new_compressed_from_p_and_r</a>(
    p: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;CompressedRistretto&gt;,
    r: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;CompressedRistretto&gt;
): <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_CompressedPendingBalance">CompressedPendingBalance</a> {
    <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_CompressedPendingBalance">CompressedPendingBalance</a> { P: p, R: r }
}
</code></pre>



</details>

<a id="0x7_confidential_pending_balance_into_p_and_r"></a>

## Function `into_p_and_r`

Destructures a PendingBalance into its P and R component vectors.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_into_p_and_r">into_p_and_r</a>(self: <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_PendingBalance">confidential_pending_balance::PendingBalance</a>): (<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_into_p_and_r">into_p_and_r</a>(self: <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_PendingBalance">PendingBalance</a>): (<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;RistrettoPoint&gt;, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;RistrettoPoint&gt;) {
    <b>let</b> <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_PendingBalance">PendingBalance</a> { P: p, R: r } = self;
    (p, r)
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


<pre><code><b>public</b> <b>fun</b> <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_split_into_chunks">split_into_chunks</a>(amount: u128): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>&gt; {
    <b>let</b> chunk_size_bits = <a href="confidential_balance.md#0x7_confidential_balance_get_chunk_size_bits">confidential_balance::get_chunk_size_bits</a>();
    <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_PENDING_BALANCE_CHUNKS">PENDING_BALANCE_CHUNKS</a>).map(|i| {
        <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_new_scalar_from_u128">ristretto255::new_scalar_from_u128</a>(amount &gt;&gt; (i * chunk_size_bits <b>as</b> u8) & 0xffff)
    })
}
</code></pre>



</details>

<a id="0x7_confidential_pending_balance_new_zero_compressed"></a>

## Function `new_zero_compressed`

Creates a new compressed zero pending balance.


<pre><code><b>public</b> <b>fun</b> <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_new_zero_compressed">new_zero_compressed</a>(): <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_CompressedPendingBalance">confidential_pending_balance::CompressedPendingBalance</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_new_zero_compressed">new_zero_compressed</a>(): <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_CompressedPendingBalance">CompressedPendingBalance</a> {
    <b>let</b> identity = <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_identity_compressed">ristretto255::point_identity_compressed</a>();
    <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_CompressedPendingBalance">CompressedPendingBalance</a> {
        P: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_PENDING_BALANCE_CHUNKS">PENDING_BALANCE_CHUNKS</a>).map(|_| identity),
        R: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_PENDING_BALANCE_CHUNKS">PENDING_BALANCE_CHUNKS</a>).map(|_| identity),
    }
}
</code></pre>



</details>

<a id="0x7_confidential_pending_balance_new_u64_no_randomness"></a>

## Function `new_u64_no_randomness`

Creates a new pending balance from a 64-bit amount with no randomness (R components are identity).
Splits the amount into four 16-bit chunks.


<pre><code><b>public</b> <b>fun</b> <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_new_u64_no_randomness">new_u64_no_randomness</a>(amount: u64): <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_PendingBalance">confidential_pending_balance::PendingBalance</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_new_u64_no_randomness">new_u64_no_randomness</a>(amount: u64): <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_PendingBalance">PendingBalance</a> {
    <b>let</b> identity = <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_identity">ristretto255::point_identity</a>();
    <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_PendingBalance">PendingBalance</a> {
        P: <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_split_into_chunks">split_into_chunks</a>((amount <b>as</b> u128)).map(|chunk| chunk.basepoint_mul()),
        R: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_PENDING_BALANCE_CHUNKS">PENDING_BALANCE_CHUNKS</a>).map(|_| identity.point_clone()),
    }
}
</code></pre>



</details>

<a id="0x7_confidential_pending_balance_new_from_byte_vectors"></a>

## Function `new_from_byte_vectors`

Creates a new pending balance from separate P and R byte vectors.
Each element in <code>p_bytes</code> and <code>r_bytes</code> is a 32-byte compressed Ristretto point.
Aborts if any point fails to deserialize or if vector lengths are inconsistent.


<pre><code><b>public</b> <b>fun</b> <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_new_from_byte_vectors">new_from_byte_vectors</a>(p_bytes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, r_bytes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;): <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_PendingBalance">confidential_pending_balance::PendingBalance</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_new_from_byte_vectors">new_from_byte_vectors</a>(
    p_bytes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    r_bytes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
): <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_PendingBalance">PendingBalance</a> {
    <b>assert</b>!(p_bytes.length() == r_bytes.length());

    <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_PendingBalance">PendingBalance</a> {
        P: p_bytes.map(|bytes| <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_new_point_from_bytes">ristretto255::new_point_from_bytes</a>(bytes).extract()),
        R: r_bytes.map(|bytes| <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_new_point_from_bytes">ristretto255::new_point_from_bytes</a>(bytes).extract()),
    }
}
</code></pre>



</details>

<a id="0x7_confidential_pending_balance_compress"></a>

## Function `compress`

Compresses a pending balance into its <code><a href="confidential_pending_balance.md#0x7_confidential_pending_balance_CompressedPendingBalance">CompressedPendingBalance</a></code> representation.


<pre><code><b>public</b> <b>fun</b> <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_compress">compress</a>(self: &<a href="confidential_pending_balance.md#0x7_confidential_pending_balance_PendingBalance">confidential_pending_balance::PendingBalance</a>): <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_CompressedPendingBalance">confidential_pending_balance::CompressedPendingBalance</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_compress">compress</a>(self: &<a href="confidential_pending_balance.md#0x7_confidential_pending_balance_PendingBalance">PendingBalance</a>): <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_CompressedPendingBalance">CompressedPendingBalance</a> {
    <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_CompressedPendingBalance">CompressedPendingBalance</a> {
        P: self.P.map_ref(|p| p.point_compress()),
        R: self.R.map_ref(|r| r.point_compress()),
    }
}
</code></pre>



</details>

<a id="0x7_confidential_pending_balance_add_assign"></a>

## Function `add_assign`

Adds a pending balance to this compressed pending balance in place.
Decompresses, adds, and recompresses internally.


<pre><code><b>public</b> <b>fun</b> <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_add_assign">add_assign</a>(self: &<b>mut</b> <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_CompressedPendingBalance">confidential_pending_balance::CompressedPendingBalance</a>, rhs: &<a href="confidential_pending_balance.md#0x7_confidential_pending_balance_PendingBalance">confidential_pending_balance::PendingBalance</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_add_assign">add_assign</a>(self: &<b>mut</b> <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_CompressedPendingBalance">CompressedPendingBalance</a>, rhs: &<a href="confidential_pending_balance.md#0x7_confidential_pending_balance_PendingBalance">PendingBalance</a>) {
    <b>let</b> decompressed = self.<a href="confidential_pending_balance.md#0x7_confidential_pending_balance_decompress">decompress</a>();
    decompressed.<a href="confidential_pending_balance.md#0x7_confidential_pending_balance_add_mut">add_mut</a>(rhs);
    *self = decompressed.<a href="confidential_pending_balance.md#0x7_confidential_pending_balance_compress">compress</a>();
}
</code></pre>



</details>

<a id="0x7_confidential_pending_balance_decompress"></a>

## Function `decompress`

Decompresses a compressed pending balance into its <code><a href="confidential_pending_balance.md#0x7_confidential_pending_balance_PendingBalance">PendingBalance</a></code> representation.


<pre><code><b>public</b> <b>fun</b> <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_decompress">decompress</a>(self: &<a href="confidential_pending_balance.md#0x7_confidential_pending_balance_CompressedPendingBalance">confidential_pending_balance::CompressedPendingBalance</a>): <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_PendingBalance">confidential_pending_balance::PendingBalance</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_decompress">decompress</a>(self: &<a href="confidential_pending_balance.md#0x7_confidential_pending_balance_CompressedPendingBalance">CompressedPendingBalance</a>): <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_PendingBalance">PendingBalance</a> {
    <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_PendingBalance">PendingBalance</a> {
        P: self.P.map_ref(|p| p.point_decompress()),
        R: self.R.map_ref(|r| r.point_decompress()),
    }
}
</code></pre>



</details>

<a id="0x7_confidential_pending_balance_add_mut"></a>

## Function `add_mut`

Adds two pending balances homomorphically, mutating the first balance in place.
The second balance must have fewer or equal chunks compared to the first.


<pre><code><b>public</b> <b>fun</b> <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_add_mut">add_mut</a>(self: &<b>mut</b> <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_PendingBalance">confidential_pending_balance::PendingBalance</a>, rhs: &<a href="confidential_pending_balance.md#0x7_confidential_pending_balance_PendingBalance">confidential_pending_balance::PendingBalance</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_add_mut">add_mut</a>(self: &<b>mut</b> <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_PendingBalance">PendingBalance</a>, rhs: &<a href="confidential_pending_balance.md#0x7_confidential_pending_balance_PendingBalance">PendingBalance</a>) {
    <b>assert</b>!(self.P.length() &gt;= rhs.P.length(), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_internal">error::internal</a>(<a href="confidential_pending_balance.md#0x7_confidential_pending_balance_EINTERNAL_ERROR">EINTERNAL_ERROR</a>));

    <b>let</b> i = 0;
    <b>let</b> rhs_len = rhs.P.length();
    <b>while</b> (i &lt; rhs_len) {
        self.P[i].point_add_assign(&rhs.P[i]);
        self.R[i].point_add_assign(&rhs.R[i]);
        i = i + 1;
    };
}
</code></pre>



</details>

<a id="0x7_confidential_pending_balance_is_zero"></a>

## Function `is_zero`

Checks if a compressed pending balance is equivalent to zero (all P and R are identity).


<pre><code><b>public</b> <b>fun</b> <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_is_zero">is_zero</a>(self: &<a href="confidential_pending_balance.md#0x7_confidential_pending_balance_CompressedPendingBalance">confidential_pending_balance::CompressedPendingBalance</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_is_zero">is_zero</a>(self: &<a href="confidential_pending_balance.md#0x7_confidential_pending_balance_CompressedPendingBalance">CompressedPendingBalance</a>): bool {
    self.P.all(|p| p.is_identity()) &&
    self.R.all(|r| r.is_identity())
}
</code></pre>



</details>

<a id="0x7_confidential_pending_balance_get_num_chunks"></a>

## Function `get_num_chunks`

Returns the number of chunks in a pending balance.


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
