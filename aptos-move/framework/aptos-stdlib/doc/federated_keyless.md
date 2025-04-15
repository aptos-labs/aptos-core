
<a id="0x1_federated_keyless"></a>

# Module `0x1::federated_keyless`

This module implements the Federated Keyless authentication scheme.


-  [Struct `PublicKey`](#0x1_federated_keyless_PublicKey)
-  [Constants](#@Constants_0)
-  [Function `new_public_key_from_bytes`](#0x1_federated_keyless_new_public_key_from_bytes)
-  [Function `deserialize_public_key`](#0x1_federated_keyless_deserialize_public_key)
-  [Function `new`](#0x1_federated_keyless_new)
-  [Function `get_jwk_address`](#0x1_federated_keyless_get_jwk_address)
-  [Function `get_keyless_public_key`](#0x1_federated_keyless_get_keyless_public_key)
-  [Specification](#@Specification_1)


<pre><code><b>use</b> <a href="bcs_stream.md#0x1_bcs_stream">0x1::bcs_stream</a>;
<b>use</b> <a href="../../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="keyless.md#0x1_keyless">0x1::keyless</a>;
</code></pre>



<a id="0x1_federated_keyless_PublicKey"></a>

## Struct `PublicKey`

An *unvalidated* any public key: not necessarily an elliptic curve point, just a sequence of 32 bytes


<pre><code><b>struct</b> <a href="federated_keyless.md#0x1_federated_keyless_PublicKey">PublicKey</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>jwk_address: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>keyless_public_key: <a href="keyless.md#0x1_keyless_PublicKey">keyless::PublicKey</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_federated_keyless_E_INVALID_FEDERATED_KEYLESS_PUBLIC_KEY_EXTRA_BYTES"></a>

There are extra bytes in the input when deserializing a Federated Keyless public key.


<pre><code><b>const</b> <a href="federated_keyless.md#0x1_federated_keyless_E_INVALID_FEDERATED_KEYLESS_PUBLIC_KEY_EXTRA_BYTES">E_INVALID_FEDERATED_KEYLESS_PUBLIC_KEY_EXTRA_BYTES</a>: u64 = 1;
</code></pre>



<a id="0x1_federated_keyless_new_public_key_from_bytes"></a>

## Function `new_public_key_from_bytes`

Parses the input bytes into a keyless public key.


<pre><code><b>public</b> <b>fun</b> <a href="federated_keyless.md#0x1_federated_keyless_new_public_key_from_bytes">new_public_key_from_bytes</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="federated_keyless.md#0x1_federated_keyless_PublicKey">federated_keyless::PublicKey</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="federated_keyless.md#0x1_federated_keyless_new_public_key_from_bytes">new_public_key_from_bytes</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="federated_keyless.md#0x1_federated_keyless_PublicKey">PublicKey</a> {
    <b>let</b> stream = <a href="bcs_stream.md#0x1_bcs_stream_new">bcs_stream::new</a>(bytes);
    <b>let</b> pk = <a href="federated_keyless.md#0x1_federated_keyless_deserialize_public_key">deserialize_public_key</a>(&<b>mut</b> stream);
    <b>assert</b>!(!<a href="bcs_stream.md#0x1_bcs_stream_has_remaining">bcs_stream::has_remaining</a>(&<b>mut</b> stream), <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="federated_keyless.md#0x1_federated_keyless_E_INVALID_FEDERATED_KEYLESS_PUBLIC_KEY_EXTRA_BYTES">E_INVALID_FEDERATED_KEYLESS_PUBLIC_KEY_EXTRA_BYTES</a>));
    pk
}
</code></pre>



</details>

<a id="0x1_federated_keyless_deserialize_public_key"></a>

## Function `deserialize_public_key`

Deserializes a Federated Keyless public key from a BCS stream.


<pre><code><b>public</b> <b>fun</b> <a href="federated_keyless.md#0x1_federated_keyless_deserialize_public_key">deserialize_public_key</a>(stream: &<b>mut</b> <a href="bcs_stream.md#0x1_bcs_stream_BCSStream">bcs_stream::BCSStream</a>): <a href="federated_keyless.md#0x1_federated_keyless_PublicKey">federated_keyless::PublicKey</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="federated_keyless.md#0x1_federated_keyless_deserialize_public_key">deserialize_public_key</a>(stream: &<b>mut</b> <a href="bcs_stream.md#0x1_bcs_stream_BCSStream">bcs_stream::BCSStream</a>): <a href="federated_keyless.md#0x1_federated_keyless_PublicKey">PublicKey</a> {
    <b>let</b> jwk_address = <a href="bcs_stream.md#0x1_bcs_stream_deserialize_address">bcs_stream::deserialize_address</a>(stream);
    <b>let</b> keyless_public_key = <a href="keyless.md#0x1_keyless_deserialize_public_key">keyless::deserialize_public_key</a>(stream);
    <a href="federated_keyless.md#0x1_federated_keyless_PublicKey">PublicKey</a> { keyless_public_key, jwk_address }
}
</code></pre>



</details>

<a id="0x1_federated_keyless_new"></a>

## Function `new`

Creates a new Federated Keyless public key from a keyless public key and a JWK address.


<pre><code><b>public</b> <b>fun</b> <a href="federated_keyless.md#0x1_federated_keyless_new">new</a>(keyless_public_key: <a href="keyless.md#0x1_keyless_PublicKey">keyless::PublicKey</a>, jwk_address: <b>address</b>): <a href="federated_keyless.md#0x1_federated_keyless_PublicKey">federated_keyless::PublicKey</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="federated_keyless.md#0x1_federated_keyless_new">new</a>(keyless_public_key: <a href="keyless.md#0x1_keyless_PublicKey">keyless::PublicKey</a>, jwk_address: <b>address</b>): <a href="federated_keyless.md#0x1_federated_keyless_PublicKey">PublicKey</a> {
    <a href="federated_keyless.md#0x1_federated_keyless_PublicKey">PublicKey</a> { keyless_public_key, jwk_address }
}
</code></pre>



</details>

<a id="0x1_federated_keyless_get_jwk_address"></a>

## Function `get_jwk_address`

Returns the identifier bytes of the public key


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="federated_keyless.md#0x1_federated_keyless_get_jwk_address">get_jwk_address</a>(self: &<a href="federated_keyless.md#0x1_federated_keyless_PublicKey">federated_keyless::PublicKey</a>): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>friend</b> <b>fun</b> <a href="federated_keyless.md#0x1_federated_keyless_get_jwk_address">get_jwk_address</a>(self: &<a href="federated_keyless.md#0x1_federated_keyless_PublicKey">PublicKey</a>): <b>address</b> {
    self.jwk_address
}
</code></pre>



</details>

<a id="0x1_federated_keyless_get_keyless_public_key"></a>

## Function `get_keyless_public_key`

Returns the keyless public key of the public key


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="federated_keyless.md#0x1_federated_keyless_get_keyless_public_key">get_keyless_public_key</a>(self: &<a href="federated_keyless.md#0x1_federated_keyless_PublicKey">federated_keyless::PublicKey</a>): <a href="keyless.md#0x1_keyless_PublicKey">keyless::PublicKey</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>friend</b> <b>fun</b> <a href="federated_keyless.md#0x1_federated_keyless_get_keyless_public_key">get_keyless_public_key</a>(self: &<a href="federated_keyless.md#0x1_federated_keyless_PublicKey">PublicKey</a>): <a href="keyless.md#0x1_keyless_PublicKey">keyless::PublicKey</a> {
    self.keyless_public_key
}
</code></pre>



</details>

<a id="@Specification_1"></a>

## Specification



<pre><code><b>pragma</b> verify = <b>false</b>;
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
