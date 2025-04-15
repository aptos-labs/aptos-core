
<a id="0x1_keyless"></a>

# Module `0x1::keyless`

This module implements the Keyless authentication scheme.


-  [Struct `PublicKey`](#0x1_keyless_PublicKey)
-  [Constants](#@Constants_0)
-  [Function `new_public_key_from_bytes`](#0x1_keyless_new_public_key_from_bytes)
-  [Function `deserialize_public_key`](#0x1_keyless_deserialize_public_key)
-  [Function `new`](#0x1_keyless_new)
-  [Function `get_iss`](#0x1_keyless_get_iss)
-  [Function `get_idc`](#0x1_keyless_get_idc)
-  [Specification](#@Specification_1)


<pre><code><b>use</b> <a href="bcs_stream.md#0x1_bcs_stream">0x1::bcs_stream</a>;
<b>use</b> <a href="../../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="../../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;
</code></pre>



<a id="0x1_keyless_PublicKey"></a>

## Struct `PublicKey`

An *unvalidated* any public key: not necessarily an elliptic curve point, just a sequence of 32 bytes


<pre><code><b>struct</b> <a href="keyless.md#0x1_keyless_PublicKey">PublicKey</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>iss: <a href="../../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
<dt>
<code>idc: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_keyless_E_INVALID_ID_COMMITMENT_BYTES_LENGTH"></a>

The length of the identifier commitment bytes in a Keyless public key is invalid.


<pre><code><b>const</b> <a href="keyless.md#0x1_keyless_E_INVALID_ID_COMMITMENT_BYTES_LENGTH">E_INVALID_ID_COMMITMENT_BYTES_LENGTH</a>: u64 = 2;
</code></pre>



<a id="0x1_keyless_E_INVALID_ISSUER_UTF8_BYTES_LENGTH"></a>

The length of the issuer string in a Keyless public key is invalid.


<pre><code><b>const</b> <a href="keyless.md#0x1_keyless_E_INVALID_ISSUER_UTF8_BYTES_LENGTH">E_INVALID_ISSUER_UTF8_BYTES_LENGTH</a>: u64 = 3;
</code></pre>



<a id="0x1_keyless_E_INVALID_KEYLESS_PUBLIC_KEY_EXTRA_BYTES"></a>

There are extra bytes in the input when deserializing a Keyless public key.


<pre><code><b>const</b> <a href="keyless.md#0x1_keyless_E_INVALID_KEYLESS_PUBLIC_KEY_EXTRA_BYTES">E_INVALID_KEYLESS_PUBLIC_KEY_EXTRA_BYTES</a>: u64 = 1;
</code></pre>



<a id="0x1_keyless_ID_COMMITMENT_BYTES_LENGTH"></a>

The length of the identifier commitment bytes in a Keyless public key.


<pre><code><b>const</b> <a href="keyless.md#0x1_keyless_ID_COMMITMENT_BYTES_LENGTH">ID_COMMITMENT_BYTES_LENGTH</a>: u64 = 32;
</code></pre>



<a id="0x1_keyless_MAX_ISSUER_UTF8_BYTES_LENGTH"></a>

The maximum length of the issuer string in bytes in a Keyless public key.


<pre><code><b>const</b> <a href="keyless.md#0x1_keyless_MAX_ISSUER_UTF8_BYTES_LENGTH">MAX_ISSUER_UTF8_BYTES_LENGTH</a>: u64 = 120;
</code></pre>



<a id="0x1_keyless_new_public_key_from_bytes"></a>

## Function `new_public_key_from_bytes`

Parses the input bytes into a keyless public key.


<pre><code><b>public</b> <b>fun</b> <a href="keyless.md#0x1_keyless_new_public_key_from_bytes">new_public_key_from_bytes</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="keyless.md#0x1_keyless_PublicKey">keyless::PublicKey</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="keyless.md#0x1_keyless_new_public_key_from_bytes">new_public_key_from_bytes</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="keyless.md#0x1_keyless_PublicKey">PublicKey</a> {
    <b>let</b> stream = <a href="bcs_stream.md#0x1_bcs_stream_new">bcs_stream::new</a>(bytes);
    <b>let</b> key = <a href="keyless.md#0x1_keyless_deserialize_public_key">deserialize_public_key</a>(&<b>mut</b> stream);
    <b>assert</b>!(!<a href="bcs_stream.md#0x1_bcs_stream_has_remaining">bcs_stream::has_remaining</a>(&<b>mut</b> stream), <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="keyless.md#0x1_keyless_E_INVALID_KEYLESS_PUBLIC_KEY_EXTRA_BYTES">E_INVALID_KEYLESS_PUBLIC_KEY_EXTRA_BYTES</a>));
    key
}
</code></pre>



</details>

<a id="0x1_keyless_deserialize_public_key"></a>

## Function `deserialize_public_key`

Deserializes a keyless public key from a BCS stream.


<pre><code><b>public</b> <b>fun</b> <a href="keyless.md#0x1_keyless_deserialize_public_key">deserialize_public_key</a>(stream: &<b>mut</b> <a href="bcs_stream.md#0x1_bcs_stream_BCSStream">bcs_stream::BCSStream</a>): <a href="keyless.md#0x1_keyless_PublicKey">keyless::PublicKey</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="keyless.md#0x1_keyless_deserialize_public_key">deserialize_public_key</a>(stream: &<b>mut</b> <a href="bcs_stream.md#0x1_bcs_stream_BCSStream">bcs_stream::BCSStream</a>): <a href="keyless.md#0x1_keyless_PublicKey">PublicKey</a> {
    <b>let</b> iss = <a href="bcs_stream.md#0x1_bcs_stream_deserialize_string">bcs_stream::deserialize_string</a>(stream);
    <b>let</b> idc = <a href="bcs_stream.md#0x1_bcs_stream_deserialize_vector">bcs_stream::deserialize_vector</a>(stream, |x| deserialize_u8(x));
    <a href="keyless.md#0x1_keyless_new">new</a>(iss, idc)
}
</code></pre>



</details>

<a id="0x1_keyless_new"></a>

## Function `new`

Creates a new keyless public key from an issuer string and an identifier bytes.


<pre><code><b>public</b> <b>fun</b> <a href="keyless.md#0x1_keyless_new">new</a>(iss: <a href="../../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, idc: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="keyless.md#0x1_keyless_PublicKey">keyless::PublicKey</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="keyless.md#0x1_keyless_new">new</a>(iss: String, idc: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="keyless.md#0x1_keyless_PublicKey">PublicKey</a> {
    <b>assert</b>!(<a href="../../move-stdlib/doc/string.md#0x1_string_bytes">string::bytes</a>(&iss).length() &lt;= <a href="keyless.md#0x1_keyless_MAX_ISSUER_UTF8_BYTES_LENGTH">MAX_ISSUER_UTF8_BYTES_LENGTH</a>, <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="keyless.md#0x1_keyless_E_INVALID_ISSUER_UTF8_BYTES_LENGTH">E_INVALID_ISSUER_UTF8_BYTES_LENGTH</a>));
    <b>assert</b>!(idc.length() == <a href="keyless.md#0x1_keyless_ID_COMMITMENT_BYTES_LENGTH">ID_COMMITMENT_BYTES_LENGTH</a>, <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="keyless.md#0x1_keyless_E_INVALID_ID_COMMITMENT_BYTES_LENGTH">E_INVALID_ID_COMMITMENT_BYTES_LENGTH</a>));
    <a href="keyless.md#0x1_keyless_PublicKey">PublicKey</a> { iss, idc }
}
</code></pre>



</details>

<a id="0x1_keyless_get_iss"></a>

## Function `get_iss`

Returns the issuer string of the public key


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="keyless.md#0x1_keyless_get_iss">get_iss</a>(self: &<a href="keyless.md#0x1_keyless_PublicKey">keyless::PublicKey</a>): <a href="../../move-stdlib/doc/string.md#0x1_string_String">string::String</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>friend</b> <b>fun</b> <a href="keyless.md#0x1_keyless_get_iss">get_iss</a>(self: &<a href="keyless.md#0x1_keyless_PublicKey">PublicKey</a>): String {
    self.iss
}
</code></pre>



</details>

<a id="0x1_keyless_get_idc"></a>

## Function `get_idc`

Returns the identifier bytes of the public key


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="keyless.md#0x1_keyless_get_idc">get_idc</a>(self: &<a href="keyless.md#0x1_keyless_PublicKey">keyless::PublicKey</a>): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>friend</b> <b>fun</b> <a href="keyless.md#0x1_keyless_get_idc">get_idc</a>(self: &<a href="keyless.md#0x1_keyless_PublicKey">PublicKey</a>): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    self.idc
}
</code></pre>



</details>

<a id="@Specification_1"></a>

## Specification



<pre><code><b>pragma</b> verify = <b>false</b>;
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
