
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


<pre><code>use 0x1::error;
use 0x1::option;
</code></pre>



<a id="0x1_secp256k1_ECDSARawPublicKey"></a>

## Struct `ECDSARawPublicKey`

A 64-byte ECDSA public key.


<pre><code>struct ECDSARawPublicKey has copy, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>bytes: vector&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_secp256k1_ECDSASignature"></a>

## Struct `ECDSASignature`

A 64-byte ECDSA signature.


<pre><code>struct ECDSASignature has copy, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>bytes: vector&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_secp256k1_SIGNATURE_NUM_BYTES"></a>

The size of a secp256k1-based ECDSA signature, in bytes.


<pre><code>const SIGNATURE_NUM_BYTES: u64 &#61; 64;
</code></pre>



<a id="0x1_secp256k1_E_DESERIALIZE"></a>

An error occurred while deserializing, for example due to wrong input size.


<pre><code>const E_DESERIALIZE: u64 &#61; 1;
</code></pre>



<a id="0x1_secp256k1_RAW_PUBLIC_KEY_NUM_BYTES"></a>

The size of a secp256k1-based ECDSA public key, in bytes.


<pre><code>const RAW_PUBLIC_KEY_NUM_BYTES: u64 &#61; 64;
</code></pre>



<a id="0x1_secp256k1_ecdsa_signature_from_bytes"></a>

## Function `ecdsa_signature_from_bytes`

Constructs an ECDSASignature struct from the given 64 bytes.


