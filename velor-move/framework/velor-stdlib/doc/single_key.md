
<a id="0x1_single_key"></a>

# Module `0x1::single_key`

This module implements Single Key representations of public keys.
It is used to represent public keys for the Ed25519, SECP256K1, WebAuthn, and Keyless schemes in a unified way.


-  [Enum `AnyPublicKey`](#0x1_single_key_AnyPublicKey)
-  [Constants](#@Constants_0)
-  [Function `new_public_key_from_bytes`](#0x1_single_key_new_public_key_from_bytes)
-  [Function `deserialize_any_public_key`](#0x1_single_key_deserialize_any_public_key)
-  [Function `is_keyless_or_federated_keyless_public_key`](#0x1_single_key_is_keyless_or_federated_keyless_public_key)
-  [Function `from_ed25519_public_key_unvalidated`](#0x1_single_key_from_ed25519_public_key_unvalidated)
-  [Function `to_authentication_key`](#0x1_single_key_to_authentication_key)
-  [Specification](#@Specification_1)


<pre><code><b>use</b> <a href="../../move-stdlib/doc/bcs.md#0x1_bcs">0x1::bcs</a>;
<b>use</b> <a href="bcs_stream.md#0x1_bcs_stream">0x1::bcs_stream</a>;
<b>use</b> <a href="ed25519.md#0x1_ed25519">0x1::ed25519</a>;
<b>use</b> <a href="../../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="federated_keyless.md#0x1_federated_keyless">0x1::federated_keyless</a>;
<b>use</b> <a href="../../move-stdlib/doc/hash.md#0x1_hash">0x1::hash</a>;
<b>use</b> <a href="keyless.md#0x1_keyless">0x1::keyless</a>;
<b>use</b> <a href="secp256k1.md#0x1_secp256k1">0x1::secp256k1</a>;
<b>use</b> <a href="secp256r1.md#0x1_secp256r1">0x1::secp256r1</a>;
</code></pre>



<a id="0x1_single_key_AnyPublicKey"></a>

## Enum `AnyPublicKey`



<pre><code>enum <a href="single_key.md#0x1_single_key_AnyPublicKey">AnyPublicKey</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>Ed25519</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>pk: <a href="ed25519.md#0x1_ed25519_UnvalidatedPublicKey">ed25519::UnvalidatedPublicKey</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

<details>
<summary>Secp256k1Ecdsa</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>pk: <a href="secp256k1.md#0x1_secp256k1_ECDSARawPublicKey">secp256k1::ECDSARawPublicKey</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

<details>
<summary>Secp256r1Ecdsa</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>pk: <a href="secp256r1.md#0x1_secp256r1_ECDSARawPublicKey">secp256r1::ECDSARawPublicKey</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

<details>
<summary>Keyless</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>pk: <a href="keyless.md#0x1_keyless_PublicKey">keyless::PublicKey</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

<details>
<summary>FederatedKeyless</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>pk: <a href="federated_keyless.md#0x1_federated_keyless_PublicKey">federated_keyless::PublicKey</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_single_key_SIGNATURE_SCHEME_ID"></a>

The identifier of the Single Key signature scheme, which is used when deriving Velor authentication keys by hashing
it together with an Single Key public key.


<pre><code><b>const</b> <a href="single_key.md#0x1_single_key_SIGNATURE_SCHEME_ID">SIGNATURE_SCHEME_ID</a>: u8 = 2;
</code></pre>



<a id="0x1_single_key_ED25519_PUBLIC_KEY_TYPE"></a>

Scheme identifier for Ed25519 single keys.


<pre><code><b>const</b> <a href="single_key.md#0x1_single_key_ED25519_PUBLIC_KEY_TYPE">ED25519_PUBLIC_KEY_TYPE</a>: u8 = 0;
</code></pre>



<a id="0x1_single_key_E_INVALID_PUBLIC_KEY_TYPE"></a>

Unrecognized public key type.


<pre><code><b>const</b> <a href="single_key.md#0x1_single_key_E_INVALID_PUBLIC_KEY_TYPE">E_INVALID_PUBLIC_KEY_TYPE</a>: u64 = 1;
</code></pre>



<a id="0x1_single_key_E_INVALID_SINGLE_KEY_EXTRA_BYTES"></a>

There are extra bytes in the input when deserializing a Single Key public key.


<pre><code><b>const</b> <a href="single_key.md#0x1_single_key_E_INVALID_SINGLE_KEY_EXTRA_BYTES">E_INVALID_SINGLE_KEY_EXTRA_BYTES</a>: u64 = 2;
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



<a id="0x1_single_key_new_public_key_from_bytes"></a>

## Function `new_public_key_from_bytes`

Parses the input bytes as a AnyPublicKey. The public key bytes are not guaranteed to be a valid
representation of a point on its corresponding curve if applicable.
It does check that the bytes deserialize into a well-formed public key for the given scheme.


<pre><code><b>public</b> <b>fun</b> <a href="single_key.md#0x1_single_key_new_public_key_from_bytes">new_public_key_from_bytes</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="single_key.md#0x1_single_key_AnyPublicKey">single_key::AnyPublicKey</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="single_key.md#0x1_single_key_new_public_key_from_bytes">new_public_key_from_bytes</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="single_key.md#0x1_single_key_AnyPublicKey">AnyPublicKey</a> {
    <b>let</b> stream = <a href="bcs_stream.md#0x1_bcs_stream_new">bcs_stream::new</a>(bytes);
    <b>let</b> pk = <a href="single_key.md#0x1_single_key_deserialize_any_public_key">deserialize_any_public_key</a>(&<b>mut</b> stream);
    <b>assert</b>!(!<a href="bcs_stream.md#0x1_bcs_stream_has_remaining">bcs_stream::has_remaining</a>(&<b>mut</b> stream), <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="single_key.md#0x1_single_key_E_INVALID_SINGLE_KEY_EXTRA_BYTES">E_INVALID_SINGLE_KEY_EXTRA_BYTES</a>));
    pk
}
</code></pre>



</details>

<a id="0x1_single_key_deserialize_any_public_key"></a>

## Function `deserialize_any_public_key`

Deserializes a Single Key public key from a BCS stream.


<pre><code><b>public</b> <b>fun</b> <a href="single_key.md#0x1_single_key_deserialize_any_public_key">deserialize_any_public_key</a>(stream: &<b>mut</b> <a href="bcs_stream.md#0x1_bcs_stream_BCSStream">bcs_stream::BCSStream</a>): <a href="single_key.md#0x1_single_key_AnyPublicKey">single_key::AnyPublicKey</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="single_key.md#0x1_single_key_deserialize_any_public_key">deserialize_any_public_key</a>(stream: &<b>mut</b> <a href="bcs_stream.md#0x1_bcs_stream_BCSStream">bcs_stream::BCSStream</a>): <a href="single_key.md#0x1_single_key_AnyPublicKey">AnyPublicKey</a> {
    <b>let</b> scheme_id = <a href="bcs_stream.md#0x1_bcs_stream_deserialize_u8">bcs_stream::deserialize_u8</a>(stream);
    <b>let</b> pk: <a href="single_key.md#0x1_single_key_AnyPublicKey">AnyPublicKey</a>;
    <b>if</b> (scheme_id == <a href="single_key.md#0x1_single_key_ED25519_PUBLIC_KEY_TYPE">ED25519_PUBLIC_KEY_TYPE</a>) {
        <b>let</b> public_key_bytes = <a href="bcs_stream.md#0x1_bcs_stream_deserialize_vector">bcs_stream::deserialize_vector</a>(stream, |x| deserialize_u8(x));
        pk = AnyPublicKey::Ed25519{pk: <a href="ed25519.md#0x1_ed25519_new_unvalidated_public_key_from_bytes">ed25519::new_unvalidated_public_key_from_bytes</a>(public_key_bytes)}
    } <b>else</b> <b>if</b> (scheme_id == <a href="single_key.md#0x1_single_key_SECP256K1_PUBLIC_KEY_TYPE">SECP256K1_PUBLIC_KEY_TYPE</a>) {
        <b>let</b> public_key_bytes = <a href="bcs_stream.md#0x1_bcs_stream_deserialize_vector">bcs_stream::deserialize_vector</a>(stream, |x| deserialize_u8(x));
        pk = AnyPublicKey::Secp256k1Ecdsa{pk: <a href="secp256k1.md#0x1_secp256k1_ecdsa_raw_public_key_from_64_bytes">secp256k1::ecdsa_raw_public_key_from_64_bytes</a>(public_key_bytes)};
    } <b>else</b> <b>if</b> (scheme_id == <a href="single_key.md#0x1_single_key_WEB_AUTHN_PUBLIC_KEY_TYPE">WEB_AUTHN_PUBLIC_KEY_TYPE</a>) {
        <b>let</b> public_key_bytes = <a href="bcs_stream.md#0x1_bcs_stream_deserialize_vector">bcs_stream::deserialize_vector</a>(stream, |x| deserialize_u8(x));
        pk = AnyPublicKey::Secp256r1Ecdsa{pk: <a href="secp256r1.md#0x1_secp256r1_ecdsa_raw_public_key_from_64_bytes">secp256r1::ecdsa_raw_public_key_from_64_bytes</a>(public_key_bytes)};
    } <b>else</b> <b>if</b> (scheme_id == <a href="single_key.md#0x1_single_key_KEYLESS_PUBLIC_KEY_TYPE">KEYLESS_PUBLIC_KEY_TYPE</a>) {
        pk = AnyPublicKey::Keyless{pk: <a href="keyless.md#0x1_keyless_deserialize_public_key">keyless::deserialize_public_key</a>(stream)};
    } <b>else</b> <b>if</b> (scheme_id == <a href="single_key.md#0x1_single_key_FEDERATED_KEYLESS_PUBLIC_KEY_TYPE">FEDERATED_KEYLESS_PUBLIC_KEY_TYPE</a>) {
        pk = AnyPublicKey::FederatedKeyless{pk: <a href="federated_keyless.md#0x1_federated_keyless_deserialize_public_key">federated_keyless::deserialize_public_key</a>(stream)}
    } <b>else</b> {
        <b>abort</b> <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="single_key.md#0x1_single_key_E_INVALID_PUBLIC_KEY_TYPE">E_INVALID_PUBLIC_KEY_TYPE</a>);
    };
    pk
}
</code></pre>



</details>

<a id="0x1_single_key_is_keyless_or_federated_keyless_public_key"></a>

## Function `is_keyless_or_federated_keyless_public_key`

Returns true if the public key is a keyless or federated keyless public key.


<pre><code><b>public</b> <b>fun</b> <a href="single_key.md#0x1_single_key_is_keyless_or_federated_keyless_public_key">is_keyless_or_federated_keyless_public_key</a>(pk: &<a href="single_key.md#0x1_single_key_AnyPublicKey">single_key::AnyPublicKey</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="single_key.md#0x1_single_key_is_keyless_or_federated_keyless_public_key">is_keyless_or_federated_keyless_public_key</a>(pk: &<a href="single_key.md#0x1_single_key_AnyPublicKey">AnyPublicKey</a>): bool {
    match (pk) {
        AnyPublicKey::Keyless { .. } =&gt; <b>true</b>,
        AnyPublicKey::FederatedKeyless { .. } =&gt; <b>true</b>,
        _ =&gt; <b>false</b>
    }
}
</code></pre>



</details>

<a id="0x1_single_key_from_ed25519_public_key_unvalidated"></a>

## Function `from_ed25519_public_key_unvalidated`

Converts an unvalidated Ed25519 public key to an AnyPublicKey.


<pre><code><b>public</b> <b>fun</b> <a href="single_key.md#0x1_single_key_from_ed25519_public_key_unvalidated">from_ed25519_public_key_unvalidated</a>(pk: <a href="ed25519.md#0x1_ed25519_UnvalidatedPublicKey">ed25519::UnvalidatedPublicKey</a>): <a href="single_key.md#0x1_single_key_AnyPublicKey">single_key::AnyPublicKey</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="single_key.md#0x1_single_key_from_ed25519_public_key_unvalidated">from_ed25519_public_key_unvalidated</a>(pk: <a href="ed25519.md#0x1_ed25519_UnvalidatedPublicKey">ed25519::UnvalidatedPublicKey</a>): <a href="single_key.md#0x1_single_key_AnyPublicKey">AnyPublicKey</a> {
    AnyPublicKey::Ed25519 { pk }
}
</code></pre>



</details>

<a id="0x1_single_key_to_authentication_key"></a>

## Function `to_authentication_key`

Gets the authentication key for the AnyPublicKey.


<pre><code><b>public</b> <b>fun</b> <a href="single_key.md#0x1_single_key_to_authentication_key">to_authentication_key</a>(self: &<a href="single_key.md#0x1_single_key_AnyPublicKey">single_key::AnyPublicKey</a>): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="single_key.md#0x1_single_key_to_authentication_key">to_authentication_key</a>(self: &<a href="single_key.md#0x1_single_key_AnyPublicKey">AnyPublicKey</a>): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <b>let</b> pk_bytes = <a href="../../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(self);
    pk_bytes.push_back(<a href="single_key.md#0x1_single_key_SIGNATURE_SCHEME_ID">SIGNATURE_SCHEME_ID</a>);
    <a href="../../move-stdlib/doc/hash.md#0x1_hash_sha3_256">hash::sha3_256</a>(pk_bytes)
}
</code></pre>



</details>

<a id="@Specification_1"></a>

## Specification



<pre><code><b>pragma</b> verify = <b>false</b>;
</code></pre>


[move-book]: https://velor.dev/move/book/SUMMARY
