
<a id="0x7_confidential_available_balance"></a>

# Module `0x7::confidential_available_balance`

This module implements a Confidential Available Balance abstraction, built on top of Twisted ElGamal encryption,
over the Ristretto255 curve.

An available balance stores the user's spendable balance, split into chunks and stored as triples of
ciphertext components <code>(P_i, R_i, R_aud_i)</code> under basepoints <code>G</code> and <code>H</code> and an encryption key <code>EK = dk^(-1) * H</code>,
where <code>dk</code> is the corresponding decryption key. Each triple represents an encrypted value <code>a_i</code> - the <code>i</code>-th 16-bit
portion of the total encrypted amount - and its associated randomness <code>r_i</code>, such that:
<code>P_i = a_i * G + r_i * H</code>
<code>R_i = r_i * EK</code>
<code>R_aud_i = r_i * EK_auditor</code> (if an auditor is set; empty otherwise)

The R_aud component allows an auditor to decrypt the available balance. After rollover, R_aud becomes stale
(since pending balances have no R_aud); it's refreshed by withdraw/transfer/normalize (which produce
fresh AvailableBalance with new R_aud).

Available balances are represented by eight ciphertext pairs/triples, supporting 128-bit values.


-  [Struct `CompressedAvailableBalance`](#0x7_confidential_available_balance_CompressedAvailableBalance)
-  [Struct `AvailableBalance`](#0x7_confidential_available_balance_AvailableBalance)
-  [Constants](#@Constants_0)
-  [Function `get_P`](#0x7_confidential_available_balance_get_P)
-  [Function `get_R`](#0x7_confidential_available_balance_get_R)
-  [Function `get_R_aud`](#0x7_confidential_available_balance_get_R_aud)
-  [Function `get_compressed_P`](#0x7_confidential_available_balance_get_compressed_P)
-  [Function `get_compressed_R`](#0x7_confidential_available_balance_get_compressed_R)
-  [Function `get_compressed_R_aud`](#0x7_confidential_available_balance_get_compressed_R_aud)
-  [Function `set_compressed_R`](#0x7_confidential_available_balance_set_compressed_R)
-  [Function `new_from_p_r_r_aud`](#0x7_confidential_available_balance_new_from_p_r_r_aud)
-  [Function `new_compressed_from_p_r_r_aud`](#0x7_confidential_available_balance_new_compressed_from_p_r_r_aud)
-  [Function `new_zero_compressed`](#0x7_confidential_available_balance_new_zero_compressed)
-  [Function `new_from_byte_vectors`](#0x7_confidential_available_balance_new_from_byte_vectors)
-  [Function `compress`](#0x7_confidential_available_balance_compress)
-  [Function `decompress`](#0x7_confidential_available_balance_decompress)
-  [Function `add_assign`](#0x7_confidential_available_balance_add_assign)
-  [Function `split_into_chunks`](#0x7_confidential_available_balance_split_into_chunks)
-  [Function `get_num_chunks`](#0x7_confidential_available_balance_get_num_chunks)


<pre><code><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255">0x1::ristretto255</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
<b>use</b> <a href="confidential_pending_balance.md#0x7_confidential_pending_balance">0x7::confidential_pending_balance</a>;
</code></pre>



<a id="0x7_confidential_available_balance_CompressedAvailableBalance"></a>

## Struct `CompressedAvailableBalance`

Represents a compressed available balance.
- <code>P[i]</code> is the value component: <code>chunk_i * G + r_i * H</code>
- <code>R[i]</code> is the EK component: <code>r_i * EK</code>
- <code>R_aud[i]</code> is the auditor component: <code>r_i * EK_auditor</code> (empty vector if no auditor)


<pre><code><b>struct</b> <a href="confidential_available_balance.md#0x7_confidential_available_balance_CompressedAvailableBalance">CompressedAvailableBalance</a> <b>has</b> <b>copy</b>, drop, store
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
<dt>
<code>R_aud: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x7_confidential_available_balance_AvailableBalance"></a>

## Struct `AvailableBalance`

Represents an uncompressed available balance.
- <code>P[i]</code> is the value component: <code>chunk_i * G + r_i * H</code>
- <code>R[i]</code> is the EK component: <code>r_i * EK</code>
- <code>R_aud[i]</code> is the auditor component: <code>r_i * EK_auditor</code> (empty vector if no auditor)


<pre><code><b>struct</b> <a href="confidential_available_balance.md#0x7_confidential_available_balance_AvailableBalance">AvailableBalance</a> <b>has</b> drop
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
<dt>
<code>R_aud: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x7_confidential_available_balance_AVAILABLE_BALANCE_CHUNKS"></a>

The number of chunks $\ell$ in an available balance.


<pre><code><b>const</b> <a href="confidential_available_balance.md#0x7_confidential_available_balance_AVAILABLE_BALANCE_CHUNKS">AVAILABLE_BALANCE_CHUNKS</a>: u64 = 8;
</code></pre>



<a id="0x7_confidential_available_balance_get_P"></a>

## Function `get_P`

Returns a reference to the P components (value components) of an available balance.


<pre><code><b>public</b> <b>fun</b> <a href="confidential_available_balance.md#0x7_confidential_available_balance_get_P">get_P</a>(self: &<a href="confidential_available_balance.md#0x7_confidential_available_balance_AvailableBalance">confidential_available_balance::AvailableBalance</a>): &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_available_balance.md#0x7_confidential_available_balance_get_P">get_P</a>(self: &<a href="confidential_available_balance.md#0x7_confidential_available_balance_AvailableBalance">AvailableBalance</a>): &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;RistrettoPoint&gt; {
    &self.P
}
</code></pre>



</details>

<a id="0x7_confidential_available_balance_get_R"></a>

## Function `get_R`

Returns a reference to the R components (EK components) of an available balance.


<pre><code><b>public</b> <b>fun</b> <a href="confidential_available_balance.md#0x7_confidential_available_balance_get_R">get_R</a>(self: &<a href="confidential_available_balance.md#0x7_confidential_available_balance_AvailableBalance">confidential_available_balance::AvailableBalance</a>): &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_available_balance.md#0x7_confidential_available_balance_get_R">get_R</a>(self: &<a href="confidential_available_balance.md#0x7_confidential_available_balance_AvailableBalance">AvailableBalance</a>): &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;RistrettoPoint&gt; {
    &self.R
}
</code></pre>



</details>

<a id="0x7_confidential_available_balance_get_R_aud"></a>

## Function `get_R_aud`

Returns a reference to the R_aud components (auditor components) of an available balance.


<pre><code><b>public</b> <b>fun</b> <a href="confidential_available_balance.md#0x7_confidential_available_balance_get_R_aud">get_R_aud</a>(self: &<a href="confidential_available_balance.md#0x7_confidential_available_balance_AvailableBalance">confidential_available_balance::AvailableBalance</a>): &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_available_balance.md#0x7_confidential_available_balance_get_R_aud">get_R_aud</a>(self: &<a href="confidential_available_balance.md#0x7_confidential_available_balance_AvailableBalance">AvailableBalance</a>): &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;RistrettoPoint&gt; {
    &self.R_aud
}
</code></pre>



</details>

<a id="0x7_confidential_available_balance_get_compressed_P"></a>

## Function `get_compressed_P`

Returns a reference to the P components (value components) of a compressed available balance.


<pre><code><b>public</b> <b>fun</b> <a href="confidential_available_balance.md#0x7_confidential_available_balance_get_compressed_P">get_compressed_P</a>(self: &<a href="confidential_available_balance.md#0x7_confidential_available_balance_CompressedAvailableBalance">confidential_available_balance::CompressedAvailableBalance</a>): &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_available_balance.md#0x7_confidential_available_balance_get_compressed_P">get_compressed_P</a>(self: &<a href="confidential_available_balance.md#0x7_confidential_available_balance_CompressedAvailableBalance">CompressedAvailableBalance</a>): &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;CompressedRistretto&gt; {
    &self.P
}
</code></pre>



</details>

<a id="0x7_confidential_available_balance_get_compressed_R"></a>

## Function `get_compressed_R`

Returns a reference to the R components (EK components) of a compressed available balance.


<pre><code><b>public</b> <b>fun</b> <a href="confidential_available_balance.md#0x7_confidential_available_balance_get_compressed_R">get_compressed_R</a>(self: &<a href="confidential_available_balance.md#0x7_confidential_available_balance_CompressedAvailableBalance">confidential_available_balance::CompressedAvailableBalance</a>): &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_available_balance.md#0x7_confidential_available_balance_get_compressed_R">get_compressed_R</a>(self: &<a href="confidential_available_balance.md#0x7_confidential_available_balance_CompressedAvailableBalance">CompressedAvailableBalance</a>): &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;CompressedRistretto&gt; {
    &self.R
}
</code></pre>



</details>

<a id="0x7_confidential_available_balance_get_compressed_R_aud"></a>

## Function `get_compressed_R_aud`

Returns a reference to the R_aud components (auditor components) of a compressed available balance.


<pre><code><b>public</b> <b>fun</b> <a href="confidential_available_balance.md#0x7_confidential_available_balance_get_compressed_R_aud">get_compressed_R_aud</a>(self: &<a href="confidential_available_balance.md#0x7_confidential_available_balance_CompressedAvailableBalance">confidential_available_balance::CompressedAvailableBalance</a>): &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_available_balance.md#0x7_confidential_available_balance_get_compressed_R_aud">get_compressed_R_aud</a>(self: &<a href="confidential_available_balance.md#0x7_confidential_available_balance_CompressedAvailableBalance">CompressedAvailableBalance</a>): &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;CompressedRistretto&gt; {
    &self.R_aud
}
</code></pre>



</details>

<a id="0x7_confidential_available_balance_set_compressed_R"></a>

## Function `set_compressed_R`

Sets the R components (EK components) of a compressed available balance.


<pre><code><b>public</b> <b>fun</b> <a href="confidential_available_balance.md#0x7_confidential_available_balance_set_compressed_R">set_compressed_R</a>(self: &<b>mut</b> <a href="confidential_available_balance.md#0x7_confidential_available_balance_CompressedAvailableBalance">confidential_available_balance::CompressedAvailableBalance</a>, new_R: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_available_balance.md#0x7_confidential_available_balance_set_compressed_R">set_compressed_R</a>(self: &<b>mut</b> <a href="confidential_available_balance.md#0x7_confidential_available_balance_CompressedAvailableBalance">CompressedAvailableBalance</a>, new_R: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;CompressedRistretto&gt;) {
    self.R = new_R;
}
</code></pre>



</details>

<a id="0x7_confidential_available_balance_new_from_p_r_r_aud"></a>

## Function `new_from_p_r_r_aud`

Creates an AvailableBalance from separate P, R, and R_aud component vectors.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="confidential_available_balance.md#0x7_confidential_available_balance_new_from_p_r_r_aud">new_from_p_r_r_aud</a>(p: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;, r: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;, r_aud: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;): <a href="confidential_available_balance.md#0x7_confidential_available_balance_AvailableBalance">confidential_available_balance::AvailableBalance</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="confidential_available_balance.md#0x7_confidential_available_balance_new_from_p_r_r_aud">new_from_p_r_r_aud</a>(
    p: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;RistrettoPoint&gt;,
    r: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;RistrettoPoint&gt;,
    r_aud: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;RistrettoPoint&gt;
): <a href="confidential_available_balance.md#0x7_confidential_available_balance_AvailableBalance">AvailableBalance</a> {
    <a href="confidential_available_balance.md#0x7_confidential_available_balance_AvailableBalance">AvailableBalance</a> { P: p, R: r, R_aud: r_aud }
}
</code></pre>



</details>

<a id="0x7_confidential_available_balance_new_compressed_from_p_r_r_aud"></a>

## Function `new_compressed_from_p_r_r_aud`

Creates a CompressedAvailableBalance from separate compressed P, R, and R_aud component vectors.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="confidential_available_balance.md#0x7_confidential_available_balance_new_compressed_from_p_r_r_aud">new_compressed_from_p_r_r_aud</a>(p: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;, r: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;, r_aud: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;): <a href="confidential_available_balance.md#0x7_confidential_available_balance_CompressedAvailableBalance">confidential_available_balance::CompressedAvailableBalance</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="confidential_available_balance.md#0x7_confidential_available_balance_new_compressed_from_p_r_r_aud">new_compressed_from_p_r_r_aud</a>(
    p: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;CompressedRistretto&gt;,
    r: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;CompressedRistretto&gt;,
    r_aud: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;CompressedRistretto&gt;
): <a href="confidential_available_balance.md#0x7_confidential_available_balance_CompressedAvailableBalance">CompressedAvailableBalance</a> {
    <a href="confidential_available_balance.md#0x7_confidential_available_balance_CompressedAvailableBalance">CompressedAvailableBalance</a> { P: p, R: r, R_aud: r_aud }
}
</code></pre>



</details>

<a id="0x7_confidential_available_balance_new_zero_compressed"></a>

## Function `new_zero_compressed`

Creates a new compressed zero available balance (R_aud = empty, since no auditor for zero balance).


<pre><code><b>public</b> <b>fun</b> <a href="confidential_available_balance.md#0x7_confidential_available_balance_new_zero_compressed">new_zero_compressed</a>(): <a href="confidential_available_balance.md#0x7_confidential_available_balance_CompressedAvailableBalance">confidential_available_balance::CompressedAvailableBalance</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_available_balance.md#0x7_confidential_available_balance_new_zero_compressed">new_zero_compressed</a>(): <a href="confidential_available_balance.md#0x7_confidential_available_balance_CompressedAvailableBalance">CompressedAvailableBalance</a> {
    <b>let</b> identity = <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_identity_compressed">ristretto255::point_identity_compressed</a>();
    <a href="confidential_available_balance.md#0x7_confidential_available_balance_CompressedAvailableBalance">CompressedAvailableBalance</a> {
        P: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, <a href="confidential_available_balance.md#0x7_confidential_available_balance_AVAILABLE_BALANCE_CHUNKS">AVAILABLE_BALANCE_CHUNKS</a>).map(|_| identity),
        R: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, <a href="confidential_available_balance.md#0x7_confidential_available_balance_AVAILABLE_BALANCE_CHUNKS">AVAILABLE_BALANCE_CHUNKS</a>).map(|_| identity),
        R_aud: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[],
    }
}
</code></pre>



</details>

<a id="0x7_confidential_available_balance_new_from_byte_vectors"></a>

## Function `new_from_byte_vectors`

Creates a new available balance from separate P, R, and R_aud byte vectors.
Each element in <code>p_bytes</code>, <code>r_bytes</code>, and <code>r_aud_bytes</code> is a 32-byte compressed Ristretto point.
<code>r_aud_bytes</code> may be empty (no auditor) or must have the same length as <code>p_bytes</code>/<code>r_bytes</code>.
Aborts if any point fails to deserialize or if vector lengths are inconsistent.


<pre><code><b>public</b> <b>fun</b> <a href="confidential_available_balance.md#0x7_confidential_available_balance_new_from_byte_vectors">new_from_byte_vectors</a>(p_bytes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, r_bytes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, r_aud_bytes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;): <a href="confidential_available_balance.md#0x7_confidential_available_balance_AvailableBalance">confidential_available_balance::AvailableBalance</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_available_balance.md#0x7_confidential_available_balance_new_from_byte_vectors">new_from_byte_vectors</a>(
    p_bytes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    r_bytes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    r_aud_bytes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
): <a href="confidential_available_balance.md#0x7_confidential_available_balance_AvailableBalance">AvailableBalance</a> {
    <b>assert</b>!(p_bytes.length() == r_bytes.length());
    <b>assert</b>!(r_aud_bytes.length() == 0 || r_aud_bytes.length() == p_bytes.length());

    <a href="confidential_available_balance.md#0x7_confidential_available_balance_AvailableBalance">AvailableBalance</a> {
        P: p_bytes.map(|bytes| <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_new_point_from_bytes">ristretto255::new_point_from_bytes</a>(bytes).extract()),
        R: r_bytes.map(|bytes| <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_new_point_from_bytes">ristretto255::new_point_from_bytes</a>(bytes).extract()),
        R_aud: r_aud_bytes.map(|bytes| <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_new_point_from_bytes">ristretto255::new_point_from_bytes</a>(bytes).extract()),
    }
}
</code></pre>



</details>

<a id="0x7_confidential_available_balance_compress"></a>

## Function `compress`

Compresses an available balance into its <code><a href="confidential_available_balance.md#0x7_confidential_available_balance_CompressedAvailableBalance">CompressedAvailableBalance</a></code> representation.


<pre><code><b>public</b> <b>fun</b> <a href="confidential_available_balance.md#0x7_confidential_available_balance_compress">compress</a>(self: &<a href="confidential_available_balance.md#0x7_confidential_available_balance_AvailableBalance">confidential_available_balance::AvailableBalance</a>): <a href="confidential_available_balance.md#0x7_confidential_available_balance_CompressedAvailableBalance">confidential_available_balance::CompressedAvailableBalance</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_available_balance.md#0x7_confidential_available_balance_compress">compress</a>(self: &<a href="confidential_available_balance.md#0x7_confidential_available_balance_AvailableBalance">AvailableBalance</a>): <a href="confidential_available_balance.md#0x7_confidential_available_balance_CompressedAvailableBalance">CompressedAvailableBalance</a> {
    <a href="confidential_available_balance.md#0x7_confidential_available_balance_CompressedAvailableBalance">CompressedAvailableBalance</a> {
        P: self.P.map_ref(|p| p.point_compress()),
        R: self.R.map_ref(|r| r.point_compress()),
        R_aud: self.R_aud.map_ref(|r_aud| r_aud.point_compress()),
    }
}
</code></pre>



</details>

<a id="0x7_confidential_available_balance_decompress"></a>

## Function `decompress`

Decompresses a compressed available balance into its <code><a href="confidential_available_balance.md#0x7_confidential_available_balance_AvailableBalance">AvailableBalance</a></code> representation.


<pre><code><b>public</b> <b>fun</b> <a href="confidential_available_balance.md#0x7_confidential_available_balance_decompress">decompress</a>(self: &<a href="confidential_available_balance.md#0x7_confidential_available_balance_CompressedAvailableBalance">confidential_available_balance::CompressedAvailableBalance</a>): <a href="confidential_available_balance.md#0x7_confidential_available_balance_AvailableBalance">confidential_available_balance::AvailableBalance</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_available_balance.md#0x7_confidential_available_balance_decompress">decompress</a>(self: &<a href="confidential_available_balance.md#0x7_confidential_available_balance_CompressedAvailableBalance">CompressedAvailableBalance</a>): <a href="confidential_available_balance.md#0x7_confidential_available_balance_AvailableBalance">AvailableBalance</a> {
    <a href="confidential_available_balance.md#0x7_confidential_available_balance_AvailableBalance">AvailableBalance</a> {
        P: self.P.map_ref(|p| p.point_decompress()),
        R: self.R.map_ref(|r| r.point_decompress()),
        R_aud: self.R_aud.map_ref(|r_aud| r_aud.point_decompress()),
    }
}
</code></pre>



</details>

<a id="0x7_confidential_available_balance_add_assign"></a>

## Function `add_assign`

Adds a compressed pending balance to this compressed available balance in place.
Decompresses both, adds the pending balance's P and R components, and recompresses.
The R_aud components remain unchanged (stale after rollover; refreshed by normalize/withdraw/transfer).


<pre><code><b>public</b> <b>fun</b> <a href="confidential_available_balance.md#0x7_confidential_available_balance_add_assign">add_assign</a>(self: &<b>mut</b> <a href="confidential_available_balance.md#0x7_confidential_available_balance_CompressedAvailableBalance">confidential_available_balance::CompressedAvailableBalance</a>, rhs: &<a href="confidential_pending_balance.md#0x7_confidential_pending_balance_CompressedPendingBalance">confidential_pending_balance::CompressedPendingBalance</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_available_balance.md#0x7_confidential_available_balance_add_assign">add_assign</a>(self: &<b>mut</b> <a href="confidential_available_balance.md#0x7_confidential_available_balance_CompressedAvailableBalance">CompressedAvailableBalance</a>, rhs: &CompressedPendingBalance) {
    <b>let</b> decompressed_self = self.<a href="confidential_available_balance.md#0x7_confidential_available_balance_decompress">decompress</a>();
    <b>let</b> decompressed_rhs = rhs.<a href="confidential_available_balance.md#0x7_confidential_available_balance_decompress">decompress</a>();

    <b>let</b> rhs_P = decompressed_rhs.<a href="confidential_available_balance.md#0x7_confidential_available_balance_get_P">get_P</a>();
    <b>let</b> rhs_R = decompressed_rhs.<a href="confidential_available_balance.md#0x7_confidential_available_balance_get_R">get_R</a>();
    <b>let</b> rhs_len = rhs_P.length();

    <b>let</b> i = 0;
    <b>while</b> (i &lt; rhs_len) {
        decompressed_self.P[i].point_add_assign(&rhs_P[i]);
        decompressed_self.R[i].point_add_assign(&rhs_R[i]);
        i = i + 1;
    };
    // Note: R_aud components are NOT modified. They become stale after rollover.
    *self = decompressed_self.<a href="confidential_available_balance.md#0x7_confidential_available_balance_compress">compress</a>();
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


<pre><code><b>public</b> <b>fun</b> <a href="confidential_available_balance.md#0x7_confidential_available_balance_split_into_chunks">split_into_chunks</a>(amount: u128): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>&gt; {
    <b>let</b> chunk_size_bits = <a href="confidential_balance.md#0x7_confidential_balance_get_chunk_size_bits">confidential_balance::get_chunk_size_bits</a>();
    <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, <a href="confidential_available_balance.md#0x7_confidential_available_balance_AVAILABLE_BALANCE_CHUNKS">AVAILABLE_BALANCE_CHUNKS</a>).map(|i| {
        <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_new_scalar_from_u128">ristretto255::new_scalar_from_u128</a>(amount &gt;&gt; (i * chunk_size_bits <b>as</b> u8) & 0xffff)
    })
}
</code></pre>



</details>

<a id="0x7_confidential_available_balance_get_num_chunks"></a>

## Function `get_num_chunks`

Returns the number of chunks in an available balance.


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
