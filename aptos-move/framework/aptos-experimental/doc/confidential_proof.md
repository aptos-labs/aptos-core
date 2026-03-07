
<a id="0x7_confidential_proof"></a>

# Module `0x7::confidential_proof`

The <code><a href="confidential_proof.md#0x7_confidential_proof">confidential_proof</a></code> module provides range proof verification helpers used by the Confidential Asset protocol.
Proof enums and their verify/prove functions live in <code><a href="confidential_asset.md#0x7_confidential_asset">confidential_asset</a></code> (since Move disallows friend
modules from constructing/destructuring enum variants).


-  [Constants](#@Constants_0)
-  [Function `assert_valid_range_proof`](#0x7_confidential_proof_assert_valid_range_proof)
-  [Function `get_bulletproofs_dst`](#0x7_confidential_proof_get_bulletproofs_dst)
-  [Function `get_bulletproofs_num_bits`](#0x7_confidential_proof_get_bulletproofs_num_bits)


<pre><code><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255">0x1::ristretto255</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255_bulletproofs.md#0x1_ristretto255_bulletproofs">0x1::ristretto255_bulletproofs</a>;
</code></pre>



<a id="@Constants_0"></a>

## Constants


<a id="0x7_confidential_proof_BULLETPROOFS_DST"></a>



<pre><code><b>const</b> <a href="confidential_proof.md#0x7_confidential_proof_BULLETPROOFS_DST">BULLETPROOFS_DST</a>: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [65, 112, 116, 111, 115, 67, 111, 110, 102, 105, 100, 101, 110, 116, 105, 97, 108, 65, 115, 115, 101, 116, 47, 66, 117, 108, 108, 101, 116, 112, 114, 111, 111, 102, 82, 97, 110, 103, 101, 80, 114, 111, 111, 102];
</code></pre>



<a id="0x7_confidential_proof_BULLETPROOFS_NUM_BITS"></a>



<pre><code><b>const</b> <a href="confidential_proof.md#0x7_confidential_proof_BULLETPROOFS_NUM_BITS">BULLETPROOFS_NUM_BITS</a>: u64 = 16;
</code></pre>



<a id="0x7_confidential_proof_ERANGE_PROOF_VERIFICATION_FAILED"></a>



<pre><code><b>const</b> <a href="confidential_proof.md#0x7_confidential_proof_ERANGE_PROOF_VERIFICATION_FAILED">ERANGE_PROOF_VERIFICATION_FAILED</a>: u64 = 2;
</code></pre>



<a id="0x7_confidential_proof_assert_valid_range_proof"></a>

## Function `assert_valid_range_proof`

Asserts that the given commitment chunks are each in [0, 2^16) via a range proof.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="confidential_proof.md#0x7_confidential_proof_assert_valid_range_proof">assert_valid_range_proof</a>(commitments: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;, zkrp: &<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255_bulletproofs.md#0x1_ristretto255_bulletproofs_RangeProof">ristretto255_bulletproofs::RangeProof</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="confidential_proof.md#0x7_confidential_proof_assert_valid_range_proof">assert_valid_range_proof</a>(
    commitments: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;RistrettoPoint&gt;,
    zkrp: &RangeProof
) {
    <b>let</b> commitments = commitments.map_ref(|c| c.point_clone());

    <b>assert</b>!(
        bulletproofs::verify_batch_range_proof(
            &commitments,
            &<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_basepoint">ristretto255::basepoint</a>(),
            &<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_hash_to_point_base">ristretto255::hash_to_point_base</a>(),
            zkrp,
            <a href="confidential_proof.md#0x7_confidential_proof_BULLETPROOFS_NUM_BITS">BULLETPROOFS_NUM_BITS</a>,
            <a href="confidential_proof.md#0x7_confidential_proof_BULLETPROOFS_DST">BULLETPROOFS_DST</a>
        ),
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="confidential_proof.md#0x7_confidential_proof_ERANGE_PROOF_VERIFICATION_FAILED">ERANGE_PROOF_VERIFICATION_FAILED</a>)
    );
}
</code></pre>



</details>

<a id="0x7_confidential_proof_get_bulletproofs_dst"></a>

## Function `get_bulletproofs_dst`

Returns the DST for the range proofs.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="confidential_proof.md#0x7_confidential_proof_get_bulletproofs_dst">get_bulletproofs_dst</a>(): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_proof.md#0x7_confidential_proof_get_bulletproofs_dst">get_bulletproofs_dst</a>(): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <a href="confidential_proof.md#0x7_confidential_proof_BULLETPROOFS_DST">BULLETPROOFS_DST</a>
}
</code></pre>



</details>

<a id="0x7_confidential_proof_get_bulletproofs_num_bits"></a>

## Function `get_bulletproofs_num_bits`

Returns the maximum number of bits of the normalized chunk for the range proofs.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="confidential_proof.md#0x7_confidential_proof_get_bulletproofs_num_bits">get_bulletproofs_num_bits</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_proof.md#0x7_confidential_proof_get_bulletproofs_num_bits">get_bulletproofs_num_bits</a>(): u64 {
    <a href="confidential_proof.md#0x7_confidential_proof_BULLETPROOFS_NUM_BITS">BULLETPROOFS_NUM_BITS</a>
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
