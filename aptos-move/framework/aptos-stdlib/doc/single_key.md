
<a id="0x1_single_key"></a>

# Module `0x1::single_key`



-  [Struct `UnvalidatedPublicKey`](#0x1_single_key_UnvalidatedPublicKey)
-  [Constants](#@Constants_0)
-  [Function `new_unvalidated_public_key_from_bytes`](#0x1_single_key_new_unvalidated_public_key_from_bytes)
-  [Function `unvalidated_public_key_to_bytes`](#0x1_single_key_unvalidated_public_key_to_bytes)
-  [Function `from_ed25519_public_key_unvalidated`](#0x1_single_key_from_ed25519_public_key_unvalidated)
-  [Function `unvalidated_public_key_to_authentication_key`](#0x1_single_key_unvalidated_public_key_to_authentication_key)
-  [Function `public_key_bytes_to_authentication_key`](#0x1_single_key_public_key_bytes_to_authentication_key)


<pre><code><b>use</b> <a href="ed25519.md#0x1_ed25519">0x1::ed25519</a>;
<b>use</b> <a href="../../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="../../move-stdlib/doc/hash.md#0x1_hash">0x1::hash</a>;
<b>use</b> <a href="../../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
</code></pre>



<a id="0x1_single_key_UnvalidatedPublicKey"></a>

## Struct `UnvalidatedPublicKey`

An *unvalidated* any public key: not necessarily an elliptic curve point, just a sequence of 32 bytes


<pre><code><b>struct</b> <a href="single_key.md#0x1_single_key_UnvalidatedPublicKey">UnvalidatedPublicKey</a> <b>has</b> <b>copy</b>, drop, store
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


<a id="0x1_single_key_SIGNATURE_SCHEME_ID"></a>

The identifier of the Single Key signature scheme, which is used when deriving Aptos authentication keys by hashing
it together with an Single Key public key.


<pre><code><b>const</b> <a href="single_key.md#0x1_single_key_SIGNATURE_SCHEME_ID">SIGNATURE_SCHEME_ID</a>: u8 = 3;
</code></pre>



<a id="0x1_single_key_ED25519_PUBLIC_KEY_TYPE"></a>

Scheme identifier for Ed25519 single keys.


<pre><code><b>const</b> <a href="single_key.md#0x1_single_key_ED25519_PUBLIC_KEY_TYPE">ED25519_PUBLIC_KEY_TYPE</a>: u8 = 0;
</code></pre>



<a id="0x1_single_key_E_FAILED_TO_DESERIALIZE"></a>

Failed to deserialize the public key.


<pre><code><b>const</b> <a href="single_key.md#0x1_single_key_E_FAILED_TO_DESERIALIZE">E_FAILED_TO_DESERIALIZE</a>: u64 = 2;
</code></pre>



<a id="0x1_single_key_E_INVALID_PUBLIC_KEY_TYPE"></a>

Wrong number of bytes were given as input when deserializing an Ed25519 public key.


<pre><code><b>const</b> <a href="single_key.md#0x1_single_key_E_INVALID_PUBLIC_KEY_TYPE">E_INVALID_PUBLIC_KEY_TYPE</a>: u64 = 1;
</code></pre>



<a id="0x1_single_key_E_INVALID_SIGNATURE_SCHEME"></a>

Wrong number of bytes were given as input when deserializing an Ed25519 signature.


<pre><code><b>const</b> <a href="single_key.md#0x1_single_key_E_INVALID_SIGNATURE_SCHEME">E_INVALID_SIGNATURE_SCHEME</a>: u64 = 3;
</code></pre>



<a id="0x1_single_key_FEDERATED_KEYLESS_PUBLIC_KEY_TYPE"></a>

Scheme identifier for Federated Keyless single keys.


<pre><code><b>const</b> <a href="single_key.md#0x1_single_key_FEDERATED_KEYLESS_PUBLIC_KEY_TYPE">FEDERATED_KEYLESS_PUBLIC_KEY_TYPE</a>: u8 = 4;
</code></pre>



<a id="0x1_single_key_KEYLESS_PUBLIC_KEY_TYPE"></a>

Scheme identifier for Keyless single keys.


<pre><code><b>const</b> <a href="single_key.md#0x1_single_key_KEYLESS_PUBLIC_KEY_TYPE">KEYLESS_PUBLIC_KEY_TYPE</a>: u8 = 3;
</code></pre>



<a id="0x1_single_key_SECP256K1_PUBLIC_KEY_TYPE"></a>

Scheme identifier for SECP256K1 single keys.


<pre><code><b>const</b> <a href="single_key.md#0x1_single_key_SECP256K1_PUBLIC_KEY_TYPE">SECP256K1_PUBLIC_KEY_TYPE</a>: u8 = 1;
</code></pre>



<a id="0x1_single_key_WEB_AUTHN_PUBLIC_KEY_TYPE"></a>

Scheme identifier for WebAuthn single keys.


<pre><code><b>const</b> <a href="single_key.md#0x1_single_key_WEB_AUTHN_PUBLIC_KEY_TYPE">WEB_AUTHN_PUBLIC_KEY_TYPE</a>: u8 = 2;
</code></pre>



<a id="0x1_single_key_new_unvalidated_public_key_from_bytes"></a>

## Function `new_unvalidated_public_key_from_bytes`

Parses the input bytes as an *unvalidated* single key.  It does check that the first byte is a valid scheme identifier.


<pre><code><b>public</b> <b>fun</b> <a href="single_key.md#0x1_single_key_new_unvalidated_public_key_from_bytes">new_unvalidated_public_key_from_bytes</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="single_key.md#0x1_single_key_UnvalidatedPublicKey">single_key::UnvalidatedPublicKey</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="single_key.md#0x1_single_key_new_unvalidated_public_key_from_bytes">new_unvalidated_public_key_from_bytes</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="single_key.md#0x1_single_key_UnvalidatedPublicKey">UnvalidatedPublicKey</a> {
    <b>let</b> first_byte = bytes[0];
    <b>assert</b>!(first_byte &lt;= 4, std::error::invalid_argument(<a href="single_key.md#0x1_single_key_E_INVALID_PUBLIC_KEY_TYPE">E_INVALID_PUBLIC_KEY_TYPE</a>));
    <a href="single_key.md#0x1_single_key_UnvalidatedPublicKey">UnvalidatedPublicKey</a> { bytes }
}
</code></pre>



</details>

<a id="0x1_single_key_unvalidated_public_key_to_bytes"></a>

## Function `unvalidated_public_key_to_bytes`

Serializes an UnvalidatedPublicKey struct to 32-bytes.


<pre><code><b>public</b> <b>fun</b> <a href="single_key.md#0x1_single_key_unvalidated_public_key_to_bytes">unvalidated_public_key_to_bytes</a>(pk: &<a href="single_key.md#0x1_single_key_UnvalidatedPublicKey">single_key::UnvalidatedPublicKey</a>): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="single_key.md#0x1_single_key_unvalidated_public_key_to_bytes">unvalidated_public_key_to_bytes</a>(pk: &<a href="single_key.md#0x1_single_key_UnvalidatedPublicKey">UnvalidatedPublicKey</a>): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    pk.bytes
}
</code></pre>



</details>

<a id="0x1_single_key_from_ed25519_public_key_unvalidated"></a>

## Function `from_ed25519_public_key_unvalidated`

Converts an unvalidated Ed25519 public key to an unvalidated single key public key.
We do this by prepending the scheme identifier and the length of the public key (32 bytes or 0x20 in hex) to
the public key bytes.


<pre><code><b>public</b> <b>fun</b> <a href="single_key.md#0x1_single_key_from_ed25519_public_key_unvalidated">from_ed25519_public_key_unvalidated</a>(pk: &<a href="ed25519.md#0x1_ed25519_UnvalidatedPublicKey">ed25519::UnvalidatedPublicKey</a>): <a href="single_key.md#0x1_single_key_UnvalidatedPublicKey">single_key::UnvalidatedPublicKey</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="single_key.md#0x1_single_key_from_ed25519_public_key_unvalidated">from_ed25519_public_key_unvalidated</a>(pk: &<a href="ed25519.md#0x1_ed25519_UnvalidatedPublicKey">ed25519::UnvalidatedPublicKey</a>): <a href="single_key.md#0x1_single_key_UnvalidatedPublicKey">UnvalidatedPublicKey</a> {
    <b>let</b> pk_bytes = <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];
    pk_bytes.push_back(<a href="single_key.md#0x1_single_key_ED25519_PUBLIC_KEY_TYPE">ED25519_PUBLIC_KEY_TYPE</a>);
    pk_bytes.push_back(0x20);
    std::vector::append(&<b>mut</b> pk_bytes, <a href="ed25519.md#0x1_ed25519_unvalidated_public_key_to_bytes">ed25519::unvalidated_public_key_to_bytes</a>(pk));
    <a href="single_key.md#0x1_single_key_UnvalidatedPublicKey">UnvalidatedPublicKey</a> {
        bytes: pk_bytes
    }
}
</code></pre>



</details>

<a id="0x1_single_key_unvalidated_public_key_to_authentication_key"></a>

## Function `unvalidated_public_key_to_authentication_key`

Derives the Aptos-specific authentication key of the given single key public key.


<pre><code><b>public</b> <b>fun</b> <a href="single_key.md#0x1_single_key_unvalidated_public_key_to_authentication_key">unvalidated_public_key_to_authentication_key</a>(pk: &<a href="single_key.md#0x1_single_key_UnvalidatedPublicKey">single_key::UnvalidatedPublicKey</a>): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="single_key.md#0x1_single_key_unvalidated_public_key_to_authentication_key">unvalidated_public_key_to_authentication_key</a>(pk: &<a href="single_key.md#0x1_single_key_UnvalidatedPublicKey">UnvalidatedPublicKey</a>): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <a href="single_key.md#0x1_single_key_public_key_bytes_to_authentication_key">public_key_bytes_to_authentication_key</a>(pk.bytes)
}
</code></pre>



</details>

<a id="0x1_single_key_public_key_bytes_to_authentication_key"></a>

## Function `public_key_bytes_to_authentication_key`

Derives the Aptos-specific authentication key of the given bytes of a single key public key.


<pre><code><b>fun</b> <a href="single_key.md#0x1_single_key_public_key_bytes_to_authentication_key">public_key_bytes_to_authentication_key</a>(pk_bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="single_key.md#0x1_single_key_public_key_bytes_to_authentication_key">public_key_bytes_to_authentication_key</a>(pk_bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    pk_bytes.push_back(<a href="single_key.md#0x1_single_key_SIGNATURE_SCHEME_ID">SIGNATURE_SCHEME_ID</a>);
    <a href="../../move-stdlib/doc/hash.md#0x1_hash_sha3_256">hash::sha3_256</a>(pk_bytes)
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
