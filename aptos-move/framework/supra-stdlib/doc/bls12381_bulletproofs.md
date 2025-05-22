
<a id="0x1_bls12381_bulletproofs"></a>

# Module `0x1::bls12381_bulletproofs`



-  [Struct `RangeProof`](#0x1_bls12381_bulletproofs_RangeProof)
-  [Constants](#@Constants_0)
-  [Function `get_max_range_bits`](#0x1_bls12381_bulletproofs_get_max_range_bits)
-  [Function `range_proof_from_bytes`](#0x1_bls12381_bulletproofs_range_proof_from_bytes)
-  [Function `range_proof_to_bytes`](#0x1_bls12381_bulletproofs_range_proof_to_bytes)
-  [Function `verify_range_proof_pedersen`](#0x1_bls12381_bulletproofs_verify_range_proof_pedersen)
-  [Function `verify_range_proof`](#0x1_bls12381_bulletproofs_verify_range_proof)
-  [Function `verify_range_proof_internal`](#0x1_bls12381_bulletproofs_verify_range_proof_internal)


<pre><code><b>use</b> <a href="../../aptos-stdlib/doc/bls12381_algebra.md#0x1_bls12381_algebra">0x1::bls12381_algebra</a>;
<b>use</b> <a href="bls12381_pedersen.md#0x1_bls12381_pedersen">0x1::bls12381_pedersen</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/crypto_algebra.md#0x1_crypto_algebra">0x1::crypto_algebra</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features">0x1::features</a>;
</code></pre>



<a id="0x1_bls12381_bulletproofs_RangeProof"></a>

## Struct `RangeProof`

Represents a zero-knowledge range proof that a value committed inside a Pedersen commitment lies in
<code>[0, 2^{<a href="bls12381_bulletproofs.md#0x1_bls12381_bulletproofs_MAX_RANGE_BITS">MAX_RANGE_BITS</a>})</code>.


<pre><code><b>struct</b> <a href="bls12381_bulletproofs.md#0x1_bls12381_bulletproofs_RangeProof">RangeProof</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_bls12381_bulletproofs_E_DESERIALIZE_RANGE_PROOF"></a>

There was an error deserializing the range proof.


<pre><code><b>const</b> <a href="bls12381_bulletproofs.md#0x1_bls12381_bulletproofs_E_DESERIALIZE_RANGE_PROOF">E_DESERIALIZE_RANGE_PROOF</a>: u64 = 1;
</code></pre>



<a id="0x1_bls12381_bulletproofs_E_NATIVE_FUN_NOT_AVAILABLE"></a>

The native functions have not been rolled out yet.


<pre><code><b>const</b> <a href="bls12381_bulletproofs.md#0x1_bls12381_bulletproofs_E_NATIVE_FUN_NOT_AVAILABLE">E_NATIVE_FUN_NOT_AVAILABLE</a>: u64 = 4;
</code></pre>



<a id="0x1_bls12381_bulletproofs_E_RANGE_NOT_SUPPORTED"></a>

The range proof system only supports proving ranges of type $[0, 2^b)$ where $b \in \{8, 16, 32, 64\}$.


<pre><code><b>const</b> <a href="bls12381_bulletproofs.md#0x1_bls12381_bulletproofs_E_RANGE_NOT_SUPPORTED">E_RANGE_NOT_SUPPORTED</a>: u64 = 3;
</code></pre>



<a id="0x1_bls12381_bulletproofs_E_VALUE_OUTSIDE_RANGE"></a>

The committed value given to the prover is too large.


<pre><code><b>const</b> <a href="bls12381_bulletproofs.md#0x1_bls12381_bulletproofs_E_VALUE_OUTSIDE_RANGE">E_VALUE_OUTSIDE_RANGE</a>: u64 = 2;
</code></pre>



<a id="0x1_bls12381_bulletproofs_MAX_RANGE_BITS"></a>

The maximum range supported by the Bulletproofs library is $[0, 2^{64})$.


<pre><code><b>const</b> <a href="bls12381_bulletproofs.md#0x1_bls12381_bulletproofs_MAX_RANGE_BITS">MAX_RANGE_BITS</a>: u64 = 32;
</code></pre>



<a id="0x1_bls12381_bulletproofs_get_max_range_bits"></a>

## Function `get_max_range_bits`

Returns the maximum # of bits that the range proof system can verify proofs for.


<pre><code><b>public</b> <b>fun</b> <a href="bls12381_bulletproofs.md#0x1_bls12381_bulletproofs_get_max_range_bits">get_max_range_bits</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bls12381_bulletproofs.md#0x1_bls12381_bulletproofs_get_max_range_bits">get_max_range_bits</a>(): u64 {
    <a href="bls12381_bulletproofs.md#0x1_bls12381_bulletproofs_MAX_RANGE_BITS">MAX_RANGE_BITS</a>
}
</code></pre>



</details>

<a id="0x1_bls12381_bulletproofs_range_proof_from_bytes"></a>

## Function `range_proof_from_bytes`

Deserializes a range proof from a sequence of bytes. The serialization format is the same as the format in
the zkcrypto's <code>bulletproofs</code> library (https://docs.rs/bulletproofs/4.0.0/bulletproofs/struct.RangeProof.html#method.from_bytes).


<pre><code><b>public</b> <b>fun</b> <a href="bls12381_bulletproofs.md#0x1_bls12381_bulletproofs_range_proof_from_bytes">range_proof_from_bytes</a>(bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="bls12381_bulletproofs.md#0x1_bls12381_bulletproofs_RangeProof">bls12381_bulletproofs::RangeProof</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bls12381_bulletproofs.md#0x1_bls12381_bulletproofs_range_proof_from_bytes">range_proof_from_bytes</a>(bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="bls12381_bulletproofs.md#0x1_bls12381_bulletproofs_RangeProof">RangeProof</a> {
    <a href="bls12381_bulletproofs.md#0x1_bls12381_bulletproofs_RangeProof">RangeProof</a> {
        bytes
    }
}
</code></pre>



</details>

<a id="0x1_bls12381_bulletproofs_range_proof_to_bytes"></a>

## Function `range_proof_to_bytes`

Returns the byte-representation of a range proof.


<pre><code><b>public</b> <b>fun</b> <a href="bls12381_bulletproofs.md#0x1_bls12381_bulletproofs_range_proof_to_bytes">range_proof_to_bytes</a>(proof: &<a href="bls12381_bulletproofs.md#0x1_bls12381_bulletproofs_RangeProof">bls12381_bulletproofs::RangeProof</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bls12381_bulletproofs.md#0x1_bls12381_bulletproofs_range_proof_to_bytes">range_proof_to_bytes</a>(proof: &<a href="bls12381_bulletproofs.md#0x1_bls12381_bulletproofs_RangeProof">RangeProof</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    proof.bytes
}
</code></pre>



</details>

<a id="0x1_bls12381_bulletproofs_verify_range_proof_pedersen"></a>

## Function `verify_range_proof_pedersen`

Verifies a zero-knowledge range proof that the value <code>v</code> committed in <code>com</code> (under the default Bulletproofs
commitment key; see <code>pedersen::new_commitment_for_bulletproof</code>) satisfies $v \in [0, 2^b)$. Only works
for $b \in \{8, 16, 32, 64\}$. Additionally, checks that the prover used <code>dst</code> as the domain-separation
tag (DST).

WARNING: The DST check is VERY important for security as it prevents proofs computed for one application
(a.k.a., a _domain_) with <code>dst_1</code> from verifying in a different application with <code>dst_2 != dst_1</code>.


<pre><code><b>public</b> <b>fun</b> <a href="bls12381_bulletproofs.md#0x1_bls12381_bulletproofs_verify_range_proof_pedersen">verify_range_proof_pedersen</a>(com: &<a href="bls12381_pedersen.md#0x1_bls12381_pedersen_Commitment">bls12381_pedersen::Commitment</a>, proof: &<a href="bls12381_bulletproofs.md#0x1_bls12381_bulletproofs_RangeProof">bls12381_bulletproofs::RangeProof</a>, num_bits: u64, dst: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bls12381_bulletproofs.md#0x1_bls12381_bulletproofs_verify_range_proof_pedersen">verify_range_proof_pedersen</a>(com: &<a href="bls12381_pedersen.md#0x1_bls12381_pedersen_Commitment">bls12381_pedersen::Commitment</a>, proof: &<a href="bls12381_bulletproofs.md#0x1_bls12381_bulletproofs_RangeProof">RangeProof</a>, num_bits: u64, dst: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool {
    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_supra_private_poll_enabled">features::supra_private_poll_enabled</a>(), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="bls12381_bulletproofs.md#0x1_bls12381_bulletproofs_E_NATIVE_FUN_NOT_AVAILABLE">E_NATIVE_FUN_NOT_AVAILABLE</a>));

    <a href="bls12381_bulletproofs.md#0x1_bls12381_bulletproofs_verify_range_proof_internal">verify_range_proof_internal</a>(
        serialize&lt;G1, FormatG1Compr&gt;(<a href="bls12381_pedersen.md#0x1_bls12381_pedersen_commitment_as_point">bls12381_pedersen::commitment_as_point</a>(com)),
        serialize&lt;G1, FormatG1Compr&gt;(&one&lt;G1&gt;()),
        serialize&lt;G1, FormatG1Compr&gt;(&<a href="bls12381_pedersen.md#0x1_bls12381_pedersen_randomness_base_for_bulletproof">bls12381_pedersen::randomness_base_for_bulletproof</a>()),
        proof.bytes,
        num_bits,
        dst
    )
}
</code></pre>



</details>

<a id="0x1_bls12381_bulletproofs_verify_range_proof"></a>

## Function `verify_range_proof`

Verifies a zero-knowledge range proof that the value <code>v</code> committed in <code>com</code> (as v * val_base + r * rand_base,
for some randomness <code>r</code>) satisfies <code>v</code> in <code>[0, 2^num_bits)</code>. Only works for <code>num_bits</code> in <code>{8, 16, 32, 64}</code>.


<pre><code><b>public</b> <b>fun</b> <a href="bls12381_bulletproofs.md#0x1_bls12381_bulletproofs_verify_range_proof">verify_range_proof</a>(com: &<a href="../../aptos-stdlib/doc/crypto_algebra.md#0x1_crypto_algebra_Element">crypto_algebra::Element</a>&lt;<a href="../../aptos-stdlib/doc/bls12381_algebra.md#0x1_bls12381_algebra_G1">bls12381_algebra::G1</a>&gt;, val_base: &<a href="../../aptos-stdlib/doc/crypto_algebra.md#0x1_crypto_algebra_Element">crypto_algebra::Element</a>&lt;<a href="../../aptos-stdlib/doc/bls12381_algebra.md#0x1_bls12381_algebra_G1">bls12381_algebra::G1</a>&gt;, rand_base: &<a href="../../aptos-stdlib/doc/crypto_algebra.md#0x1_crypto_algebra_Element">crypto_algebra::Element</a>&lt;<a href="../../aptos-stdlib/doc/bls12381_algebra.md#0x1_bls12381_algebra_G1">bls12381_algebra::G1</a>&gt;, proof: &<a href="bls12381_bulletproofs.md#0x1_bls12381_bulletproofs_RangeProof">bls12381_bulletproofs::RangeProof</a>, num_bits: u64, dst: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bls12381_bulletproofs.md#0x1_bls12381_bulletproofs_verify_range_proof">verify_range_proof</a>(
    com: &Element&lt;G1&gt;,
    val_base: &Element&lt;G1&gt;, rand_base: &Element&lt;G1&gt;,
    proof: &<a href="bls12381_bulletproofs.md#0x1_bls12381_bulletproofs_RangeProof">RangeProof</a>, num_bits: u64, dst: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool {
    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_supra_private_poll_enabled">features::supra_private_poll_enabled</a>(), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="bls12381_bulletproofs.md#0x1_bls12381_bulletproofs_E_NATIVE_FUN_NOT_AVAILABLE">E_NATIVE_FUN_NOT_AVAILABLE</a>));

    <a href="bls12381_bulletproofs.md#0x1_bls12381_bulletproofs_verify_range_proof_internal">verify_range_proof_internal</a>(
        serialize&lt;G1, FormatG1Compr&gt;(com),
        serialize&lt;G1, FormatG1Compr&gt;(val_base),
        serialize&lt;G1, FormatG1Compr&gt;(rand_base),
        proof.bytes, num_bits, dst
    )
}
</code></pre>



</details>

<a id="0x1_bls12381_bulletproofs_verify_range_proof_internal"></a>

## Function `verify_range_proof_internal`

Aborts with <code><a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="bls12381_bulletproofs.md#0x1_bls12381_bulletproofs_E_DESERIALIZE_RANGE_PROOF">E_DESERIALIZE_RANGE_PROOF</a>)</code> if <code>proof</code> is not a valid serialization of a
range proof.
Aborts with <code><a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="bls12381_bulletproofs.md#0x1_bls12381_bulletproofs_E_RANGE_NOT_SUPPORTED">E_RANGE_NOT_SUPPORTED</a>)</code> if an unsupported <code>num_bits</code> is provided.


<pre><code><b>fun</b> <a href="bls12381_bulletproofs.md#0x1_bls12381_bulletproofs_verify_range_proof_internal">verify_range_proof_internal</a>(com: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, val_base: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, rand_base: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, proof: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, num_bits: u64, dst: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="bls12381_bulletproofs.md#0x1_bls12381_bulletproofs_verify_range_proof_internal">verify_range_proof_internal</a>(
    com: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    val_base: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    rand_base: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    proof: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    num_bits: u64,
    dst: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool;
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
