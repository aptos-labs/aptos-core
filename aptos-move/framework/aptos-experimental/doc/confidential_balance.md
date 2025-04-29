
<a id="0x7_confidential_balance"></a>

# Module `0x7::confidential_balance`

This module implements a Confidential Balance abstraction, built on top of Twisted ElGamal encryption,
over the Ristretto255 curve.

The Confidential Balance encapsulates encrypted representations of a balance, split into chunks and stored as pairs of
ciphertext components <code>(C_i, D_i)</code> under basepoints <code>G</code> and <code>H</code> and an encryption key <code>P = dk^(-1) * H</code>, where <code>dk</code>
is the corresponding decryption key. Each pair represents an encrypted value <code>a_i</code> - the <code>i</code>-th 16-bit portion of
the total encrypted amount - and its associated randomness <code>r_i</code>, such that <code>C_i = a_i * G + r_i * H</code> and <code>D_i = r_i * P</code>.

The module supports two types of balances:
- Pending balances are represented by four ciphertext pairs <code>(C_i, D_i), i = 1..4</code>, suitable for 64-bit values.
- Actual balances are represented by eight ciphertext pairs <code>(C_i, D_i), i = 1..8</code>, capable of handling 128-bit values.

This implementation leverages the homomorphic properties of Twisted ElGamal encryption to allow arithmetic operations
directly on encrypted data.


