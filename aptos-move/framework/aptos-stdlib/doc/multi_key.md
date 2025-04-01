
<a id="0x1_multi_key"></a>

# Module `0x1::multi_key`



-  [Struct `UnvalidatedPublicKey`](#0x1_multi_key_UnvalidatedPublicKey)
-  [Constants](#@Constants_0)
-  [Function `new_unvalidated_public_key_from_bytes`](#0x1_multi_key_new_unvalidated_public_key_from_bytes)
-  [Function `unvalidated_public_key_to_authentication_key`](#0x1_multi_key_unvalidated_public_key_to_authentication_key)
-  [Function `public_key_bytes_to_authentication_key`](#0x1_multi_key_public_key_bytes_to_authentication_key)


<pre><code><b>use</b> <a href="../../move-stdlib/doc/hash.md#0x1_hash">0x1::hash</a>;
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



<a id="0x1_multi_key_new_unvalidated_public_key_from_bytes"></a>

## Function `new_unvalidated_public_key_from_bytes`



<pre><code><b>public</b> <b>fun</b> <a href="multi_key.md#0x1_multi_key_new_unvalidated_public_key_from_bytes">new_unvalidated_public_key_from_bytes</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="multi_key.md#0x1_multi_key_UnvalidatedPublicKey">multi_key::UnvalidatedPublicKey</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multi_key.md#0x1_multi_key_new_unvalidated_public_key_from_bytes">new_unvalidated_public_key_from_bytes</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="multi_key.md#0x1_multi_key_UnvalidatedPublicKey">UnvalidatedPublicKey</a> {
    <a href="multi_key.md#0x1_multi_key_UnvalidatedPublicKey">UnvalidatedPublicKey</a> {
        bytes: bytes
    }
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
    <a href="../../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> pk_bytes, <a href="multi_key.md#0x1_multi_key_SIGNATURE_SCHEME_ID">SIGNATURE_SCHEME_ID</a>);
    std::hash::sha3_256(pk_bytes)
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
