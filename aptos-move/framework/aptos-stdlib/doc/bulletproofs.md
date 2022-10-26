
<a name="0x1_bulletproofs"></a>

# Module `0x1::bulletproofs`

This module implements Bulletproof-based zero-knowledge range proof: i.e., a proof that a value <code>v</code> committed in a
Pedersen commitment <code>com</code> satisfies $v \in [0, 2^{num_bits})$. Currently, this module only supports num_bits \in
{8, 16, 32, 64}.


-  [Struct `RangeProof`](#0x1_bulletproofs_RangeProof)
-  [Constants](#@Constants_0)
-  [Function `get_max_range_bits`](#0x1_bulletproofs_get_max_range_bits)
-  [Function `range_proof_from_bytes`](#0x1_bulletproofs_range_proof_from_bytes)
-  [Function `range_proof_to_bytes`](#0x1_bulletproofs_range_proof_to_bytes)
-  [Function `verify_range_proof`](#0x1_bulletproofs_verify_range_proof)
-  [Function `verify_range_proof_custom_ck`](#0x1_bulletproofs_verify_range_proof_custom_ck)
-  [Function `verify_range_proof_internal`](#0x1_bulletproofs_verify_range_proof_internal)
-  [Function `verify_range_proof_custom_ck_internal`](#0x1_bulletproofs_verify_range_proof_custom_ck_internal)
-  [Specification](#@Specification_1)
    -  [Function `verify_range_proof_internal`](#@Specification_1_verify_range_proof_internal)
    -  [Function `verify_range_proof_custom_ck_internal`](#@Specification_1_verify_range_proof_custom_ck_internal)


<pre><code><b>use</b> <a href="../../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="../../move-stdlib/doc/features.md#0x1_features">0x1::features</a>;
<b>use</b> <a href="pedersen.md#0x1_pedersen">0x1::pedersen</a>;
<b>use</b> <a href="ristretto255.md#0x1_ristretto255">0x1::ristretto255</a>;
</code></pre>



<a name="0x1_bulletproofs_RangeProof"></a>

## Struct `RangeProof`

Represents a zero-knowledge range proof that a value committed inside a Pedersen commitment lies in [0, 2^{MAX_RANGE_BITS}).


<pre><code><b>struct</b> <a href="bulletproofs.md#0x1_bulletproofs_RangeProof">RangeProof</a> <b>has</b> <b>copy</b>, drop, store
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


<a name="0x1_bulletproofs_E_NATIVE_FUN_NOT_AVAILABLE"></a>

The native functions have not been rolled out yet.


<pre><code><b>const</b> <a href="bulletproofs.md#0x1_bulletproofs_E_NATIVE_FUN_NOT_AVAILABLE">E_NATIVE_FUN_NOT_AVAILABLE</a>: u64 = 4;
</code></pre>



<a name="0x1_bulletproofs_E_DESERIALIZE_RANGE_PROOF"></a>

Error deserializing one of the arguments.


<pre><code><b>const</b> <a href="bulletproofs.md#0x1_bulletproofs_E_DESERIALIZE_RANGE_PROOF">E_DESERIALIZE_RANGE_PROOF</a>: u64 = 1;
</code></pre>



<a name="0x1_bulletproofs_E_RANGE_NOT_SUPPORTED"></a>

The range proof system only supports proving ranges of type [0, 2^{bits}) where bits \in {8, 16, 32, 64}.


<pre><code><b>const</b> <a href="bulletproofs.md#0x1_bulletproofs_E_RANGE_NOT_SUPPORTED">E_RANGE_NOT_SUPPORTED</a>: u64 = 3;
</code></pre>



<a name="0x1_bulletproofs_E_VALUE_OUTSIDE_RANGE"></a>

The input value given to the prover is too large.


<pre><code><b>const</b> <a href="bulletproofs.md#0x1_bulletproofs_E_VALUE_OUTSIDE_RANGE">E_VALUE_OUTSIDE_RANGE</a>: u64 = 2;
</code></pre>



<a name="0x1_bulletproofs_MAX_RANGE_BITS"></a>

The maximum range supported by the Bulletproofs library is [0, 2^{64}).


<pre><code><b>const</b> <a href="bulletproofs.md#0x1_bulletproofs_MAX_RANGE_BITS">MAX_RANGE_BITS</a>: u64 = 64;
</code></pre>



<a name="0x1_bulletproofs_get_max_range_bits"></a>

## Function `get_max_range_bits`

Returns the maximum # of bits the range proof system can verify proofs for.


<pre><code><b>public</b> <b>fun</b> <a href="bulletproofs.md#0x1_bulletproofs_get_max_range_bits">get_max_range_bits</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bulletproofs.md#0x1_bulletproofs_get_max_range_bits">get_max_range_bits</a>(): u64 {
    <a href="bulletproofs.md#0x1_bulletproofs_MAX_RANGE_BITS">MAX_RANGE_BITS</a>
}
</code></pre>



</details>

<a name="0x1_bulletproofs_range_proof_from_bytes"></a>

## Function `range_proof_from_bytes`

Deserializes a range proof from a sequence of bytes.


<pre><code><b>public</b> <b>fun</b> <a href="bulletproofs.md#0x1_bulletproofs_range_proof_from_bytes">range_proof_from_bytes</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="bulletproofs.md#0x1_bulletproofs_RangeProof">bulletproofs::RangeProof</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bulletproofs.md#0x1_bulletproofs_range_proof_from_bytes">range_proof_from_bytes</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="bulletproofs.md#0x1_bulletproofs_RangeProof">RangeProof</a> {
    // TODO: Can we do <a href="any.md#0x1_any">any</a> pre-validation here?
    <a href="bulletproofs.md#0x1_bulletproofs_RangeProof">RangeProof</a> {
        bytes
    }
}
</code></pre>



</details>

<a name="0x1_bulletproofs_range_proof_to_bytes"></a>

## Function `range_proof_to_bytes`

Returns the byte-representation of a range proof.


<pre><code><b>public</b> <b>fun</b> <a href="bulletproofs.md#0x1_bulletproofs_range_proof_to_bytes">range_proof_to_bytes</a>(proof: &<a href="bulletproofs.md#0x1_bulletproofs_RangeProof">bulletproofs::RangeProof</a>): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bulletproofs.md#0x1_bulletproofs_range_proof_to_bytes">range_proof_to_bytes</a>(proof: &<a href="bulletproofs.md#0x1_bulletproofs_RangeProof">RangeProof</a>): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    proof.bytes
}
</code></pre>



</details>

<a name="0x1_bulletproofs_verify_range_proof"></a>

## Function `verify_range_proof`

Verifies a zero-knowledge range proof that the value <code>v</code> committed in <code>com</code> (under the default Bulletproofs
commitment; see <code><a href="pedersen.md#0x1_pedersen_new_commitment_for_bulletproof">pedersen::new_commitment_for_bulletproof</a></code>) satisfies $v \in [0, 2^{num_bits})$. Only works
for <code>num_bits</code> \in {8, 16, 32, 64}.


<pre><code><b>public</b> <b>fun</b> <a href="bulletproofs.md#0x1_bulletproofs_verify_range_proof">verify_range_proof</a>(com: &<a href="pedersen.md#0x1_pedersen_Commitment">pedersen::Commitment</a>, proof: &<a href="bulletproofs.md#0x1_bulletproofs_RangeProof">bulletproofs::RangeProof</a>, num_bits: u64, dst: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bulletproofs.md#0x1_bulletproofs_verify_range_proof">verify_range_proof</a>(com: &<a href="pedersen.md#0x1_pedersen_Commitment">pedersen::Commitment</a>, proof: &<a href="bulletproofs.md#0x1_bulletproofs_RangeProof">RangeProof</a>, num_bits: u64, dst: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool {
    <b>if</b>(!<a href="../../move-stdlib/doc/features.md#0x1_features_bulletproofs_enabled">features::bulletproofs_enabled</a>()) {
        <b>abort</b>(std::error::invalid_state(<a href="bulletproofs.md#0x1_bulletproofs_E_NATIVE_FUN_NOT_AVAILABLE">E_NATIVE_FUN_NOT_AVAILABLE</a>))
    };

    <a href="bulletproofs.md#0x1_bulletproofs_verify_range_proof_internal">verify_range_proof_internal</a>(
        <a href="ristretto255.md#0x1_ristretto255_point_to_bytes">ristretto255::point_to_bytes</a>(&<a href="pedersen.md#0x1_pedersen_commitment_as_compressed_point">pedersen::commitment_as_compressed_point</a>(com)),
        proof.bytes,
        num_bits,
        dst
    )
}
</code></pre>



</details>

<a name="0x1_bulletproofs_verify_range_proof_custom_ck"></a>

## Function `verify_range_proof_custom_ck`

Verifies a zero-knowledge range proof that the value <code>v</code> committed in <code>com</code> (as v * val_base + r * rand_base,
for some randomness <code>r</code>) satisfies $v \in [0, 2^{num_bits})$. Only works for <code>num_bits</code> \in {8, 16, 32, 64}.


<pre><code><b>public</b> <b>fun</b> <a href="bulletproofs.md#0x1_bulletproofs_verify_range_proof_custom_ck">verify_range_proof_custom_ck</a>(com: &<a href="pedersen.md#0x1_pedersen_Commitment">pedersen::Commitment</a>, val_base: &<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>, rand_base: &<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>, proof: &<a href="bulletproofs.md#0x1_bulletproofs_RangeProof">bulletproofs::RangeProof</a>, num_bits: u64, dst: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bulletproofs.md#0x1_bulletproofs_verify_range_proof_custom_ck">verify_range_proof_custom_ck</a>(
    com: &<a href="pedersen.md#0x1_pedersen_Commitment">pedersen::Commitment</a>,
    val_base: &RistrettoPoint, rand_base: &RistrettoPoint,
    proof: &<a href="bulletproofs.md#0x1_bulletproofs_RangeProof">RangeProof</a>, num_bits: u64, dst: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool
{
    <b>if</b>(!<a href="../../move-stdlib/doc/features.md#0x1_features_bulletproofs_enabled">features::bulletproofs_enabled</a>()) {
        <b>abort</b> (std::error::invalid_state(<a href="bulletproofs.md#0x1_bulletproofs_E_NATIVE_FUN_NOT_AVAILABLE">E_NATIVE_FUN_NOT_AVAILABLE</a>))
    };

    <a href="bulletproofs.md#0x1_bulletproofs_verify_range_proof_custom_ck_internal">verify_range_proof_custom_ck_internal</a>(
        <a href="ristretto255.md#0x1_ristretto255_point_to_bytes">ristretto255::point_to_bytes</a>(&<a href="pedersen.md#0x1_pedersen_commitment_as_compressed_point">pedersen::commitment_as_compressed_point</a>(com)),
        val_base, rand_base,
        proof.bytes, num_bits, dst
    )
}
</code></pre>



</details>

<a name="0x1_bulletproofs_verify_range_proof_internal"></a>

## Function `verify_range_proof_internal`



<pre><code><b>fun</b> <a href="bulletproofs.md#0x1_bulletproofs_verify_range_proof_internal">verify_range_proof_internal</a>(com: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, proof: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, num_bits: u64, dst: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="bulletproofs.md#0x1_bulletproofs_verify_range_proof_internal">verify_range_proof_internal</a>(
    com: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    proof: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    num_bits: u64,
    dst: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool;
</code></pre>



</details>

<a name="0x1_bulletproofs_verify_range_proof_custom_ck_internal"></a>

## Function `verify_range_proof_custom_ck_internal`



<pre><code><b>fun</b> <a href="bulletproofs.md#0x1_bulletproofs_verify_range_proof_custom_ck_internal">verify_range_proof_custom_ck_internal</a>(com: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, val_base: &<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>, rand_base: &<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>, proof: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, num_bits: u64, dst: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="bulletproofs.md#0x1_bulletproofs_verify_range_proof_custom_ck_internal">verify_range_proof_custom_ck_internal</a>(
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


<pre><code><b>fun</b> <a href="bulletproofs.md#0x1_bulletproofs_verify_range_proof_internal">verify_range_proof_internal</a>(com: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, proof: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, num_bits: u64, dst: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool
</code></pre>




<pre><code><b>pragma</b> opaque;
</code></pre>



<a name="@Specification_1_verify_range_proof_custom_ck_internal"></a>

### Function `verify_range_proof_custom_ck_internal`


<pre><code><b>fun</b> <a href="bulletproofs.md#0x1_bulletproofs_verify_range_proof_custom_ck_internal">verify_range_proof_custom_ck_internal</a>(com: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, val_base: &<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>, rand_base: &<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>, proof: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, num_bits: u64, dst: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool
</code></pre>




<pre><code><b>pragma</b> opaque;
</code></pre>


[move-book]: https://move-language.github.io/move/introduction.html
