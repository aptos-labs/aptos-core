
<a id="0x7_proof"></a>

# Module `0x7::proof`



-  [Struct `Proof`](#0x7_proof_Proof)
-  [Function `new`](#0x7_proof_new)
-  [Function `response_to_witness`](#0x7_proof_response_to_witness)
-  [Function `get_response_length`](#0x7_proof_get_response_length)
-  [Function `get_commitment`](#0x7_proof_get_commitment)


<pre><code><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255">0x1::ristretto255</a>;
<b>use</b> <a href="secret_witness.md#0x7_secret_witness">0x7::secret_witness</a>;
</code></pre>



<a id="0x7_proof_Proof"></a>

## Struct `Proof`

A sigma protocol *proof* always consists of:
1. a *commitment* $A \in \mathbb{G}^m$
2. a *response* $\sigma \in \mathbb{F}^k$


<pre><code><b>struct</b> <a href="proof.md#0x7_proof_Proof">Proof</a> <b>has</b> drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>A: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>sigma: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x7_proof_new"></a>

## Function `new`

Creates a new proof consisting of the commitment $A \in \mathbb{G}^m$ and the scalars $\sigma \in \mathbb{F}^k$.


<pre><code><b>public</b> <b>fun</b> <a href="proof.md#0x7_proof_new">new</a>(_A: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;, sigma: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>&gt;): <a href="proof.md#0x7_proof_Proof">proof::Proof</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="proof.md#0x7_proof_new">new</a>(_A: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;RistrettoPoint&gt;, sigma: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Scalar&gt;): <a href="proof.md#0x7_proof_Proof">Proof</a> {
    <a href="proof.md#0x7_proof_Proof">Proof</a> {
        A: _A,
        sigma,
    }
}
</code></pre>



</details>

<a id="0x7_proof_response_to_witness"></a>

## Function `response_to_witness`

Returns a <code>SecretWitness</code> with the <code>w</code> field is to the proof's $\sigma$ field.
This is needed during proof verification: when calling the homomorphism on the <code><a href="proof.md#0x7_proof_Proof">Proof</a></code>'s $\sigma$, it expects a
<code>SecretWitness</code> not a <code><a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Scalar&gt;</code>.


<pre><code><b>public</b> <b>fun</b> <a href="proof.md#0x7_proof_response_to_witness">response_to_witness</a>(self: &<a href="proof.md#0x7_proof_Proof">proof::Proof</a>): <a href="secret_witness.md#0x7_secret_witness_SecretWitness">secret_witness::SecretWitness</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="proof.md#0x7_proof_response_to_witness">response_to_witness</a>(self: &<a href="proof.md#0x7_proof_Proof">Proof</a>): SecretWitness {
    new_secret_witness(self.sigma)
}
</code></pre>



</details>

<a id="0x7_proof_get_response_length"></a>

## Function `get_response_length`

Returns $k = |\sigma|$.


<pre><code><b>public</b> <b>fun</b> <a href="proof.md#0x7_proof_get_response_length">get_response_length</a>(self: &<a href="proof.md#0x7_proof_Proof">proof::Proof</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="proof.md#0x7_proof_get_response_length">get_response_length</a>(self: &<a href="proof.md#0x7_proof_Proof">Proof</a>): u64 {
    self.sigma.length()
}
</code></pre>



</details>

<a id="0x7_proof_get_commitment"></a>

## Function `get_commitment`

Returns the commitment component $A$ of the proof.


<pre><code><b>public</b> <b>fun</b> <a href="proof.md#0x7_proof_get_commitment">get_commitment</a>(self: &<a href="proof.md#0x7_proof_Proof">proof::Proof</a>): &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="proof.md#0x7_proof_get_commitment">get_commitment</a>(self: &<a href="proof.md#0x7_proof_Proof">Proof</a>): &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;RistrettoPoint&gt; {
    &self.A
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
