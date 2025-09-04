
<a id="0x1_multi_key"></a>

# Module `0x1::multi_key`

This module implements MultiKey type of public key.
A MultiKey public key is a collection of single key public keys and a number representing the number of signatures required to authenticate a transaction.
Unlike MultiEd25519, the individual single keys can be of different schemes.


-  [Struct `MultiKey`](#0x1_multi_key_MultiKey)
-  [Constants](#@Constants_0)
-  [Function `new_public_key_from_bytes`](#0x1_multi_key_new_public_key_from_bytes)
-  [Function `new_multi_key_from_single_keys`](#0x1_multi_key_new_multi_key_from_single_keys)
-  [Function `deserialize_multi_key`](#0x1_multi_key_deserialize_multi_key)
-  [Function `to_authentication_key`](#0x1_multi_key_to_authentication_key)
-  [Specification](#@Specification_1)


<pre><code><b>use</b> <a href="../../move-stdlib/doc/bcs.md#0x1_bcs">0x1::bcs</a>;
<b>use</b> <a href="bcs_stream.md#0x1_bcs_stream">0x1::bcs_stream</a>;
<b>use</b> <a href="../../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="../../move-stdlib/doc/hash.md#0x1_hash">0x1::hash</a>;
<b>use</b> <a href="single_key.md#0x1_single_key">0x1::single_key</a>;
</code></pre>



<a id="0x1_multi_key_MultiKey"></a>

## Struct `MultiKey`

An *unvalidated*, k out of n MultiKey public key. The <code>bytes</code> field contains (1) a vector of single key public keys and
(2) a single byte encoding the threshold k.
*Unvalidated* means there is no guarantee that the underlying PKs are valid elliptic curve points of non-small
order.  Nor is there a guarantee that it would deserialize correctly (i.e., for Keyless public keys).


<pre><code><b>struct</b> <a href="multi_key.md#0x1_multi_key_MultiKey">MultiKey</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>public_keys: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="single_key.md#0x1_single_key_AnyPublicKey">single_key::AnyPublicKey</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>signatures_required: u8</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_multi_key_SIGNATURE_SCHEME_ID"></a>

The identifier of the MultiEd25519 signature scheme, which is used when deriving Velor authentication keys by hashing
it together with an MultiEd25519 public key.


<pre><code><b>const</b> <a href="multi_key.md#0x1_multi_key_SIGNATURE_SCHEME_ID">SIGNATURE_SCHEME_ID</a>: u8 = 3;
</code></pre>



<a id="0x1_multi_key_MAX_NUMBER_OF_PUBLIC_KEYS"></a>

Max number of ed25519 public keys allowed in multi-ed25519 keys


<pre><code><b>const</b> <a href="multi_key.md#0x1_multi_key_MAX_NUMBER_OF_PUBLIC_KEYS">MAX_NUMBER_OF_PUBLIC_KEYS</a>: u64 = 32;
</code></pre>



<a id="0x1_multi_key_E_INVALID_MULTI_KEY_EXTRA_BYTES"></a>

There are extra bytes in the input when deserializing a MultiKey public key.


<pre><code><b>const</b> <a href="multi_key.md#0x1_multi_key_E_INVALID_MULTI_KEY_EXTRA_BYTES">E_INVALID_MULTI_KEY_EXTRA_BYTES</a>: u64 = 4;
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



<a id="0x1_multi_key_new_public_key_from_bytes"></a>

## Function `new_public_key_from_bytes`

Parses the input bytes into a MultiKey public key.


<pre><code><b>public</b> <b>fun</b> <a href="multi_key.md#0x1_multi_key_new_public_key_from_bytes">new_public_key_from_bytes</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="multi_key.md#0x1_multi_key_MultiKey">multi_key::MultiKey</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multi_key.md#0x1_multi_key_new_public_key_from_bytes">new_public_key_from_bytes</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="multi_key.md#0x1_multi_key_MultiKey">MultiKey</a> {
    <b>let</b> stream = <a href="bcs_stream.md#0x1_bcs_stream_new">bcs_stream::new</a>(bytes);
    <b>let</b> pk = <a href="multi_key.md#0x1_multi_key_deserialize_multi_key">deserialize_multi_key</a>(&<b>mut</b> stream);
    <b>assert</b>!(!<a href="bcs_stream.md#0x1_bcs_stream_has_remaining">bcs_stream::has_remaining</a>(&<b>mut</b> stream), <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="multi_key.md#0x1_multi_key_E_INVALID_MULTI_KEY_EXTRA_BYTES">E_INVALID_MULTI_KEY_EXTRA_BYTES</a>));
    pk
}
</code></pre>



</details>

<a id="0x1_multi_key_new_multi_key_from_single_keys"></a>

## Function `new_multi_key_from_single_keys`

Creates a new MultiKey public key from a vector of single key public keys and a number representing the number of signatures required to authenticate a transaction.


<pre><code><b>public</b> <b>fun</b> <a href="multi_key.md#0x1_multi_key_new_multi_key_from_single_keys">new_multi_key_from_single_keys</a>(single_keys: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="single_key.md#0x1_single_key_AnyPublicKey">single_key::AnyPublicKey</a>&gt;, signatures_required: u8): <a href="multi_key.md#0x1_multi_key_MultiKey">multi_key::MultiKey</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multi_key.md#0x1_multi_key_new_multi_key_from_single_keys">new_multi_key_from_single_keys</a>(single_keys: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="single_key.md#0x1_single_key_AnyPublicKey">single_key::AnyPublicKey</a>&gt;, signatures_required: u8): <a href="multi_key.md#0x1_multi_key_MultiKey">MultiKey</a> {
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
    <a href="multi_key.md#0x1_multi_key_MultiKey">MultiKey</a> { public_keys: single_keys, signatures_required }
}
</code></pre>



</details>

<a id="0x1_multi_key_deserialize_multi_key"></a>

## Function `deserialize_multi_key`

Deserializes a MultiKey public key from a BCS stream.


<pre><code><b>public</b> <b>fun</b> <a href="multi_key.md#0x1_multi_key_deserialize_multi_key">deserialize_multi_key</a>(stream: &<b>mut</b> <a href="bcs_stream.md#0x1_bcs_stream_BCSStream">bcs_stream::BCSStream</a>): <a href="multi_key.md#0x1_multi_key_MultiKey">multi_key::MultiKey</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multi_key.md#0x1_multi_key_deserialize_multi_key">deserialize_multi_key</a>(stream: &<b>mut</b> <a href="bcs_stream.md#0x1_bcs_stream_BCSStream">bcs_stream::BCSStream</a>): <a href="multi_key.md#0x1_multi_key_MultiKey">MultiKey</a> {
    <b>let</b> public_keys = <a href="bcs_stream.md#0x1_bcs_stream_deserialize_vector">bcs_stream::deserialize_vector</a>(stream, |x| <a href="single_key.md#0x1_single_key_deserialize_any_public_key">single_key::deserialize_any_public_key</a>(x));
    <b>let</b> signatures_required = <a href="bcs_stream.md#0x1_bcs_stream_deserialize_u8">bcs_stream::deserialize_u8</a>(stream);
    <a href="multi_key.md#0x1_multi_key_MultiKey">MultiKey</a> { public_keys, signatures_required }
}
</code></pre>



</details>

<a id="0x1_multi_key_to_authentication_key"></a>

## Function `to_authentication_key`

Returns the authentication key for a MultiKey public key.


<pre><code><b>public</b> <b>fun</b> <a href="multi_key.md#0x1_multi_key_to_authentication_key">to_authentication_key</a>(self: &<a href="multi_key.md#0x1_multi_key_MultiKey">multi_key::MultiKey</a>): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multi_key.md#0x1_multi_key_to_authentication_key">to_authentication_key</a>(self: &<a href="multi_key.md#0x1_multi_key_MultiKey">MultiKey</a>): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <b>let</b> pk_bytes = <a href="../../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(self);
    pk_bytes.push_back(<a href="multi_key.md#0x1_multi_key_SIGNATURE_SCHEME_ID">SIGNATURE_SCHEME_ID</a>);
    <a href="../../move-stdlib/doc/hash.md#0x1_hash_sha3_256">hash::sha3_256</a>(pk_bytes)
}
</code></pre>



</details>

<a id="@Specification_1"></a>

## Specification



<pre><code><b>pragma</b> verify = <b>false</b>;
</code></pre>


[move-book]: https://velor.dev/move/book/SUMMARY
