
<a id="0x1_multi_ed25519"></a>

# Module `0x1::multi_ed25519`

Exports MultiEd25519 multi-signatures in Move.
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


<pre><code>use 0x1::bcs;
use 0x1::ed25519;
use 0x1::error;
use 0x1::features;
use 0x1::hash;
use 0x1::option;
</code></pre>



<a id="0x1_multi_ed25519_UnvalidatedPublicKey"></a>

## Struct `UnvalidatedPublicKey`

An *unvalidated*, k out of n MultiEd25519 public key. The <code>bytes</code> field contains (1) several chunks of
<code>ed25519::PUBLIC_KEY_NUM_BYTES</code> bytes, each encoding a Ed25519 PK, and (2) a single byte encoding the threshold k.
*Unvalidated* means there is no guarantee that the underlying PKs are valid elliptic curve points of non-small
order.


<pre><code>struct UnvalidatedPublicKey has copy, drop, store
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

<a id="0x1_multi_ed25519_ValidatedPublicKey"></a>

## Struct `ValidatedPublicKey`

A *validated* k out of n MultiEd25519 public key. *Validated* means that all the underlying PKs will be
elliptic curve points that are NOT of small-order. It does not necessarily mean they will be prime-order points.
This struct encodes the public key exactly as <code>UnvalidatedPublicKey</code>.

For now, this struct is not used in any verification functions, but it might be in the future.


<pre><code>struct ValidatedPublicKey has copy, drop, store
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

<a id="0x1_multi_ed25519_Signature"></a>

## Struct `Signature`

A purported MultiEd25519 multi-signature that can be verified via <code>signature_verify_strict</code> or
<code>signature_verify_strict_t</code>. The <code>bytes</code> field contains (1) several chunks of <code>ed25519::SIGNATURE_NUM_BYTES</code>
bytes, each encoding a Ed25519 signature, and (2) a <code>BITMAP_NUM_OF_BYTES</code>-byte bitmap encoding the signer
identities.


<pre><code>struct Signature has copy, drop, store
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


<a id="0x1_multi_ed25519_E_NATIVE_FUN_NOT_AVAILABLE"></a>

The native functions have not been rolled out yet.


<pre><code>const E_NATIVE_FUN_NOT_AVAILABLE: u64 &#61; 4;
</code></pre>



<a id="0x1_multi_ed25519_E_WRONG_PUBKEY_SIZE"></a>

Wrong number of bytes were given as input when deserializing an Ed25519 public key.


<pre><code>const E_WRONG_PUBKEY_SIZE: u64 &#61; 1;
</code></pre>



<a id="0x1_multi_ed25519_E_WRONG_SIGNATURE_SIZE"></a>

Wrong number of bytes were given as input when deserializing an Ed25519 signature.


<pre><code>const E_WRONG_SIGNATURE_SIZE: u64 &#61; 2;
</code></pre>



<a id="0x1_multi_ed25519_SIGNATURE_SCHEME_ID"></a>

The identifier of the MultiEd25519 signature scheme, which is used when deriving Aptos authentication keys by hashing
it together with an MultiEd25519 public key.


<pre><code>const SIGNATURE_SCHEME_ID: u8 &#61; 1;
</code></pre>



<a id="0x1_multi_ed25519_BITMAP_NUM_OF_BYTES"></a>

When serializing a MultiEd25519 signature, the bitmap that indicates the signers will be encoded using this many
bytes.


<pre><code>const BITMAP_NUM_OF_BYTES: u64 &#61; 4;
</code></pre>



<a id="0x1_multi_ed25519_E_INVALID_THRESHOLD_OR_NUMBER_OF_SIGNERS"></a>

The threshold must be in the range <code>[1, n]</code>, where n is the total number of signers.


<pre><code>const E_INVALID_THRESHOLD_OR_NUMBER_OF_SIGNERS: u64 &#61; 3;
</code></pre>



<a id="0x1_multi_ed25519_INDIVIDUAL_PUBLIC_KEY_NUM_BYTES"></a>

The size of an individual Ed25519 public key, in bytes.
(A MultiEd25519 public key consists of several of these, plus the threshold.)


<pre><code>const INDIVIDUAL_PUBLIC_KEY_NUM_BYTES: u64 &#61; 32;
</code></pre>



<a id="0x1_multi_ed25519_INDIVIDUAL_SIGNATURE_NUM_BYTES"></a>

The size of an individual Ed25519 signature, in bytes.
(A MultiEd25519 signature consists of several of these, plus the signer bitmap.)


