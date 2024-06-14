
<a id="0x1_secp256k1"></a>

# Module `0x1::secp256k1`

This module implements ECDSA signatures based on the prime&#45;order secp256k1 ellptic curve (i.e., cofactor is 1).


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


<pre><code><b>use</b> <a href="../../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;<br /><b>use</b> <a href="../../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;<br /></code></pre>



<a id="0x1_secp256k1_ECDSARawPublicKey"></a>

## Struct `ECDSARawPublicKey`

A 64&#45;byte ECDSA public key.


<pre><code><b>struct</b> <a href="secp256k1.md#0x1_secp256k1_ECDSARawPublicKey">ECDSARawPublicKey</a> <b>has</b> <b>copy</b>, drop, store<br /></code></pre>



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

A 64&#45;byte ECDSA signature.


<pre><code><b>struct</b> <a href="secp256k1.md#0x1_secp256k1_ECDSASignature">ECDSASignature</a> <b>has</b> <b>copy</b>, drop, store<br /></code></pre>



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

The size of a secp256k1&#45;based ECDSA signature, in bytes.


<pre><code><b>const</b> <a href="secp256k1.md#0x1_secp256k1_SIGNATURE_NUM_BYTES">SIGNATURE_NUM_BYTES</a>: u64 &#61; 64;<br /></code></pre>



<a id="0x1_secp256k1_E_DESERIALIZE"></a>

An error occurred while deserializing, for example due to wrong input size.


<pre><code><b>const</b> <a href="secp256k1.md#0x1_secp256k1_E_DESERIALIZE">E_DESERIALIZE</a>: u64 &#61; 1;<br /></code></pre>



<a id="0x1_secp256k1_RAW_PUBLIC_KEY_NUM_BYTES"></a>

The size of a secp256k1&#45;based ECDSA public key, in bytes.


<pre><code><b>const</b> <a href="secp256k1.md#0x1_secp256k1_RAW_PUBLIC_KEY_NUM_BYTES">RAW_PUBLIC_KEY_NUM_BYTES</a>: u64 &#61; 64;<br /></code></pre>



<a id="0x1_secp256k1_ecdsa_signature_from_bytes"></a>

## Function `ecdsa_signature_from_bytes`

Constructs an ECDSASignature struct from the given 64 bytes.


<pre><code><b>public</b> <b>fun</b> <a href="secp256k1.md#0x1_secp256k1_ecdsa_signature_from_bytes">ecdsa_signature_from_bytes</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="secp256k1.md#0x1_secp256k1_ECDSASignature">secp256k1::ECDSASignature</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="secp256k1.md#0x1_secp256k1_ecdsa_signature_from_bytes">ecdsa_signature_from_bytes</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="secp256k1.md#0x1_secp256k1_ECDSASignature">ECDSASignature</a> &#123;<br />    <b>assert</b>!(std::vector::length(&amp;bytes) &#61;&#61; <a href="secp256k1.md#0x1_secp256k1_SIGNATURE_NUM_BYTES">SIGNATURE_NUM_BYTES</a>, std::error::invalid_argument(<a href="secp256k1.md#0x1_secp256k1_E_DESERIALIZE">E_DESERIALIZE</a>));<br />    <a href="secp256k1.md#0x1_secp256k1_ECDSASignature">ECDSASignature</a> &#123; bytes &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_secp256k1_ecdsa_raw_public_key_from_64_bytes"></a>

## Function `ecdsa_raw_public_key_from_64_bytes`

Constructs an ECDSARawPublicKey struct, given a 64&#45;byte raw representation.


<pre><code><b>public</b> <b>fun</b> <a href="secp256k1.md#0x1_secp256k1_ecdsa_raw_public_key_from_64_bytes">ecdsa_raw_public_key_from_64_bytes</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="secp256k1.md#0x1_secp256k1_ECDSARawPublicKey">secp256k1::ECDSARawPublicKey</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="secp256k1.md#0x1_secp256k1_ecdsa_raw_public_key_from_64_bytes">ecdsa_raw_public_key_from_64_bytes</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="secp256k1.md#0x1_secp256k1_ECDSARawPublicKey">ECDSARawPublicKey</a> &#123;<br />    <b>assert</b>!(std::vector::length(&amp;bytes) &#61;&#61; <a href="secp256k1.md#0x1_secp256k1_RAW_PUBLIC_KEY_NUM_BYTES">RAW_PUBLIC_KEY_NUM_BYTES</a>, std::error::invalid_argument(<a href="secp256k1.md#0x1_secp256k1_E_DESERIALIZE">E_DESERIALIZE</a>));<br />    <a href="secp256k1.md#0x1_secp256k1_ECDSARawPublicKey">ECDSARawPublicKey</a> &#123; bytes &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_secp256k1_ecdsa_raw_public_key_to_bytes"></a>

