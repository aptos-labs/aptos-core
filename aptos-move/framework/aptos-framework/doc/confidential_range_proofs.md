
<a id="0x1_confidential_range_proofs"></a>

# Module `0x1::confidential_range_proofs`

The <code><a href="confidential_range_proofs.md#0x1_confidential_range_proofs">confidential_range_proofs</a></code> module provides range proof verification helpers used by the Confidential Asset protocol.
Proof enums and their verify/prove functions live in <code><a href="confidential_asset.md#0x1_confidential_asset">confidential_asset</a></code> (since Move disallows friend
modules from constructing/destructuring enum variants).


-  [Constants](#@Constants_0)
-  [Function `assert_valid_range_proof`](#0x1_confidential_range_proofs_assert_valid_range_proof)
-  [Function `verify_batch_range_proof`](#0x1_confidential_range_proofs_verify_batch_range_proof)
-  [Function `get_bulletproofs_dst`](#0x1_confidential_range_proofs_get_bulletproofs_dst)
-  [Function `verify_batch_range_proof_internal`](#0x1_confidential_range_proofs_verify_batch_range_proof_internal)
-  [Specification](#@Specification_1)
    -  [Function `verify_batch_range_proof_internal`](#@Specification_1_verify_batch_range_proof_internal)


<pre><code><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features">0x1::features</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255">0x1::ristretto255</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/ristretto255_bulletproofs.md#0x1_ristretto255_bulletproofs">0x1::ristretto255_bulletproofs</a>;
</code></pre>



<a id="@Constants_0"></a>

## Constants


<a id="0x1_confidential_range_proofs_E_NATIVE_FUN_NOT_AVAILABLE"></a>

The native functions have not been rolled out yet.


<pre><code><b>const</b> <a href="confidential_range_proofs.md#0x1_confidential_range_proofs_E_NATIVE_FUN_NOT_AVAILABLE">E_NATIVE_FUN_NOT_AVAILABLE</a>: u64 = 4;
</code></pre>



<a id="0x1_confidential_range_proofs_E_DST_TOO_LONG"></a>

DST exceeds 256 bytes.


<pre><code><b>const</b> <a href="confidential_range_proofs.md#0x1_confidential_range_proofs_E_DST_TOO_LONG">E_DST_TOO_LONG</a>: u64 = 3;
</code></pre>



<a id="0x1_confidential_range_proofs_BULLETPROOFS_DST"></a>



<pre><code><b>const</b> <a href="confidential_range_proofs.md#0x1_confidential_range_proofs_BULLETPROOFS_DST">BULLETPROOFS_DST</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [65, 112, 116, 111, 115, 67, 111, 110, 102, 105, 100, 101, 110, 116, 105, 97, 108, 65, 115, 115, 101, 116, 47, 66, 117, 108, 108, 101, 116, 112, 114, 111, 111, 102, 82, 97, 110, 103, 101, 80, 114, 111, 111, 102];
</code></pre>



<a id="0x1_confidential_range_proofs_ERANGE_PROOF_VERIFICATION_FAILED"></a>



<pre><code><b>const</b> <a href="confidential_range_proofs.md#0x1_confidential_range_proofs_ERANGE_PROOF_VERIFICATION_FAILED">ERANGE_PROOF_VERIFICATION_FAILED</a>: u64 = 2;
</code></pre>



<a id="0x1_confidential_range_proofs_assert_valid_range_proof"></a>

## Function `assert_valid_range_proof`

Asserts that the given commitment chunks are each in [0, 2^16) via a range proof.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="confidential_range_proofs.md#0x1_confidential_range_proofs_assert_valid_range_proof">assert_valid_range_proof</a>(commitments: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;, zkrp: &<a href="../../aptos-stdlib/doc/ristretto255_bulletproofs.md#0x1_ristretto255_bulletproofs_RangeProof">ristretto255_bulletproofs::RangeProof</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="confidential_range_proofs.md#0x1_confidential_range_proofs_assert_valid_range_proof">assert_valid_range_proof</a>(
    commitments: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;RistrettoPoint&gt;,
    zkrp: &RangeProof
) {
    <b>let</b> commitments = commitments.map_ref(|c| c.point_clone());

    <b>assert</b>!(
        <a href="confidential_range_proofs.md#0x1_confidential_range_proofs_verify_batch_range_proof">verify_batch_range_proof</a>(
            &commitments,
            &<a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_basepoint">ristretto255::basepoint</a>(),
            &<a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_hash_to_point_base">ristretto255::hash_to_point_base</a>(),
            zkrp,
            <a href="confidential_balance.md#0x1_confidential_balance_get_chunk_size_bits">confidential_balance::get_chunk_size_bits</a>(),
            <a href="confidential_range_proofs.md#0x1_confidential_range_proofs_BULLETPROOFS_DST">BULLETPROOFS_DST</a>
        ),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="confidential_range_proofs.md#0x1_confidential_range_proofs_ERANGE_PROOF_VERIFICATION_FAILED">ERANGE_PROOF_VERIFICATION_FAILED</a>)
    );
}
</code></pre>



</details>

<a id="0x1_confidential_range_proofs_verify_batch_range_proof"></a>

## Function `verify_batch_range_proof`

Verifies a batch range proof for commitments, ensuring all committed values are in [0, 2^num_bits).


<pre><code><b>fun</b> <a href="confidential_range_proofs.md#0x1_confidential_range_proofs_verify_batch_range_proof">verify_batch_range_proof</a>(comms: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;, val_base: &<a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>, rand_base: &<a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>, proof: &<a href="../../aptos-stdlib/doc/ristretto255_bulletproofs.md#0x1_ristretto255_bulletproofs_RangeProof">ristretto255_bulletproofs::RangeProof</a>, num_bits: u64, dst: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="confidential_range_proofs.md#0x1_confidential_range_proofs_verify_batch_range_proof">verify_batch_range_proof</a>(
    comms: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;RistrettoPoint&gt;,
    val_base: &RistrettoPoint, rand_base: &RistrettoPoint,
    proof: &RangeProof, num_bits: u64, dst: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool
{
    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_bulletproofs_batch_enabled">features::bulletproofs_batch_enabled</a>(), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="confidential_range_proofs.md#0x1_confidential_range_proofs_E_NATIVE_FUN_NOT_AVAILABLE">E_NATIVE_FUN_NOT_AVAILABLE</a>));
    <b>assert</b>!(dst.length() &lt;= 256, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="confidential_range_proofs.md#0x1_confidential_range_proofs_E_DST_TOO_LONG">E_DST_TOO_LONG</a>));

    <b>let</b> comms = comms.map_ref(|com| <a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_to_bytes">ristretto255::point_to_bytes</a>(&<a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_compress">ristretto255::point_compress</a>(com)));

    <a href="confidential_range_proofs.md#0x1_confidential_range_proofs_verify_batch_range_proof_internal">verify_batch_range_proof_internal</a>(
        comms,
        val_base, rand_base,
        bulletproofs::range_proof_to_bytes(proof), num_bits, dst
    )
}
</code></pre>



</details>

<a id="0x1_confidential_range_proofs_get_bulletproofs_dst"></a>

## Function `get_bulletproofs_dst`

Returns the DST for the range proofs.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="confidential_range_proofs.md#0x1_confidential_range_proofs_get_bulletproofs_dst">get_bulletproofs_dst</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_range_proofs.md#0x1_confidential_range_proofs_get_bulletproofs_dst">get_bulletproofs_dst</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <a href="confidential_range_proofs.md#0x1_confidential_range_proofs_BULLETPROOFS_DST">BULLETPROOFS_DST</a>
}
</code></pre>



</details>

<a id="0x1_confidential_range_proofs_verify_batch_range_proof_internal"></a>

## Function `verify_batch_range_proof_internal`



<pre><code><b>fun</b> <a href="confidential_range_proofs.md#0x1_confidential_range_proofs_verify_batch_range_proof_internal">verify_batch_range_proof_internal</a>(comms: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, val_base: &<a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>, rand_base: &<a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>, proof: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, num_bits: u64, dst: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="confidential_range_proofs.md#0x1_confidential_range_proofs_verify_batch_range_proof_internal">verify_batch_range_proof_internal</a>(
    comms: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    val_base: &RistrettoPoint,
    rand_base: &RistrettoPoint,
    proof: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    num_bits: u64,
    dst: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool;
</code></pre>



</details>

<a id="@Specification_1"></a>

## Specification


<a id="@Specification_1_verify_batch_range_proof_internal"></a>

### Function `verify_batch_range_proof_internal`


<pre><code><b>fun</b> <a href="confidential_range_proofs.md#0x1_confidential_range_proofs_verify_batch_range_proof_internal">verify_batch_range_proof_internal</a>(comms: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, val_base: &<a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>, rand_base: &<a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>, proof: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, num_bits: u64, dst: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool
</code></pre>




<pre><code><b>pragma</b> opaque;
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