<pre><code>const INDIVIDUAL_SIGNATURE_NUM_BYTES: u64 &#61; 64;
</code></pre>



<a id="0x1_multi_ed25519_MAX_NUMBER_OF_PUBLIC_KEYS"></a>

Max number of ed25519 public keys allowed in multi-ed25519 keys


<pre><code>const MAX_NUMBER_OF_PUBLIC_KEYS: u64 &#61; 32;
</code></pre>



<a id="0x1_multi_ed25519_THRESHOLD_SIZE_BYTES"></a>

When serializing a MultiEd25519 public key, the threshold k will be encoded using this many bytes.


<pre><code>const THRESHOLD_SIZE_BYTES: u64 &#61; 1;
</code></pre>



<a id="0x1_multi_ed25519_new_unvalidated_public_key_from_bytes"></a>

## Function `new_unvalidated_public_key_from_bytes`

Parses the input 32 bytes as an *unvalidated* MultiEd25519 public key.

NOTE: This function could have also checked that the # of sub-PKs is > 0, but it did not. However, since such
invalid PKs are rejected during signature verification  (see <code>bugfix_unvalidated_pk_from_zero_subpks</code>) they
will not cause problems.

We could fix this API by adding a new one that checks the # of sub-PKs is > 0, but it is likely not a good idea
to reproduce the PK validation logic in Move. We should not have done so in the first place. Instead, we will
leave it as is and continue assuming <code>UnvalidatedPublicKey</code> objects could be invalid PKs that will safely be
rejected during signature verification.


