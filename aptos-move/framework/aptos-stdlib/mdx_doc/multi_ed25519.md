
<a id="0x1_multi_ed25519"></a>

# Module `0x1::multi_ed25519`

Exports MultiEd25519 multi&#45;signatures in Move.
This module has the exact same interface as the Ed25519 module.


-  [Struct `UnvalidatedPublicKey`](#0x1_multi_ed25519_UnvalidatedPublicKey)
-  [Struct `ValidatedPublicKey`](#0x1_multi_ed25519_ValidatedPublicKey)
-  [Struct `Signature`](#0x1_multi_ed25519_Signature)
-  [Constants](#@Constants_0)
-  [Function `new_unvalidated_public_key_from_bytes`](#0x1_multi_ed25519_new_unvalidated_public_key_from_bytes)
-  [Function `new_validated_public_key_from_bytes`](#0x1_multi_ed25519_new_validated_public_key_from_bytes)
-  [Function `new_validated_public_key_from_bytes_v2`](#0x1_multi_ed25519_new_validated_public_key_from_bytes_v2)
-  [Function `new_signature_from_bytes`](#0x1_multi_ed25519_new_signature_from_bytes)
-  [Function `public_key_to_unvalidated`](#0x1_multi_ed25519_public_key_to_unvalidated)
-  [Function `public_key_into_unvalidated`](#0x1_multi_ed25519_public_key_into_unvalidated)
-  [Function `unvalidated_public_key_to_bytes`](#0x1_multi_ed25519_unvalidated_public_key_to_bytes)
-  [Function `validated_public_key_to_bytes`](#0x1_multi_ed25519_validated_public_key_to_bytes)
-  [Function `signature_to_bytes`](#0x1_multi_ed25519_signature_to_bytes)
-  [Function `public_key_validate`](#0x1_multi_ed25519_public_key_validate)
-  [Function `public_key_validate_v2`](#0x1_multi_ed25519_public_key_validate_v2)
-  [Function `signature_verify_strict`](#0x1_multi_ed25519_signature_verify_strict)
-  [Function `signature_verify_strict_t`](#0x1_multi_ed25519_signature_verify_strict_t)
-  [Function `unvalidated_public_key_to_authentication_key`](#0x1_multi_ed25519_unvalidated_public_key_to_authentication_key)
-  [Function `unvalidated_public_key_num_sub_pks`](#0x1_multi_ed25519_unvalidated_public_key_num_sub_pks)
-  [Function `unvalidated_public_key_threshold`](#0x1_multi_ed25519_unvalidated_public_key_threshold)
-  [Function `validated_public_key_to_authentication_key`](#0x1_multi_ed25519_validated_public_key_to_authentication_key)
-  [Function `validated_public_key_num_sub_pks`](#0x1_multi_ed25519_validated_public_key_num_sub_pks)
-  [Function `validated_public_key_threshold`](#0x1_multi_ed25519_validated_public_key_threshold)
-  [Function `check_and_get_threshold`](#0x1_multi_ed25519_check_and_get_threshold)
-  [Function `public_key_bytes_to_authentication_key`](#0x1_multi_ed25519_public_key_bytes_to_authentication_key)
-  [Function `public_key_validate_internal`](#0x1_multi_ed25519_public_key_validate_internal)
-  [Function `public_key_validate_v2_internal`](#0x1_multi_ed25519_public_key_validate_v2_internal)
-  [Function `signature_verify_strict_internal`](#0x1_multi_ed25519_signature_verify_strict_internal)
-  [Specification](#@Specification_1)
    -  [Function `new_unvalidated_public_key_from_bytes`](#@Specification_1_new_unvalidated_public_key_from_bytes)
    -  [Function `new_validated_public_key_from_bytes`](#@Specification_1_new_validated_public_key_from_bytes)
    -  [Function `new_validated_public_key_from_bytes_v2`](#@Specification_1_new_validated_public_key_from_bytes_v2)
    -  [Function `new_signature_from_bytes`](#@Specification_1_new_signature_from_bytes)
    -  [Function `unvalidated_public_key_num_sub_pks`](#@Specification_1_unvalidated_public_key_num_sub_pks)
    -  [Function `unvalidated_public_key_threshold`](#@Specification_1_unvalidated_public_key_threshold)
    -  [Function `validated_public_key_num_sub_pks`](#@Specification_1_validated_public_key_num_sub_pks)
    -  [Function `validated_public_key_threshold`](#@Specification_1_validated_public_key_threshold)
    -  [Function `check_and_get_threshold`](#@Specification_1_check_and_get_threshold)
    -  [Function `public_key_bytes_to_authentication_key`](#@Specification_1_public_key_bytes_to_authentication_key)
    -  [Function `public_key_validate_internal`](#@Specification_1_public_key_validate_internal)
    -  [Function `public_key_validate_v2_internal`](#@Specification_1_public_key_validate_v2_internal)
    -  [Function `signature_verify_strict_internal`](#@Specification_1_signature_verify_strict_internal)
    -  [Helper functions](#@Helper_functions_2)


<pre><code><b>use</b> <a href="../../move-stdlib/doc/bcs.md#0x1_bcs">0x1::bcs</a>;<br /><b>use</b> <a href="ed25519.md#0x1_ed25519">0x1::ed25519</a>;<br /><b>use</b> <a href="../../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;<br /><b>use</b> <a href="../../move-stdlib/doc/features.md#0x1_features">0x1::features</a>;<br /><b>use</b> <a href="../../move-stdlib/doc/hash.md#0x1_hash">0x1::hash</a>;<br /><b>use</b> <a href="../../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;<br /></code></pre>



<a id="0x1_multi_ed25519_UnvalidatedPublicKey"></a>

## Struct `UnvalidatedPublicKey`

An &#42;unvalidated&#42;, k out of n MultiEd25519 public key. The <code>bytes</code> field contains (1) several chunks of
<code><a href="ed25519.md#0x1_ed25519_PUBLIC_KEY_NUM_BYTES">ed25519::PUBLIC_KEY_NUM_BYTES</a></code> bytes, each encoding a Ed25519 PK, and (2) a single byte encoding the threshold k.
&#42;Unvalidated&#42; means there is no guarantee that the underlying PKs are valid elliptic curve points of non&#45;small
order.


<pre><code><b>struct</b> <a href="multi_ed25519.md#0x1_multi_ed25519_UnvalidatedPublicKey">UnvalidatedPublicKey</a> <b>has</b> <b>copy</b>, drop, store<br /></code></pre>



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

<a id="0x1_multi_ed25519_ValidatedPublicKey"></a>

## Struct `ValidatedPublicKey`

A &#42;validated&#42; k out of n MultiEd25519 public key. &#42;Validated&#42; means that all the underlying PKs will be
elliptic curve points that are NOT of small&#45;order. It does not necessarily mean they will be prime&#45;order points.
This struct encodes the public key exactly as <code><a href="multi_ed25519.md#0x1_multi_ed25519_UnvalidatedPublicKey">UnvalidatedPublicKey</a></code>.

For now, this struct is not used in any verification functions, but it might be in the future.


<pre><code><b>struct</b> <a href="multi_ed25519.md#0x1_multi_ed25519_ValidatedPublicKey">ValidatedPublicKey</a> <b>has</b> <b>copy</b>, drop, store<br /></code></pre>



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

<a id="0x1_multi_ed25519_Signature"></a>

## Struct `Signature`

A purported MultiEd25519 multi&#45;signature that can be verified via <code>signature_verify_strict</code> or
<code>signature_verify_strict_t</code>. The <code>bytes</code> field contains (1) several chunks of <code><a href="ed25519.md#0x1_ed25519_SIGNATURE_NUM_BYTES">ed25519::SIGNATURE_NUM_BYTES</a></code>
bytes, each encoding a Ed25519 signature, and (2) a <code><a href="multi_ed25519.md#0x1_multi_ed25519_BITMAP_NUM_OF_BYTES">BITMAP_NUM_OF_BYTES</a></code>&#45;byte bitmap encoding the signer
identities.


<pre><code><b>struct</b> <a href="multi_ed25519.md#0x1_multi_ed25519_Signature">Signature</a> <b>has</b> <b>copy</b>, drop, store<br /></code></pre>



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


<a id="0x1_multi_ed25519_E_NATIVE_FUN_NOT_AVAILABLE"></a>

The native functions have not been rolled out yet.


<pre><code><b>const</b> <a href="multi_ed25519.md#0x1_multi_ed25519_E_NATIVE_FUN_NOT_AVAILABLE">E_NATIVE_FUN_NOT_AVAILABLE</a>: u64 &#61; 4;<br /></code></pre>



<a id="0x1_multi_ed25519_E_WRONG_PUBKEY_SIZE"></a>

Wrong number of bytes were given as input when deserializing an Ed25519 public key.


<pre><code><b>const</b> <a href="multi_ed25519.md#0x1_multi_ed25519_E_WRONG_PUBKEY_SIZE">E_WRONG_PUBKEY_SIZE</a>: u64 &#61; 1;<br /></code></pre>



<a id="0x1_multi_ed25519_E_WRONG_SIGNATURE_SIZE"></a>

Wrong number of bytes were given as input when deserializing an Ed25519 signature.


<pre><code><b>const</b> <a href="multi_ed25519.md#0x1_multi_ed25519_E_WRONG_SIGNATURE_SIZE">E_WRONG_SIGNATURE_SIZE</a>: u64 &#61; 2;<br /></code></pre>



<a id="0x1_multi_ed25519_SIGNATURE_SCHEME_ID"></a>

The identifier of the MultiEd25519 signature scheme, which is used when deriving Aptos authentication keys by hashing
it together with an MultiEd25519 public key.


<pre><code><b>const</b> <a href="multi_ed25519.md#0x1_multi_ed25519_SIGNATURE_SCHEME_ID">SIGNATURE_SCHEME_ID</a>: u8 &#61; 1;<br /></code></pre>



<a id="0x1_multi_ed25519_BITMAP_NUM_OF_BYTES"></a>

When serializing a MultiEd25519 signature, the bitmap that indicates the signers will be encoded using this many
bytes.


<pre><code><b>const</b> <a href="multi_ed25519.md#0x1_multi_ed25519_BITMAP_NUM_OF_BYTES">BITMAP_NUM_OF_BYTES</a>: u64 &#61; 4;<br /></code></pre>



<a id="0x1_multi_ed25519_E_INVALID_THRESHOLD_OR_NUMBER_OF_SIGNERS"></a>

The threshold must be in the range <code>[1, n]</code>, where n is the total number of signers.


<pre><code><b>const</b> <a href="multi_ed25519.md#0x1_multi_ed25519_E_INVALID_THRESHOLD_OR_NUMBER_OF_SIGNERS">E_INVALID_THRESHOLD_OR_NUMBER_OF_SIGNERS</a>: u64 &#61; 3;<br /></code></pre>



<a id="0x1_multi_ed25519_INDIVIDUAL_PUBLIC_KEY_NUM_BYTES"></a>

The size of an individual Ed25519 public key, in bytes.
(A MultiEd25519 public key consists of several of these, plus the threshold.)


<pre><code><b>const</b> <a href="multi_ed25519.md#0x1_multi_ed25519_INDIVIDUAL_PUBLIC_KEY_NUM_BYTES">INDIVIDUAL_PUBLIC_KEY_NUM_BYTES</a>: u64 &#61; 32;<br /></code></pre>



<a id="0x1_multi_ed25519_INDIVIDUAL_SIGNATURE_NUM_BYTES"></a>

The size of an individual Ed25519 signature, in bytes.
(A MultiEd25519 signature consists of several of these, plus the signer bitmap.)


<pre><code><b>const</b> <a href="multi_ed25519.md#0x1_multi_ed25519_INDIVIDUAL_SIGNATURE_NUM_BYTES">INDIVIDUAL_SIGNATURE_NUM_BYTES</a>: u64 &#61; 64;<br /></code></pre>



<a id="0x1_multi_ed25519_MAX_NUMBER_OF_PUBLIC_KEYS"></a>

Max number of ed25519 public keys allowed in multi&#45;ed25519 keys


<pre><code><b>const</b> <a href="multi_ed25519.md#0x1_multi_ed25519_MAX_NUMBER_OF_PUBLIC_KEYS">MAX_NUMBER_OF_PUBLIC_KEYS</a>: u64 &#61; 32;<br /></code></pre>



<a id="0x1_multi_ed25519_THRESHOLD_SIZE_BYTES"></a>

When serializing a MultiEd25519 public key, the threshold k will be encoded using this many bytes.


<pre><code><b>const</b> <a href="multi_ed25519.md#0x1_multi_ed25519_THRESHOLD_SIZE_BYTES">THRESHOLD_SIZE_BYTES</a>: u64 &#61; 1;<br /></code></pre>



<a id="0x1_multi_ed25519_new_unvalidated_public_key_from_bytes"></a>

## Function `new_unvalidated_public_key_from_bytes`

Parses the input 32 bytes as an &#42;unvalidated&#42; MultiEd25519 public key.

NOTE: This function could have also checked that the # of sub&#45;PKs is &gt; 0, but it did not. However, since such
invalid PKs are rejected during signature verification  (see <code>bugfix_unvalidated_pk_from_zero_subpks</code>) they
will not cause problems.

We could fix this API by adding a new one that checks the # of sub&#45;PKs is &gt; 0, but it is likely not a good idea
to reproduce the PK validation logic in Move. We should not have done so in the first place. Instead, we will
leave it as is and continue assuming <code><a href="multi_ed25519.md#0x1_multi_ed25519_UnvalidatedPublicKey">UnvalidatedPublicKey</a></code> objects could be invalid PKs that will safely be
rejected during signature verification.


<pre><code><b>public</b> <b>fun</b> <a href="multi_ed25519.md#0x1_multi_ed25519_new_unvalidated_public_key_from_bytes">new_unvalidated_public_key_from_bytes</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="multi_ed25519.md#0x1_multi_ed25519_UnvalidatedPublicKey">multi_ed25519::UnvalidatedPublicKey</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multi_ed25519.md#0x1_multi_ed25519_new_unvalidated_public_key_from_bytes">new_unvalidated_public_key_from_bytes</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="multi_ed25519.md#0x1_multi_ed25519_UnvalidatedPublicKey">UnvalidatedPublicKey</a> &#123;<br />    <b>let</b> len &#61; <a href="../../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&amp;bytes);<br />    <b>let</b> num_sub_pks &#61; len / <a href="multi_ed25519.md#0x1_multi_ed25519_INDIVIDUAL_PUBLIC_KEY_NUM_BYTES">INDIVIDUAL_PUBLIC_KEY_NUM_BYTES</a>;<br /><br />    <b>assert</b>!(num_sub_pks &lt;&#61; <a href="multi_ed25519.md#0x1_multi_ed25519_MAX_NUMBER_OF_PUBLIC_KEYS">MAX_NUMBER_OF_PUBLIC_KEYS</a>, <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="multi_ed25519.md#0x1_multi_ed25519_E_WRONG_PUBKEY_SIZE">E_WRONG_PUBKEY_SIZE</a>));<br />    <b>assert</b>!(len % <a href="multi_ed25519.md#0x1_multi_ed25519_INDIVIDUAL_PUBLIC_KEY_NUM_BYTES">INDIVIDUAL_PUBLIC_KEY_NUM_BYTES</a> &#61;&#61; <a href="multi_ed25519.md#0x1_multi_ed25519_THRESHOLD_SIZE_BYTES">THRESHOLD_SIZE_BYTES</a>, <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="multi_ed25519.md#0x1_multi_ed25519_E_WRONG_PUBKEY_SIZE">E_WRONG_PUBKEY_SIZE</a>));<br />    <a href="multi_ed25519.md#0x1_multi_ed25519_UnvalidatedPublicKey">UnvalidatedPublicKey</a> &#123; bytes &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_multi_ed25519_new_validated_public_key_from_bytes"></a>

## Function `new_validated_public_key_from_bytes`

DEPRECATED: Use <code>new_validated_public_key_from_bytes_v2</code> instead. See <code>public_key_validate_internal</code> comments.

(Incorrectly) parses the input bytes as a &#42;validated&#42; MultiEd25519 public key.


<pre><code><b>public</b> <b>fun</b> <a href="multi_ed25519.md#0x1_multi_ed25519_new_validated_public_key_from_bytes">new_validated_public_key_from_bytes</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="multi_ed25519.md#0x1_multi_ed25519_ValidatedPublicKey">multi_ed25519::ValidatedPublicKey</a>&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multi_ed25519.md#0x1_multi_ed25519_new_validated_public_key_from_bytes">new_validated_public_key_from_bytes</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): Option&lt;<a href="multi_ed25519.md#0x1_multi_ed25519_ValidatedPublicKey">ValidatedPublicKey</a>&gt; &#123;<br />    // Note that `public_key_validate_internal` will check that `<a href="../../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&amp;bytes) / <a href="multi_ed25519.md#0x1_multi_ed25519_INDIVIDUAL_PUBLIC_KEY_NUM_BYTES">INDIVIDUAL_PUBLIC_KEY_NUM_BYTES</a> &lt;&#61; <a href="multi_ed25519.md#0x1_multi_ed25519_MAX_NUMBER_OF_PUBLIC_KEYS">MAX_NUMBER_OF_PUBLIC_KEYS</a>`.<br />    <b>if</b> (<a href="../../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&amp;bytes) % <a href="multi_ed25519.md#0x1_multi_ed25519_INDIVIDUAL_PUBLIC_KEY_NUM_BYTES">INDIVIDUAL_PUBLIC_KEY_NUM_BYTES</a> &#61;&#61; <a href="multi_ed25519.md#0x1_multi_ed25519_THRESHOLD_SIZE_BYTES">THRESHOLD_SIZE_BYTES</a> &amp;&amp;<br />        <a href="multi_ed25519.md#0x1_multi_ed25519_public_key_validate_internal">public_key_validate_internal</a>(bytes)) &#123;<br />        <a href="../../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(<a href="multi_ed25519.md#0x1_multi_ed25519_ValidatedPublicKey">ValidatedPublicKey</a> &#123;<br />            bytes<br />        &#125;)<br />    &#125; <b>else</b> &#123;<br />        <a href="../../move-stdlib/doc/option.md#0x1_option_none">option::none</a>&lt;<a href="multi_ed25519.md#0x1_multi_ed25519_ValidatedPublicKey">ValidatedPublicKey</a>&gt;()<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_multi_ed25519_new_validated_public_key_from_bytes_v2"></a>

## Function `new_validated_public_key_from_bytes_v2`

Parses the input bytes as a &#42;validated&#42; MultiEd25519 public key (see <code>public_key_validate_internal_v2</code>).


<pre><code><b>public</b> <b>fun</b> <a href="multi_ed25519.md#0x1_multi_ed25519_new_validated_public_key_from_bytes_v2">new_validated_public_key_from_bytes_v2</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="multi_ed25519.md#0x1_multi_ed25519_ValidatedPublicKey">multi_ed25519::ValidatedPublicKey</a>&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multi_ed25519.md#0x1_multi_ed25519_new_validated_public_key_from_bytes_v2">new_validated_public_key_from_bytes_v2</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): Option&lt;<a href="multi_ed25519.md#0x1_multi_ed25519_ValidatedPublicKey">ValidatedPublicKey</a>&gt; &#123;<br />    <b>if</b> (!<a href="../../move-stdlib/doc/features.md#0x1_features_multi_ed25519_pk_validate_v2_enabled">features::multi_ed25519_pk_validate_v2_enabled</a>()) &#123;<br />        <b>abort</b>(<a href="../../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="multi_ed25519.md#0x1_multi_ed25519_E_NATIVE_FUN_NOT_AVAILABLE">E_NATIVE_FUN_NOT_AVAILABLE</a>))<br />    &#125;;<br /><br />    <b>if</b> (<a href="multi_ed25519.md#0x1_multi_ed25519_public_key_validate_v2_internal">public_key_validate_v2_internal</a>(bytes)) &#123;<br />        <a href="../../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(<a href="multi_ed25519.md#0x1_multi_ed25519_ValidatedPublicKey">ValidatedPublicKey</a> &#123;<br />            bytes<br />        &#125;)<br />    &#125; <b>else</b> &#123;<br />        <a href="../../move-stdlib/doc/option.md#0x1_option_none">option::none</a>&lt;<a href="multi_ed25519.md#0x1_multi_ed25519_ValidatedPublicKey">ValidatedPublicKey</a>&gt;()<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_multi_ed25519_new_signature_from_bytes"></a>

## Function `new_signature_from_bytes`

Parses the input bytes as a purported MultiEd25519 multi&#45;signature.


<pre><code><b>public</b> <b>fun</b> <a href="multi_ed25519.md#0x1_multi_ed25519_new_signature_from_bytes">new_signature_from_bytes</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="multi_ed25519.md#0x1_multi_ed25519_Signature">multi_ed25519::Signature</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multi_ed25519.md#0x1_multi_ed25519_new_signature_from_bytes">new_signature_from_bytes</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="multi_ed25519.md#0x1_multi_ed25519_Signature">Signature</a> &#123;<br />    <b>assert</b>!(<a href="../../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&amp;bytes) % <a href="multi_ed25519.md#0x1_multi_ed25519_INDIVIDUAL_SIGNATURE_NUM_BYTES">INDIVIDUAL_SIGNATURE_NUM_BYTES</a> &#61;&#61; <a href="multi_ed25519.md#0x1_multi_ed25519_BITMAP_NUM_OF_BYTES">BITMAP_NUM_OF_BYTES</a>, <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="multi_ed25519.md#0x1_multi_ed25519_E_WRONG_SIGNATURE_SIZE">E_WRONG_SIGNATURE_SIZE</a>));<br />    <a href="multi_ed25519.md#0x1_multi_ed25519_Signature">Signature</a> &#123; bytes &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_multi_ed25519_public_key_to_unvalidated"></a>

## Function `public_key_to_unvalidated`

Converts a ValidatedPublicKey to an UnvalidatedPublicKey, which can be used in the strict verification APIs.


<pre><code><b>public</b> <b>fun</b> <a href="multi_ed25519.md#0x1_multi_ed25519_public_key_to_unvalidated">public_key_to_unvalidated</a>(pk: &amp;<a href="multi_ed25519.md#0x1_multi_ed25519_ValidatedPublicKey">multi_ed25519::ValidatedPublicKey</a>): <a href="multi_ed25519.md#0x1_multi_ed25519_UnvalidatedPublicKey">multi_ed25519::UnvalidatedPublicKey</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multi_ed25519.md#0x1_multi_ed25519_public_key_to_unvalidated">public_key_to_unvalidated</a>(pk: &amp;<a href="multi_ed25519.md#0x1_multi_ed25519_ValidatedPublicKey">ValidatedPublicKey</a>): <a href="multi_ed25519.md#0x1_multi_ed25519_UnvalidatedPublicKey">UnvalidatedPublicKey</a> &#123;<br />    <a href="multi_ed25519.md#0x1_multi_ed25519_UnvalidatedPublicKey">UnvalidatedPublicKey</a> &#123;<br />        bytes: pk.bytes<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_multi_ed25519_public_key_into_unvalidated"></a>

## Function `public_key_into_unvalidated`

Moves a ValidatedPublicKey into an UnvalidatedPublicKey, which can be used in the strict verification APIs.


<pre><code><b>public</b> <b>fun</b> <a href="multi_ed25519.md#0x1_multi_ed25519_public_key_into_unvalidated">public_key_into_unvalidated</a>(pk: <a href="multi_ed25519.md#0x1_multi_ed25519_ValidatedPublicKey">multi_ed25519::ValidatedPublicKey</a>): <a href="multi_ed25519.md#0x1_multi_ed25519_UnvalidatedPublicKey">multi_ed25519::UnvalidatedPublicKey</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multi_ed25519.md#0x1_multi_ed25519_public_key_into_unvalidated">public_key_into_unvalidated</a>(pk: <a href="multi_ed25519.md#0x1_multi_ed25519_ValidatedPublicKey">ValidatedPublicKey</a>): <a href="multi_ed25519.md#0x1_multi_ed25519_UnvalidatedPublicKey">UnvalidatedPublicKey</a> &#123;<br />    <a href="multi_ed25519.md#0x1_multi_ed25519_UnvalidatedPublicKey">UnvalidatedPublicKey</a> &#123;<br />        bytes: pk.bytes<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_multi_ed25519_unvalidated_public_key_to_bytes"></a>

## Function `unvalidated_public_key_to_bytes`

Serializes an UnvalidatedPublicKey struct to 32&#45;bytes.


<pre><code><b>public</b> <b>fun</b> <a href="multi_ed25519.md#0x1_multi_ed25519_unvalidated_public_key_to_bytes">unvalidated_public_key_to_bytes</a>(pk: &amp;<a href="multi_ed25519.md#0x1_multi_ed25519_UnvalidatedPublicKey">multi_ed25519::UnvalidatedPublicKey</a>): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multi_ed25519.md#0x1_multi_ed25519_unvalidated_public_key_to_bytes">unvalidated_public_key_to_bytes</a>(pk: &amp;<a href="multi_ed25519.md#0x1_multi_ed25519_UnvalidatedPublicKey">UnvalidatedPublicKey</a>): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; &#123;<br />    pk.bytes<br />&#125;<br /></code></pre>



</details>

<a id="0x1_multi_ed25519_validated_public_key_to_bytes"></a>

## Function `validated_public_key_to_bytes`

Serializes a ValidatedPublicKey struct to 32&#45;bytes.


<pre><code><b>public</b> <b>fun</b> <a href="multi_ed25519.md#0x1_multi_ed25519_validated_public_key_to_bytes">validated_public_key_to_bytes</a>(pk: &amp;<a href="multi_ed25519.md#0x1_multi_ed25519_ValidatedPublicKey">multi_ed25519::ValidatedPublicKey</a>): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multi_ed25519.md#0x1_multi_ed25519_validated_public_key_to_bytes">validated_public_key_to_bytes</a>(pk: &amp;<a href="multi_ed25519.md#0x1_multi_ed25519_ValidatedPublicKey">ValidatedPublicKey</a>): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; &#123;<br />    pk.bytes<br />&#125;<br /></code></pre>



</details>

<a id="0x1_multi_ed25519_signature_to_bytes"></a>

## Function `signature_to_bytes`

Serializes a Signature struct to 64&#45;bytes.


<pre><code><b>public</b> <b>fun</b> <a href="multi_ed25519.md#0x1_multi_ed25519_signature_to_bytes">signature_to_bytes</a>(sig: &amp;<a href="multi_ed25519.md#0x1_multi_ed25519_Signature">multi_ed25519::Signature</a>): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multi_ed25519.md#0x1_multi_ed25519_signature_to_bytes">signature_to_bytes</a>(sig: &amp;<a href="multi_ed25519.md#0x1_multi_ed25519_Signature">Signature</a>): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; &#123;<br />    sig.bytes<br />&#125;<br /></code></pre>



</details>

<a id="0x1_multi_ed25519_public_key_validate"></a>

## Function `public_key_validate`

DEPRECATED: Use <code>public_key_validate_v2</code> instead. See <code>public_key_validate_internal</code> comments.

Takes in an &#42;unvalidated&#42; public key and attempts to validate it.
Returns <code>Some(<a href="multi_ed25519.md#0x1_multi_ed25519_ValidatedPublicKey">ValidatedPublicKey</a>)</code> if successful and <code>None</code> otherwise.


<pre><code><b>public</b> <b>fun</b> <a href="multi_ed25519.md#0x1_multi_ed25519_public_key_validate">public_key_validate</a>(pk: &amp;<a href="multi_ed25519.md#0x1_multi_ed25519_UnvalidatedPublicKey">multi_ed25519::UnvalidatedPublicKey</a>): <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="multi_ed25519.md#0x1_multi_ed25519_ValidatedPublicKey">multi_ed25519::ValidatedPublicKey</a>&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multi_ed25519.md#0x1_multi_ed25519_public_key_validate">public_key_validate</a>(pk: &amp;<a href="multi_ed25519.md#0x1_multi_ed25519_UnvalidatedPublicKey">UnvalidatedPublicKey</a>): Option&lt;<a href="multi_ed25519.md#0x1_multi_ed25519_ValidatedPublicKey">ValidatedPublicKey</a>&gt; &#123;<br />    <a href="multi_ed25519.md#0x1_multi_ed25519_new_validated_public_key_from_bytes">new_validated_public_key_from_bytes</a>(pk.bytes)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_multi_ed25519_public_key_validate_v2"></a>

## Function `public_key_validate_v2`

Takes in an &#42;unvalidated&#42; public key and attempts to validate it.
Returns <code>Some(<a href="multi_ed25519.md#0x1_multi_ed25519_ValidatedPublicKey">ValidatedPublicKey</a>)</code> if successful and <code>None</code> otherwise.


<pre><code><b>public</b> <b>fun</b> <a href="multi_ed25519.md#0x1_multi_ed25519_public_key_validate_v2">public_key_validate_v2</a>(pk: &amp;<a href="multi_ed25519.md#0x1_multi_ed25519_UnvalidatedPublicKey">multi_ed25519::UnvalidatedPublicKey</a>): <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="multi_ed25519.md#0x1_multi_ed25519_ValidatedPublicKey">multi_ed25519::ValidatedPublicKey</a>&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multi_ed25519.md#0x1_multi_ed25519_public_key_validate_v2">public_key_validate_v2</a>(pk: &amp;<a href="multi_ed25519.md#0x1_multi_ed25519_UnvalidatedPublicKey">UnvalidatedPublicKey</a>): Option&lt;<a href="multi_ed25519.md#0x1_multi_ed25519_ValidatedPublicKey">ValidatedPublicKey</a>&gt; &#123;<br />    <a href="multi_ed25519.md#0x1_multi_ed25519_new_validated_public_key_from_bytes_v2">new_validated_public_key_from_bytes_v2</a>(pk.bytes)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_multi_ed25519_signature_verify_strict"></a>

## Function `signature_verify_strict`

Verifies a purported MultiEd25519 <code>multisignature</code> under an &#42;unvalidated&#42; <code>public_key</code> on the specified <code>message</code>.
This call will validate the public key by checking it is NOT in the small subgroup.


<pre><code><b>public</b> <b>fun</b> <a href="multi_ed25519.md#0x1_multi_ed25519_signature_verify_strict">signature_verify_strict</a>(multisignature: &amp;<a href="multi_ed25519.md#0x1_multi_ed25519_Signature">multi_ed25519::Signature</a>, public_key: &amp;<a href="multi_ed25519.md#0x1_multi_ed25519_UnvalidatedPublicKey">multi_ed25519::UnvalidatedPublicKey</a>, message: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multi_ed25519.md#0x1_multi_ed25519_signature_verify_strict">signature_verify_strict</a>(<br />    multisignature: &amp;<a href="multi_ed25519.md#0x1_multi_ed25519_Signature">Signature</a>,<br />    public_key: &amp;<a href="multi_ed25519.md#0x1_multi_ed25519_UnvalidatedPublicKey">UnvalidatedPublicKey</a>,<br />    message: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;<br />): bool &#123;<br />    <a href="multi_ed25519.md#0x1_multi_ed25519_signature_verify_strict_internal">signature_verify_strict_internal</a>(multisignature.bytes, public_key.bytes, message)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_multi_ed25519_signature_verify_strict_t"></a>

## Function `signature_verify_strict_t`

This function is used to verify a multi&#45;signature on any BCS&#45;serializable type T. For now, it is used to verify the
proof of private key ownership when rotating authentication keys.


<pre><code><b>public</b> <b>fun</b> <a href="multi_ed25519.md#0x1_multi_ed25519_signature_verify_strict_t">signature_verify_strict_t</a>&lt;T: drop&gt;(multisignature: &amp;<a href="multi_ed25519.md#0x1_multi_ed25519_Signature">multi_ed25519::Signature</a>, public_key: &amp;<a href="multi_ed25519.md#0x1_multi_ed25519_UnvalidatedPublicKey">multi_ed25519::UnvalidatedPublicKey</a>, data: T): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multi_ed25519.md#0x1_multi_ed25519_signature_verify_strict_t">signature_verify_strict_t</a>&lt;T: drop&gt;(multisignature: &amp;<a href="multi_ed25519.md#0x1_multi_ed25519_Signature">Signature</a>, public_key: &amp;<a href="multi_ed25519.md#0x1_multi_ed25519_UnvalidatedPublicKey">UnvalidatedPublicKey</a>, data: T): bool &#123;<br />    <b>let</b> encoded &#61; <a href="ed25519.md#0x1_ed25519_new_signed_message">ed25519::new_signed_message</a>(data);<br /><br />    <a href="multi_ed25519.md#0x1_multi_ed25519_signature_verify_strict_internal">signature_verify_strict_internal</a>(multisignature.bytes, public_key.bytes, <a href="../../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(&amp;encoded))<br />&#125;<br /></code></pre>



</details>

<a id="0x1_multi_ed25519_unvalidated_public_key_to_authentication_key"></a>

## Function `unvalidated_public_key_to_authentication_key`

Derives the Aptos&#45;specific authentication key of the given Ed25519 public key.


<pre><code><b>public</b> <b>fun</b> <a href="multi_ed25519.md#0x1_multi_ed25519_unvalidated_public_key_to_authentication_key">unvalidated_public_key_to_authentication_key</a>(pk: &amp;<a href="multi_ed25519.md#0x1_multi_ed25519_UnvalidatedPublicKey">multi_ed25519::UnvalidatedPublicKey</a>): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multi_ed25519.md#0x1_multi_ed25519_unvalidated_public_key_to_authentication_key">unvalidated_public_key_to_authentication_key</a>(pk: &amp;<a href="multi_ed25519.md#0x1_multi_ed25519_UnvalidatedPublicKey">UnvalidatedPublicKey</a>): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; &#123;<br />    <a href="multi_ed25519.md#0x1_multi_ed25519_public_key_bytes_to_authentication_key">public_key_bytes_to_authentication_key</a>(pk.bytes)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_multi_ed25519_unvalidated_public_key_num_sub_pks"></a>

## Function `unvalidated_public_key_num_sub_pks`

Returns the number n of sub&#45;PKs in an unvalidated t&#45;out&#45;of&#45;n MultiEd25519 PK.
If this <code><a href="multi_ed25519.md#0x1_multi_ed25519_UnvalidatedPublicKey">UnvalidatedPublicKey</a></code> would pass validation in <code>public_key_validate</code>, then the returned # of sub&#45;PKs
can be relied upon as correct.

We provide this API as a cheaper alternative to calling <code>public_key_validate</code> and then <code>validated_public_key_num_sub_pks</code>
when the input <code>pk</code> is known to be valid.


<pre><code><b>public</b> <b>fun</b> <a href="multi_ed25519.md#0x1_multi_ed25519_unvalidated_public_key_num_sub_pks">unvalidated_public_key_num_sub_pks</a>(pk: &amp;<a href="multi_ed25519.md#0x1_multi_ed25519_UnvalidatedPublicKey">multi_ed25519::UnvalidatedPublicKey</a>): u8<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multi_ed25519.md#0x1_multi_ed25519_unvalidated_public_key_num_sub_pks">unvalidated_public_key_num_sub_pks</a>(pk: &amp;<a href="multi_ed25519.md#0x1_multi_ed25519_UnvalidatedPublicKey">UnvalidatedPublicKey</a>): u8 &#123;<br />    <b>let</b> len &#61; <a href="../../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&amp;pk.bytes);<br /><br />    ((len / <a href="multi_ed25519.md#0x1_multi_ed25519_INDIVIDUAL_PUBLIC_KEY_NUM_BYTES">INDIVIDUAL_PUBLIC_KEY_NUM_BYTES</a>) <b>as</b> u8)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_multi_ed25519_unvalidated_public_key_threshold"></a>

## Function `unvalidated_public_key_threshold`

Returns the number t of sub&#45;PKs in an unvalidated t&#45;out&#45;of&#45;n MultiEd25519 PK (i.e., the threshold) or <code>None</code>
if <code>bytes</code> does not correctly encode such a PK.


<pre><code><b>public</b> <b>fun</b> <a href="multi_ed25519.md#0x1_multi_ed25519_unvalidated_public_key_threshold">unvalidated_public_key_threshold</a>(pk: &amp;<a href="multi_ed25519.md#0x1_multi_ed25519_UnvalidatedPublicKey">multi_ed25519::UnvalidatedPublicKey</a>): <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u8&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multi_ed25519.md#0x1_multi_ed25519_unvalidated_public_key_threshold">unvalidated_public_key_threshold</a>(pk: &amp;<a href="multi_ed25519.md#0x1_multi_ed25519_UnvalidatedPublicKey">UnvalidatedPublicKey</a>): Option&lt;u8&gt; &#123;<br />    <a href="multi_ed25519.md#0x1_multi_ed25519_check_and_get_threshold">check_and_get_threshold</a>(pk.bytes)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_multi_ed25519_validated_public_key_to_authentication_key"></a>

## Function `validated_public_key_to_authentication_key`

Derives the Aptos&#45;specific authentication key of the given Ed25519 public key.


<pre><code><b>public</b> <b>fun</b> <a href="multi_ed25519.md#0x1_multi_ed25519_validated_public_key_to_authentication_key">validated_public_key_to_authentication_key</a>(pk: &amp;<a href="multi_ed25519.md#0x1_multi_ed25519_ValidatedPublicKey">multi_ed25519::ValidatedPublicKey</a>): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multi_ed25519.md#0x1_multi_ed25519_validated_public_key_to_authentication_key">validated_public_key_to_authentication_key</a>(pk: &amp;<a href="multi_ed25519.md#0x1_multi_ed25519_ValidatedPublicKey">ValidatedPublicKey</a>): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; &#123;<br />    <a href="multi_ed25519.md#0x1_multi_ed25519_public_key_bytes_to_authentication_key">public_key_bytes_to_authentication_key</a>(pk.bytes)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_multi_ed25519_validated_public_key_num_sub_pks"></a>

## Function `validated_public_key_num_sub_pks`

Returns the number n of sub&#45;PKs in a validated t&#45;out&#45;of&#45;n MultiEd25519 PK.
Since the format of this PK has been validated, the returned # of sub&#45;PKs is guaranteed to be correct.


<pre><code><b>public</b> <b>fun</b> <a href="multi_ed25519.md#0x1_multi_ed25519_validated_public_key_num_sub_pks">validated_public_key_num_sub_pks</a>(pk: &amp;<a href="multi_ed25519.md#0x1_multi_ed25519_ValidatedPublicKey">multi_ed25519::ValidatedPublicKey</a>): u8<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multi_ed25519.md#0x1_multi_ed25519_validated_public_key_num_sub_pks">validated_public_key_num_sub_pks</a>(pk: &amp;<a href="multi_ed25519.md#0x1_multi_ed25519_ValidatedPublicKey">ValidatedPublicKey</a>): u8 &#123;<br />    <b>let</b> len &#61; <a href="../../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&amp;pk.bytes);<br /><br />    ((len / <a href="multi_ed25519.md#0x1_multi_ed25519_INDIVIDUAL_PUBLIC_KEY_NUM_BYTES">INDIVIDUAL_PUBLIC_KEY_NUM_BYTES</a>) <b>as</b> u8)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_multi_ed25519_validated_public_key_threshold"></a>

## Function `validated_public_key_threshold`

Returns the number t of sub&#45;PKs in a validated t&#45;out&#45;of&#45;n MultiEd25519 PK (i.e., the threshold).


<pre><code><b>public</b> <b>fun</b> <a href="multi_ed25519.md#0x1_multi_ed25519_validated_public_key_threshold">validated_public_key_threshold</a>(pk: &amp;<a href="multi_ed25519.md#0x1_multi_ed25519_ValidatedPublicKey">multi_ed25519::ValidatedPublicKey</a>): u8<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multi_ed25519.md#0x1_multi_ed25519_validated_public_key_threshold">validated_public_key_threshold</a>(pk: &amp;<a href="multi_ed25519.md#0x1_multi_ed25519_ValidatedPublicKey">ValidatedPublicKey</a>): u8 &#123;<br />    <b>let</b> len &#61; <a href="../../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&amp;pk.bytes);<br />    <b>let</b> threshold_byte &#61; &#42;<a href="../../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&amp;pk.bytes, len &#45; 1);<br /><br />    threshold_byte<br />&#125;<br /></code></pre>



</details>

<a id="0x1_multi_ed25519_check_and_get_threshold"></a>

## Function `check_and_get_threshold`

Checks that the serialized format of a t&#45;out&#45;of&#45;n MultiEd25519 PK correctly encodes 1 &lt;&#61; n &lt;&#61; 32 sub&#45;PKs.
(All <code><a href="multi_ed25519.md#0x1_multi_ed25519_ValidatedPublicKey">ValidatedPublicKey</a></code> objects are guaranteed to pass this check.)
Returns the threshold t &lt;&#61; n of the PK.


<pre><code><b>public</b> <b>fun</b> <a href="multi_ed25519.md#0x1_multi_ed25519_check_and_get_threshold">check_and_get_threshold</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u8&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multi_ed25519.md#0x1_multi_ed25519_check_and_get_threshold">check_and_get_threshold</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): Option&lt;u8&gt; &#123;<br />    <b>let</b> len &#61; <a href="../../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&amp;bytes);<br />    <b>if</b> (len &#61;&#61; 0) &#123;<br />        <b>return</b> <a href="../../move-stdlib/doc/option.md#0x1_option_none">option::none</a>&lt;u8&gt;()<br />    &#125;;<br /><br />    <b>let</b> threshold_num_of_bytes &#61; len % <a href="multi_ed25519.md#0x1_multi_ed25519_INDIVIDUAL_PUBLIC_KEY_NUM_BYTES">INDIVIDUAL_PUBLIC_KEY_NUM_BYTES</a>;<br />    <b>let</b> num_of_keys &#61; len / <a href="multi_ed25519.md#0x1_multi_ed25519_INDIVIDUAL_PUBLIC_KEY_NUM_BYTES">INDIVIDUAL_PUBLIC_KEY_NUM_BYTES</a>;<br />    <b>let</b> threshold_byte &#61; &#42;<a href="../../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&amp;bytes, len &#45; 1);<br /><br />    <b>if</b> (num_of_keys &#61;&#61; 0 &#124;&#124; num_of_keys &gt; <a href="multi_ed25519.md#0x1_multi_ed25519_MAX_NUMBER_OF_PUBLIC_KEYS">MAX_NUMBER_OF_PUBLIC_KEYS</a> &#124;&#124; threshold_num_of_bytes !&#61; 1) &#123;<br />        <b>return</b> <a href="../../move-stdlib/doc/option.md#0x1_option_none">option::none</a>&lt;u8&gt;()<br />    &#125; <b>else</b> <b>if</b> (threshold_byte &#61;&#61; 0 &#124;&#124; threshold_byte &gt; (num_of_keys <b>as</b> u8)) &#123;<br />        <b>return</b> <a href="../../move-stdlib/doc/option.md#0x1_option_none">option::none</a>&lt;u8&gt;()<br />    &#125; <b>else</b> &#123;<br />        <b>return</b> <a href="../../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(threshold_byte)<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_multi_ed25519_public_key_bytes_to_authentication_key"></a>

## Function `public_key_bytes_to_authentication_key`

Derives the Aptos&#45;specific authentication key of the given Ed25519 public key.


<pre><code><b>fun</b> <a href="multi_ed25519.md#0x1_multi_ed25519_public_key_bytes_to_authentication_key">public_key_bytes_to_authentication_key</a>(pk_bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="multi_ed25519.md#0x1_multi_ed25519_public_key_bytes_to_authentication_key">public_key_bytes_to_authentication_key</a>(pk_bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; &#123;<br />    <a href="../../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&amp;<b>mut</b> pk_bytes, <a href="multi_ed25519.md#0x1_multi_ed25519_SIGNATURE_SCHEME_ID">SIGNATURE_SCHEME_ID</a>);<br />    std::hash::sha3_256(pk_bytes)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_multi_ed25519_public_key_validate_internal"></a>

## Function `public_key_validate_internal`

DEPRECATED: Use <code>public_key_validate_internal_v2</code> instead. This function was NOT correctly implemented:

1. It does not check that the # of sub public keys is &gt; 0, which leads to invalid <code><a href="multi_ed25519.md#0x1_multi_ed25519_ValidatedPublicKey">ValidatedPublicKey</a></code> objects
against which no signature will verify, since <code>signature_verify_strict_internal</code> will reject such invalid PKs.
This is not a security issue, but a correctness issue. See <code>bugfix_validated_pk_from_zero_subpks</code>.
2. It charges too much gas: if the first sub&#45;PK is invalid, it will charge for verifying all remaining sub&#45;PKs.

DEPRECATES:
&#45; new_validated_public_key_from_bytes
&#45; public_key_validate

Return <code><b>true</b></code> if the bytes in <code>public_key</code> can be parsed as a valid MultiEd25519 public key: i.e., all underlying
PKs pass point&#45;on&#45;curve and not&#45;in&#45;small&#45;subgroup checks.
Returns <code><b>false</b></code> otherwise.


<pre><code><b>fun</b> <a href="multi_ed25519.md#0x1_multi_ed25519_public_key_validate_internal">public_key_validate_internal</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="multi_ed25519.md#0x1_multi_ed25519_public_key_validate_internal">public_key_validate_internal</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool;<br /></code></pre>



</details>

<a id="0x1_multi_ed25519_public_key_validate_v2_internal"></a>

## Function `public_key_validate_v2_internal`

Return <code><b>true</b></code> if the bytes in <code>public_key</code> can be parsed as a valid MultiEd25519 public key: i.e., all underlying
sub&#45;PKs pass point&#45;on&#45;curve and not&#45;in&#45;small&#45;subgroup checks.
Returns <code><b>false</b></code> otherwise.


<pre><code><b>fun</b> <a href="multi_ed25519.md#0x1_multi_ed25519_public_key_validate_v2_internal">public_key_validate_v2_internal</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="multi_ed25519.md#0x1_multi_ed25519_public_key_validate_v2_internal">public_key_validate_v2_internal</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool;<br /></code></pre>



</details>

<a id="0x1_multi_ed25519_signature_verify_strict_internal"></a>

## Function `signature_verify_strict_internal`

Return true if the MultiEd25519 <code>multisignature</code> on <code>message</code> verifies against the MultiEd25519 <code>public_key</code>.
Returns <code><b>false</b></code> if either:
&#45; The PKs in <code>public_key</code> do not all pass points&#45;on&#45;curve or not&#45;in&#45;small&#45;subgroup checks,
&#45; The signatures in <code>multisignature</code> do not all pass points&#45;on&#45;curve or not&#45;in&#45;small&#45;subgroup checks,
&#45; the <code>multisignature</code> on <code>message</code> does not verify.


<pre><code><b>fun</b> <a href="multi_ed25519.md#0x1_multi_ed25519_signature_verify_strict_internal">signature_verify_strict_internal</a>(multisignature: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, public_key: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, message: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="multi_ed25519.md#0x1_multi_ed25519_signature_verify_strict_internal">signature_verify_strict_internal</a>(<br />    multisignature: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,<br />    public_key: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,<br />    message: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;<br />): bool;<br /></code></pre>



</details>

<a id="@Specification_1"></a>

## Specification


<a id="@Specification_1_new_unvalidated_public_key_from_bytes"></a>

### Function `new_unvalidated_public_key_from_bytes`


<pre><code><b>public</b> <b>fun</b> <a href="multi_ed25519.md#0x1_multi_ed25519_new_unvalidated_public_key_from_bytes">new_unvalidated_public_key_from_bytes</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="multi_ed25519.md#0x1_multi_ed25519_UnvalidatedPublicKey">multi_ed25519::UnvalidatedPublicKey</a><br /></code></pre>




<pre><code><b>include</b> <a href="multi_ed25519.md#0x1_multi_ed25519_NewUnvalidatedPublicKeyFromBytesAbortsIf">NewUnvalidatedPublicKeyFromBytesAbortsIf</a>;<br /><b>ensures</b> result &#61;&#61; <a href="multi_ed25519.md#0x1_multi_ed25519_UnvalidatedPublicKey">UnvalidatedPublicKey</a> &#123; bytes &#125;;<br /></code></pre>




<a id="0x1_multi_ed25519_NewUnvalidatedPublicKeyFromBytesAbortsIf"></a>


<pre><code><b>schema</b> <a href="multi_ed25519.md#0x1_multi_ed25519_NewUnvalidatedPublicKeyFromBytesAbortsIf">NewUnvalidatedPublicKeyFromBytesAbortsIf</a> &#123;<br />bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;<br /><b>let</b> length &#61; len(bytes);<br /><b>aborts_if</b> length / <a href="multi_ed25519.md#0x1_multi_ed25519_INDIVIDUAL_PUBLIC_KEY_NUM_BYTES">INDIVIDUAL_PUBLIC_KEY_NUM_BYTES</a> &gt; <a href="multi_ed25519.md#0x1_multi_ed25519_MAX_NUMBER_OF_PUBLIC_KEYS">MAX_NUMBER_OF_PUBLIC_KEYS</a>;<br /><b>aborts_if</b> length % <a href="multi_ed25519.md#0x1_multi_ed25519_INDIVIDUAL_PUBLIC_KEY_NUM_BYTES">INDIVIDUAL_PUBLIC_KEY_NUM_BYTES</a> !&#61; <a href="multi_ed25519.md#0x1_multi_ed25519_THRESHOLD_SIZE_BYTES">THRESHOLD_SIZE_BYTES</a>;<br />&#125;<br /></code></pre>



<a id="@Specification_1_new_validated_public_key_from_bytes"></a>

### Function `new_validated_public_key_from_bytes`


<pre><code><b>public</b> <b>fun</b> <a href="multi_ed25519.md#0x1_multi_ed25519_new_validated_public_key_from_bytes">new_validated_public_key_from_bytes</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="multi_ed25519.md#0x1_multi_ed25519_ValidatedPublicKey">multi_ed25519::ValidatedPublicKey</a>&gt;<br /></code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;<br /><b>let</b> cond &#61; len(bytes) % <a href="multi_ed25519.md#0x1_multi_ed25519_INDIVIDUAL_PUBLIC_KEY_NUM_BYTES">INDIVIDUAL_PUBLIC_KEY_NUM_BYTES</a> &#61;&#61; <a href="multi_ed25519.md#0x1_multi_ed25519_THRESHOLD_SIZE_BYTES">THRESHOLD_SIZE_BYTES</a><br />    &amp;&amp; <a href="multi_ed25519.md#0x1_multi_ed25519_spec_public_key_validate_internal">spec_public_key_validate_internal</a>(bytes);<br /><b>ensures</b> cond &#61;&#61;&gt; result &#61;&#61; <a href="../../move-stdlib/doc/option.md#0x1_option_spec_some">option::spec_some</a>(<a href="multi_ed25519.md#0x1_multi_ed25519_ValidatedPublicKey">ValidatedPublicKey</a>&#123;bytes&#125;);<br /><b>ensures</b> !cond &#61;&#61;&gt; result &#61;&#61; <a href="../../move-stdlib/doc/option.md#0x1_option_spec_none">option::spec_none</a>&lt;<a href="multi_ed25519.md#0x1_multi_ed25519_ValidatedPublicKey">ValidatedPublicKey</a>&gt;();<br /></code></pre>



<a id="@Specification_1_new_validated_public_key_from_bytes_v2"></a>

### Function `new_validated_public_key_from_bytes_v2`


<pre><code><b>public</b> <b>fun</b> <a href="multi_ed25519.md#0x1_multi_ed25519_new_validated_public_key_from_bytes_v2">new_validated_public_key_from_bytes_v2</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="multi_ed25519.md#0x1_multi_ed25519_ValidatedPublicKey">multi_ed25519::ValidatedPublicKey</a>&gt;<br /></code></pre>




<pre><code><b>let</b> cond &#61; <a href="multi_ed25519.md#0x1_multi_ed25519_spec_public_key_validate_v2_internal">spec_public_key_validate_v2_internal</a>(bytes);<br /><b>ensures</b> cond &#61;&#61;&gt; result &#61;&#61; <a href="../../move-stdlib/doc/option.md#0x1_option_spec_some">option::spec_some</a>(<a href="multi_ed25519.md#0x1_multi_ed25519_ValidatedPublicKey">ValidatedPublicKey</a>&#123;bytes&#125;);<br /><b>ensures</b> !cond &#61;&#61;&gt; result &#61;&#61; <a href="../../move-stdlib/doc/option.md#0x1_option_spec_none">option::spec_none</a>&lt;<a href="multi_ed25519.md#0x1_multi_ed25519_ValidatedPublicKey">ValidatedPublicKey</a>&gt;();<br /></code></pre>



<a id="@Specification_1_new_signature_from_bytes"></a>

### Function `new_signature_from_bytes`


<pre><code><b>public</b> <b>fun</b> <a href="multi_ed25519.md#0x1_multi_ed25519_new_signature_from_bytes">new_signature_from_bytes</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="multi_ed25519.md#0x1_multi_ed25519_Signature">multi_ed25519::Signature</a><br /></code></pre>




<pre><code><b>include</b> <a href="multi_ed25519.md#0x1_multi_ed25519_NewSignatureFromBytesAbortsIf">NewSignatureFromBytesAbortsIf</a>;<br /><b>ensures</b> result &#61;&#61; <a href="multi_ed25519.md#0x1_multi_ed25519_Signature">Signature</a> &#123; bytes &#125;;<br /></code></pre>




<a id="0x1_multi_ed25519_NewSignatureFromBytesAbortsIf"></a>


<pre><code><b>schema</b> <a href="multi_ed25519.md#0x1_multi_ed25519_NewSignatureFromBytesAbortsIf">NewSignatureFromBytesAbortsIf</a> &#123;<br />bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;<br /><b>aborts_if</b> len(bytes) % <a href="multi_ed25519.md#0x1_multi_ed25519_INDIVIDUAL_SIGNATURE_NUM_BYTES">INDIVIDUAL_SIGNATURE_NUM_BYTES</a> !&#61; <a href="multi_ed25519.md#0x1_multi_ed25519_BITMAP_NUM_OF_BYTES">BITMAP_NUM_OF_BYTES</a>;<br />&#125;<br /></code></pre>



<a id="@Specification_1_unvalidated_public_key_num_sub_pks"></a>

### Function `unvalidated_public_key_num_sub_pks`


<pre><code><b>public</b> <b>fun</b> <a href="multi_ed25519.md#0x1_multi_ed25519_unvalidated_public_key_num_sub_pks">unvalidated_public_key_num_sub_pks</a>(pk: &amp;<a href="multi_ed25519.md#0x1_multi_ed25519_UnvalidatedPublicKey">multi_ed25519::UnvalidatedPublicKey</a>): u8<br /></code></pre>




<pre><code><b>let</b> bytes &#61; pk.bytes;<br /><b>include</b> <a href="multi_ed25519.md#0x1_multi_ed25519_PkDivision">PkDivision</a>;<br /></code></pre>



<a id="@Specification_1_unvalidated_public_key_threshold"></a>

### Function `unvalidated_public_key_threshold`


<pre><code><b>public</b> <b>fun</b> <a href="multi_ed25519.md#0x1_multi_ed25519_unvalidated_public_key_threshold">unvalidated_public_key_threshold</a>(pk: &amp;<a href="multi_ed25519.md#0x1_multi_ed25519_UnvalidatedPublicKey">multi_ed25519::UnvalidatedPublicKey</a>): <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u8&gt;<br /></code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;<br /><b>ensures</b> result &#61;&#61; <a href="multi_ed25519.md#0x1_multi_ed25519_spec_check_and_get_threshold">spec_check_and_get_threshold</a>(pk.bytes);<br /></code></pre>



<a id="@Specification_1_validated_public_key_num_sub_pks"></a>

### Function `validated_public_key_num_sub_pks`


<pre><code><b>public</b> <b>fun</b> <a href="multi_ed25519.md#0x1_multi_ed25519_validated_public_key_num_sub_pks">validated_public_key_num_sub_pks</a>(pk: &amp;<a href="multi_ed25519.md#0x1_multi_ed25519_ValidatedPublicKey">multi_ed25519::ValidatedPublicKey</a>): u8<br /></code></pre>




<pre><code><b>let</b> bytes &#61; pk.bytes;<br /><b>include</b> <a href="multi_ed25519.md#0x1_multi_ed25519_PkDivision">PkDivision</a>;<br /></code></pre>



<a id="@Specification_1_validated_public_key_threshold"></a>

### Function `validated_public_key_threshold`


<pre><code><b>public</b> <b>fun</b> <a href="multi_ed25519.md#0x1_multi_ed25519_validated_public_key_threshold">validated_public_key_threshold</a>(pk: &amp;<a href="multi_ed25519.md#0x1_multi_ed25519_ValidatedPublicKey">multi_ed25519::ValidatedPublicKey</a>): u8<br /></code></pre>




<pre><code><b>aborts_if</b> len(pk.bytes) &#61;&#61; 0;<br /><b>ensures</b> result &#61;&#61; pk.bytes[len(pk.bytes) &#45; 1];<br /></code></pre>



<a id="@Specification_1_check_and_get_threshold"></a>

### Function `check_and_get_threshold`


<pre><code><b>public</b> <b>fun</b> <a href="multi_ed25519.md#0x1_multi_ed25519_check_and_get_threshold">check_and_get_threshold</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u8&gt;<br /></code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;<br /><b>ensures</b> result &#61;&#61; <a href="multi_ed25519.md#0x1_multi_ed25519_spec_check_and_get_threshold">spec_check_and_get_threshold</a>(bytes);<br /></code></pre>




<a id="0x1_multi_ed25519_PkDivision"></a>


<pre><code><b>schema</b> <a href="multi_ed25519.md#0x1_multi_ed25519_PkDivision">PkDivision</a> &#123;<br />bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;<br />result: u8;<br /><b>aborts_if</b> len(bytes) / <a href="multi_ed25519.md#0x1_multi_ed25519_INDIVIDUAL_PUBLIC_KEY_NUM_BYTES">INDIVIDUAL_PUBLIC_KEY_NUM_BYTES</a> &gt; MAX_U8;<br /><b>ensures</b> result &#61;&#61; len(bytes) / <a href="multi_ed25519.md#0x1_multi_ed25519_INDIVIDUAL_PUBLIC_KEY_NUM_BYTES">INDIVIDUAL_PUBLIC_KEY_NUM_BYTES</a>;<br />&#125;<br /></code></pre>



<a id="@Specification_1_public_key_bytes_to_authentication_key"></a>

### Function `public_key_bytes_to_authentication_key`


<pre><code><b>fun</b> <a href="multi_ed25519.md#0x1_multi_ed25519_public_key_bytes_to_authentication_key">public_key_bytes_to_authentication_key</a>(pk_bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;<br /></code></pre>




<pre><code><b>pragma</b> opaque;<br /><b>aborts_if</b> <b>false</b>;<br /><b>ensures</b> [abstract] result &#61;&#61; <a href="multi_ed25519.md#0x1_multi_ed25519_spec_public_key_bytes_to_authentication_key">spec_public_key_bytes_to_authentication_key</a>(pk_bytes);<br /></code></pre>



<a id="@Specification_1_public_key_validate_internal"></a>

### Function `public_key_validate_internal`


<pre><code><b>fun</b> <a href="multi_ed25519.md#0x1_multi_ed25519_public_key_validate_internal">public_key_validate_internal</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool<br /></code></pre>




<pre><code><b>pragma</b> opaque;<br /><b>aborts_if</b> <b>false</b>;<br /><b>ensures</b> (len(bytes) / <a href="multi_ed25519.md#0x1_multi_ed25519_INDIVIDUAL_PUBLIC_KEY_NUM_BYTES">INDIVIDUAL_PUBLIC_KEY_NUM_BYTES</a> &gt; <a href="multi_ed25519.md#0x1_multi_ed25519_MAX_NUMBER_OF_PUBLIC_KEYS">MAX_NUMBER_OF_PUBLIC_KEYS</a>) &#61;&#61;&gt; (result &#61;&#61; <b>false</b>);<br /><b>ensures</b> result &#61;&#61; <a href="multi_ed25519.md#0x1_multi_ed25519_spec_public_key_validate_internal">spec_public_key_validate_internal</a>(bytes);<br /></code></pre>



<a id="@Specification_1_public_key_validate_v2_internal"></a>

### Function `public_key_validate_v2_internal`


<pre><code><b>fun</b> <a href="multi_ed25519.md#0x1_multi_ed25519_public_key_validate_v2_internal">public_key_validate_v2_internal</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool<br /></code></pre>




<pre><code><b>pragma</b> opaque;<br /><b>ensures</b> result &#61;&#61; <a href="multi_ed25519.md#0x1_multi_ed25519_spec_public_key_validate_v2_internal">spec_public_key_validate_v2_internal</a>(bytes);<br /></code></pre>



<a id="@Specification_1_signature_verify_strict_internal"></a>

### Function `signature_verify_strict_internal`


<pre><code><b>fun</b> <a href="multi_ed25519.md#0x1_multi_ed25519_signature_verify_strict_internal">signature_verify_strict_internal</a>(multisignature: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, public_key: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, message: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool<br /></code></pre>




<pre><code><b>pragma</b> opaque;<br /><b>aborts_if</b> <b>false</b>;<br /><b>ensures</b> result &#61;&#61; <a href="multi_ed25519.md#0x1_multi_ed25519_spec_signature_verify_strict_internal">spec_signature_verify_strict_internal</a>(multisignature, public_key, message);<br /></code></pre>



<a id="@Helper_functions_2"></a>

### Helper functions



<a id="0x1_multi_ed25519_spec_check_and_get_threshold"></a>


<pre><code><b>fun</b> <a href="multi_ed25519.md#0x1_multi_ed25519_spec_check_and_get_threshold">spec_check_and_get_threshold</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): Option&lt;u8&gt; &#123;<br />   <b>let</b> len &#61; len(bytes);<br />   <b>if</b> (len &#61;&#61; 0) &#123;<br />       <a href="../../move-stdlib/doc/option.md#0x1_option_none">option::none</a>&lt;u8&gt;()<br />   &#125; <b>else</b> &#123;<br />       <b>let</b> threshold_num_of_bytes &#61; len % <a href="multi_ed25519.md#0x1_multi_ed25519_INDIVIDUAL_PUBLIC_KEY_NUM_BYTES">INDIVIDUAL_PUBLIC_KEY_NUM_BYTES</a>;<br />       <b>let</b> num_of_keys &#61; len / <a href="multi_ed25519.md#0x1_multi_ed25519_INDIVIDUAL_PUBLIC_KEY_NUM_BYTES">INDIVIDUAL_PUBLIC_KEY_NUM_BYTES</a>;<br />       <b>let</b> threshold_byte &#61; bytes[len &#45; 1];<br />       <b>if</b> (num_of_keys &#61;&#61; 0 &#124;&#124; num_of_keys &gt; <a href="multi_ed25519.md#0x1_multi_ed25519_MAX_NUMBER_OF_PUBLIC_KEYS">MAX_NUMBER_OF_PUBLIC_KEYS</a> &#124;&#124; len % <a href="multi_ed25519.md#0x1_multi_ed25519_INDIVIDUAL_PUBLIC_KEY_NUM_BYTES">INDIVIDUAL_PUBLIC_KEY_NUM_BYTES</a> !&#61; 1) &#123;<br />           <a href="../../move-stdlib/doc/option.md#0x1_option_none">option::none</a>&lt;u8&gt;()<br />       &#125; <b>else</b> <b>if</b> (threshold_byte &#61;&#61; 0 &#124;&#124; threshold_byte &gt; (num_of_keys <b>as</b> u8)) &#123;<br />           <a href="../../move-stdlib/doc/option.md#0x1_option_none">option::none</a>&lt;u8&gt;()<br />       &#125; <b>else</b> &#123;<br />           <a href="../../move-stdlib/doc/option.md#0x1_option_spec_some">option::spec_some</a>(threshold_byte)<br />       &#125;<br />   &#125;<br />&#125;<br /></code></pre>




<a id="0x1_multi_ed25519_spec_signature_verify_strict_internal"></a>


<pre><code><b>fun</b> <a href="multi_ed25519.md#0x1_multi_ed25519_spec_signature_verify_strict_internal">spec_signature_verify_strict_internal</a>(<br />   multisignature: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,<br />   public_key: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,<br />   message: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;<br />): bool;<br /></code></pre>




<a id="0x1_multi_ed25519_spec_public_key_validate_internal"></a>


<pre><code><b>fun</b> <a href="multi_ed25519.md#0x1_multi_ed25519_spec_public_key_validate_internal">spec_public_key_validate_internal</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool;<br /></code></pre>




<a id="0x1_multi_ed25519_spec_public_key_validate_v2_internal"></a>


<pre><code><b>fun</b> <a href="multi_ed25519.md#0x1_multi_ed25519_spec_public_key_validate_v2_internal">spec_public_key_validate_v2_internal</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool;<br /></code></pre>




<a id="0x1_multi_ed25519_spec_public_key_bytes_to_authentication_key"></a>


<pre><code><b>fun</b> <a href="multi_ed25519.md#0x1_multi_ed25519_spec_public_key_bytes_to_authentication_key">spec_public_key_bytes_to_authentication_key</a>(pk_bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;<br /></code></pre>




<a id="0x1_multi_ed25519_spec_signature_verify_strict_t"></a>


<pre><code><b>fun</b> <a href="multi_ed25519.md#0x1_multi_ed25519_spec_signature_verify_strict_t">spec_signature_verify_strict_t</a>&lt;T&gt;(signature: <a href="multi_ed25519.md#0x1_multi_ed25519_Signature">Signature</a>, public_key: <a href="multi_ed25519.md#0x1_multi_ed25519_UnvalidatedPublicKey">UnvalidatedPublicKey</a>, data: T): bool &#123;<br />   <b>let</b> encoded &#61; <a href="ed25519.md#0x1_ed25519_new_signed_message">ed25519::new_signed_message</a>&lt;T&gt;(data);<br />   <b>let</b> message &#61; <a href="../../move-stdlib/doc/bcs.md#0x1_bcs_serialize">bcs::serialize</a>(encoded);<br />   <a href="multi_ed25519.md#0x1_multi_ed25519_spec_signature_verify_strict_internal">spec_signature_verify_strict_internal</a>(signature.bytes, public_key.bytes, message)<br />&#125;<br /></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