## Function `ecdsa_raw_public_key_to_bytes`

Serializes an ECDSARawPublicKey struct to 64&#45;bytes.


<pre><code><b>public</b> <b>fun</b> <a href="secp256k1.md#0x1_secp256k1_ecdsa_raw_public_key_to_bytes">ecdsa_raw_public_key_to_bytes</a>(pk: &amp;<a href="secp256k1.md#0x1_secp256k1_ECDSARawPublicKey">secp256k1::ECDSARawPublicKey</a>): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="secp256k1.md#0x1_secp256k1_ecdsa_raw_public_key_to_bytes">ecdsa_raw_public_key_to_bytes</a>(pk: &amp;<a href="secp256k1.md#0x1_secp256k1_ECDSARawPublicKey">ECDSARawPublicKey</a>): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; &#123;<br />    pk.bytes<br />&#125;<br /></code></pre>



</details>

<a id="0x1_secp256k1_ecdsa_signature_to_bytes"></a>

## Function `ecdsa_signature_to_bytes`

Serializes an ECDSASignature struct to 64&#45;bytes.


<pre><code><b>public</b> <b>fun</b> <a href="secp256k1.md#0x1_secp256k1_ecdsa_signature_to_bytes">ecdsa_signature_to_bytes</a>(sig: &amp;<a href="secp256k1.md#0x1_secp256k1_ECDSASignature">secp256k1::ECDSASignature</a>): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="secp256k1.md#0x1_secp256k1_ecdsa_signature_to_bytes">ecdsa_signature_to_bytes</a>(sig: &amp;<a href="secp256k1.md#0x1_secp256k1_ECDSASignature">ECDSASignature</a>): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; &#123;<br />    sig.bytes<br />&#125;<br /></code></pre>



</details>

<a id="0x1_secp256k1_ecdsa_recover"></a>

## Function `ecdsa_recover`

