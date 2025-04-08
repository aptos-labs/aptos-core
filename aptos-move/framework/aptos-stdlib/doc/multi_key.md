
<a id="0x1_multi_key"></a>

# Module `0x1::multi_key`

This module implements MultiKey type of public key.


-  [Struct `UnvalidatedPublicKey`](#0x1_multi_key_UnvalidatedPublicKey)
-  [Constants](#@Constants_0)
-  [Function `new_unvalidated_public_key_from_bytes`](#0x1_multi_key_new_unvalidated_public_key_from_bytes)
-  [Function `new_unvalidated_public_key_from_single_keys`](#0x1_multi_key_new_unvalidated_public_key_from_single_keys)
-  [Function `unvalidated_public_key_to_bytes`](#0x1_multi_key_unvalidated_public_key_to_bytes)
-  [Function `unvalidated_public_key_to_authentication_key`](#0x1_multi_key_unvalidated_public_key_to_authentication_key)
-  [Function `public_key_bytes_to_authentication_key`](#0x1_multi_key_public_key_bytes_to_authentication_key)


<pre><code><b>use</b> <a href="../../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="../../move-stdlib/doc/hash.md#0x1_hash">0x1::hash</a>;
<b>use</b> <a href="single_key.md#0x1_single_key">0x1::single_key</a>;
<b>use</b> <a href="../../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
</code></pre>



<a id="0x1_multi_key_UnvalidatedPublicKey"></a>

## Struct `UnvalidatedPublicKey`

An *unvalidated*, k out of n MultiKey public key. The <code>bytes</code> field contains (1) a vector of single key public keys and
(2) a single byte encoding the threshold k.
*Unvalidated* means there is no guarantee that the underlying PKs are valid elliptic curve points of non-small
order.  Nor is there a guarantee that it would deserialize correctly (i.e., for Keyless public keys).


<pre><code><b>struct</b> <a href="multi_key.md#0x1_multi_key_UnvalidatedPublicKey">UnvalidatedPublicKey</a> <b>has</b> <b>copy</b>, drop, store
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


<a id="0x1_multi_key_SIGNATURE_SCHEME_ID"></a>

The identifier of the MultiEd25519 signature scheme, which is used when deriving Aptos authentication keys by hashing
it together with an MultiEd25519 public key.


<pre><code><b>const</b> <a href="multi_key.md#0x1_multi_key_SIGNATURE_SCHEME_ID">SIGNATURE_SCHEME_ID</a>: u8 = 3;
</code></pre>



<a id="0x1_multi_key_MAX_NUMBER_OF_PUBLIC_KEYS"></a>

Max number of ed25519 public keys allowed in multi-ed25519 keys


<pre><code><b>const</b> <a href="multi_key.md#0x1_multi_key_MAX_NUMBER_OF_PUBLIC_KEYS">MAX_NUMBER_OF_PUBLIC_KEYS</a>: u64 = 32;
</code></pre>



<a id="0x1_multi_key_E_INVALID_MULTI_KEY_NO_KEYS"></a>

No keys were provided when creating a MultiKey public key.


<pre><code><b>const</b> <a href="multi_key.md#0x1_multi_key_E_INVALID_MULTI_KEY_NO_KEYS">E_INVALID_MULTI_KEY_NO_KEYS</a>: u64 = 1;
</code></pre>



<a id="0x1_multi_key_E_INVALID_MULTI_KEY_SIGNATURES_REQUIRED"></a>

The number of signatures required is greater than the number of keys provided.


<pre><code><b>const</b> <a href="multi_key.md#0x1_multi_key_E_INVALID_MULTI_KEY_SIGNATURES_REQUIRED">E_INVALID_MULTI_KEY_SIGNATURES_REQUIRED</a>: u64 = 3;
</code></pre>



<a id="0x1_multi_key_E_INVALID_MULTI_KEY_TOO_MANY_KEYS"></a>

The number of keys provided is greater than the maximum allowed.


<pre><code><b>const</b> <a href="multi_key.md#0x1_multi_key_E_INVALID_MULTI_KEY_TOO_MANY_KEYS">E_INVALID_MULTI_KEY_TOO_MANY_KEYS</a>: u64 = 2;
</code></pre>



<a id="0x1_multi_key_new_unvalidated_public_key_from_bytes"></a>

## Function `new_unvalidated_public_key_from_bytes`



<pre><code><b>public</b> <b>fun</b> <a href="multi_key.md#0x1_multi_key_new_unvalidated_public_key_from_bytes">new_unvalidated_public_key_from_bytes</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="multi_key.md#0x1_multi_key_UnvalidatedPublicKey">multi_key::UnvalidatedPublicKey</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multi_key.md#0x1_multi_key_new_unvalidated_public_key_from_bytes">new_unvalidated_public_key_from_bytes</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="multi_key.md#0x1_multi_key_UnvalidatedPublicKey">UnvalidatedPublicKey</a> {
    <a href="multi_key.md#0x1_multi_key_UnvalidatedPublicKey">UnvalidatedPublicKey</a> { bytes }
}
</code></pre>



</details>

<a id="0x1_multi_key_new_unvalidated_public_key_from_single_keys"></a>

## Function `new_unvalidated_public_key_from_single_keys`



<pre><code><b>public</b> <b>fun</b> <a href="multi_key.md#0x1_multi_key_new_unvalidated_public_key_from_single_keys">new_unvalidated_public_key_from_single_keys</a>(single_keys: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="single_key.md#0x1_single_key_UnvalidatedPublicKey">single_key::UnvalidatedPublicKey</a>&gt;, signatures_required: u8): <a href="multi_key.md#0x1_multi_key_UnvalidatedPublicKey">multi_key::UnvalidatedPublicKey</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multi_key.md#0x1_multi_key_new_unvalidated_public_key_from_single_keys">new_unvalidated_public_key_from_single_keys</a>(single_keys: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="single_key.md#0x1_single_key_UnvalidatedPublicKey">single_key::UnvalidatedPublicKey</a>&gt;, signatures_required: u8): <a href="multi_key.md#0x1_multi_key_UnvalidatedPublicKey">UnvalidatedPublicKey</a> {
    <b>let</b> num_keys = single_keys.length();
    <b>assert</b>!(
        num_keys &gt; 0,
        <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="multi_key.md#0x1_multi_key_E_INVALID_MULTI_KEY_NO_KEYS">E_INVALID_MULTI_KEY_NO_KEYS</a>)
    );
    <b>assert</b>!(
        num_keys &lt;= <a href="multi_key.md#0x1_multi_key_MAX_NUMBER_OF_PUBLIC_KEYS">MAX_NUMBER_OF_PUBLIC_KEYS</a>,
        <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="multi_key.md#0x1_multi_key_E_INVALID_MULTI_KEY_TOO_MANY_KEYS">E_INVALID_MULTI_KEY_TOO_MANY_KEYS</a>)
    );
    <b>assert</b>!(
        (signatures_required <b>as</b> u64) &lt;= num_keys,
        <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="multi_key.md#0x1_multi_key_E_INVALID_MULTI_KEY_SIGNATURES_REQUIRED">E_INVALID_MULTI_KEY_SIGNATURES_REQUIRED</a>)
    );
    <b>let</b> bytes = <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>[num_keys <b>as</b> u8];
    for (i in 0..single_keys.length()) {
        bytes.append(<a href="single_key.md#0x1_single_key_unvalidated_public_key_to_bytes">single_key::unvalidated_public_key_to_bytes</a>(&single_keys[i]));
    };
    bytes.push_back(signatures_required);
    <a href="multi_key.md#0x1_multi_key_UnvalidatedPublicKey">UnvalidatedPublicKey</a> { bytes }
}
</code></pre>



</details>

<a id="0x1_multi_key_unvalidated_public_key_to_bytes"></a>

## Function `unvalidated_public_key_to_bytes`

Serializes an UnvalidatedPublicKey struct to byte vec.


<pre><code><b>public</b> <b>fun</b> <a href="multi_key.md#0x1_multi_key_unvalidated_public_key_to_bytes">unvalidated_public_key_to_bytes</a>(pk: &<a href="multi_key.md#0x1_multi_key_UnvalidatedPublicKey">multi_key::UnvalidatedPublicKey</a>): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multi_key.md#0x1_multi_key_unvalidated_public_key_to_bytes">unvalidated_public_key_to_bytes</a>(pk: &<a href="multi_key.md#0x1_multi_key_UnvalidatedPublicKey">UnvalidatedPublicKey</a>): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    pk.bytes
}
</code></pre>



</details>

<a id="0x1_multi_key_unvalidated_public_key_to_authentication_key"></a>

## Function `unvalidated_public_key_to_authentication_key`

Derives the Aptos-specific authentication key of the given MultiKey public key.


<pre><code><b>public</b> <b>fun</b> <a href="multi_key.md#0x1_multi_key_unvalidated_public_key_to_authentication_key">unvalidated_public_key_to_authentication_key</a>(pk: &<a href="multi_key.md#0x1_multi_key_UnvalidatedPublicKey">multi_key::UnvalidatedPublicKey</a>): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multi_key.md#0x1_multi_key_unvalidated_public_key_to_authentication_key">unvalidated_public_key_to_authentication_key</a>(pk: &<a href="multi_key.md#0x1_multi_key_UnvalidatedPublicKey">UnvalidatedPublicKey</a>): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <a href="multi_key.md#0x1_multi_key_public_key_bytes_to_authentication_key">public_key_bytes_to_authentication_key</a>(pk.bytes)
}
</code></pre>



</details>

<a id="0x1_multi_key_public_key_bytes_to_authentication_key"></a>

## Function `public_key_bytes_to_authentication_key`

Derives the Aptos-specific authentication key of the given MultiKey public key.


<pre><code><b>fun</b> <a href="multi_key.md#0x1_multi_key_public_key_bytes_to_authentication_key">public_key_bytes_to_authentication_key</a>(pk_bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="multi_key.md#0x1_multi_key_public_key_bytes_to_authentication_key">public_key_bytes_to_authentication_key</a>(pk_bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    pk_bytes.push_back(<a href="multi_key.md#0x1_multi_key_SIGNATURE_SCHEME_ID">SIGNATURE_SCHEME_ID</a>);
    <a href="../../move-stdlib/doc/hash.md#0x1_hash_sha3_256">hash::sha3_256</a>(pk_bytes)
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