-  [Struct `CompressedConfidentialBalance`](#0x7_confidential_balance_CompressedConfidentialBalance)
-  [Struct `ConfidentialBalance`](#0x7_confidential_balance_ConfidentialBalance)
-  [Constants](#@Constants_0)
-  [Function `new_pending_balance_no_randomness`](#0x7_confidential_balance_new_pending_balance_no_randomness)
-  [Function `new_actual_balance_no_randomness`](#0x7_confidential_balance_new_actual_balance_no_randomness)
-  [Function `new_compressed_pending_balance_no_randomness`](#0x7_confidential_balance_new_compressed_pending_balance_no_randomness)
-  [Function `new_compressed_actual_balance_no_randomness`](#0x7_confidential_balance_new_compressed_actual_balance_no_randomness)
-  [Function `new_pending_balance_u64_no_randonmess`](#0x7_confidential_balance_new_pending_balance_u64_no_randonmess)
-  [Function `new_pending_balance_from_bytes`](#0x7_confidential_balance_new_pending_balance_from_bytes)
-  [Function `new_actual_balance_from_bytes`](#0x7_confidential_balance_new_actual_balance_from_bytes)
-  [Function `compress_balance`](#0x7_confidential_balance_compress_balance)
-  [Function `decompress_balance`](#0x7_confidential_balance_decompress_balance)
-  [Function `balance_to_bytes`](#0x7_confidential_balance_balance_to_bytes)
-  [Function `balance_to_points_c`](#0x7_confidential_balance_balance_to_points_c)
-  [Function `balance_to_points_d`](#0x7_confidential_balance_balance_to_points_d)
-  [Function `add_balances_mut`](#0x7_confidential_balance_add_balances_mut)
-  [Function `sub_balances_mut`](#0x7_confidential_balance_sub_balances_mut)
-  [Function `balance_equals`](#0x7_confidential_balance_balance_equals)
-  [Function `balance_c_equals`](#0x7_confidential_balance_balance_c_equals)
-  [Function `is_zero_balance`](#0x7_confidential_balance_is_zero_balance)
-  [Function `split_into_chunks_u64`](#0x7_confidential_balance_split_into_chunks_u64)
-  [Function `split_into_chunks_u128`](#0x7_confidential_balance_split_into_chunks_u128)
-  [Function `get_pending_balance_chunks`](#0x7_confidential_balance_get_pending_balance_chunks)
-  [Function `get_actual_balance_chunks`](#0x7_confidential_balance_get_actual_balance_chunks)
-  [Function `get_chunk_size_bits`](#0x7_confidential_balance_get_chunk_size_bits)


<pre><code><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255">0x1::ristretto255</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
<b>use</b> <a href="ristretto255_twisted_elgamal.md#0x7_ristretto255_twisted_elgamal">0x7::ristretto255_twisted_elgamal</a>;
</code></pre>



<a id="0x7_confidential_balance_CompressedConfidentialBalance"></a>

## Struct `CompressedConfidentialBalance`

Represents a compressed confidential balance, where each chunk is a compressed Twisted ElGamal ciphertext.


<pre><code><b>struct</b> <a href="confidential_balance.md#0x7_confidential_balance_CompressedConfidentialBalance">CompressedConfidentialBalance</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>chunks: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="ristretto255_twisted_elgamal.md#0x7_ristretto255_twisted_elgamal_CompressedCiphertext">ristretto255_twisted_elgamal::CompressedCiphertext</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x7_confidential_balance_ConfidentialBalance"></a>

## Struct `ConfidentialBalance`

Represents a confidential balance, where each chunk is a Twisted ElGamal ciphertext.


<pre><code><b>struct</b> <a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">ConfidentialBalance</a> <b>has</b> drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>chunks: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="ristretto255_twisted_elgamal.md#0x7_ristretto255_twisted_elgamal_Ciphertext">ristretto255_twisted_elgamal::Ciphertext</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x7_confidential_balance_ACTUAL_BALANCE_CHUNKS"></a>

The number of chunks in an actual balance.


<pre><code><b>const</b> <a href="confidential_balance.md#0x7_confidential_balance_ACTUAL_BALANCE_CHUNKS">ACTUAL_BALANCE_CHUNKS</a>: u64 = 8;
</code></pre>



<a id="0x7_confidential_balance_CHUNK_SIZE_BITS"></a>

The number of bits in a single chunk.


<pre><code><b>const</b> <a href="confidential_balance.md#0x7_confidential_balance_CHUNK_SIZE_BITS">CHUNK_SIZE_BITS</a>: u64 = 16;
</code></pre>



<a id="0x7_confidential_balance_EINTERNAL_ERROR"></a>

An internal error occurred, indicating unexpected behavior.


<pre><code><b>const</b> <a href="confidential_balance.md#0x7_confidential_balance_EINTERNAL_ERROR">EINTERNAL_ERROR</a>: u64 = 1;
</code></pre>



<a id="0x7_confidential_balance_PENDING_BALANCE_CHUNKS"></a>

The number of chunks in a pending balance.


<pre><code><b>const</b> <a href="confidential_balance.md#0x7_confidential_balance_PENDING_BALANCE_CHUNKS">PENDING_BALANCE_CHUNKS</a>: u64 = 4;
</code></pre>



<a id="0x7_confidential_balance_new_pending_balance_no_randomness"></a>

## Function `new_pending_balance_no_randomness`

Creates a new zero pending balance, where each chunk is set to zero Twisted ElGamal ciphertext.


<pre><code><b>public</b> <b>fun</b> <a href="confidential_balance.md#0x7_confidential_balance_new_pending_balance_no_randomness">new_pending_balance_no_randomness</a>(): <a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">confidential_balance::ConfidentialBalance</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_balance.md#0x7_confidential_balance_new_pending_balance_no_randomness">new_pending_balance_no_randomness</a>(): <a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">ConfidentialBalance</a> {
    <a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">ConfidentialBalance</a> {
        chunks: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, <a href="confidential_balance.md#0x7_confidential_balance_PENDING_BALANCE_CHUNKS">PENDING_BALANCE_CHUNKS</a>).map(|_| {
            twisted_elgamal::ciphertext_from_points(<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_identity">ristretto255::point_identity</a>(), <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_identity">ristretto255::point_identity</a>())
        })
    }
}
</code></pre>



</details>

<a id="0x7_confidential_balance_new_actual_balance_no_randomness"></a>

## Function `new_actual_balance_no_randomness`

Creates a new zero actual balance, where each chunk is set to zero Twisted ElGamal ciphertext.


<pre><code><b>public</b> <b>fun</b> <a href="confidential_balance.md#0x7_confidential_balance_new_actual_balance_no_randomness">new_actual_balance_no_randomness</a>(): <a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">confidential_balance::ConfidentialBalance</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_balance.md#0x7_confidential_balance_new_actual_balance_no_randomness">new_actual_balance_no_randomness</a>(): <a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">ConfidentialBalance</a> {
    <a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">ConfidentialBalance</a> {
        chunks: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, <a href="confidential_balance.md#0x7_confidential_balance_ACTUAL_BALANCE_CHUNKS">ACTUAL_BALANCE_CHUNKS</a>).map(|_| {
            twisted_elgamal::ciphertext_from_points(<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_identity">ristretto255::point_identity</a>(), <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_identity">ristretto255::point_identity</a>())
        })
    }
}
</code></pre>



</details>

<a id="0x7_confidential_balance_new_compressed_pending_balance_no_randomness"></a>

## Function `new_compressed_pending_balance_no_randomness`

Creates a new compressed zero pending balance, where each chunk is set to compressed zero Twisted ElGamal ciphertext.


<pre><code><b>public</b> <b>fun</b> <a href="confidential_balance.md#0x7_confidential_balance_new_compressed_pending_balance_no_randomness">new_compressed_pending_balance_no_randomness</a>(): <a href="confidential_balance.md#0x7_confidential_balance_CompressedConfidentialBalance">confidential_balance::CompressedConfidentialBalance</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_balance.md#0x7_confidential_balance_new_compressed_pending_balance_no_randomness">new_compressed_pending_balance_no_randomness</a>(): <a href="confidential_balance.md#0x7_confidential_balance_CompressedConfidentialBalance">CompressedConfidentialBalance</a> {
    <a href="confidential_balance.md#0x7_confidential_balance_CompressedConfidentialBalance">CompressedConfidentialBalance</a> {
        chunks: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, <a href="confidential_balance.md#0x7_confidential_balance_PENDING_BALANCE_CHUNKS">PENDING_BALANCE_CHUNKS</a>).map(|_| {
            twisted_elgamal::ciphertext_from_compressed_points(
                <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_identity_compressed">ristretto255::point_identity_compressed</a>(), <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_identity_compressed">ristretto255::point_identity_compressed</a>())
        })
    }
}
</code></pre>



</details>

<a id="0x7_confidential_balance_new_compressed_actual_balance_no_randomness"></a>

## Function `new_compressed_actual_balance_no_randomness`

Creates a new compressed zero actual balance, where each chunk is set to compressed zero Twisted ElGamal ciphertext.


<pre><code><b>public</b> <b>fun</b> <a href="confidential_balance.md#0x7_confidential_balance_new_compressed_actual_balance_no_randomness">new_compressed_actual_balance_no_randomness</a>(): <a href="confidential_balance.md#0x7_confidential_balance_CompressedConfidentialBalance">confidential_balance::CompressedConfidentialBalance</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_balance.md#0x7_confidential_balance_new_compressed_actual_balance_no_randomness">new_compressed_actual_balance_no_randomness</a>(): <a href="confidential_balance.md#0x7_confidential_balance_CompressedConfidentialBalance">CompressedConfidentialBalance</a> {
    <a href="confidential_balance.md#0x7_confidential_balance_CompressedConfidentialBalance">CompressedConfidentialBalance</a> {
        chunks: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, <a href="confidential_balance.md#0x7_confidential_balance_ACTUAL_BALANCE_CHUNKS">ACTUAL_BALANCE_CHUNKS</a>).map(|_| {
            twisted_elgamal::ciphertext_from_compressed_points(
                <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_identity_compressed">ristretto255::point_identity_compressed</a>(), <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_identity_compressed">ristretto255::point_identity_compressed</a>())
        })
    }
}
</code></pre>



</details>

<a id="0x7_confidential_balance_new_pending_balance_u64_no_randonmess"></a>

## Function `new_pending_balance_u64_no_randonmess`

Creates a new pending balance from a 64-bit amount with no randomness, splitting the amount into four 16-bit chunks.


<pre><code><b>public</b> <b>fun</b> <a href="confidential_balance.md#0x7_confidential_balance_new_pending_balance_u64_no_randonmess">new_pending_balance_u64_no_randonmess</a>(amount: u64): <a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">confidential_balance::ConfidentialBalance</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_balance.md#0x7_confidential_balance_new_pending_balance_u64_no_randonmess">new_pending_balance_u64_no_randonmess</a>(amount: u64): <a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">ConfidentialBalance</a> {
    <a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">ConfidentialBalance</a> {
        chunks: <a href="confidential_balance.md#0x7_confidential_balance_split_into_chunks_u64">split_into_chunks_u64</a>(amount).map(|chunk| {
            twisted_elgamal::new_ciphertext_no_randomness(&chunk)
        })
    }
}
</code></pre>



</details>

<a id="0x7_confidential_balance_new_pending_balance_from_bytes"></a>

## Function `new_pending_balance_from_bytes`

Creates a new pending balance from a serialized byte array representation.
Returns <code>Some(<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">ConfidentialBalance</a>)</code> if deserialization succeeds, otherwise <code>None</code>.


<pre><code><b>public</b> <b>fun</b> <a href="confidential_balance.md#0x7_confidential_balance_new_pending_balance_from_bytes">new_pending_balance_from_bytes</a>(bytes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">confidential_balance::ConfidentialBalance</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_balance.md#0x7_confidential_balance_new_pending_balance_from_bytes">new_pending_balance_from_bytes</a>(bytes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): Option&lt;<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">ConfidentialBalance</a>&gt; {
    <b>if</b> (bytes.length() != 64 * <a href="confidential_balance.md#0x7_confidential_balance_PENDING_BALANCE_CHUNKS">PENDING_BALANCE_CHUNKS</a>) {
        <b>return</b> std::option::none()
    };

    <b>let</b> chunks = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, <a href="confidential_balance.md#0x7_confidential_balance_PENDING_BALANCE_CHUNKS">PENDING_BALANCE_CHUNKS</a>).map(|i| {
        twisted_elgamal::new_ciphertext_from_bytes(bytes.slice(i * 64, (i + 1) * 64))
    });

    <b>if</b> (chunks.<a href="../../aptos-framework/../aptos-stdlib/doc/any.md#0x1_any">any</a>(|chunk| chunk.is_none())) {
        <b>return</b> std::option::none()
    };

    <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">ConfidentialBalance</a> {
        chunks: chunks.map(|chunk| chunk.extract())
    })
}
</code></pre>



</details>

<a id="0x7_confidential_balance_new_actual_balance_from_bytes"></a>

## Function `new_actual_balance_from_bytes`

Creates a new actual balance from a serialized byte array representation.
Returns <code>Some(<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">ConfidentialBalance</a>)</code> if deserialization succeeds, otherwise <code>None</code>.


<pre><code><b>public</b> <b>fun</b> <a href="confidential_balance.md#0x7_confidential_balance_new_actual_balance_from_bytes">new_actual_balance_from_bytes</a>(bytes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">confidential_balance::ConfidentialBalance</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_balance.md#0x7_confidential_balance_new_actual_balance_from_bytes">new_actual_balance_from_bytes</a>(bytes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): Option&lt;<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">ConfidentialBalance</a>&gt; {
    <b>if</b> (bytes.length() != 64 * <a href="confidential_balance.md#0x7_confidential_balance_ACTUAL_BALANCE_CHUNKS">ACTUAL_BALANCE_CHUNKS</a>) {
        <b>return</b> std::option::none()
    };

    <b>let</b> chunks = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, <a href="confidential_balance.md#0x7_confidential_balance_ACTUAL_BALANCE_CHUNKS">ACTUAL_BALANCE_CHUNKS</a>).map(|i| {
        twisted_elgamal::new_ciphertext_from_bytes(bytes.slice(i * 64, (i + 1) * 64))
    });

    <b>if</b> (chunks.<a href="../../aptos-framework/../aptos-stdlib/doc/any.md#0x1_any">any</a>(|chunk| chunk.is_none())) {
        <b>return</b> std::option::none()
    };

    <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">ConfidentialBalance</a> {
        chunks: chunks.map(|chunk| chunk.extract())
    })
}
</code></pre>



</details>

<a id="0x7_confidential_balance_compress_balance"></a>

## Function `compress_balance`

Compresses a confidential balance into its <code><a href="confidential_balance.md#0x7_confidential_balance_CompressedConfidentialBalance">CompressedConfidentialBalance</a></code> representation.


<pre><code><b>public</b> <b>fun</b> <a href="confidential_balance.md#0x7_confidential_balance_compress_balance">compress_balance</a>(balance: &<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">confidential_balance::ConfidentialBalance</a>): <a href="confidential_balance.md#0x7_confidential_balance_CompressedConfidentialBalance">confidential_balance::CompressedConfidentialBalance</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_balance.md#0x7_confidential_balance_compress_balance">compress_balance</a>(balance: &<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">ConfidentialBalance</a>): <a href="confidential_balance.md#0x7_confidential_balance_CompressedConfidentialBalance">CompressedConfidentialBalance</a> {
    <a href="confidential_balance.md#0x7_confidential_balance_CompressedConfidentialBalance">CompressedConfidentialBalance</a> {
        chunks: balance.chunks.map_ref(|ciphertext| twisted_elgamal::compress_ciphertext(ciphertext))
    }
}
</code></pre>



</details>

<a id="0x7_confidential_balance_decompress_balance"></a>

## Function `decompress_balance`

Decompresses a compressed confidential balance into its <code><a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">ConfidentialBalance</a></code> representation.


<pre><code><b>public</b> <b>fun</b> <a href="confidential_balance.md#0x7_confidential_balance_decompress_balance">decompress_balance</a>(balance: &<a href="confidential_balance.md#0x7_confidential_balance_CompressedConfidentialBalance">confidential_balance::CompressedConfidentialBalance</a>): <a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">confidential_balance::ConfidentialBalance</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_balance.md#0x7_confidential_balance_decompress_balance">decompress_balance</a>(balance: &<a href="confidential_balance.md#0x7_confidential_balance_CompressedConfidentialBalance">CompressedConfidentialBalance</a>): <a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">ConfidentialBalance</a> {
    <a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">ConfidentialBalance</a> {
        chunks: balance.chunks.map_ref(|ciphertext| twisted_elgamal::decompress_ciphertext(ciphertext))
    }
}
</code></pre>



</details>

<a id="0x7_confidential_balance_balance_to_bytes"></a>

## Function `balance_to_bytes`

Serializes a confidential balance into a byte array representation.


<pre><code><b>public</b> <b>fun</b> <a href="confidential_balance.md#0x7_confidential_balance_balance_to_bytes">balance_to_bytes</a>(balance: &<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">confidential_balance::ConfidentialBalance</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_balance.md#0x7_confidential_balance_balance_to_bytes">balance_to_bytes</a>(balance: &<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">ConfidentialBalance</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <b>let</b> bytes = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;[];

    balance.chunks.for_each_ref(|ciphertext| {
        bytes.append(twisted_elgamal::ciphertext_to_bytes(ciphertext));
    });

    bytes
}
</code></pre>



</details>

<a id="0x7_confidential_balance_balance_to_points_c"></a>

## Function `balance_to_points_c`

Extracts the <code>C</code> value component (<code>a * H + r * G</code>) of each chunk in a confidential balance as a vector of <code>RistrettoPoint</code>s.


<pre><code><b>public</b> <b>fun</b> <a href="confidential_balance.md#0x7_confidential_balance_balance_to_points_c">balance_to_points_c</a>(balance: &<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">confidential_balance::ConfidentialBalance</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_balance.md#0x7_confidential_balance_balance_to_points_c">balance_to_points_c</a>(balance: &<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">ConfidentialBalance</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;RistrettoPoint&gt; {
    balance.chunks.map_ref(|chunk| {
        <b>let</b> (c, _) = twisted_elgamal::ciphertext_as_points(chunk);
        <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_clone">ristretto255::point_clone</a>(c)
    })
}
</code></pre>



</details>

<a id="0x7_confidential_balance_balance_to_points_d"></a>

## Function `balance_to_points_d`

Extracts the <code>D</code> randomness component (<code>r * Y</code>) of each chunk in a confidential balance as a vector of <code>RistrettoPoint</code>s.


<pre><code><b>public</b> <b>fun</b> <a href="confidential_balance.md#0x7_confidential_balance_balance_to_points_d">balance_to_points_d</a>(balance: &<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">confidential_balance::ConfidentialBalance</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_balance.md#0x7_confidential_balance_balance_to_points_d">balance_to_points_d</a>(balance: &<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">ConfidentialBalance</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;RistrettoPoint&gt; {
    balance.chunks.map_ref(|chunk| {
        <b>let</b> (_, d) = twisted_elgamal::ciphertext_as_points(chunk);
        <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_clone">ristretto255::point_clone</a>(d)
    })
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
    <b>assert</b>!(lhs.chunks.length() &gt;= rhs.chunks.length(), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_internal">error::internal</a>(<a href="confidential_balance.md#0x7_confidential_balance_EINTERNAL_ERROR">EINTERNAL_ERROR</a>));

    lhs.chunks.enumerate_mut(|i, chunk| {
        <b>if</b> (i &lt; rhs.chunks.length()) {
            twisted_elgamal::ciphertext_add_assign(chunk, &rhs.chunks[i])
        }
    })
}
</code></pre>



</details>

<a id="0x7_confidential_balance_sub_balances_mut"></a>

## Function `sub_balances_mut`

Subtracts one confidential balance from another homomorphically, mutating the first balance in place.
The second balance must have fewer or equal chunks compared to the first.


<pre><code><b>public</b> <b>fun</b> <a href="confidential_balance.md#0x7_confidential_balance_sub_balances_mut">sub_balances_mut</a>(lhs: &<b>mut</b> <a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">confidential_balance::ConfidentialBalance</a>, rhs: &<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">confidential_balance::ConfidentialBalance</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_balance.md#0x7_confidential_balance_sub_balances_mut">sub_balances_mut</a>(lhs: &<b>mut</b> <a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">ConfidentialBalance</a>, rhs: &<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">ConfidentialBalance</a>) {
    <b>assert</b>!(lhs.chunks.length() &gt;= rhs.chunks.length(), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_internal">error::internal</a>(<a href="confidential_balance.md#0x7_confidential_balance_EINTERNAL_ERROR">EINTERNAL_ERROR</a>));

    lhs.chunks.enumerate_mut(|i, chunk| {
        <b>if</b> (i &lt; rhs.chunks.length()) {
            twisted_elgamal::ciphertext_add_assign(chunk, &rhs.chunks[i])
        }
    })
}
</code></pre>



</details>

<a id="0x7_confidential_balance_balance_equals"></a>

## Function `balance_equals`

Checks if two confidential balances are equivalent, including both value and randomness components.


<pre><code><b>public</b> <b>fun</b> <a href="confidential_balance.md#0x7_confidential_balance_balance_equals">balance_equals</a>(lhs: &<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">confidential_balance::ConfidentialBalance</a>, rhs: &<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">confidential_balance::ConfidentialBalance</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_balance.md#0x7_confidential_balance_balance_equals">balance_equals</a>(lhs: &<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">ConfidentialBalance</a>, rhs: &<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">ConfidentialBalance</a>): bool {
    <b>assert</b>!(lhs.chunks.length() == rhs.chunks.length(), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_internal">error::internal</a>(<a href="confidential_balance.md#0x7_confidential_balance_EINTERNAL_ERROR">EINTERNAL_ERROR</a>));

    <b>let</b> ok = <b>true</b>;

    lhs.chunks.zip_ref(&rhs.chunks, |l, r| {
        ok = ok && twisted_elgamal::ciphertext_equals(l, r);
    });

    ok
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
    <b>assert</b>!(lhs.chunks.length() == rhs.chunks.length(), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_internal">error::internal</a>(<a href="confidential_balance.md#0x7_confidential_balance_EINTERNAL_ERROR">EINTERNAL_ERROR</a>));

    <b>let</b> ok = <b>true</b>;

    lhs.chunks.zip_ref(&rhs.chunks, |l, r| {
        <b>let</b> (lc, _) = twisted_elgamal::ciphertext_as_points(l);
        <b>let</b> (rc, _) = twisted_elgamal::ciphertext_as_points(r);

        ok = ok && <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_equals">ristretto255::point_equals</a>(lc, rc);
    });

    ok
}
</code></pre>



</details>

<a id="0x7_confidential_balance_is_zero_balance"></a>

## Function `is_zero_balance`

Checks if a confidential balance is equivalent to zero, where all chunks are the identity element.


<pre><code><b>public</b> <b>fun</b> <a href="confidential_balance.md#0x7_confidential_balance_is_zero_balance">is_zero_balance</a>(balance: &<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">confidential_balance::ConfidentialBalance</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_balance.md#0x7_confidential_balance_is_zero_balance">is_zero_balance</a>(balance: &<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">ConfidentialBalance</a>): bool {
    balance.chunks.all(|chunk| {
        twisted_elgamal::ciphertext_equals(
            chunk,
            &twisted_elgamal::ciphertext_from_points(<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_identity">ristretto255::point_identity</a>(), <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_identity">ristretto255::point_identity</a>())
        )
    })
}
</code></pre>



</details>

<a id="0x7_confidential_balance_split_into_chunks_u64"></a>

## Function `split_into_chunks_u64`

Splits a 64-bit integer amount into four 16-bit chunks, represented as <code>Scalar</code> values.


<pre><code><b>public</b> <b>fun</b> <a href="confidential_balance.md#0x7_confidential_balance_split_into_chunks_u64">split_into_chunks_u64</a>(amount: u64): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_balance.md#0x7_confidential_balance_split_into_chunks_u64">split_into_chunks_u64</a>(amount: u64): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Scalar&gt; {
    <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, <a href="confidential_balance.md#0x7_confidential_balance_PENDING_BALANCE_CHUNKS">PENDING_BALANCE_CHUNKS</a>).map(|i| {
        <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_new_scalar_from_u64">ristretto255::new_scalar_from_u64</a>(amount &gt;&gt; (i * <a href="confidential_balance.md#0x7_confidential_balance_CHUNK_SIZE_BITS">CHUNK_SIZE_BITS</a> <b>as</b> u8) & 0xffff)
    })
}
</code></pre>



</details>

<a id="0x7_confidential_balance_split_into_chunks_u128"></a>

## Function `split_into_chunks_u128`

Splits a 128-bit integer amount into eight 16-bit chunks, represented as <code>Scalar</code> values.


<pre><code><b>public</b> <b>fun</b> <a href="confidential_balance.md#0x7_confidential_balance_split_into_chunks_u128">split_into_chunks_u128</a>(amount: u128): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_balance.md#0x7_confidential_balance_split_into_chunks_u128">split_into_chunks_u128</a>(amount: u128): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Scalar&gt; {
    <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, <a href="confidential_balance.md#0x7_confidential_balance_ACTUAL_BALANCE_CHUNKS">ACTUAL_BALANCE_CHUNKS</a>).map(|i| {
        <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_new_scalar_from_u128">ristretto255::new_scalar_from_u128</a>(amount &gt;&gt; (i * <a href="confidential_balance.md#0x7_confidential_balance_CHUNK_SIZE_BITS">CHUNK_SIZE_BITS</a> <b>as</b> u8) & 0xffff)
    })
}
</code></pre>



</details>

<a id="0x7_confidential_balance_get_pending_balance_chunks"></a>

## Function `get_pending_balance_chunks`

Returns the number of chunks in a pending balance.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="confidential_balance.md#0x7_confidential_balance_get_pending_balance_chunks">get_pending_balance_chunks</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_balance.md#0x7_confidential_balance_get_pending_balance_chunks">get_pending_balance_chunks</a>(): u64 {
    <a href="confidential_balance.md#0x7_confidential_balance_PENDING_BALANCE_CHUNKS">PENDING_BALANCE_CHUNKS</a>
}
</code></pre>



</details>

<a id="0x7_confidential_balance_get_actual_balance_chunks"></a>

## Function `get_actual_balance_chunks`

Returns the number of chunks in an actual balance.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="confidential_balance.md#0x7_confidential_balance_get_actual_balance_chunks">get_actual_balance_chunks</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_balance.md#0x7_confidential_balance_get_actual_balance_chunks">get_actual_balance_chunks</a>(): u64 {
    <a href="confidential_balance.md#0x7_confidential_balance_ACTUAL_BALANCE_CHUNKS">ACTUAL_BALANCE_CHUNKS</a>
}
</code></pre>



</details>

<a id="0x7_confidential_balance_get_chunk_size_bits"></a>

## Function `get_chunk_size_bits`

Returns the number of bits in a single chunk.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="confidential_balance.md#0x7_confidential_balance_get_chunk_size_bits">get_chunk_size_bits</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_balance.md#0x7_confidential_balance_get_chunk_size_bits">get_chunk_size_bits</a>(): u64 {
    <a href="confidential_balance.md#0x7_confidential_balance_CHUNK_SIZE_BITS">CHUNK_SIZE_BITS</a>
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
