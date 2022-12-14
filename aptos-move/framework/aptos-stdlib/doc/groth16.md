
<a name="0x1_groth16"></a>

# Module `0x1::groth16`



-  [Struct `VerifyingKey`](#0x1_groth16_VerifyingKey)
-  [Struct `Proof`](#0x1_groth16_Proof)
-  [Struct `Scalar`](#0x1_groth16_Scalar)
-  [Function `verify_proof`](#0x1_groth16_verify_proof)
-  [Function `new_verifying_key_from_bytes`](#0x1_groth16_new_verifying_key_from_bytes)
-  [Function `new_proof_from_bytes`](#0x1_groth16_new_proof_from_bytes)
-  [Function `new_scalar_from_bytes`](#0x1_groth16_new_scalar_from_bytes)


<pre><code></code></pre>



<a name="0x1_groth16_VerifyingKey"></a>

## Struct `VerifyingKey`



<pre><code><b>struct</b> <a href="groth16.md#0x1_groth16_VerifyingKey">VerifyingKey</a> <b>has</b> drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>handle: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_groth16_Proof"></a>

## Struct `Proof`



<pre><code><b>struct</b> <a href="groth16.md#0x1_groth16_Proof">Proof</a> <b>has</b> drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>handle: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_groth16_Scalar"></a>

## Struct `Scalar`



<pre><code><b>struct</b> <a href="groth16.md#0x1_groth16_Scalar">Scalar</a> <b>has</b> drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>handle: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_groth16_verify_proof"></a>

## Function `verify_proof`



<pre><code><b>public</b> <b>fun</b> <a href="groth16.md#0x1_groth16_verify_proof">verify_proof</a>(_verifying_key: &<a href="groth16.md#0x1_groth16_VerifyingKey">groth16::VerifyingKey</a>, _public_inputs: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="groth16.md#0x1_groth16_Scalar">groth16::Scalar</a>&gt;, _proof: &<a href="groth16.md#0x1_groth16_Proof">groth16::Proof</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>public</b> <b>fun</b> <a href="groth16.md#0x1_groth16_verify_proof">verify_proof</a>(
    _verifying_key: &<a href="groth16.md#0x1_groth16_VerifyingKey">VerifyingKey</a>,
    _public_inputs: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="groth16.md#0x1_groth16_Scalar">Scalar</a>&gt;,
    _proof: &<a href="groth16.md#0x1_groth16_Proof">Proof</a>,
): bool;
</code></pre>



</details>

<a name="0x1_groth16_new_verifying_key_from_bytes"></a>

## Function `new_verifying_key_from_bytes`



<pre><code><b>public</b> <b>fun</b> <a href="groth16.md#0x1_groth16_new_verifying_key_from_bytes">new_verifying_key_from_bytes</a>(bytes: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="groth16.md#0x1_groth16_VerifyingKey">groth16::VerifyingKey</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>public</b> <b>fun</b> <a href="groth16.md#0x1_groth16_new_verifying_key_from_bytes">new_verifying_key_from_bytes</a>(bytes: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="groth16.md#0x1_groth16_VerifyingKey">VerifyingKey</a>;
</code></pre>



</details>

<a name="0x1_groth16_new_proof_from_bytes"></a>

## Function `new_proof_from_bytes`



<pre><code><b>public</b> <b>fun</b> <a href="groth16.md#0x1_groth16_new_proof_from_bytes">new_proof_from_bytes</a>(bytes: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="groth16.md#0x1_groth16_Proof">groth16::Proof</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>public</b> <b>fun</b> <a href="groth16.md#0x1_groth16_new_proof_from_bytes">new_proof_from_bytes</a>(bytes: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="groth16.md#0x1_groth16_Proof">Proof</a>;
</code></pre>



</details>

<a name="0x1_groth16_new_scalar_from_bytes"></a>

## Function `new_scalar_from_bytes`



<pre><code><b>public</b> <b>fun</b> <a href="groth16.md#0x1_groth16_new_scalar_from_bytes">new_scalar_from_bytes</a>(bytes: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="groth16.md#0x1_groth16_Scalar">groth16::Scalar</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>public</b> <b>fun</b> <a href="groth16.md#0x1_groth16_new_scalar_from_bytes">new_scalar_from_bytes</a>(bytes: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="groth16.md#0x1_groth16_Scalar">Scalar</a>;
</code></pre>



</details>


[move-book]: https://move-language.github.io/move/introduction.html