<pre><code>public fun new_unvalidated_public_key_from_bytes(bytes: vector&lt;u8&gt;): multi_ed25519::UnvalidatedPublicKey
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun new_unvalidated_public_key_from_bytes(bytes: vector&lt;u8&gt;): UnvalidatedPublicKey &#123;
    let len &#61; vector::length(&amp;bytes);
    let num_sub_pks &#61; len / INDIVIDUAL_PUBLIC_KEY_NUM_BYTES;

    assert!(num_sub_pks &lt;&#61; MAX_NUMBER_OF_PUBLIC_KEYS, error::invalid_argument(E_WRONG_PUBKEY_SIZE));
    assert!(len % INDIVIDUAL_PUBLIC_KEY_NUM_BYTES &#61;&#61; THRESHOLD_SIZE_BYTES, error::invalid_argument(E_WRONG_PUBKEY_SIZE));
    UnvalidatedPublicKey &#123; bytes &#125;
&#125;
</code></pre>



</details>

<a id="0x1_multi_ed25519_new_validated_public_key_from_bytes"></a>

## Function `new_validated_public_key_from_bytes`

DEPRECATED: Use <code>new_validated_public_key_from_bytes_v2</code> instead. See <code>public_key_validate_internal</code> comments.

(Incorrectly) parses the input bytes as a *validated* MultiEd25519 public key.


<pre><code>public fun new_validated_public_key_from_bytes(bytes: vector&lt;u8&gt;): option::Option&lt;multi_ed25519::ValidatedPublicKey&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun new_validated_public_key_from_bytes(bytes: vector&lt;u8&gt;): Option&lt;ValidatedPublicKey&gt; &#123;
    // Note that `public_key_validate_internal` will check that `vector::length(&amp;bytes) / INDIVIDUAL_PUBLIC_KEY_NUM_BYTES &lt;&#61; MAX_NUMBER_OF_PUBLIC_KEYS`.
    if (vector::length(&amp;bytes) % INDIVIDUAL_PUBLIC_KEY_NUM_BYTES &#61;&#61; THRESHOLD_SIZE_BYTES &amp;&amp;
        public_key_validate_internal(bytes)) &#123;
        option::some(ValidatedPublicKey &#123;
            bytes
        &#125;)
    &#125; else &#123;
        option::none&lt;ValidatedPublicKey&gt;()
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_multi_ed25519_new_validated_public_key_from_bytes_v2"></a>

## Function `new_validated_public_key_from_bytes_v2`

Parses the input bytes as a *validated* MultiEd25519 public key (see <code>public_key_validate_internal_v2</code>).


<pre><code>public fun new_validated_public_key_from_bytes_v2(bytes: vector&lt;u8&gt;): option::Option&lt;multi_ed25519::ValidatedPublicKey&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun new_validated_public_key_from_bytes_v2(bytes: vector&lt;u8&gt;): Option&lt;ValidatedPublicKey&gt; &#123;
    if (!features::multi_ed25519_pk_validate_v2_enabled()) &#123;
        abort(error::invalid_state(E_NATIVE_FUN_NOT_AVAILABLE))
    &#125;;

    if (public_key_validate_v2_internal(bytes)) &#123;
        option::some(ValidatedPublicKey &#123;
            bytes
        &#125;)
    &#125; else &#123;
        option::none&lt;ValidatedPublicKey&gt;()
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_multi_ed25519_new_signature_from_bytes"></a>

## Function `new_signature_from_bytes`

Parses the input bytes as a purported MultiEd25519 multi-signature.


<pre><code>public fun new_signature_from_bytes(bytes: vector&lt;u8&gt;): multi_ed25519::Signature
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun new_signature_from_bytes(bytes: vector&lt;u8&gt;): Signature &#123;
    assert!(vector::length(&amp;bytes) % INDIVIDUAL_SIGNATURE_NUM_BYTES &#61;&#61; BITMAP_NUM_OF_BYTES, error::invalid_argument(E_WRONG_SIGNATURE_SIZE));
    Signature &#123; bytes &#125;
&#125;
</code></pre>



</details>

<a id="0x1_multi_ed25519_public_key_to_unvalidated"></a>

## Function `public_key_to_unvalidated`

Converts a ValidatedPublicKey to an UnvalidatedPublicKey, which can be used in the strict verification APIs.


<pre><code>public fun public_key_to_unvalidated(pk: &amp;multi_ed25519::ValidatedPublicKey): multi_ed25519::UnvalidatedPublicKey
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun public_key_to_unvalidated(pk: &amp;ValidatedPublicKey): UnvalidatedPublicKey &#123;
    UnvalidatedPublicKey &#123;
        bytes: pk.bytes
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_multi_ed25519_public_key_into_unvalidated"></a>

## Function `public_key_into_unvalidated`

Moves a ValidatedPublicKey into an UnvalidatedPublicKey, which can be used in the strict verification APIs.


<pre><code>public fun public_key_into_unvalidated(pk: multi_ed25519::ValidatedPublicKey): multi_ed25519::UnvalidatedPublicKey
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun public_key_into_unvalidated(pk: ValidatedPublicKey): UnvalidatedPublicKey &#123;
    UnvalidatedPublicKey &#123;
        bytes: pk.bytes
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_multi_ed25519_unvalidated_public_key_to_bytes"></a>

## Function `unvalidated_public_key_to_bytes`

Serializes an UnvalidatedPublicKey struct to 32-bytes.


<pre><code>public fun unvalidated_public_key_to_bytes(pk: &amp;multi_ed25519::UnvalidatedPublicKey): vector&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun unvalidated_public_key_to_bytes(pk: &amp;UnvalidatedPublicKey): vector&lt;u8&gt; &#123;
    pk.bytes
&#125;
</code></pre>



</details>

<a id="0x1_multi_ed25519_validated_public_key_to_bytes"></a>

## Function `validated_public_key_to_bytes`

Serializes a ValidatedPublicKey struct to 32-bytes.


<pre><code>public fun validated_public_key_to_bytes(pk: &amp;multi_ed25519::ValidatedPublicKey): vector&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun validated_public_key_to_bytes(pk: &amp;ValidatedPublicKey): vector&lt;u8&gt; &#123;
    pk.bytes
&#125;
</code></pre>



</details>

<a id="0x1_multi_ed25519_signature_to_bytes"></a>

## Function `signature_to_bytes`

Serializes a Signature struct to 64-bytes.


<pre><code>public fun signature_to_bytes(sig: &amp;multi_ed25519::Signature): vector&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun signature_to_bytes(sig: &amp;Signature): vector&lt;u8&gt; &#123;
    sig.bytes
&#125;
</code></pre>



</details>

<a id="0x1_multi_ed25519_public_key_validate"></a>

## Function `public_key_validate`

DEPRECATED: Use <code>public_key_validate_v2</code> instead. See <code>public_key_validate_internal</code> comments.

Takes in an *unvalidated* public key and attempts to validate it.
Returns <code>Some(ValidatedPublicKey)</code> if successful and <code>None</code> otherwise.


<pre><code>public fun public_key_validate(pk: &amp;multi_ed25519::UnvalidatedPublicKey): option::Option&lt;multi_ed25519::ValidatedPublicKey&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun public_key_validate(pk: &amp;UnvalidatedPublicKey): Option&lt;ValidatedPublicKey&gt; &#123;
    new_validated_public_key_from_bytes(pk.bytes)
&#125;
</code></pre>



</details>

<a id="0x1_multi_ed25519_public_key_validate_v2"></a>

## Function `public_key_validate_v2`

Takes in an *unvalidated* public key and attempts to validate it.
Returns <code>Some(ValidatedPublicKey)</code> if successful and <code>None</code> otherwise.


<pre><code>public fun public_key_validate_v2(pk: &amp;multi_ed25519::UnvalidatedPublicKey): option::Option&lt;multi_ed25519::ValidatedPublicKey&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun public_key_validate_v2(pk: &amp;UnvalidatedPublicKey): Option&lt;ValidatedPublicKey&gt; &#123;
    new_validated_public_key_from_bytes_v2(pk.bytes)
&#125;
</code></pre>



</details>

<a id="0x1_multi_ed25519_signature_verify_strict"></a>

## Function `signature_verify_strict`

Verifies a purported MultiEd25519 <code>multisignature</code> under an *unvalidated* <code>public_key</code> on the specified <code>message</code>.
This call will validate the public key by checking it is NOT in the small subgroup.


<pre><code>public fun signature_verify_strict(multisignature: &amp;multi_ed25519::Signature, public_key: &amp;multi_ed25519::UnvalidatedPublicKey, message: vector&lt;u8&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun signature_verify_strict(
    multisignature: &amp;Signature,
    public_key: &amp;UnvalidatedPublicKey,
    message: vector&lt;u8&gt;
): bool &#123;
    signature_verify_strict_internal(multisignature.bytes, public_key.bytes, message)
&#125;
</code></pre>



</details>

<a id="0x1_multi_ed25519_signature_verify_strict_t"></a>

## Function `signature_verify_strict_t`

This function is used to verify a multi-signature on any BCS-serializable type T. For now, it is used to verify the
proof of private key ownership when rotating authentication keys.


<pre><code>public fun signature_verify_strict_t&lt;T: drop&gt;(multisignature: &amp;multi_ed25519::Signature, public_key: &amp;multi_ed25519::UnvalidatedPublicKey, data: T): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun signature_verify_strict_t&lt;T: drop&gt;(multisignature: &amp;Signature, public_key: &amp;UnvalidatedPublicKey, data: T): bool &#123;
    let encoded &#61; ed25519::new_signed_message(data);

    signature_verify_strict_internal(multisignature.bytes, public_key.bytes, bcs::to_bytes(&amp;encoded))
&#125;
</code></pre>



</details>

<a id="0x1_multi_ed25519_unvalidated_public_key_to_authentication_key"></a>

## Function `unvalidated_public_key_to_authentication_key`

Derives the Aptos-specific authentication key of the given Ed25519 public key.


<pre><code>public fun unvalidated_public_key_to_authentication_key(pk: &amp;multi_ed25519::UnvalidatedPublicKey): vector&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun unvalidated_public_key_to_authentication_key(pk: &amp;UnvalidatedPublicKey): vector&lt;u8&gt; &#123;
    public_key_bytes_to_authentication_key(pk.bytes)
&#125;
</code></pre>



</details>

<a id="0x1_multi_ed25519_unvalidated_public_key_num_sub_pks"></a>

## Function `unvalidated_public_key_num_sub_pks`

Returns the number n of sub-PKs in an unvalidated t-out-of-n MultiEd25519 PK.
If this <code>UnvalidatedPublicKey</code> would pass validation in <code>public_key_validate</code>, then the returned # of sub-PKs
can be relied upon as correct.

We provide this API as a cheaper alternative to calling <code>public_key_validate</code> and then <code>validated_public_key_num_sub_pks</code>
when the input <code>pk</code> is known to be valid.


<pre><code>public fun unvalidated_public_key_num_sub_pks(pk: &amp;multi_ed25519::UnvalidatedPublicKey): u8
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun unvalidated_public_key_num_sub_pks(pk: &amp;UnvalidatedPublicKey): u8 &#123;
    let len &#61; vector::length(&amp;pk.bytes);

    ((len / INDIVIDUAL_PUBLIC_KEY_NUM_BYTES) as u8)
&#125;
</code></pre>



</details>

<a id="0x1_multi_ed25519_unvalidated_public_key_threshold"></a>

## Function `unvalidated_public_key_threshold`

Returns the number t of sub-PKs in an unvalidated t-out-of-n MultiEd25519 PK (i.e., the threshold) or <code>None</code>
if <code>bytes</code> does not correctly encode such a PK.


<pre><code>public fun unvalidated_public_key_threshold(pk: &amp;multi_ed25519::UnvalidatedPublicKey): option::Option&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun unvalidated_public_key_threshold(pk: &amp;UnvalidatedPublicKey): Option&lt;u8&gt; &#123;
    check_and_get_threshold(pk.bytes)
&#125;
</code></pre>



</details>

<a id="0x1_multi_ed25519_validated_public_key_to_authentication_key"></a>

## Function `validated_public_key_to_authentication_key`

Derives the Aptos-specific authentication key of the given Ed25519 public key.


<pre><code>public fun validated_public_key_to_authentication_key(pk: &amp;multi_ed25519::ValidatedPublicKey): vector&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun validated_public_key_to_authentication_key(pk: &amp;ValidatedPublicKey): vector&lt;u8&gt; &#123;
    public_key_bytes_to_authentication_key(pk.bytes)
&#125;
</code></pre>



</details>

<a id="0x1_multi_ed25519_validated_public_key_num_sub_pks"></a>

## Function `validated_public_key_num_sub_pks`

Returns the number n of sub-PKs in a validated t-out-of-n MultiEd25519 PK.
Since the format of this PK has been validated, the returned # of sub-PKs is guaranteed to be correct.


<pre><code>public fun validated_public_key_num_sub_pks(pk: &amp;multi_ed25519::ValidatedPublicKey): u8
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun validated_public_key_num_sub_pks(pk: &amp;ValidatedPublicKey): u8 &#123;
    let len &#61; vector::length(&amp;pk.bytes);

    ((len / INDIVIDUAL_PUBLIC_KEY_NUM_BYTES) as u8)
&#125;
</code></pre>



</details>

<a id="0x1_multi_ed25519_validated_public_key_threshold"></a>

## Function `validated_public_key_threshold`

Returns the number t of sub-PKs in a validated t-out-of-n MultiEd25519 PK (i.e., the threshold).


<pre><code>public fun validated_public_key_threshold(pk: &amp;multi_ed25519::ValidatedPublicKey): u8
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun validated_public_key_threshold(pk: &amp;ValidatedPublicKey): u8 &#123;
    let len &#61; vector::length(&amp;pk.bytes);
    let threshold_byte &#61; &#42;vector::borrow(&amp;pk.bytes, len &#45; 1);

    threshold_byte
&#125;
</code></pre>



</details>

<a id="0x1_multi_ed25519_check_and_get_threshold"></a>

## Function `check_and_get_threshold`

Checks that the serialized format of a t-out-of-n MultiEd25519 PK correctly encodes 1 <= n <= 32 sub-PKs.
(All <code>ValidatedPublicKey</code> objects are guaranteed to pass this check.)
Returns the threshold t <= n of the PK.


<pre><code>public fun check_and_get_threshold(bytes: vector&lt;u8&gt;): option::Option&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun check_and_get_threshold(bytes: vector&lt;u8&gt;): Option&lt;u8&gt; &#123;
    let len &#61; vector::length(&amp;bytes);
    if (len &#61;&#61; 0) &#123;
        return option::none&lt;u8&gt;()
    &#125;;

    let threshold_num_of_bytes &#61; len % INDIVIDUAL_PUBLIC_KEY_NUM_BYTES;
    let num_of_keys &#61; len / INDIVIDUAL_PUBLIC_KEY_NUM_BYTES;
    let threshold_byte &#61; &#42;vector::borrow(&amp;bytes, len &#45; 1);

    if (num_of_keys &#61;&#61; 0 &#124;&#124; num_of_keys &gt; MAX_NUMBER_OF_PUBLIC_KEYS &#124;&#124; threshold_num_of_bytes !&#61; 1) &#123;
        return option::none&lt;u8&gt;()
    &#125; else if (threshold_byte &#61;&#61; 0 &#124;&#124; threshold_byte &gt; (num_of_keys as u8)) &#123;
        return option::none&lt;u8&gt;()
    &#125; else &#123;
        return option::some(threshold_byte)
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_multi_ed25519_public_key_bytes_to_authentication_key"></a>

## Function `public_key_bytes_to_authentication_key`

Derives the Aptos-specific authentication key of the given Ed25519 public key.


<pre><code>fun public_key_bytes_to_authentication_key(pk_bytes: vector&lt;u8&gt;): vector&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun public_key_bytes_to_authentication_key(pk_bytes: vector&lt;u8&gt;): vector&lt;u8&gt; &#123;
    vector::push_back(&amp;mut pk_bytes, SIGNATURE_SCHEME_ID);
    std::hash::sha3_256(pk_bytes)
&#125;
</code></pre>



</details>

<a id="0x1_multi_ed25519_public_key_validate_internal"></a>

## Function `public_key_validate_internal`

DEPRECATED: Use <code>public_key_validate_internal_v2</code> instead. This function was NOT correctly implemented:

1. It does not check that the # of sub public keys is > 0, which leads to invalid <code>ValidatedPublicKey</code> objects
against which no signature will verify, since <code>signature_verify_strict_internal</code> will reject such invalid PKs.
This is not a security issue, but a correctness issue. See <code>bugfix_validated_pk_from_zero_subpks</code>.
2. It charges too much gas: if the first sub-PK is invalid, it will charge for verifying all remaining sub-PKs.

DEPRECATES:
- new_validated_public_key_from_bytes
- public_key_validate

Return <code>true</code> if the bytes in <code>public_key</code> can be parsed as a valid MultiEd25519 public key: i.e., all underlying
PKs pass point-on-curve and not-in-small-subgroup checks.
Returns <code>false</code> otherwise.


<pre><code>fun public_key_validate_internal(bytes: vector&lt;u8&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun public_key_validate_internal(bytes: vector&lt;u8&gt;): bool;
</code></pre>



</details>

<a id="0x1_multi_ed25519_public_key_validate_v2_internal"></a>

## Function `public_key_validate_v2_internal`

Return <code>true</code> if the bytes in <code>public_key</code> can be parsed as a valid MultiEd25519 public key: i.e., all underlying
sub-PKs pass point-on-curve and not-in-small-subgroup checks.
Returns <code>false</code> otherwise.


<pre><code>fun public_key_validate_v2_internal(bytes: vector&lt;u8&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun public_key_validate_v2_internal(bytes: vector&lt;u8&gt;): bool;
</code></pre>



</details>

<a id="0x1_multi_ed25519_signature_verify_strict_internal"></a>

## Function `signature_verify_strict_internal`

Return true if the MultiEd25519 <code>multisignature</code> on <code>message</code> verifies against the MultiEd25519 <code>public_key</code>.
Returns <code>false</code> if either:
- The PKs in <code>public_key</code> do not all pass points-on-curve or not-in-small-subgroup checks,
- The signatures in <code>multisignature</code> do not all pass points-on-curve or not-in-small-subgroup checks,
- the <code>multisignature</code> on <code>message</code> does not verify.


<pre><code>fun signature_verify_strict_internal(multisignature: vector&lt;u8&gt;, public_key: vector&lt;u8&gt;, message: vector&lt;u8&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun signature_verify_strict_internal(
    multisignature: vector&lt;u8&gt;,
    public_key: vector&lt;u8&gt;,
    message: vector&lt;u8&gt;
): bool;
</code></pre>



</details>

<a id="@Specification_1"></a>

## Specification


<a id="@Specification_1_new_unvalidated_public_key_from_bytes"></a>

### Function `new_unvalidated_public_key_from_bytes`


<pre><code>public fun new_unvalidated_public_key_from_bytes(bytes: vector&lt;u8&gt;): multi_ed25519::UnvalidatedPublicKey
</code></pre>




<pre><code>include NewUnvalidatedPublicKeyFromBytesAbortsIf;
ensures result &#61;&#61; UnvalidatedPublicKey &#123; bytes &#125;;
</code></pre>




<a id="0x1_multi_ed25519_NewUnvalidatedPublicKeyFromBytesAbortsIf"></a>


<pre><code>schema NewUnvalidatedPublicKeyFromBytesAbortsIf &#123;
    bytes: vector&lt;u8&gt;;
    let length &#61; len(bytes);
    aborts_if length / INDIVIDUAL_PUBLIC_KEY_NUM_BYTES &gt; MAX_NUMBER_OF_PUBLIC_KEYS;
    aborts_if length % INDIVIDUAL_PUBLIC_KEY_NUM_BYTES !&#61; THRESHOLD_SIZE_BYTES;
&#125;
</code></pre>



<a id="@Specification_1_new_validated_public_key_from_bytes"></a>

### Function `new_validated_public_key_from_bytes`


<pre><code>public fun new_validated_public_key_from_bytes(bytes: vector&lt;u8&gt;): option::Option&lt;multi_ed25519::ValidatedPublicKey&gt;
</code></pre>




<pre><code>aborts_if false;
let cond &#61; len(bytes) % INDIVIDUAL_PUBLIC_KEY_NUM_BYTES &#61;&#61; THRESHOLD_SIZE_BYTES
    &amp;&amp; spec_public_key_validate_internal(bytes);
ensures cond &#61;&#61;&gt; result &#61;&#61; option::spec_some(ValidatedPublicKey&#123;bytes&#125;);
ensures !cond &#61;&#61;&gt; result &#61;&#61; option::spec_none&lt;ValidatedPublicKey&gt;();
</code></pre>



<a id="@Specification_1_new_validated_public_key_from_bytes_v2"></a>

### Function `new_validated_public_key_from_bytes_v2`


<pre><code>public fun new_validated_public_key_from_bytes_v2(bytes: vector&lt;u8&gt;): option::Option&lt;multi_ed25519::ValidatedPublicKey&gt;
</code></pre>




<pre><code>let cond &#61; spec_public_key_validate_v2_internal(bytes);
ensures cond &#61;&#61;&gt; result &#61;&#61; option::spec_some(ValidatedPublicKey&#123;bytes&#125;);
ensures !cond &#61;&#61;&gt; result &#61;&#61; option::spec_none&lt;ValidatedPublicKey&gt;();
</code></pre>



<a id="@Specification_1_new_signature_from_bytes"></a>

### Function `new_signature_from_bytes`


<pre><code>public fun new_signature_from_bytes(bytes: vector&lt;u8&gt;): multi_ed25519::Signature
</code></pre>




<pre><code>include NewSignatureFromBytesAbortsIf;
ensures result &#61;&#61; Signature &#123; bytes &#125;;
</code></pre>




<a id="0x1_multi_ed25519_NewSignatureFromBytesAbortsIf"></a>


<pre><code>schema NewSignatureFromBytesAbortsIf &#123;
    bytes: vector&lt;u8&gt;;
    aborts_if len(bytes) % INDIVIDUAL_SIGNATURE_NUM_BYTES !&#61; BITMAP_NUM_OF_BYTES;
&#125;
</code></pre>



<a id="@Specification_1_unvalidated_public_key_num_sub_pks"></a>

### Function `unvalidated_public_key_num_sub_pks`


<pre><code>public fun unvalidated_public_key_num_sub_pks(pk: &amp;multi_ed25519::UnvalidatedPublicKey): u8
</code></pre>




<pre><code>let bytes &#61; pk.bytes;
include PkDivision;
</code></pre>



<a id="@Specification_1_unvalidated_public_key_threshold"></a>

### Function `unvalidated_public_key_threshold`


<pre><code>public fun unvalidated_public_key_threshold(pk: &amp;multi_ed25519::UnvalidatedPublicKey): option::Option&lt;u8&gt;
</code></pre>




<pre><code>aborts_if false;
ensures result &#61;&#61; spec_check_and_get_threshold(pk.bytes);
</code></pre>



<a id="@Specification_1_validated_public_key_num_sub_pks"></a>

### Function `validated_public_key_num_sub_pks`


<pre><code>public fun validated_public_key_num_sub_pks(pk: &amp;multi_ed25519::ValidatedPublicKey): u8
</code></pre>




<pre><code>let bytes &#61; pk.bytes;
include PkDivision;
</code></pre>



<a id="@Specification_1_validated_public_key_threshold"></a>

### Function `validated_public_key_threshold`


<pre><code>public fun validated_public_key_threshold(pk: &amp;multi_ed25519::ValidatedPublicKey): u8
</code></pre>




<pre><code>aborts_if len(pk.bytes) &#61;&#61; 0;
ensures result &#61;&#61; pk.bytes[len(pk.bytes) &#45; 1];
</code></pre>



<a id="@Specification_1_check_and_get_threshold"></a>

### Function `check_and_get_threshold`


<pre><code>public fun check_and_get_threshold(bytes: vector&lt;u8&gt;): option::Option&lt;u8&gt;
</code></pre>




<pre><code>aborts_if false;
ensures result &#61;&#61; spec_check_and_get_threshold(bytes);
</code></pre>




<a id="0x1_multi_ed25519_PkDivision"></a>


<pre><code>schema PkDivision &#123;
    bytes: vector&lt;u8&gt;;
    result: u8;
    aborts_if len(bytes) / INDIVIDUAL_PUBLIC_KEY_NUM_BYTES &gt; MAX_U8;
    ensures result &#61;&#61; len(bytes) / INDIVIDUAL_PUBLIC_KEY_NUM_BYTES;
&#125;
</code></pre>



<a id="@Specification_1_public_key_bytes_to_authentication_key"></a>

### Function `public_key_bytes_to_authentication_key`


<pre><code>fun public_key_bytes_to_authentication_key(pk_bytes: vector&lt;u8&gt;): vector&lt;u8&gt;
</code></pre>




<pre><code>pragma opaque;
aborts_if false;
ensures [abstract] result &#61;&#61; spec_public_key_bytes_to_authentication_key(pk_bytes);
</code></pre>



<a id="@Specification_1_public_key_validate_internal"></a>

### Function `public_key_validate_internal`


<pre><code>fun public_key_validate_internal(bytes: vector&lt;u8&gt;): bool
</code></pre>




<pre><code>pragma opaque;
aborts_if false;
ensures (len(bytes) / INDIVIDUAL_PUBLIC_KEY_NUM_BYTES &gt; MAX_NUMBER_OF_PUBLIC_KEYS) &#61;&#61;&gt; (result &#61;&#61; false);
ensures result &#61;&#61; spec_public_key_validate_internal(bytes);
</code></pre>



<a id="@Specification_1_public_key_validate_v2_internal"></a>

### Function `public_key_validate_v2_internal`


<pre><code>fun public_key_validate_v2_internal(bytes: vector&lt;u8&gt;): bool
</code></pre>




<pre><code>pragma opaque;
ensures result &#61;&#61; spec_public_key_validate_v2_internal(bytes);
</code></pre>



<a id="@Specification_1_signature_verify_strict_internal"></a>

### Function `signature_verify_strict_internal`


<pre><code>fun signature_verify_strict_internal(multisignature: vector&lt;u8&gt;, public_key: vector&lt;u8&gt;, message: vector&lt;u8&gt;): bool
</code></pre>




<pre><code>pragma opaque;
aborts_if false;
ensures result &#61;&#61; spec_signature_verify_strict_internal(multisignature, public_key, message);
</code></pre>



<a id="@Helper_functions_2"></a>

### Helper functions



<a id="0x1_multi_ed25519_spec_check_and_get_threshold"></a>


<pre><code>fun spec_check_and_get_threshold(bytes: vector&lt;u8&gt;): Option&lt;u8&gt; &#123;
   let len &#61; len(bytes);
   if (len &#61;&#61; 0) &#123;
       option::none&lt;u8&gt;()
   &#125; else &#123;
       let threshold_num_of_bytes &#61; len % INDIVIDUAL_PUBLIC_KEY_NUM_BYTES;
       let num_of_keys &#61; len / INDIVIDUAL_PUBLIC_KEY_NUM_BYTES;
       let threshold_byte &#61; bytes[len &#45; 1];
       if (num_of_keys &#61;&#61; 0 &#124;&#124; num_of_keys &gt; MAX_NUMBER_OF_PUBLIC_KEYS &#124;&#124; len % INDIVIDUAL_PUBLIC_KEY_NUM_BYTES !&#61; 1) &#123;
           option::none&lt;u8&gt;()
       &#125; else if (threshold_byte &#61;&#61; 0 &#124;&#124; threshold_byte &gt; (num_of_keys as u8)) &#123;
           option::none&lt;u8&gt;()
       &#125; else &#123;
           option::spec_some(threshold_byte)
       &#125;
   &#125;
&#125;
</code></pre>




<a id="0x1_multi_ed25519_spec_signature_verify_strict_internal"></a>


<pre><code>fun spec_signature_verify_strict_internal(
   multisignature: vector&lt;u8&gt;,
   public_key: vector&lt;u8&gt;,
   message: vector&lt;u8&gt;
): bool;
</code></pre>




<a id="0x1_multi_ed25519_spec_public_key_validate_internal"></a>


<pre><code>fun spec_public_key_validate_internal(bytes: vector&lt;u8&gt;): bool;
</code></pre>




<a id="0x1_multi_ed25519_spec_public_key_validate_v2_internal"></a>


<pre><code>fun spec_public_key_validate_v2_internal(bytes: vector&lt;u8&gt;): bool;
</code></pre>




<a id="0x1_multi_ed25519_spec_public_key_bytes_to_authentication_key"></a>


<pre><code>fun spec_public_key_bytes_to_authentication_key(pk_bytes: vector&lt;u8&gt;): vector&lt;u8&gt;;
</code></pre>




<a id="0x1_multi_ed25519_spec_signature_verify_strict_t"></a>


<pre><code>fun spec_signature_verify_strict_t&lt;T&gt;(signature: Signature, public_key: UnvalidatedPublicKey, data: T): bool &#123;
   let encoded &#61; ed25519::new_signed_message&lt;T&gt;(data);
   let message &#61; bcs::serialize(encoded);
   spec_signature_verify_strict_internal(signature.bytes, public_key.bytes, message)
&#125;
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
