
<a name="0x1_ristretto255_bulletproofs"></a>

# Module `0x1::ristretto255_bulletproofs`

This module implements a Bulletproof range proof verifier on the Ristretto255 curve.

A Bulletproof-based zero-knowledge range proof is a proof that a Pedersen commitment
$c = v G + r H$ commits to an $n$-bit value $v$ (i.e., $v \in [0, 2^n)$). Currently, this module only supports
$n \in \{8, 16, 32, 64\}$ for the number of bits.


-  [Struct `RangeProof`](#0x1_ristretto255_bulletproofs_RangeProof)
-  [Constants](#@Constants_0)
-  [Function `get_max_range_bits`](#0x1_ristretto255_bulletproofs_get_max_range_bits)
-  [Function `range_proof_from_bytes`](#0x1_ristretto255_bulletproofs_range_proof_from_bytes)
-  [Function `range_proof_to_bytes`](#0x1_ristretto255_bulletproofs_range_proof_to_bytes)
-  [Function `verify_range_proof_pedersen`](#0x1_ristretto255_bulletproofs_verify_range_proof_pedersen)
-  [Function `verify_range_proof`](#0x1_ristretto255_bulletproofs_verify_range_proof)
-  [Function `verify_range_proof_internal`](#0x1_ristretto255_bulletproofs_verify_range_proof_internal)
-  [Specification](#@Specification_1)
    -  [Function `verify_range_proof_internal`](#@Specification_1_verify_range_proof_internal)


<pre><code><b>use</b> <a href="../../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="../../move-stdlib/doc/features.md#0x1_features">0x1::features</a>;
<b>use</b> <a href="ristretto255.md#0x1_ristretto255">0x1::ristretto255</a>;
<b>use</b> <a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen">0x1::ristretto255_pedersen</a>;
</code></pre>



<a name="0x1_ristretto255_bulletproofs_RangeProof"></a>

## Struct `RangeProof`

Represents a zero-knowledge range proof that a value committed inside a Pedersen commitment lies in
<code>[0, 2^{<a href="ristretto255_bulletproofs.md#0x1_ristretto255_bulletproofs_MAX_RANGE_BITS">MAX_RANGE_BITS</a>})</code>.


<pre><code><b>struct</b> <a href="ristretto255_bulletproofs.md#0x1_ristretto255_bulletproofs_RangeProof">RangeProof</a> <b>has</b> <b>copy</b>, drop, store
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

<a name="@Constants_0"></a>

## Constants


<a name="0x1_ristretto255_bulletproofs_E_NATIVE_FUN_NOT_AVAILABLE"></a>

The native functions have not been rolled out yet.


<pre><code><b>const</b> <a href="ristretto255_bulletproofs.md#0x1_ristretto255_bulletproofs_E_NATIVE_FUN_NOT_AVAILABLE">E_NATIVE_FUN_NOT_AVAILABLE</a>: u64 = 4;
</code></pre>



<a name="0x1_ristretto255_bulletproofs_E_DESERIALIZE_RANGE_PROOF"></a>

There was an error deserializing the range proof.


<pre><code><b>const</b> <a href="ristretto255_bulletproofs.md#0x1_ristretto255_bulletproofs_E_DESERIALIZE_RANGE_PROOF">E_DESERIALIZE_RANGE_PROOF</a>: u64 = 1;
</code></pre>



<a name="0x1_ristretto255_bulletproofs_E_RANGE_NOT_SUPPORTED"></a>

The range proof system only supports proving ranges of type $[0, 2^b)$ where $b \in \{8, 16, 32, 64\}$.


<pre><code><b>const</b> <a href="ristretto255_bulletproofs.md#0x1_ristretto255_bulletproofs_E_RANGE_NOT_SUPPORTED">E_RANGE_NOT_SUPPORTED</a>: u64 = 3;
</code></pre>



<a name="0x1_ristretto255_bulletproofs_E_VALUE_OUTSIDE_RANGE"></a>

The committed value given to the prover is too large.


<pre><code><b>const</b> <a href="ristretto255_bulletproofs.md#0x1_ristretto255_bulletproofs_E_VALUE_OUTSIDE_RANGE">E_VALUE_OUTSIDE_RANGE</a>: u64 = 2;
</code></pre>



<a name="0x1_ristretto255_bulletproofs_MAX_RANGE_BITS"></a>

The maximum range supported by the Bulletproofs library is $[0, 2^{64})$.


<pre><code><b>const</b> <a href="ristretto255_bulletproofs.md#0x1_ristretto255_bulletproofs_MAX_RANGE_BITS">MAX_RANGE_BITS</a>: u64 = 64;
</code></pre>



<a name="0x1_ristretto255_bulletproofs_get_max_range_bits"></a>

## Function `get_max_range_bits`

Returns the maximum # of bits that the range proof system can verify proofs for.


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255_bulletproofs.md#0x1_ristretto255_bulletproofs_get_max_range_bits">get_max_range_bits</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255_bulletproofs.md#0x1_ristretto255_bulletproofs_get_max_range_bits">get_max_range_bits</a>(): u64 {
    <a href="ristretto255_bulletproofs.md#0x1_ristretto255_bulletproofs_MAX_RANGE_BITS">MAX_RANGE_BITS</a>
}
</code></pre>



</details>

<a name="0x1_ristretto255_bulletproofs_range_proof_from_bytes"></a>

## Function `range_proof_from_bytes`

Deserializes a range proof from a sequence of bytes. The serialization format is the same as the format in
the zkcrypto's <code>bulletproofs</code> library (https://docs.rs/bulletproofs/4.0.0/bulletproofs/struct.RangeProof.html#method.from_bytes).


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255_bulletproofs.md#0x1_ristretto255_bulletproofs_range_proof_from_bytes">range_proof_from_bytes</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="ristretto255_bulletproofs.md#0x1_ristretto255_bulletproofs_RangeProof">ristretto255_bulletproofs::RangeProof</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255_bulletproofs.md#0x1_ristretto255_bulletproofs_range_proof_from_bytes">range_proof_from_bytes</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="ristretto255_bulletproofs.md#0x1_ristretto255_bulletproofs_RangeProof">RangeProof</a> {
    <a href="ristretto255_bulletproofs.md#0x1_ristretto255_bulletproofs_RangeProof">RangeProof</a> {
        bytes
    }
}
</code></pre>



</details>

<a name="0x1_ristretto255_bulletproofs_range_proof_to_bytes"></a>

## Function `range_proof_to_bytes`

Returns the byte-representation of a range proof.


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255_bulletproofs.md#0x1_ristretto255_bulletproofs_range_proof_to_bytes">range_proof_to_bytes</a>(proof: &<a href="ristretto255_bulletproofs.md#0x1_ristretto255_bulletproofs_RangeProof">ristretto255_bulletproofs::RangeProof</a>): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255_bulletproofs.md#0x1_ristretto255_bulletproofs_range_proof_to_bytes">range_proof_to_bytes</a>(proof: &<a href="ristretto255_bulletproofs.md#0x1_ristretto255_bulletproofs_RangeProof">RangeProof</a>): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    proof.bytes
}
</code></pre>



</details>

<a name="0x1_ristretto255_bulletproofs_verify_range_proof_pedersen"></a>

## Function `verify_range_proof_pedersen`

Verifies a zero-knowledge range proof that the value <code>v</code> committed in <code>com</code> (under the default Bulletproofs
commitment key; see <code>pedersen::new_commitment_for_bulletproof</code>) satisfies $v \in [0, 2^b)$. Only works
for $b \in \{8, 16, 32, 64\}$. Additionally, checks that the prover used <code>dst</code> as the domain-separation
tag (DST).

WARNING: The DST check is VERY important for security as it prevents proofs computed for one application
(a.k.a., a _domain_) with <code>dst_1</code> from verifying in a different application with <code>dst_2 != dst_1</code>.


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255_bulletproofs.md#0x1_ristretto255_bulletproofs_verify_range_proof_pedersen">verify_range_proof_pedersen</a>(com: &<a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_Commitment">ristretto255_pedersen::Commitment</a>, proof: &<a href="ristretto255_bulletproofs.md#0x1_ristretto255_bulletproofs_RangeProof">ristretto255_bulletproofs::RangeProof</a>, num_bits: u64, dst: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255_bulletproofs.md#0x1_ristretto255_bulletproofs_verify_range_proof_pedersen">verify_range_proof_pedersen</a>(com: &pedersen::Commitment, proof: &<a href="ristretto255_bulletproofs.md#0x1_ristretto255_bulletproofs_RangeProof">RangeProof</a>, num_bits: u64, dst: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool {
    <b>assert</b>!(<a href="../../move-stdlib/doc/features.md#0x1_features_bulletproofs_enabled">features::bulletproofs_enabled</a>(), <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="ristretto255_bulletproofs.md#0x1_ristretto255_bulletproofs_E_NATIVE_FUN_NOT_AVAILABLE">E_NATIVE_FUN_NOT_AVAILABLE</a>));

    <a href="ristretto255_bulletproofs.md#0x1_ristretto255_bulletproofs_verify_range_proof_internal">verify_range_proof_internal</a>(
        <a href="ristretto255.md#0x1_ristretto255_point_to_bytes">ristretto255::point_to_bytes</a>(&pedersen::commitment_as_compressed_point(com)),
        &<a href="ristretto255.md#0x1_ristretto255_basepoint">ristretto255::basepoint</a>(), &<a href="ristretto255.md#0x1_ristretto255_hash_to_point_base">ristretto255::hash_to_point_base</a>(),
        proof.bytes,
        num_bits,
        dst
    )
}
</code></pre>



</details>

<a name="0x1_ristretto255_bulletproofs_verify_range_proof"></a>

## Function `verify_range_proof`

Verifies a zero-knowledge range proof that the value <code>v</code> committed in <code>com</code> (as v * val_base + r * rand_base,
for some randomness <code>r</code>) satisfies <code>v</code> in <code>[0, 2^num_bits)</code>. Only works for <code>num_bits</code> in <code>{8, 16, 32, 64}</code>.


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255_bulletproofs.md#0x1_ristretto255_bulletproofs_verify_range_proof">verify_range_proof</a>(com: &<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>, val_base: &<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>, rand_base: &<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>, proof: &<a href="ristretto255_bulletproofs.md#0x1_ristretto255_bulletproofs_RangeProof">ristretto255_bulletproofs::RangeProof</a>, num_bits: u64, dst: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255_bulletproofs.md#0x1_ristretto255_bulletproofs_verify_range_proof">verify_range_proof</a>(
    com: &RistrettoPoint,
    val_base: &RistrettoPoint, rand_base: &RistrettoPoint,
    proof: &<a href="ristretto255_bulletproofs.md#0x1_ristretto255_bulletproofs_RangeProof">RangeProof</a>, num_bits: u64, dst: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool
{
    <b>assert</b>!(<a href="../../move-stdlib/doc/features.md#0x1_features_bulletproofs_enabled">features::bulletproofs_enabled</a>(), <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="ristretto255_bulletproofs.md#0x1_ristretto255_bulletproofs_E_NATIVE_FUN_NOT_AVAILABLE">E_NATIVE_FUN_NOT_AVAILABLE</a>));

    <a href="ristretto255_bulletproofs.md#0x1_ristretto255_bulletproofs_verify_range_proof_internal">verify_range_proof_internal</a>(
        <a href="ristretto255.md#0x1_ristretto255_point_to_bytes">ristretto255::point_to_bytes</a>(&<a href="ristretto255.md#0x1_ristretto255_point_compress">ristretto255::point_compress</a>(com)),
        val_base, rand_base,
        proof.bytes, num_bits, dst
    )
}
</code></pre>



</details>

<a name="0x1_ristretto255_bulletproofs_verify_range_proof_internal"></a>

## Function `verify_range_proof_internal`

Aborts with <code><a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="ristretto255_bulletproofs.md#0x1_ristretto255_bulletproofs_E_DESERIALIZE_RANGE_PROOF">E_DESERIALIZE_RANGE_PROOF</a>)</code> if <code>proof</code> is not a valid serialization of a
range proof.
Aborts with <code><a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="ristretto255_bulletproofs.md#0x1_ristretto255_bulletproofs_E_RANGE_NOT_SUPPORTED">E_RANGE_NOT_SUPPORTED</a>)</code> if an unsupported <code>num_bits</code> is provided.


<pre><code><b>fun</b> <a href="ristretto255_bulletproofs.md#0x1_ristretto255_bulletproofs_verify_range_proof_internal">verify_range_proof_internal</a>(com: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, val_base: &<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>, rand_base: &<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>, proof: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, num_bits: u64, dst: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="ristretto255_bulletproofs.md#0x1_ristretto255_bulletproofs_verify_range_proof_internal">verify_range_proof_internal</a>(
    com: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    val_base: &RistrettoPoint,
    rand_base: &RistrettoPoint,
    proof: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    num_bits: u64,
    dst: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool;
</code></pre>



</details>

<a name="@Specification_1"></a>

## Specification


<a name="@Specification_1_verify_range_proof_internal"></a>

### Function `verify_range_proof_internal`


<pre><code><b>fun</b> <a href="ristretto255_bulletproofs.md#0x1_ristretto255_bulletproofs_verify_range_proof_internal">verify_range_proof_internal</a>(com: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, val_base: &<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>, rand_base: &<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>, proof: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, num_bits: u64, dst: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool
</code></pre>




<pre><code><b>pragma</b> opaque;
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
