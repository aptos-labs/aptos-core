
<a id="0x7_sigma_protocol_proof"></a>

# Module `0x7::sigma_protocol_proof`



-  [Struct `Proof`](#0x7_sigma_protocol_proof_Proof)
-  [Constants](#@Constants_0)
-  [Function `new_proof`](#0x7_sigma_protocol_proof_new_proof)
-  [Function `new_proof_from_bytes`](#0x7_sigma_protocol_proof_new_proof_from_bytes)
-  [Function `response_to_witness`](#0x7_sigma_protocol_proof_response_to_witness)
-  [Function `get_commitment`](#0x7_sigma_protocol_proof_get_commitment)
-  [Function `get_compressed_commitment`](#0x7_sigma_protocol_proof_get_compressed_commitment)
-  [Function `get_response_length`](#0x7_sigma_protocol_proof_get_response_length)
-  [Function `get_response`](#0x7_sigma_protocol_proof_get_response)


<pre><code><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255">0x1::ristretto255</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
<b>use</b> <a href="sigma_protocol_utils.md#0x7_sigma_protocol_utils">0x7::sigma_protocol_utils</a>;
<b>use</b> <a href="sigma_protocol_witness.md#0x7_sigma_protocol_witness">0x7::sigma_protocol_witness</a>;
</code></pre>



<a id="0x7_sigma_protocol_proof_Proof"></a>

## Struct `Proof`

A sigma protocol *proof* always consists of:
1. a *commitment* $A \in \mathbb{G}^m$
2. a *compressed commitment* (redundant, for faster Fiat-Shamir)
3. a *response* $\sigma \in \mathbb{F}^k$


<pre><code><b>struct</b> <a href="sigma_protocol_proof.md#0x7_sigma_protocol_proof_Proof">Proof</a> <b>has</b> drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>comm_A: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>compressed_comm_A: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>resp_sigma: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x7_sigma_protocol_proof_E_MISMATCHED_NUMBER_OF_COMPRESSED_POINTS"></a>

When creating a <code><a href="sigma_protocol_proof.md#0x7_sigma_protocol_proof_Proof">Proof</a></code>, the # of commitment points must match the # of compressed commitment points.


<pre><code><b>const</b> <a href="sigma_protocol_proof.md#0x7_sigma_protocol_proof_E_MISMATCHED_NUMBER_OF_COMPRESSED_POINTS">E_MISMATCHED_NUMBER_OF_COMPRESSED_POINTS</a>: u64 = 1;
</code></pre>



<a id="0x7_sigma_protocol_proof_new_proof"></a>

## Function `new_proof`

Creates a new proof consisting of the commitment $A \in \mathbb{G}^m$ and the scalars $\sigma \in \mathbb{F}^k$.


<pre><code><b>public</b> <b>fun</b> <a href="sigma_protocol_proof.md#0x7_sigma_protocol_proof_new_proof">new_proof</a>(_A: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;, compressed_A: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;, sigma: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>&gt;): <a href="sigma_protocol_proof.md#0x7_sigma_protocol_proof_Proof">sigma_protocol_proof::Proof</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="sigma_protocol_proof.md#0x7_sigma_protocol_proof_new_proof">new_proof</a>(
    _A: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;RistrettoPoint&gt;,
    compressed_A: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;CompressedRistretto&gt;,
    sigma: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Scalar&gt;
): <a href="sigma_protocol_proof.md#0x7_sigma_protocol_proof_Proof">Proof</a> {
    <b>assert</b>!(_A.length() == compressed_A.length(), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="sigma_protocol_proof.md#0x7_sigma_protocol_proof_E_MISMATCHED_NUMBER_OF_COMPRESSED_POINTS">E_MISMATCHED_NUMBER_OF_COMPRESSED_POINTS</a>));

    <a href="sigma_protocol_proof.md#0x7_sigma_protocol_proof_Proof">Proof</a> {
        comm_A: _A,
        compressed_comm_A: compressed_A,
        resp_sigma: sigma,
    }
}
</code></pre>



</details>

<a id="0x7_sigma_protocol_proof_new_proof_from_bytes"></a>

## Function `new_proof_from_bytes`

Deserializes the elliptic curve points and scalars and then calls <code>new_proof</code>.


<pre><code><b>public</b> <b>fun</b> <a href="sigma_protocol_proof.md#0x7_sigma_protocol_proof_new_proof_from_bytes">new_proof_from_bytes</a>(_A_bytes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, sigma_bytes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;): <a href="sigma_protocol_proof.md#0x7_sigma_protocol_proof_Proof">sigma_protocol_proof::Proof</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="sigma_protocol_proof.md#0x7_sigma_protocol_proof_new_proof_from_bytes">new_proof_from_bytes</a>(
    _A_bytes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    sigma_bytes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;
): <a href="sigma_protocol_proof.md#0x7_sigma_protocol_proof_Proof">Proof</a> {
    <b>let</b> (_A, compressed_A) = <a href="sigma_protocol_utils.md#0x7_sigma_protocol_utils_deserialize_points">sigma_protocol_utils::deserialize_points</a>(_A_bytes);

    <a href="sigma_protocol_proof.md#0x7_sigma_protocol_proof_new_proof">new_proof</a>(_A, compressed_A, <a href="sigma_protocol_utils.md#0x7_sigma_protocol_utils_deserialize_scalars">sigma_protocol_utils::deserialize_scalars</a>(sigma_bytes))
}
</code></pre>



</details>

<a id="0x7_sigma_protocol_proof_response_to_witness"></a>

## Function `response_to_witness`

Returns a <code>Witness</code> with the <code>w</code> field set to the proof's $\sigma$ field.
This is needed during proof verification: when calling the homomorphism on the <code><a href="sigma_protocol_proof.md#0x7_sigma_protocol_proof_Proof">Proof</a></code>'s $\sigma$, it expects a
<code>Witness</code> not a <code><a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Scalar&gt;</code>.


<pre><code><b>public</b> <b>fun</b> <a href="sigma_protocol_proof.md#0x7_sigma_protocol_proof_response_to_witness">response_to_witness</a>(self: &<a href="sigma_protocol_proof.md#0x7_sigma_protocol_proof_Proof">sigma_protocol_proof::Proof</a>): <a href="sigma_protocol_witness.md#0x7_sigma_protocol_witness_Witness">sigma_protocol_witness::Witness</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="sigma_protocol_proof.md#0x7_sigma_protocol_proof_response_to_witness">response_to_witness</a>(self: &<a href="sigma_protocol_proof.md#0x7_sigma_protocol_proof_Proof">Proof</a>): Witness {
    new_secret_witness(self.resp_sigma)
}
</code></pre>



</details>

<a id="0x7_sigma_protocol_proof_get_commitment"></a>

## Function `get_commitment`



<pre><code><b>public</b> <b>fun</b> <a href="sigma_protocol_proof.md#0x7_sigma_protocol_proof_get_commitment">get_commitment</a>(self: &<a href="sigma_protocol_proof.md#0x7_sigma_protocol_proof_Proof">sigma_protocol_proof::Proof</a>): &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="sigma_protocol_proof.md#0x7_sigma_protocol_proof_get_commitment">get_commitment</a>(self: &<a href="sigma_protocol_proof.md#0x7_sigma_protocol_proof_Proof">Proof</a>): &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;RistrettoPoint&gt; {
    &self.comm_A
}
</code></pre>



</details>

<a id="0x7_sigma_protocol_proof_get_compressed_commitment"></a>

## Function `get_compressed_commitment`



<pre><code><b>public</b> <b>fun</b> <a href="sigma_protocol_proof.md#0x7_sigma_protocol_proof_get_compressed_commitment">get_compressed_commitment</a>(self: &<a href="sigma_protocol_proof.md#0x7_sigma_protocol_proof_Proof">sigma_protocol_proof::Proof</a>): &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="sigma_protocol_proof.md#0x7_sigma_protocol_proof_get_compressed_commitment">get_compressed_commitment</a>(self: &<a href="sigma_protocol_proof.md#0x7_sigma_protocol_proof_Proof">Proof</a>): &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;CompressedRistretto&gt; {
    &self.compressed_comm_A
}
</code></pre>



</details>

<a id="0x7_sigma_protocol_proof_get_response_length"></a>

## Function `get_response_length`



<pre><code><b>public</b> <b>fun</b> <a href="sigma_protocol_proof.md#0x7_sigma_protocol_proof_get_response_length">get_response_length</a>(self: &<a href="sigma_protocol_proof.md#0x7_sigma_protocol_proof_Proof">sigma_protocol_proof::Proof</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="sigma_protocol_proof.md#0x7_sigma_protocol_proof_get_response_length">get_response_length</a>(self: &<a href="sigma_protocol_proof.md#0x7_sigma_protocol_proof_Proof">Proof</a>): u64 {
    self.resp_sigma.length()
}
</code></pre>



</details>

<a id="0x7_sigma_protocol_proof_get_response"></a>

## Function `get_response`



<pre><code><b>public</b> <b>fun</b> <a href="sigma_protocol_proof.md#0x7_sigma_protocol_proof_get_response">get_response</a>(self: &<a href="sigma_protocol_proof.md#0x7_sigma_protocol_proof_Proof">sigma_protocol_proof::Proof</a>): &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="sigma_protocol_proof.md#0x7_sigma_protocol_proof_get_response">get_response</a>(self: &<a href="sigma_protocol_proof.md#0x7_sigma_protocol_proof_Proof">Proof</a>): &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Scalar&gt; {
    &self.resp_sigma
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