Recovers the signer&apos;s raw (64&#45;byte) public key from a secp256k1 ECDSA <code>signature</code> given the <code>recovery_id</code> and the signed
<code>message</code> (32 byte digest).

Note that an invalid signature, or a signature from a different message, will result in the recovery of an
incorrect public key. This recovery algorithm can only be used to check validity of a signature if the signer&apos;s
public key (or its hash) is known beforehand.


<pre><code><b>public</b> <b>fun</b> <a href="secp256k1.md#0x1_secp256k1_ecdsa_recover">ecdsa_recover</a>(message: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, recovery_id: u8, signature: &amp;<a href="secp256k1.md#0x1_secp256k1_ECDSASignature">secp256k1::ECDSASignature</a>): <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="secp256k1.md#0x1_secp256k1_ECDSARawPublicKey">secp256k1::ECDSARawPublicKey</a>&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="secp256k1.md#0x1_secp256k1_ecdsa_recover">ecdsa_recover</a>(<br />    message: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,<br />    recovery_id: u8,<br />    signature: &amp;<a href="secp256k1.md#0x1_secp256k1_ECDSASignature">ECDSASignature</a>,<br />): Option&lt;<a href="secp256k1.md#0x1_secp256k1_ECDSARawPublicKey">ECDSARawPublicKey</a>&gt; &#123;<br />    <b>let</b> (pk, success) &#61; <a href="secp256k1.md#0x1_secp256k1_ecdsa_recover_internal">ecdsa_recover_internal</a>(message, recovery_id, signature.bytes);<br />    <b>if</b> (success) &#123;<br />        std::option::some(<a href="secp256k1.md#0x1_secp256k1_ecdsa_raw_public_key_from_64_bytes">ecdsa_raw_public_key_from_64_bytes</a>(pk))<br />    &#125; <b>else</b> &#123;<br />        std::option::none&lt;<a href="secp256k1.md#0x1_secp256k1_ECDSARawPublicKey">ECDSARawPublicKey</a>&gt;()<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_secp256k1_ecdsa_recover_internal"></a>

## Function `ecdsa_recover_internal`

Returns <code>(public_key, <b>true</b>)</code> if <code>signature</code> verifies on <code>message</code> under the recovered <code>public_key</code>
and returns <code>([], <b>false</b>)</code> otherwise.


<pre><code><b>fun</b> <a href="secp256k1.md#0x1_secp256k1_ecdsa_recover_internal">ecdsa_recover_internal</a>(message: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, recovery_id: u8, signature: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): (<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, bool)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="secp256k1.md#0x1_secp256k1_ecdsa_recover_internal">ecdsa_recover_internal</a>(<br />    message: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,<br />    recovery_id: u8,<br />    signature: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;<br />): (<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, bool);<br /></code></pre>



</details>

<a id="@Specification_1"></a>

## Specification


<a id="@Specification_1_ecdsa_signature_from_bytes"></a>

### Function `ecdsa_signature_from_bytes`


<pre><code><b>public</b> <b>fun</b> <a href="secp256k1.md#0x1_secp256k1_ecdsa_signature_from_bytes">ecdsa_signature_from_bytes</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="secp256k1.md#0x1_secp256k1_ECDSASignature">secp256k1::ECDSASignature</a><br /></code></pre>




<pre><code><b>aborts_if</b> len(bytes) !&#61; <a href="secp256k1.md#0x1_secp256k1_SIGNATURE_NUM_BYTES">SIGNATURE_NUM_BYTES</a>;<br /><b>ensures</b> result &#61;&#61; <a href="secp256k1.md#0x1_secp256k1_ECDSASignature">ECDSASignature</a> &#123; bytes &#125;;<br /></code></pre>



<a id="@Specification_1_ecdsa_raw_public_key_from_64_bytes"></a>

### Function `ecdsa_raw_public_key_from_64_bytes`


<pre><code><b>public</b> <b>fun</b> <a href="secp256k1.md#0x1_secp256k1_ecdsa_raw_public_key_from_64_bytes">ecdsa_raw_public_key_from_64_bytes</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="secp256k1.md#0x1_secp256k1_ECDSARawPublicKey">secp256k1::ECDSARawPublicKey</a><br /></code></pre>




<pre><code><b>aborts_if</b> len(bytes) !&#61; <a href="secp256k1.md#0x1_secp256k1_RAW_PUBLIC_KEY_NUM_BYTES">RAW_PUBLIC_KEY_NUM_BYTES</a>;<br /><b>ensures</b> result &#61;&#61; <a href="secp256k1.md#0x1_secp256k1_ECDSARawPublicKey">ECDSARawPublicKey</a> &#123; bytes &#125;;<br /></code></pre>



<a id="@Specification_1_ecdsa_raw_public_key_to_bytes"></a>

### Function `ecdsa_raw_public_key_to_bytes`


<pre><code><b>public</b> <b>fun</b> <a href="secp256k1.md#0x1_secp256k1_ecdsa_raw_public_key_to_bytes">ecdsa_raw_public_key_to_bytes</a>(pk: &amp;<a href="secp256k1.md#0x1_secp256k1_ECDSARawPublicKey">secp256k1::ECDSARawPublicKey</a>): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;<br /></code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;<br /><b>ensures</b> result &#61;&#61; pk.bytes;<br /></code></pre>



<a id="@Specification_1_ecdsa_signature_to_bytes"></a>

### Function `ecdsa_signature_to_bytes`


<pre><code><b>public</b> <b>fun</b> <a href="secp256k1.md#0x1_secp256k1_ecdsa_signature_to_bytes">ecdsa_signature_to_bytes</a>(sig: &amp;<a href="secp256k1.md#0x1_secp256k1_ECDSASignature">secp256k1::ECDSASignature</a>): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;<br /></code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;<br /><b>ensures</b> result &#61;&#61; sig.bytes;<br /></code></pre>



<a id="@Specification_1_ecdsa_recover"></a>

### Function `ecdsa_recover`


<pre><code><b>public</b> <b>fun</b> <a href="secp256k1.md#0x1_secp256k1_ecdsa_recover">ecdsa_recover</a>(message: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, recovery_id: u8, signature: &amp;<a href="secp256k1.md#0x1_secp256k1_ECDSASignature">secp256k1::ECDSASignature</a>): <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="secp256k1.md#0x1_secp256k1_ECDSARawPublicKey">secp256k1::ECDSARawPublicKey</a>&gt;<br /></code></pre>




<pre><code><b>aborts_if</b> <a href="secp256k1.md#0x1_secp256k1_ecdsa_recover_internal_abort_condition">ecdsa_recover_internal_abort_condition</a>(message, recovery_id, signature.bytes);<br /><b>let</b> pk &#61; <a href="secp256k1.md#0x1_secp256k1_spec_ecdsa_recover_internal_result_1">spec_ecdsa_recover_internal_result_1</a>(message, recovery_id, signature.bytes);<br /><b>let</b> success &#61; <a href="secp256k1.md#0x1_secp256k1_spec_ecdsa_recover_internal_result_2">spec_ecdsa_recover_internal_result_2</a>(message, recovery_id, signature.bytes);<br /><b>ensures</b> success &#61;&#61;&gt; result &#61;&#61; std::option::spec_some(<a href="secp256k1.md#0x1_secp256k1_ecdsa_raw_public_key_from_64_bytes">ecdsa_raw_public_key_from_64_bytes</a>(pk));<br /><b>ensures</b> !success &#61;&#61;&gt; result &#61;&#61; std::option::spec_none&lt;<a href="secp256k1.md#0x1_secp256k1_ECDSARawPublicKey">ECDSARawPublicKey</a>&gt;();<br /></code></pre>



<a id="@Specification_1_ecdsa_recover_internal"></a>

### Function `ecdsa_recover_internal`


<pre><code><b>fun</b> <a href="secp256k1.md#0x1_secp256k1_ecdsa_recover_internal">ecdsa_recover_internal</a>(message: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, recovery_id: u8, signature: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): (<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, bool)<br /></code></pre>




<pre><code><b>pragma</b> opaque;<br /><b>aborts_if</b> <a href="secp256k1.md#0x1_secp256k1_ecdsa_recover_internal_abort_condition">ecdsa_recover_internal_abort_condition</a>(message, recovery_id, signature);<br /><b>ensures</b> result_1 &#61;&#61; <a href="secp256k1.md#0x1_secp256k1_spec_ecdsa_recover_internal_result_1">spec_ecdsa_recover_internal_result_1</a>(message, recovery_id, signature);<br /><b>ensures</b> result_2 &#61;&#61; <a href="secp256k1.md#0x1_secp256k1_spec_ecdsa_recover_internal_result_2">spec_ecdsa_recover_internal_result_2</a>(message, recovery_id, signature);<br /><b>ensures</b> len(result_1) &#61;&#61; <b>if</b> (result_2) &#123; <a href="secp256k1.md#0x1_secp256k1_RAW_PUBLIC_KEY_NUM_BYTES">RAW_PUBLIC_KEY_NUM_BYTES</a> &#125; <b>else</b> &#123; 0 &#125;;<br /></code></pre>




<a id="0x1_secp256k1_ecdsa_recover_internal_abort_condition"></a>


<pre><code><b>fun</b> <a href="secp256k1.md#0x1_secp256k1_ecdsa_recover_internal_abort_condition">ecdsa_recover_internal_abort_condition</a>(message: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, recovery_id: u8, signature: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool;<br /></code></pre>




<a id="0x1_secp256k1_spec_ecdsa_recover_internal_result_1"></a>


<pre><code><b>fun</b> <a href="secp256k1.md#0x1_secp256k1_spec_ecdsa_recover_internal_result_1">spec_ecdsa_recover_internal_result_1</a>(message: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, recovery_id: u8, signature: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;<br /></code></pre>




<a id="0x1_secp256k1_spec_ecdsa_recover_internal_result_2"></a>


<pre><code><b>fun</b> <a href="secp256k1.md#0x1_secp256k1_spec_ecdsa_recover_internal_result_2">spec_ecdsa_recover_internal_result_2</a>(message: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, recovery_id: u8, signature: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool;<br /></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