<pre><code>public fun ecdsa_signature_from_bytes(bytes: vector&lt;u8&gt;): secp256k1::ECDSASignature
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun ecdsa_signature_from_bytes(bytes: vector&lt;u8&gt;): ECDSASignature &#123;
    assert!(std::vector::length(&amp;bytes) &#61;&#61; SIGNATURE_NUM_BYTES, std::error::invalid_argument(E_DESERIALIZE));
    ECDSASignature &#123; bytes &#125;
&#125;
</code></pre>



</details>

<a id="0x1_secp256k1_ecdsa_raw_public_key_from_64_bytes"></a>

## Function `ecdsa_raw_public_key_from_64_bytes`

Constructs an ECDSARawPublicKey struct, given a 64-byte raw representation.


<pre><code>public fun ecdsa_raw_public_key_from_64_bytes(bytes: vector&lt;u8&gt;): secp256k1::ECDSARawPublicKey
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun ecdsa_raw_public_key_from_64_bytes(bytes: vector&lt;u8&gt;): ECDSARawPublicKey &#123;
    assert!(std::vector::length(&amp;bytes) &#61;&#61; RAW_PUBLIC_KEY_NUM_BYTES, std::error::invalid_argument(E_DESERIALIZE));
    ECDSARawPublicKey &#123; bytes &#125;
&#125;
</code></pre>



</details>

<a id="0x1_secp256k1_ecdsa_raw_public_key_to_bytes"></a>

## Function `ecdsa_raw_public_key_to_bytes`

Serializes an ECDSARawPublicKey struct to 64-bytes.


<pre><code>public fun ecdsa_raw_public_key_to_bytes(pk: &amp;secp256k1::ECDSARawPublicKey): vector&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun ecdsa_raw_public_key_to_bytes(pk: &amp;ECDSARawPublicKey): vector&lt;u8&gt; &#123;
    pk.bytes
&#125;
</code></pre>



</details>

<a id="0x1_secp256k1_ecdsa_signature_to_bytes"></a>

## Function `ecdsa_signature_to_bytes`

Serializes an ECDSASignature struct to 64-bytes.


<pre><code>public fun ecdsa_signature_to_bytes(sig: &amp;secp256k1::ECDSASignature): vector&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun ecdsa_signature_to_bytes(sig: &amp;ECDSASignature): vector&lt;u8&gt; &#123;
    sig.bytes
&#125;
</code></pre>



</details>

<a id="0x1_secp256k1_ecdsa_recover"></a>

## Function `ecdsa_recover`

Recovers the signer's raw (64-byte) public key from a secp256k1 ECDSA <code>signature</code> given the <code>recovery_id</code> and the signed
<code>message</code> (32 byte digest).

Note that an invalid signature, or a signature from a different message, will result in the recovery of an
incorrect public key. This recovery algorithm can only be used to check validity of a signature if the signer's
public key (or its hash) is known beforehand.


<pre><code>public fun ecdsa_recover(message: vector&lt;u8&gt;, recovery_id: u8, signature: &amp;secp256k1::ECDSASignature): option::Option&lt;secp256k1::ECDSARawPublicKey&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun ecdsa_recover(
    message: vector&lt;u8&gt;,
    recovery_id: u8,
    signature: &amp;ECDSASignature,
): Option&lt;ECDSARawPublicKey&gt; &#123;
    let (pk, success) &#61; ecdsa_recover_internal(message, recovery_id, signature.bytes);
    if (success) &#123;
        std::option::some(ecdsa_raw_public_key_from_64_bytes(pk))
    &#125; else &#123;
        std::option::none&lt;ECDSARawPublicKey&gt;()
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_secp256k1_ecdsa_recover_internal"></a>

## Function `ecdsa_recover_internal`

Returns <code>(public_key, true)</code> if <code>signature</code> verifies on <code>message</code> under the recovered <code>public_key</code>
and returns <code>([], false)</code> otherwise.


<pre><code>fun ecdsa_recover_internal(message: vector&lt;u8&gt;, recovery_id: u8, signature: vector&lt;u8&gt;): (vector&lt;u8&gt;, bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun ecdsa_recover_internal(
    message: vector&lt;u8&gt;,
    recovery_id: u8,
    signature: vector&lt;u8&gt;
): (vector&lt;u8&gt;, bool);
</code></pre>



</details>

<a id="@Specification_1"></a>

## Specification


<a id="@Specification_1_ecdsa_signature_from_bytes"></a>

### Function `ecdsa_signature_from_bytes`


<pre><code>public fun ecdsa_signature_from_bytes(bytes: vector&lt;u8&gt;): secp256k1::ECDSASignature
</code></pre>




<pre><code>aborts_if len(bytes) !&#61; SIGNATURE_NUM_BYTES;
ensures result &#61;&#61; ECDSASignature &#123; bytes &#125;;
</code></pre>



<a id="@Specification_1_ecdsa_raw_public_key_from_64_bytes"></a>

### Function `ecdsa_raw_public_key_from_64_bytes`


<pre><code>public fun ecdsa_raw_public_key_from_64_bytes(bytes: vector&lt;u8&gt;): secp256k1::ECDSARawPublicKey
</code></pre>




<pre><code>aborts_if len(bytes) !&#61; RAW_PUBLIC_KEY_NUM_BYTES;
ensures result &#61;&#61; ECDSARawPublicKey &#123; bytes &#125;;
</code></pre>



<a id="@Specification_1_ecdsa_raw_public_key_to_bytes"></a>

### Function `ecdsa_raw_public_key_to_bytes`


<pre><code>public fun ecdsa_raw_public_key_to_bytes(pk: &amp;secp256k1::ECDSARawPublicKey): vector&lt;u8&gt;
</code></pre>




<pre><code>aborts_if false;
ensures result &#61;&#61; pk.bytes;
</code></pre>



<a id="@Specification_1_ecdsa_signature_to_bytes"></a>

### Function `ecdsa_signature_to_bytes`


<pre><code>public fun ecdsa_signature_to_bytes(sig: &amp;secp256k1::ECDSASignature): vector&lt;u8&gt;
</code></pre>




<pre><code>aborts_if false;
ensures result &#61;&#61; sig.bytes;
</code></pre>



<a id="@Specification_1_ecdsa_recover"></a>

### Function `ecdsa_recover`


<pre><code>public fun ecdsa_recover(message: vector&lt;u8&gt;, recovery_id: u8, signature: &amp;secp256k1::ECDSASignature): option::Option&lt;secp256k1::ECDSARawPublicKey&gt;
</code></pre>




<pre><code>aborts_if ecdsa_recover_internal_abort_condition(message, recovery_id, signature.bytes);
let pk &#61; spec_ecdsa_recover_internal_result_1(message, recovery_id, signature.bytes);
let success &#61; spec_ecdsa_recover_internal_result_2(message, recovery_id, signature.bytes);
ensures success &#61;&#61;&gt; result &#61;&#61; std::option::spec_some(ecdsa_raw_public_key_from_64_bytes(pk));
ensures !success &#61;&#61;&gt; result &#61;&#61; std::option::spec_none&lt;ECDSARawPublicKey&gt;();
</code></pre>



<a id="@Specification_1_ecdsa_recover_internal"></a>

### Function `ecdsa_recover_internal`


<pre><code>fun ecdsa_recover_internal(message: vector&lt;u8&gt;, recovery_id: u8, signature: vector&lt;u8&gt;): (vector&lt;u8&gt;, bool)
</code></pre>




<pre><code>pragma opaque;
aborts_if ecdsa_recover_internal_abort_condition(message, recovery_id, signature);
ensures result_1 &#61;&#61; spec_ecdsa_recover_internal_result_1(message, recovery_id, signature);
ensures result_2 &#61;&#61; spec_ecdsa_recover_internal_result_2(message, recovery_id, signature);
ensures len(result_1) &#61;&#61; if (result_2) &#123; RAW_PUBLIC_KEY_NUM_BYTES &#125; else &#123; 0 &#125;;
</code></pre>




<a id="0x1_secp256k1_ecdsa_recover_internal_abort_condition"></a>


<pre><code>fun ecdsa_recover_internal_abort_condition(message: vector&lt;u8&gt;, recovery_id: u8, signature: vector&lt;u8&gt;): bool;
</code></pre>




<a id="0x1_secp256k1_spec_ecdsa_recover_internal_result_1"></a>


<pre><code>fun spec_ecdsa_recover_internal_result_1(message: vector&lt;u8&gt;, recovery_id: u8, signature: vector&lt;u8&gt;): vector&lt;u8&gt;;
</code></pre>




<a id="0x1_secp256k1_spec_ecdsa_recover_internal_result_2"></a>


<pre><code>fun spec_ecdsa_recover_internal_result_2(message: vector&lt;u8&gt;, recovery_id: u8, signature: vector&lt;u8&gt;): bool;
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
