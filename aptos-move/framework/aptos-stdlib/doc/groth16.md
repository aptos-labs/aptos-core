
<a name="0x1_groth16"></a>

# Module `0x1::groth16`



-  [Struct `VerifyingKey`](#0x1_groth16_VerifyingKey)
-  [Struct `Proof`](#0x1_groth16_Proof)
-  [Struct `Scalar`](#0x1_groth16_Scalar)
-  [Function `verify_proof`](#0x1_groth16_verify_proof)
-  [Function `new_verifying_key_from_bytes`](#0x1_groth16_new_verifying_key_from_bytes)
-  [Function `new_proof_from_bytes`](#0x1_groth16_new_proof_from_bytes)
-  [Function `new_scalar_from_bytes`](#0x1_groth16_new_scalar_from_bytes)
-  [Function `new_verifying_key_from_bytes_internal`](#0x1_groth16_new_verifying_key_from_bytes_internal)
-  [Function `new_proof_from_bytes_internal`](#0x1_groth16_new_proof_from_bytes_internal)
-  [Function `new_scalar_from_bytes_internal`](#0x1_groth16_new_scalar_from_bytes_internal)
-  [Function `verify_proof_internal`](#0x1_groth16_verify_proof_internal)


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


<pre><code><b>public</b> <b>fun</b> <a href="groth16.md#0x1_groth16_verify_proof">verify_proof</a>(
    _verifying_key: &<a href="groth16.md#0x1_groth16_VerifyingKey">VerifyingKey</a>,
    _public_inputs: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="groth16.md#0x1_groth16_Scalar">Scalar</a>&gt;,
    _proof: &<a href="groth16.md#0x1_groth16_Proof">Proof</a>,
): bool {
    <b>let</b> public_input_handles: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];
    <b>let</b> num_public_inputs = std::vector::length(_public_inputs);
    <b>let</b> i = 0;
    <b>while</b> (i &lt; num_public_inputs) {
        std::vector::push_back(&<b>mut</b> public_input_handles, (std::vector::borrow(_public_inputs, i).handle <b>as</b> u8));
        i = i + 1;
    };

    <a href="groth16.md#0x1_groth16_verify_proof_internal">verify_proof_internal</a>(_verifying_key.handle, public_input_handles, _proof.handle)
}
</code></pre>



</details>

<a name="0x1_groth16_new_verifying_key_from_bytes"></a>

## Function `new_verifying_key_from_bytes`



<pre><code><b>public</b> <b>fun</b> <a href="groth16.md#0x1_groth16_new_verifying_key_from_bytes">new_verifying_key_from_bytes</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="groth16.md#0x1_groth16_VerifyingKey">groth16::VerifyingKey</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="groth16.md#0x1_groth16_new_verifying_key_from_bytes">new_verifying_key_from_bytes</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="groth16.md#0x1_groth16_VerifyingKey">VerifyingKey</a> {
    <a href="groth16.md#0x1_groth16_VerifyingKey">VerifyingKey</a> {
        handle: <a href="groth16.md#0x1_groth16_new_verifying_key_from_bytes_internal">new_verifying_key_from_bytes_internal</a>(bytes)
    }
}
</code></pre>



</details>

<a name="0x1_groth16_new_proof_from_bytes"></a>

## Function `new_proof_from_bytes`



<pre><code><b>public</b> <b>fun</b> <a href="groth16.md#0x1_groth16_new_proof_from_bytes">new_proof_from_bytes</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="groth16.md#0x1_groth16_Proof">groth16::Proof</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="groth16.md#0x1_groth16_new_proof_from_bytes">new_proof_from_bytes</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="groth16.md#0x1_groth16_Proof">Proof</a> {
    <a href="groth16.md#0x1_groth16_Proof">Proof</a> {
        handle: <a href="groth16.md#0x1_groth16_new_proof_from_bytes_internal">new_proof_from_bytes_internal</a>(bytes)
    }
}
</code></pre>



</details>

<a name="0x1_groth16_new_scalar_from_bytes"></a>

## Function `new_scalar_from_bytes`



<pre><code><b>public</b> <b>fun</b> <a href="groth16.md#0x1_groth16_new_scalar_from_bytes">new_scalar_from_bytes</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="groth16.md#0x1_groth16_Scalar">groth16::Scalar</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="groth16.md#0x1_groth16_new_scalar_from_bytes">new_scalar_from_bytes</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="groth16.md#0x1_groth16_Scalar">Scalar</a> {
    <a href="groth16.md#0x1_groth16_Scalar">Scalar</a> {
        handle: <a href="groth16.md#0x1_groth16_new_scalar_from_bytes_internal">new_scalar_from_bytes_internal</a>(bytes)
    }
}
</code></pre>



</details>

<a name="0x1_groth16_new_verifying_key_from_bytes_internal"></a>

## Function `new_verifying_key_from_bytes_internal`



<pre><code><b>fun</b> <a href="groth16.md#0x1_groth16_new_verifying_key_from_bytes_internal">new_verifying_key_from_bytes_internal</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="groth16.md#0x1_groth16_new_verifying_key_from_bytes_internal">new_verifying_key_from_bytes_internal</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): u64;
</code></pre>



</details>

<a name="0x1_groth16_new_proof_from_bytes_internal"></a>

## Function `new_proof_from_bytes_internal`



<pre><code><b>fun</b> <a href="groth16.md#0x1_groth16_new_proof_from_bytes_internal">new_proof_from_bytes_internal</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="groth16.md#0x1_groth16_new_proof_from_bytes_internal">new_proof_from_bytes_internal</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): u64;
</code></pre>



</details>

<a name="0x1_groth16_new_scalar_from_bytes_internal"></a>

## Function `new_scalar_from_bytes_internal`



<pre><code><b>fun</b> <a href="groth16.md#0x1_groth16_new_scalar_from_bytes_internal">new_scalar_from_bytes_internal</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="groth16.md#0x1_groth16_new_scalar_from_bytes_internal">new_scalar_from_bytes_internal</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): u64;
</code></pre>



</details>

<a name="0x1_groth16_verify_proof_internal"></a>

## Function `verify_proof_internal`



<pre><code><b>fun</b> <a href="groth16.md#0x1_groth16_verify_proof_internal">verify_proof_internal</a>(vk_handle: u64, public_inputs: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, proof_handle: u64): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="groth16.md#0x1_groth16_verify_proof_internal">verify_proof_internal</a>(vk_handle: u64, public_inputs: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, proof_handle: u64): bool;
</code></pre>



</details>


[move-book]: https://move-language.github.io/move/introduction.html
