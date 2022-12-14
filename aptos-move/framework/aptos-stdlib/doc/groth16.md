
<a name="0x1_groth16"></a>

# Module `0x1::groth16`



-  [Struct `VerifyingKey`](#0x1_groth16_VerifyingKey)
-  [Struct `Proof`](#0x1_groth16_Proof)
-  [Function `verify_proof`](#0x1_groth16_verify_proof)
-  [Function `new_verifying_key_from_bytes`](#0x1_groth16_new_verifying_key_from_bytes)
-  [Function `new_proof_from_bytes`](#0x1_groth16_new_proof_from_bytes)


<pre><code><b>use</b> <a href="curves.md#0x1_curves">0x1::curves</a>;
</code></pre>



<a name="0x1_groth16_VerifyingKey"></a>

## Struct `VerifyingKey`



<pre><code><b>struct</b> <a href="groth16.md#0x1_groth16_VerifyingKey">VerifyingKey</a>&lt;G1, G2, Gt&gt; <b>has</b> drop
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

<a name="0x1_groth16_Proof"></a>

## Struct `Proof`



<pre><code><b>struct</b> <a href="groth16.md#0x1_groth16_Proof">Proof</a>&lt;G1, G2, Gt&gt; <b>has</b> drop
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

<a name="0x1_groth16_verify_proof"></a>

## Function `verify_proof`



<pre><code><b>public</b> <b>fun</b> <a href="groth16.md#0x1_groth16_verify_proof">verify_proof</a>&lt;G1, G2, Gt&gt;(_verifying_key: &<a href="groth16.md#0x1_groth16_VerifyingKey">groth16::VerifyingKey</a>&lt;G1, G2, Gt&gt;, _public_inputs: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="curves.md#0x1_curves_Scalar">curves::Scalar</a>&lt;G1&gt;&gt;, _proof: &<a href="groth16.md#0x1_groth16_Proof">groth16::Proof</a>&lt;G1, G2, Gt&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>public</b> <b>fun</b> <a href="groth16.md#0x1_groth16_verify_proof">verify_proof</a>&lt;G1,G2,Gt&gt;(
    _verifying_key: &<a href="groth16.md#0x1_groth16_VerifyingKey">VerifyingKey</a>&lt;G1,G2,Gt&gt;,
    _public_inputs: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="curves.md#0x1_curves_Scalar">curves::Scalar</a>&lt;G1&gt;&gt;,
    _proof: &<a href="groth16.md#0x1_groth16_Proof">Proof</a>&lt;G1,G2,Gt&gt;,
): bool;
</code></pre>



</details>

<a name="0x1_groth16_new_verifying_key_from_bytes"></a>

## Function `new_verifying_key_from_bytes`



<pre><code><b>public</b> <b>fun</b> <a href="groth16.md#0x1_groth16_new_verifying_key_from_bytes">new_verifying_key_from_bytes</a>&lt;G1, G2, Gt&gt;(bytes: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="groth16.md#0x1_groth16_VerifyingKey">groth16::VerifyingKey</a>&lt;G1, G2, Gt&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>public</b> <b>fun</b> <a href="groth16.md#0x1_groth16_new_verifying_key_from_bytes">new_verifying_key_from_bytes</a>&lt;G1,G2,Gt&gt;(bytes: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="groth16.md#0x1_groth16_VerifyingKey">VerifyingKey</a>&lt;G1,G2,Gt&gt;;
</code></pre>



</details>

<a name="0x1_groth16_new_proof_from_bytes"></a>

## Function `new_proof_from_bytes`



<pre><code><b>public</b> <b>fun</b> <a href="groth16.md#0x1_groth16_new_proof_from_bytes">new_proof_from_bytes</a>&lt;G1, G2, Gt&gt;(bytes: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="groth16.md#0x1_groth16_Proof">groth16::Proof</a>&lt;G1, G2, Gt&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>public</b> <b>fun</b> <a href="groth16.md#0x1_groth16_new_proof_from_bytes">new_proof_from_bytes</a>&lt;G1,G2,Gt&gt;(bytes: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="groth16.md#0x1_groth16_Proof">Proof</a>&lt;G1,G2,Gt&gt;;
</code></pre>



</details>


[move-book]: https://move-language.github.io/move/introduction.html
