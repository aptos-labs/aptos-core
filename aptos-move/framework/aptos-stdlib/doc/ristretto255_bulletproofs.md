
<a id="0x1_ristretto255_bulletproofs"></a>

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


<pre><code>use 0x1::error;
use 0x1::features;
use 0x1::ristretto255;
use 0x1::ristretto255_pedersen;
</code></pre>



<a id="0x1_ristretto255_bulletproofs_RangeProof"></a>

## Struct `RangeProof`

Represents a zero-knowledge range proof that a value committed inside a Pedersen commitment lies in
<code>[0, 2^&#123;MAX_RANGE_BITS&#125;)</code>.


<pre><code>struct RangeProof has copy, drop, store
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


<a id="0x1_ristretto255_bulletproofs_E_NATIVE_FUN_NOT_AVAILABLE"></a>

The native functions have not been rolled out yet.


<pre><code>const E_NATIVE_FUN_NOT_AVAILABLE: u64 &#61; 4;
</code></pre>



<a id="0x1_ristretto255_bulletproofs_E_DESERIALIZE_RANGE_PROOF"></a>

There was an error deserializing the range proof.


<pre><code>const E_DESERIALIZE_RANGE_PROOF: u64 &#61; 1;
</code></pre>



<a id="0x1_ristretto255_bulletproofs_E_RANGE_NOT_SUPPORTED"></a>

The range proof system only supports proving ranges of type $[0, 2^b)$ where $b \in \{8, 16, 32, 64\}$.


<pre><code>const E_RANGE_NOT_SUPPORTED: u64 &#61; 3;
</code></pre>



<a id="0x1_ristretto255_bulletproofs_E_VALUE_OUTSIDE_RANGE"></a>

The committed value given to the prover is too large.


<pre><code>const E_VALUE_OUTSIDE_RANGE: u64 &#61; 2;
</code></pre>



<a id="0x1_ristretto255_bulletproofs_MAX_RANGE_BITS"></a>

The maximum range supported by the Bulletproofs library is $[0, 2^{64})$.


<pre><code>const MAX_RANGE_BITS: u64 &#61; 64;
</code></pre>



<a id="0x1_ristretto255_bulletproofs_get_max_range_bits"></a>

## Function `get_max_range_bits`

Returns the maximum # of bits that the range proof system can verify proofs for.


<pre><code>public fun get_max_range_bits(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_max_range_bits(): u64 &#123;
    MAX_RANGE_BITS
&#125;
</code></pre>



</details>

<a id="0x1_ristretto255_bulletproofs_range_proof_from_bytes"></a>

## Function `range_proof_from_bytes`

Deserializes a range proof from a sequence of bytes. The serialization format is the same as the format in
the zkcrypto's <code>bulletproofs</code> library (https://docs.rs/bulletproofs/4.0.0/bulletproofs/struct.RangeProof.html#method.from_bytes).


<pre><code>public fun range_proof_from_bytes(bytes: vector&lt;u8&gt;): ristretto255_bulletproofs::RangeProof
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun range_proof_from_bytes(bytes: vector&lt;u8&gt;): RangeProof &#123;
    RangeProof &#123;
        bytes
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_ristretto255_bulletproofs_range_proof_to_bytes"></a>

## Function `range_proof_to_bytes`

Returns the byte-representation of a range proof.


<pre><code>public fun range_proof_to_bytes(proof: &amp;ristretto255_bulletproofs::RangeProof): vector&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun range_proof_to_bytes(proof: &amp;RangeProof): vector&lt;u8&gt; &#123;
    proof.bytes
&#125;
</code></pre>



</details>

<a id="0x1_ristretto255_bulletproofs_verify_range_proof_pedersen"></a>

## Function `verify_range_proof_pedersen`

Verifies a zero-knowledge range proof that the value <code>v</code> committed in <code>com</code> (under the default Bulletproofs
commitment key; see <code>pedersen::new_commitment_for_bulletproof</code>) satisfies $v \in [0, 2^b)$. Only works
for $b \in \{8, 16, 32, 64\}$. Additionally, checks that the prover used <code>dst</code> as the domain-separation
tag (DST).

WARNING: The DST check is VERY important for security as it prevents proofs computed for one application
(a.k.a., a _domain_) with <code>dst_1</code> from verifying in a different application with <code>dst_2 !&#61; dst_1</code>.


<pre><code>public fun verify_range_proof_pedersen(com: &amp;ristretto255_pedersen::Commitment, proof: &amp;ristretto255_bulletproofs::RangeProof, num_bits: u64, dst: vector&lt;u8&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun verify_range_proof_pedersen(com: &amp;pedersen::Commitment, proof: &amp;RangeProof, num_bits: u64, dst: vector&lt;u8&gt;): bool &#123;
    assert!(features::bulletproofs_enabled(), error::invalid_state(E_NATIVE_FUN_NOT_AVAILABLE));

    verify_range_proof_internal(
        ristretto255::point_to_bytes(&amp;pedersen::commitment_as_compressed_point(com)),
        &amp;ristretto255::basepoint(), &amp;ristretto255::hash_to_point_base(),
        proof.bytes,
        num_bits,
        dst
    )
&#125;
</code></pre>



</details>

<a id="0x1_ristretto255_bulletproofs_verify_range_proof"></a>

## Function `verify_range_proof`

Verifies a zero-knowledge range proof that the value <code>v</code> committed in <code>com</code> (as v * val_base + r * rand_base,
for some randomness <code>r</code>) satisfies <code>v</code> in <code>[0, 2^num_bits)</code>. Only works for <code>num_bits</code> in <code>&#123;8, 16, 32, 64&#125;</code>.


<pre><code>public fun verify_range_proof(com: &amp;ristretto255::RistrettoPoint, val_base: &amp;ristretto255::RistrettoPoint, rand_base: &amp;ristretto255::RistrettoPoint, proof: &amp;ristretto255_bulletproofs::RangeProof, num_bits: u64, dst: vector&lt;u8&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun verify_range_proof(
    com: &amp;RistrettoPoint,
    val_base: &amp;RistrettoPoint, rand_base: &amp;RistrettoPoint,
    proof: &amp;RangeProof, num_bits: u64, dst: vector&lt;u8&gt;): bool
&#123;
    assert!(features::bulletproofs_enabled(), error::invalid_state(E_NATIVE_FUN_NOT_AVAILABLE));

    verify_range_proof_internal(
        ristretto255::point_to_bytes(&amp;ristretto255::point_compress(com)),
        val_base, rand_base,
        proof.bytes, num_bits, dst
    )
&#125;
</code></pre>



</details>

<a id="0x1_ristretto255_bulletproofs_verify_range_proof_internal"></a>

## Function `verify_range_proof_internal`

Aborts with <code>error::invalid_argument(E_DESERIALIZE_RANGE_PROOF)</code> if <code>proof</code> is not a valid serialization of a
range proof.
Aborts with <code>error::invalid_argument(E_RANGE_NOT_SUPPORTED)</code> if an unsupported <code>num_bits</code> is provided.


<pre><code>fun verify_range_proof_internal(com: vector&lt;u8&gt;, val_base: &amp;ristretto255::RistrettoPoint, rand_base: &amp;ristretto255::RistrettoPoint, proof: vector&lt;u8&gt;, num_bits: u64, dst: vector&lt;u8&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun verify_range_proof_internal(
    com: vector&lt;u8&gt;,
    val_base: &amp;RistrettoPoint,
    rand_base: &amp;RistrettoPoint,
    proof: vector&lt;u8&gt;,
    num_bits: u64,
    dst: vector&lt;u8&gt;): bool;
</code></pre>



</details>

<a id="@Specification_1"></a>

## Specification


<a id="@Specification_1_verify_range_proof_internal"></a>

### Function `verify_range_proof_internal`


<pre><code>fun verify_range_proof_internal(com: vector&lt;u8&gt;, val_base: &amp;ristretto255::RistrettoPoint, rand_base: &amp;ristretto255::RistrettoPoint, proof: vector&lt;u8&gt;, num_bits: u64, dst: vector&lt;u8&gt;): bool
</code></pre>




<pre><code>pragma opaque;
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
