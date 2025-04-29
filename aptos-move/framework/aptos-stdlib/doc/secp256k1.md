
<a id="0x1_secp256k1"></a>

# Module `0x1::secp256k1`

This module implements ECDSA signatures based on the prime-order secp256k1 ellptic curve (i.e., cofactor is 1).


-  [Struct `ECDSARawPublicKey`](#0x1_secp256k1_ECDSARawPublicKey)
-  [Struct `ECDSASignature`](#0x1_secp256k1_ECDSASignature)
-  [Constants](#@Constants_0)
-  [Function `ecdsa_signature_from_bytes`](#0x1_secp256k1_ecdsa_signature_from_bytes)
-  [Function `ecdsa_raw_public_key_from_64_bytes`](#0x1_secp256k1_ecdsa_raw_public_key_from_64_bytes)
-  [Function `ecdsa_raw_public_key_to_bytes`](#0x1_secp256k1_ecdsa_raw_public_key_to_bytes)
-  [Function `ecdsa_signature_to_bytes`](#0x1_secp256k1_ecdsa_signature_to_bytes)
-  [Function `ecdsa_recover`](#0x1_secp256k1_ecdsa_recover)
-  [Function `ecdsa_recover_internal`](#0x1_secp256k1_ecdsa_recover_internal)
-  [Specification](#@Specification_1)
    -  [Function `ecdsa_signature_from_bytes`](#@Specification_1_ecdsa_signature_from_bytes)
    -  [Function `ecdsa_raw_public_key_from_64_bytes`](#@Specification_1_ecdsa_raw_public_key_from_64_bytes)
    -  [Function `ecdsa_raw_public_key_to_bytes`](#@Specification_1_ecdsa_raw_public_key_to_bytes)
    -  [Function `ecdsa_signature_to_bytes`](#@Specification_1_ecdsa_signature_to_bytes)
    -  [Function `ecdsa_recover`](#@Specification_1_ecdsa_recover)
    -  [Function `ecdsa_recover_internal`](#@Specification_1_ecdsa_recover_internal)


<pre><code><b>use</b> <a href="../../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="../../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
</code></pre>



<a id="0x1_secp256k1_ECDSARawPublicKey"></a>

## Struct `ECDSARawPublicKey`

A 64-byte ECDSA public key.


<pre><code><b>struct</b> <a href="secp256k1.md#0x1_secp256k1_ECDSARawPublicKey">ECDSARawPublicKey</a> <b>has</b> <b>copy</b>, drop, store
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

<a id="0x1_secp256k1_ECDSASignature"></a>

## Struct `ECDSASignature`

A 64-byte ECDSA signature.


<pre><code><b>struct</b> <a href="secp256k1.md#0x1_secp256k1_ECDSASignature">ECDSASignature</a> <b>has</b> <b>copy</b>, drop, store
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


<a id="0x1_secp256k1_SIGNATURE_NUM_BYTES"></a>

The size of a secp256k1-based ECDSA signature, in bytes.


<pre><code><b>const</b> <a href="secp256k1.md#0x1_secp256k1_SIGNATURE_NUM_BYTES">SIGNATURE_NUM_BYTES</a>: u64 = 64;
</code></pre>



<a id="0x1_secp256k1_E_DESERIALIZE"></a>

An error occurred while deserializing, for example due to wrong input size.


<pre><code><b>const</b> <a href="secp256k1.md#0x1_secp256k1_E_DESERIALIZE">E_DESERIALIZE</a>: u64 = 1;
</code></pre>



<a id="0x1_secp256k1_RAW_PUBLIC_KEY_NUM_BYTES"></a>

The size of a secp256k1-based ECDSA public key, in bytes.


<pre><code><b>const</b> <a href="secp256k1.md#0x1_secp256k1_RAW_PUBLIC_KEY_NUM_BYTES">RAW_PUBLIC_KEY_NUM_BYTES</a>: u64 = 64;
</code></pre>



<a id="0x1_secp256k1_E_BAD_RECOVERY_ID"></a>

Recovery ID needs to be either 0, 1, 2 or 3. If you are recovering from an (r, s, v) Ethereum signature, take its v value and, set the recovery_id as follows: if v == 27, set to 0, if v == 28, set to 1, if v == 37, set to 0, if v == 38, set to 1.


<pre><code><b>const</b> <a href="secp256k1.md#0x1_secp256k1_E_BAD_RECOVERY_ID">E_BAD_RECOVERY_ID</a>: u64 = 2;
</code></pre>



<a id="0x1_secp256k1_ecdsa_signature_from_bytes"></a>

## Function `ecdsa_signature_from_bytes`

Constructs an ECDSASignature struct from the given 64 bytes.


<pre><code><b>public</b> <b>fun</b> <a href="secp256k1.md#0x1_secp256k1_ecdsa_signature_from_bytes">ecdsa_signature_from_bytes</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="secp256k1.md#0x1_secp256k1_ECDSASignature">secp256k1::ECDSASignature</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="secp256k1.md#0x1_secp256k1_ecdsa_signature_from_bytes">ecdsa_signature_from_bytes</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="secp256k1.md#0x1_secp256k1_ECDSASignature">ECDSASignature</a> {
    <b>assert</b>!(bytes.length() == <a href="secp256k1.md#0x1_secp256k1_SIGNATURE_NUM_BYTES">SIGNATURE_NUM_BYTES</a>, std::error::invalid_argument(<a href="secp256k1.md#0x1_secp256k1_E_DESERIALIZE">E_DESERIALIZE</a>));
    <a href="secp256k1.md#0x1_secp256k1_ECDSASignature">ECDSASignature</a> { bytes }
}
</code></pre>



</details>

<a id="0x1_secp256k1_ecdsa_raw_public_key_from_64_bytes"></a>

## Function `ecdsa_raw_public_key_from_64_bytes`

Constructs an ECDSARawPublicKey struct, given a 64-byte raw representation.


<pre><code><b>public</b> <b>fun</b> <a href="secp256k1.md#0x1_secp256k1_ecdsa_raw_public_key_from_64_bytes">ecdsa_raw_public_key_from_64_bytes</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="secp256k1.md#0x1_secp256k1_ECDSARawPublicKey">secp256k1::ECDSARawPublicKey</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="secp256k1.md#0x1_secp256k1_ecdsa_raw_public_key_from_64_bytes">ecdsa_raw_public_key_from_64_bytes</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="secp256k1.md#0x1_secp256k1_ECDSARawPublicKey">ECDSARawPublicKey</a> {
    <b>assert</b>!(bytes.length() == <a href="secp256k1.md#0x1_secp256k1_RAW_PUBLIC_KEY_NUM_BYTES">RAW_PUBLIC_KEY_NUM_BYTES</a>, std::error::invalid_argument(<a href="secp256k1.md#0x1_secp256k1_E_DESERIALIZE">E_DESERIALIZE</a>));
    <a href="secp256k1.md#0x1_secp256k1_ECDSARawPublicKey">ECDSARawPublicKey</a> { bytes }
}
</code></pre>



</details>

<a id="0x1_secp256k1_ecdsa_raw_public_key_to_bytes"></a>

## Function `ecdsa_raw_public_key_to_bytes`

Serializes an ECDSARawPublicKey struct to 64-bytes.


<pre><code><b>public</b> <b>fun</b> <a href="secp256k1.md#0x1_secp256k1_ecdsa_raw_public_key_to_bytes">ecdsa_raw_public_key_to_bytes</a>(pk: &<a href="secp256k1.md#0x1_secp256k1_ECDSARawPublicKey">secp256k1::ECDSARawPublicKey</a>): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="secp256k1.md#0x1_secp256k1_ecdsa_raw_public_key_to_bytes">ecdsa_raw_public_key_to_bytes</a>(pk: &<a href="secp256k1.md#0x1_secp256k1_ECDSARawPublicKey">ECDSARawPublicKey</a>): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    pk.bytes
}
</code></pre>



</details>

<a id="0x1_secp256k1_ecdsa_signature_to_bytes"></a>

## Function `ecdsa_signature_to_bytes`

Serializes an ECDSASignature struct to 64-bytes.


<pre><code><b>public</b> <b>fun</b> <a href="secp256k1.md#0x1_secp256k1_ecdsa_signature_to_bytes">ecdsa_signature_to_bytes</a>(sig: &<a href="secp256k1.md#0x1_secp256k1_ECDSASignature">secp256k1::ECDSASignature</a>): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="secp256k1.md#0x1_secp256k1_ecdsa_signature_to_bytes">ecdsa_signature_to_bytes</a>(sig: &<a href="secp256k1.md#0x1_secp256k1_ECDSASignature">ECDSASignature</a>): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    sig.bytes
}
</code></pre>



</details>

<a id="0x1_secp256k1_ecdsa_recover"></a>

## Function `ecdsa_recover`

Recovers the signer's raw (64-byte) public key from a secp256k1 ECDSA <code>signature</code> given the (2-bit) <code>recovery_id</code>
and the signed <code>message</code> (32 byte digest).

This recovery algorithm can only be used to check validity of a signature if the signer's public key (or its
hash) is known beforehand. When the algorithm returns a public key <code>pk</code>, this means that the signature in
<code>signature</code> verified on <code>message</code> under that <code>pk</code>. But, again, that is only meaningful if <code>pk</code> is the "right"
one (e.g., in Ethereum, the "right" <code>pk</code> is the one whose hash matches the account's address).

If you do not understand this nuance, please learn more about ECDSA and pubkey recovery (see
https://alinush.github.io/ecdsa#pubkey-recovery), or you risk writing completely-insecure code.

Note: This function does not apply any additional hashing on the <code>message</code>; it simply passes in the message as
raw bytes to the ECDSA recovery function. (The max allowed size ~32 bytes.)
+ Nonetheless, most applications will first hash the message to be signed. So, typically, <code>message</code> here tends
to be a hash rather than an actual message. Therefore, the developer should be aware of what hash function
was used for this.
+ In particular, if using this function to verify an Ethereum signature, you will likely have to input
a keccak256 hash of the message as the <code>message</code> parameter.


<pre><code><b>public</b> <b>fun</b> <a href="secp256k1.md#0x1_secp256k1_ecdsa_recover">ecdsa_recover</a>(message: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, recovery_id: u8, signature: &<a href="secp256k1.md#0x1_secp256k1_ECDSASignature">secp256k1::ECDSASignature</a>): <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="secp256k1.md#0x1_secp256k1_ECDSARawPublicKey">secp256k1::ECDSARawPublicKey</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="secp256k1.md#0x1_secp256k1_ecdsa_recover">ecdsa_recover</a>(
    message: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    recovery_id: u8,
    signature: &<a href="secp256k1.md#0x1_secp256k1_ECDSASignature">ECDSASignature</a>,
): Option&lt;<a href="secp256k1.md#0x1_secp256k1_ECDSARawPublicKey">ECDSARawPublicKey</a>&gt; {

    // If recovery ID is not 0 or 1 or 2 or 3, help the caller out by aborting <b>with</b> `<a href="secp256k1.md#0x1_secp256k1_E_BAD_RECOVERY_ID">E_BAD_RECOVERY_ID</a>`
    <b>if</b>(recovery_id != 0 && recovery_id != 1 && recovery_id != 2 && recovery_id != 3) {
        <b>abort</b> std::error::invalid_argument(<a href="secp256k1.md#0x1_secp256k1_E_BAD_RECOVERY_ID">E_BAD_RECOVERY_ID</a>);
    };

    <b>let</b> (pk, success) = <a href="secp256k1.md#0x1_secp256k1_ecdsa_recover_internal">ecdsa_recover_internal</a>(message, recovery_id, signature.bytes);

    <b>if</b> (success) {
        std::option::some(<a href="secp256k1.md#0x1_secp256k1_ecdsa_raw_public_key_from_64_bytes">ecdsa_raw_public_key_from_64_bytes</a>(pk))
    } <b>else</b> {
        std::option::none&lt;<a href="secp256k1.md#0x1_secp256k1_ECDSARawPublicKey">ECDSARawPublicKey</a>&gt;()
    }
}
</code></pre>



</details>

<a id="0x1_secp256k1_ecdsa_recover_internal"></a>

## Function `ecdsa_recover_internal`

Returns <code>(public_key, <b>true</b>)</code> if <code>signature</code> verifies on <code>message</code> under the recovered <code>public_key</code>
and returns <code>([], <b>false</b>)</code> otherwise.


<pre><code><b>fun</b> <a href="secp256k1.md#0x1_secp256k1_ecdsa_recover_internal">ecdsa_recover_internal</a>(message: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, recovery_id: u8, signature: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): (<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="secp256k1.md#0x1_secp256k1_ecdsa_recover_internal">ecdsa_recover_internal</a>(
    message: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    recovery_id: u8,
    signature: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
): (<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, bool);
</code></pre>



</details>

<a id="@Specification_1"></a>

## Specification


<a id="@Specification_1_ecdsa_signature_from_bytes"></a>

### Function `ecdsa_signature_from_bytes`


<pre><code><b>public</b> <b>fun</b> <a href="secp256k1.md#0x1_secp256k1_ecdsa_signature_from_bytes">ecdsa_signature_from_bytes</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="secp256k1.md#0x1_secp256k1_ECDSASignature">secp256k1::ECDSASignature</a>
</code></pre>




<pre><code><b>aborts_if</b> len(bytes) != <a href="secp256k1.md#0x1_secp256k1_SIGNATURE_NUM_BYTES">SIGNATURE_NUM_BYTES</a>;
<b>ensures</b> result == <a href="secp256k1.md#0x1_secp256k1_ECDSASignature">ECDSASignature</a> { bytes };
</code></pre>



<a id="@Specification_1_ecdsa_raw_public_key_from_64_bytes"></a>

### Function `ecdsa_raw_public_key_from_64_bytes`


<pre><code><b>public</b> <b>fun</b> <a href="secp256k1.md#0x1_secp256k1_ecdsa_raw_public_key_from_64_bytes">ecdsa_raw_public_key_from_64_bytes</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="secp256k1.md#0x1_secp256k1_ECDSARawPublicKey">secp256k1::ECDSARawPublicKey</a>
</code></pre>




<pre><code><b>aborts_if</b> len(bytes) != <a href="secp256k1.md#0x1_secp256k1_RAW_PUBLIC_KEY_NUM_BYTES">RAW_PUBLIC_KEY_NUM_BYTES</a>;
<b>ensures</b> result == <a href="secp256k1.md#0x1_secp256k1_ECDSARawPublicKey">ECDSARawPublicKey</a> { bytes };
</code></pre>



<a id="@Specification_1_ecdsa_raw_public_key_to_bytes"></a>

### Function `ecdsa_raw_public_key_to_bytes`


<pre><code><b>public</b> <b>fun</b> <a href="secp256k1.md#0x1_secp256k1_ecdsa_raw_public_key_to_bytes">ecdsa_raw_public_key_to_bytes</a>(pk: &<a href="secp256k1.md#0x1_secp256k1_ECDSARawPublicKey">secp256k1::ECDSARawPublicKey</a>): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;
<b>ensures</b> result == pk.bytes;
</code></pre>



<a id="@Specification_1_ecdsa_signature_to_bytes"></a>

### Function `ecdsa_signature_to_bytes`


<pre><code><b>public</b> <b>fun</b> <a href="secp256k1.md#0x1_secp256k1_ecdsa_signature_to_bytes">ecdsa_signature_to_bytes</a>(sig: &<a href="secp256k1.md#0x1_secp256k1_ECDSASignature">secp256k1::ECDSASignature</a>): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;
<b>ensures</b> result == sig.bytes;
</code></pre>



<a id="@Specification_1_ecdsa_recover"></a>

### Function `ecdsa_recover`


<pre><code><b>public</b> <b>fun</b> <a href="secp256k1.md#0x1_secp256k1_ecdsa_recover">ecdsa_recover</a>(message: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, recovery_id: u8, signature: &<a href="secp256k1.md#0x1_secp256k1_ECDSASignature">secp256k1::ECDSASignature</a>): <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="secp256k1.md#0x1_secp256k1_ECDSARawPublicKey">secp256k1::ECDSARawPublicKey</a>&gt;
</code></pre>




<pre><code><b>aborts_if</b> recovery_id &gt; 3;
<b>aborts_if</b> <a href="secp256k1.md#0x1_secp256k1_ecdsa_recover_internal_abort_condition">ecdsa_recover_internal_abort_condition</a>(message, recovery_id, signature.bytes);
<b>let</b> pk = <a href="secp256k1.md#0x1_secp256k1_spec_ecdsa_recover_internal_result_1">spec_ecdsa_recover_internal_result_1</a>(message, recovery_id, signature.bytes);
<b>let</b> success = <a href="secp256k1.md#0x1_secp256k1_spec_ecdsa_recover_internal_result_2">spec_ecdsa_recover_internal_result_2</a>(message, recovery_id, signature.bytes);
<b>ensures</b> success ==&gt; result == std::option::spec_some(<a href="secp256k1.md#0x1_secp256k1_ecdsa_raw_public_key_from_64_bytes">ecdsa_raw_public_key_from_64_bytes</a>(pk));
<b>ensures</b> !success ==&gt; result == std::option::spec_none&lt;<a href="secp256k1.md#0x1_secp256k1_ECDSARawPublicKey">ECDSARawPublicKey</a>&gt;();
</code></pre>



<a id="@Specification_1_ecdsa_recover_internal"></a>

### Function `ecdsa_recover_internal`


<pre><code><b>fun</b> <a href="secp256k1.md#0x1_secp256k1_ecdsa_recover_internal">ecdsa_recover_internal</a>(message: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, recovery_id: u8, signature: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): (<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, bool)
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>aborts_if</b> <a href="secp256k1.md#0x1_secp256k1_ecdsa_recover_internal_abort_condition">ecdsa_recover_internal_abort_condition</a>(message, recovery_id, signature);
<b>ensures</b> result_1 == <a href="secp256k1.md#0x1_secp256k1_spec_ecdsa_recover_internal_result_1">spec_ecdsa_recover_internal_result_1</a>(message, recovery_id, signature);
<b>ensures</b> result_2 == <a href="secp256k1.md#0x1_secp256k1_spec_ecdsa_recover_internal_result_2">spec_ecdsa_recover_internal_result_2</a>(message, recovery_id, signature);
<b>ensures</b> len(result_1) == <b>if</b> (result_2) { <a href="secp256k1.md#0x1_secp256k1_RAW_PUBLIC_KEY_NUM_BYTES">RAW_PUBLIC_KEY_NUM_BYTES</a> } <b>else</b> { 0 };
</code></pre>




<a id="0x1_secp256k1_ecdsa_recover_internal_abort_condition"></a>


<pre><code><b>fun</b> <a href="secp256k1.md#0x1_secp256k1_ecdsa_recover_internal_abort_condition">ecdsa_recover_internal_abort_condition</a>(message: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, recovery_id: u8, signature: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool;
</code></pre>




<a id="0x1_secp256k1_spec_ecdsa_recover_internal_result_1"></a>


<pre><code><b>fun</b> <a href="secp256k1.md#0x1_secp256k1_spec_ecdsa_recover_internal_result_1">spec_ecdsa_recover_internal_result_1</a>(message: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, recovery_id: u8, signature: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;
</code></pre>




<a id="0x1_secp256k1_spec_ecdsa_recover_internal_result_2"></a>


<pre><code><b>fun</b> <a href="secp256k1.md#0x1_secp256k1_spec_ecdsa_recover_internal_result_2">spec_ecdsa_recover_internal_result_2</a>(message: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, recovery_id: u8, signature: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool;
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
