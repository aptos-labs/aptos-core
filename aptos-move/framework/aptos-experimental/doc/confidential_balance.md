
<a id="0x7_confidential_balance"></a>

# Module `0x7::confidential_balance`

This module implements a Confidential Balance abstraction, built on top of Twisted ElGamal encryption,
over the Ristretto255 curve.

The Confidential Balance encapsulates encrypted representations of a balance, split into chunks and stored as pairs of
ciphertext components <code>(C_i, D_i)</code> under basepoints <code>G</code> and <code>H</code> and an encryption key <code>EK = dk^(-1) * H</code>, where <code>dk</code>
is the corresponding decryption key. Each pair represents an encrypted value <code>a_i</code> - the <code>i</code>-th 16-bit portion of
the total encrypted amount - and its associated randomness <code>r_i</code>, such that <code>C_i = a_i * G + r_i * H</code> and <code>D_i = r_i * EK</code>.

The module supports two types of balances:
- Pending balances are represented by four ciphertext pairs <code>(C_i, D_i), i = 1..4</code>, suitable for 64-bit values.
- Available balances are represented by eight ciphertext pairs <code>(C_i, D_i), i = 1..8</code>, capable of handling 128-bit values.

This implementation leverages the homomorphic properties of Twisted ElGamal encryption to allow arithmetic operations
directly on encrypted data.


