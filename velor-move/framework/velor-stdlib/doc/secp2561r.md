
<a id="0x1_secp256r1"></a>

# Module `0x1::secp256r1`

This module implements ECDSA signatures based on the prime-order secp256r1 ellptic curve (i.e., cofactor is 1).


-  [Struct `ECDSARawPublicKey`](#0x1_secp256r1_ECDSARawPublicKey)
-  [Constants](#@Constants_0)
-  [Function `ecdsa_raw_public_key_from_64_bytes`](#0x1_secp256r1_ecdsa_raw_public_key_from_64_bytes)
-  [Function `ecdsa_raw_public_key_to_bytes`](#0x1_secp256r1_ecdsa_raw_public_key_to_bytes)
-  [Specification](#@Specification_1)


<pre><code><b>use</b> <a href="../../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
</code></pre>



<a id="0x1_secp256r1_ECDSARawPublicKey"></a>

## Struct `ECDSARawPublicKey`

A 64-byte ECDSA public key.


<pre><code><b>struct</b> <a href="secp2561r.md#0x1_secp256r1_ECDSARawPublicKey">ECDSARawPublicKey</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_secp256r1_E_DESERIALIZE"></a>

An error occurred while deserializing, for example due to wrong input size.


<pre><code><b>const</b> <a href="secp2561r.md#0x1_secp256r1_E_DESERIALIZE">E_DESERIALIZE</a>: u64 = 1;
</code></pre>



<a id="0x1_secp256r1_RAW_PUBLIC_KEY_NUM_BYTES"></a>

The size of a secp256k1-based ECDSA public key, in bytes.


<pre><code><b>const</b> <a href="secp2561r.md#0x1_secp256r1_RAW_PUBLIC_KEY_NUM_BYTES">RAW_PUBLIC_KEY_NUM_BYTES</a>: u64 = 64;
</code></pre>



<a id="0x1_secp256r1_ecdsa_raw_public_key_from_64_bytes"></a>

## Function `ecdsa_raw_public_key_from_64_bytes`

Constructs an ECDSARawPublicKey struct, given a 64-byte raw representation.


<pre><code><b>public</b> <b>fun</b> <a href="secp2561r.md#0x1_secp256r1_ecdsa_raw_public_key_from_64_bytes">ecdsa_raw_public_key_from_64_bytes</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="secp2561r.md#0x1_secp256r1_ECDSARawPublicKey">secp256r1::ECDSARawPublicKey</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="secp2561r.md#0x1_secp256r1_ecdsa_raw_public_key_from_64_bytes">ecdsa_raw_public_key_from_64_bytes</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="secp2561r.md#0x1_secp256r1_ECDSARawPublicKey">ECDSARawPublicKey</a> {
    <b>assert</b>!(bytes.length() == <a href="secp2561r.md#0x1_secp256r1_RAW_PUBLIC_KEY_NUM_BYTES">RAW_PUBLIC_KEY_NUM_BYTES</a>, std::error::invalid_argument(<a href="secp2561r.md#0x1_secp256r1_E_DESERIALIZE">E_DESERIALIZE</a>));
    <a href="secp2561r.md#0x1_secp256r1_ECDSARawPublicKey">ECDSARawPublicKey</a> { bytes }
}
</code></pre>



</details>

<a id="0x1_secp256r1_ecdsa_raw_public_key_to_bytes"></a>

## Function `ecdsa_raw_public_key_to_bytes`

Serializes an ECDSARawPublicKey struct to 64-bytes.


<pre><code><b>public</b> <b>fun</b> <a href="secp2561r.md#0x1_secp256r1_ecdsa_raw_public_key_to_bytes">ecdsa_raw_public_key_to_bytes</a>(pk: &<a href="secp2561r.md#0x1_secp256r1_ECDSARawPublicKey">secp256r1::ECDSARawPublicKey</a>): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="secp2561r.md#0x1_secp256r1_ecdsa_raw_public_key_to_bytes">ecdsa_raw_public_key_to_bytes</a>(pk: &<a href="secp2561r.md#0x1_secp256r1_ECDSARawPublicKey">ECDSARawPublicKey</a>): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    pk.bytes
}
</code></pre>



</details>

<a id="@Specification_1"></a>

## Specification



<pre><code><b>pragma</b> verify = <b>false</b>;
</code></pre>


[move-book]: https://velor.dev/move/book/SUMMARY