-  [Struct `CompressedConfidentialBalance`](#0x7_confidential_balance_CompressedConfidentialBalance)
-  [Struct `ConfidentialBalance`](#0x7_confidential_balance_ConfidentialBalance)
-  [Constants](#@Constants_0)
-  [Function `get_C`](#0x7_confidential_balance_get_C)
-  [Function `get_D`](#0x7_confidential_balance_get_D)
-  [Function `get_compressed_C`](#0x7_confidential_balance_get_compressed_C)
-  [Function `get_compressed_D`](#0x7_confidential_balance_get_compressed_D)
-  [Function `set_compressed_D`](#0x7_confidential_balance_set_compressed_D)
-  [Function `new_compressed_zero_balance`](#0x7_confidential_balance_new_compressed_zero_balance)
-  [Function `new_pending_balance_u64_no_randomness`](#0x7_confidential_balance_new_pending_balance_u64_no_randomness)
-  [Function `new_balance_from_bytes`](#0x7_confidential_balance_new_balance_from_bytes)
-  [Function `compress`](#0x7_confidential_balance_compress)
-  [Function `decompress`](#0x7_confidential_balance_decompress)
-  [Function `balance_to_bytes`](#0x7_confidential_balance_balance_to_bytes)
-  [Function `add_balances_mut`](#0x7_confidential_balance_add_balances_mut)
-  [Function `balance_c_equals`](#0x7_confidential_balance_balance_c_equals)
-  [Function `is_zero_balance`](#0x7_confidential_balance_is_zero_balance)
-  [Function `split_into_chunks`](#0x7_confidential_balance_split_into_chunks)
-  [Function `get_num_pending_chunks`](#0x7_confidential_balance_get_num_pending_chunks)
-  [Function `get_num_available_chunks`](#0x7_confidential_balance_get_num_available_chunks)


<pre><code><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255">0x1::ristretto255</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
</code></pre>



<a id="0x7_confidential_balance_CompressedConfidentialBalance"></a>

## Struct `CompressedConfidentialBalance`

Represents a compressed confidential balance.
- <code>C[i]</code> is the value component: <code>chunk_i * G + r_i * H</code>
- <code>D[i]</code> is the EK component: <code>r_i * EK</code>


<pre><code><b>struct</b> <a href="confidential_balance.md#0x7_confidential_balance_CompressedConfidentialBalance">CompressedConfidentialBalance</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>C: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>D: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x7_confidential_balance_ConfidentialBalance"></a>

## Struct `ConfidentialBalance`

Represents an uncompressed confidential balance.
- <code>C[i]</code> is the value component: <code>chunk_i * G + r_i * H</code>
- <code>D[i]</code> is the EK component: <code>r_i * EK</code>


<pre><code><b>struct</b> <a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">ConfidentialBalance</a> <b>has</b> drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>C: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>D: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x7_confidential_balance_AVAILABLE_BALANCE_CHUNKS"></a>

The number of chunks $\ell$ in an available balance.


<pre><code><b>const</b> <a href="confidential_balance.md#0x7_confidential_balance_AVAILABLE_BALANCE_CHUNKS">AVAILABLE_BALANCE_CHUNKS</a>: u64 = 8;
</code></pre>



<a id="0x7_confidential_balance_CHUNK_SIZE_BITS"></a>

The number of bits $b$ in a single chunk.


<pre><code><b>const</b> <a href="confidential_balance.md#0x7_confidential_balance_CHUNK_SIZE_BITS">CHUNK_SIZE_BITS</a>: u64 = 16;
</code></pre>



<a id="0x7_confidential_balance_EINTERNAL_ERROR"></a>

An internal error occurred, indicating unexpected behavior.


<pre><code><b>const</b> <a href="confidential_balance.md#0x7_confidential_balance_EINTERNAL_ERROR">EINTERNAL_ERROR</a>: u64 = 1;
</code></pre>



<a id="0x7_confidential_balance_PENDING_BALANCE_CHUNKS"></a>

The number of chunks $n$ in a pending balance.


<pre><code><b>const</b> <a href="confidential_balance.md#0x7_confidential_balance_PENDING_BALANCE_CHUNKS">PENDING_BALANCE_CHUNKS</a>: u64 = 4;
</code></pre>



<a id="0x7_confidential_balance_get_C"></a>

## Function `get_C`

Returns a reference to the C components (value components) of a confidential balance.


<pre><code><b>public</b> <b>fun</b> <a href="confidential_balance.md#0x7_confidential_balance_get_C">get_C</a>(self: &<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">confidential_balance::ConfidentialBalance</a>): &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_balance.md#0x7_confidential_balance_get_C">get_C</a>(self: &<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">ConfidentialBalance</a>): &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;RistrettoPoint&gt; {
    &self.C
}
</code></pre>



</details>

<a id="0x7_confidential_balance_get_D"></a>

## Function `get_D`

Returns a reference to the D components (EK components) of a confidential balance.


<pre><code><b>public</b> <b>fun</b> <a href="confidential_balance.md#0x7_confidential_balance_get_D">get_D</a>(self: &<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">confidential_balance::ConfidentialBalance</a>): &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_balance.md#0x7_confidential_balance_get_D">get_D</a>(self: &<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">ConfidentialBalance</a>): &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;RistrettoPoint&gt; {
    &self.D
}
</code></pre>



</details>

<a id="0x7_confidential_balance_get_compressed_C"></a>

## Function `get_compressed_C`

Returns a reference to the C components (value components) of a compressed confidential balance.


<pre><code><b>public</b> <b>fun</b> <a href="confidential_balance.md#0x7_confidential_balance_get_compressed_C">get_compressed_C</a>(self: &<a href="confidential_balance.md#0x7_confidential_balance_CompressedConfidentialBalance">confidential_balance::CompressedConfidentialBalance</a>): &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_balance.md#0x7_confidential_balance_get_compressed_C">get_compressed_C</a>(self: &<a href="confidential_balance.md#0x7_confidential_balance_CompressedConfidentialBalance">CompressedConfidentialBalance</a>): &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;CompressedRistretto&gt; {
    &self.C
}
</code></pre>



</details>

<a id="0x7_confidential_balance_get_compressed_D"></a>

## Function `get_compressed_D`

Returns a reference to the D components (EK components) of a compressed confidential balance.


<pre><code><b>public</b> <b>fun</b> <a href="confidential_balance.md#0x7_confidential_balance_get_compressed_D">get_compressed_D</a>(self: &<a href="confidential_balance.md#0x7_confidential_balance_CompressedConfidentialBalance">confidential_balance::CompressedConfidentialBalance</a>): &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_balance.md#0x7_confidential_balance_get_compressed_D">get_compressed_D</a>(self: &<a href="confidential_balance.md#0x7_confidential_balance_CompressedConfidentialBalance">CompressedConfidentialBalance</a>): &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;CompressedRistretto&gt; {
    &self.D
}
</code></pre>



</details>

<a id="0x7_confidential_balance_set_compressed_D"></a>

## Function `set_compressed_D`

Sets the D components (EK components) of a compressed confidential balance.


<pre><code><b>public</b> <b>fun</b> <a href="confidential_balance.md#0x7_confidential_balance_set_compressed_D">set_compressed_D</a>(self: &<b>mut</b> <a href="confidential_balance.md#0x7_confidential_balance_CompressedConfidentialBalance">confidential_balance::CompressedConfidentialBalance</a>, new_D: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_balance.md#0x7_confidential_balance_set_compressed_D">set_compressed_D</a>(self: &<b>mut</b> <a href="confidential_balance.md#0x7_confidential_balance_CompressedConfidentialBalance">CompressedConfidentialBalance</a>, new_D: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;CompressedRistretto&gt;) {
    self.D = new_D;
}
</code></pre>



</details>

<a id="0x7_confidential_balance_new_compressed_zero_balance"></a>

## Function `new_compressed_zero_balance`

Creates a new compressed zero balance with the specified number of chunks.


<pre><code><b>public</b> <b>fun</b> <a href="confidential_balance.md#0x7_confidential_balance_new_compressed_zero_balance">new_compressed_zero_balance</a>(num_chunks: u64): <a href="confidential_balance.md#0x7_confidential_balance_CompressedConfidentialBalance">confidential_balance::CompressedConfidentialBalance</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_balance.md#0x7_confidential_balance_new_compressed_zero_balance">new_compressed_zero_balance</a>(num_chunks: u64): <a href="confidential_balance.md#0x7_confidential_balance_CompressedConfidentialBalance">CompressedConfidentialBalance</a> {
    <b>let</b> identity = <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_identity_compressed">ristretto255::point_identity_compressed</a>();
    <a href="confidential_balance.md#0x7_confidential_balance_CompressedConfidentialBalance">CompressedConfidentialBalance</a> {
        C: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, num_chunks).map(|_| identity),
        D: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, num_chunks).map(|_| identity),
    }
}
</code></pre>



</details>

<a id="0x7_confidential_balance_new_pending_balance_u64_no_randomness"></a>

## Function `new_pending_balance_u64_no_randomness`

Creates a new pending balance from a 64-bit amount with no randomness (D components are identity).
Splits the amount into four 16-bit chunks.


<pre><code><b>public</b> <b>fun</b> <a href="confidential_balance.md#0x7_confidential_balance_new_pending_balance_u64_no_randomness">new_pending_balance_u64_no_randomness</a>(amount: u64): <a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">confidential_balance::ConfidentialBalance</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_balance.md#0x7_confidential_balance_new_pending_balance_u64_no_randomness">new_pending_balance_u64_no_randomness</a>(amount: u64): <a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">ConfidentialBalance</a> {
    <b>let</b> identity = <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_identity">ristretto255::point_identity</a>();
    <a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">ConfidentialBalance</a> {
        C: <a href="confidential_balance.md#0x7_confidential_balance_split_into_chunks">split_into_chunks</a>((amount <b>as</b> u128), <a href="confidential_balance.md#0x7_confidential_balance_PENDING_BALANCE_CHUNKS">PENDING_BALANCE_CHUNKS</a>).map(|chunk| <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_basepoint_mul">ristretto255::basepoint_mul</a>(&chunk)),
        D: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, <a href="confidential_balance.md#0x7_confidential_balance_PENDING_BALANCE_CHUNKS">PENDING_BALANCE_CHUNKS</a>).map(|_| <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_clone">ristretto255::point_clone</a>(&identity)),
    }
}
</code></pre>



</details>

<a id="0x7_confidential_balance_new_balance_from_bytes"></a>

## Function `new_balance_from_bytes`

Creates a new balance from a serialized byte array representation.
Format: [C_0 (32 bytes), D_0 (32 bytes), C_1, D_1, ...] - interleaved for SDK compatibility.
Returns <code>Some(<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">ConfidentialBalance</a>)</code> if deserialization succeeds, otherwise <code>None</code>.


<pre><code><b>public</b> <b>fun</b> <a href="confidential_balance.md#0x7_confidential_balance_new_balance_from_bytes">new_balance_from_bytes</a>(bytes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, num_chunks: u64): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">confidential_balance::ConfidentialBalance</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_balance.md#0x7_confidential_balance_new_balance_from_bytes">new_balance_from_bytes</a>(bytes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, num_chunks: u64): Option&lt;<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">ConfidentialBalance</a>&gt; {
    <b>if</b> (bytes.length() != 64 * num_chunks) {
        <b>return</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
    };

    <b>let</b> c_vec = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];
    <b>let</b> d_vec = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];

    <b>let</b> i = 0;
    <b>while</b> (i &lt; num_chunks) {
        <b>let</b> c_opt = <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_new_point_from_bytes">ristretto255::new_point_from_bytes</a>(bytes.slice(i * 64, i * 64 + 32));
        <b>let</b> d_opt = <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_new_point_from_bytes">ristretto255::new_point_from_bytes</a>(bytes.slice(i * 64 + 32, i * 64 + 64));

        <b>if</b> (c_opt.is_none() || d_opt.is_none()) {
            <b>return</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
        };

        c_vec.push_back(c_opt.extract());
        d_vec.push_back(d_opt.extract());
        i = i + 1;
    };

    <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">ConfidentialBalance</a> { C: c_vec, D: d_vec })
}
</code></pre>



</details>

<a id="0x7_confidential_balance_compress"></a>

## Function `compress`

Compresses a confidential balance into its <code><a href="confidential_balance.md#0x7_confidential_balance_CompressedConfidentialBalance">CompressedConfidentialBalance</a></code> representation.


<pre><code><b>public</b> <b>fun</b> <a href="confidential_balance.md#0x7_confidential_balance_compress">compress</a>(balance: &<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">confidential_balance::ConfidentialBalance</a>): <a href="confidential_balance.md#0x7_confidential_balance_CompressedConfidentialBalance">confidential_balance::CompressedConfidentialBalance</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_balance.md#0x7_confidential_balance_compress">compress</a>(balance: &<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">ConfidentialBalance</a>): <a href="confidential_balance.md#0x7_confidential_balance_CompressedConfidentialBalance">CompressedConfidentialBalance</a> {
    <a href="confidential_balance.md#0x7_confidential_balance_CompressedConfidentialBalance">CompressedConfidentialBalance</a> {
        C: balance.C.map_ref(|c| <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_compress">ristretto255::point_compress</a>(c)),
        D: balance.D.map_ref(|d| <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_compress">ristretto255::point_compress</a>(d)),
    }
}
</code></pre>



</details>

<a id="0x7_confidential_balance_decompress"></a>

## Function `decompress`

Decompresses a compressed confidential balance into its <code><a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">ConfidentialBalance</a></code> representation.


<pre><code><b>public</b> <b>fun</b> <a href="confidential_balance.md#0x7_confidential_balance_decompress">decompress</a>(balance: &<a href="confidential_balance.md#0x7_confidential_balance_CompressedConfidentialBalance">confidential_balance::CompressedConfidentialBalance</a>): <a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">confidential_balance::ConfidentialBalance</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_balance.md#0x7_confidential_balance_decompress">decompress</a>(balance: &<a href="confidential_balance.md#0x7_confidential_balance_CompressedConfidentialBalance">CompressedConfidentialBalance</a>): <a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">ConfidentialBalance</a> {
    <a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">ConfidentialBalance</a> {
        C: balance.C.map_ref(|c| <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_decompress">ristretto255::point_decompress</a>(c)),
        D: balance.D.map_ref(|d| <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_decompress">ristretto255::point_decompress</a>(d)),
    }
}
</code></pre>



</details>

<a id="0x7_confidential_balance_balance_to_bytes"></a>

## Function `balance_to_bytes`

Serializes a confidential balance into a byte array representation.
Format: [C_0 (32 bytes), D_0 (32 bytes), C_1, D_1, ...] - interleaved for SDK compatibility.


<pre><code><b>public</b> <b>fun</b> <a href="confidential_balance.md#0x7_confidential_balance_balance_to_bytes">balance_to_bytes</a>(balance: &<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">confidential_balance::ConfidentialBalance</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_balance.md#0x7_confidential_balance_balance_to_bytes">balance_to_bytes</a>(balance: &<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">ConfidentialBalance</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <b>let</b> bytes = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];
    <b>let</b> i = 0;
    <b>let</b> len = balance.C.length();

    <b>while</b> (i &lt; len) {
        bytes.append(<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_to_bytes">ristretto255::point_to_bytes</a>(&<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_compress">ristretto255::point_compress</a>(&balance.C[i])));
        bytes.append(<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_to_bytes">ristretto255::point_to_bytes</a>(&<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_compress">ristretto255::point_compress</a>(&balance.D[i])));
        i = i + 1;
    };

    bytes
}
</code></pre>



</details>

<a id="0x7_confidential_balance_add_balances_mut"></a>

## Function `add_balances_mut`

Adds two confidential balances homomorphically, mutating the first balance in place.
The second balance must have fewer or equal chunks compared to the first.


<pre><code><b>public</b> <b>fun</b> <a href="confidential_balance.md#0x7_confidential_balance_add_balances_mut">add_balances_mut</a>(lhs: &<b>mut</b> <a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">confidential_balance::ConfidentialBalance</a>, rhs: &<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">confidential_balance::ConfidentialBalance</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_balance.md#0x7_confidential_balance_add_balances_mut">add_balances_mut</a>(lhs: &<b>mut</b> <a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">ConfidentialBalance</a>, rhs: &<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">ConfidentialBalance</a>) {
    <b>assert</b>!(lhs.C.length() &gt;= rhs.C.length(), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_internal">error::internal</a>(<a href="confidential_balance.md#0x7_confidential_balance_EINTERNAL_ERROR">EINTERNAL_ERROR</a>));

    <b>let</b> i = 0;
    <b>let</b> rhs_len = rhs.C.length();
    <b>while</b> (i &lt; rhs_len) {
        <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_add_assign">ristretto255::point_add_assign</a>(&<b>mut</b> lhs.C[i], &rhs.C[i]);
        <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_add_assign">ristretto255::point_add_assign</a>(&<b>mut</b> lhs.D[i], &rhs.D[i]);
        i = i + 1;
    };
}
</code></pre>



</details>

<a id="0x7_confidential_balance_balance_c_equals"></a>

## Function `balance_c_equals`

Checks if the corresponding value components (<code>C</code>) of two confidential balances are equivalent.


<pre><code><b>public</b> <b>fun</b> <a href="confidential_balance.md#0x7_confidential_balance_balance_c_equals">balance_c_equals</a>(lhs: &<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">confidential_balance::ConfidentialBalance</a>, rhs: &<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">confidential_balance::ConfidentialBalance</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_balance.md#0x7_confidential_balance_balance_c_equals">balance_c_equals</a>(lhs: &<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">ConfidentialBalance</a>, rhs: &<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">ConfidentialBalance</a>): bool {
    <b>assert</b>!(lhs.C.length() == rhs.C.length(), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_internal">error::internal</a>(<a href="confidential_balance.md#0x7_confidential_balance_EINTERNAL_ERROR">EINTERNAL_ERROR</a>));

    <b>let</b> ok = <b>true</b>;
    <b>let</b> i = 0;
    <b>let</b> len = lhs.C.length();

    <b>while</b> (i &lt; len) {
        ok = ok && <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_equals">ristretto255::point_equals</a>(&lhs.C[i], &rhs.C[i]);
        i = i + 1;
    };

    ok
}
</code></pre>



</details>

<a id="0x7_confidential_balance_is_zero_balance"></a>

## Function `is_zero_balance`

Checks if a confidential balance is equivalent to zero (all C and D are identity).


<pre><code><b>public</b> <b>fun</b> <a href="confidential_balance.md#0x7_confidential_balance_is_zero_balance">is_zero_balance</a>(balance: &<a href="confidential_balance.md#0x7_confidential_balance_CompressedConfidentialBalance">confidential_balance::CompressedConfidentialBalance</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_balance.md#0x7_confidential_balance_is_zero_balance">is_zero_balance</a>(balance: &<a href="confidential_balance.md#0x7_confidential_balance_CompressedConfidentialBalance">CompressedConfidentialBalance</a>): bool {
    balance.C.all(|c| c.is_identity()) &&
    balance.D.all(|d| d.is_identity())
}
</code></pre>



</details>

<a id="0x7_confidential_balance_split_into_chunks"></a>

## Function `split_into_chunks`

Splits an integer amount into <code>num_chunks</code> 16-bit chunks, represented as <code>Scalar</code> values.


<pre><code><b>public</b> <b>fun</b> <a href="confidential_balance.md#0x7_confidential_balance_split_into_chunks">split_into_chunks</a>(amount: u128, num_chunks: u64): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_balance.md#0x7_confidential_balance_split_into_chunks">split_into_chunks</a>(amount: u128, num_chunks: u64): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Scalar&gt; {
    <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, num_chunks).map(|i| {
        <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_new_scalar_from_u128">ristretto255::new_scalar_from_u128</a>(amount &gt;&gt; (i * <a href="confidential_balance.md#0x7_confidential_balance_CHUNK_SIZE_BITS">CHUNK_SIZE_BITS</a> <b>as</b> u8) & 0xffff)
    })
}
</code></pre>



</details>

<a id="0x7_confidential_balance_get_num_pending_chunks"></a>

## Function `get_num_pending_chunks`

Returns the number of chunks in a pending balance.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="confidential_balance.md#0x7_confidential_balance_get_num_pending_chunks">get_num_pending_chunks</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_balance.md#0x7_confidential_balance_get_num_pending_chunks">get_num_pending_chunks</a>(): u64 {
    <a href="confidential_balance.md#0x7_confidential_balance_PENDING_BALANCE_CHUNKS">PENDING_BALANCE_CHUNKS</a>
}
</code></pre>



</details>

<a id="0x7_confidential_balance_get_num_available_chunks"></a>

## Function `get_num_available_chunks`

Returns the number of chunks in an available balance.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="confidential_balance.md#0x7_confidential_balance_get_num_available_chunks">get_num_available_chunks</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_balance.md#0x7_confidential_balance_get_num_available_chunks">get_num_available_chunks</a>(): u64 {
    <a href="confidential_balance.md#0x7_confidential_balance_AVAILABLE_BALANCE_CHUNKS">AVAILABLE_BALANCE_CHUNKS</a>
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
