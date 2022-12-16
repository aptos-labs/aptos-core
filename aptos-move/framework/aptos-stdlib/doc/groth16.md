
<a name="0x1_groth16"></a>

# Module `0x1::groth16`



-  [Struct `VerifyingKey`](#0x1_groth16_VerifyingKey)
-  [Struct `Proof`](#0x1_groth16_Proof)
-  [Function `new_vk`](#0x1_groth16_new_vk)
-  [Function `new_proof`](#0x1_groth16_new_proof)
-  [Function `verify_proof`](#0x1_groth16_verify_proof)
-  [Function `verify_proof_internal`](#0x1_groth16_verify_proof_internal)


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
<code>alpha_g1: <a href="curves.md#0x1_curves_Point">curves::Point</a>&lt;G1&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>beta_g2: <a href="curves.md#0x1_curves_Point">curves::Point</a>&lt;G2&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>gamma_g2: <a href="curves.md#0x1_curves_Point">curves::Point</a>&lt;G2&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>delta_g2: <a href="curves.md#0x1_curves_Point">curves::Point</a>&lt;G2&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>gamma_abc_g1: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="curves.md#0x1_curves_Point">curves::Point</a>&lt;G1&gt;&gt;</code>
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
<code>a: <a href="curves.md#0x1_curves_Point">curves::Point</a>&lt;G1&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>b: <a href="curves.md#0x1_curves_Point">curves::Point</a>&lt;G2&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>c: <a href="curves.md#0x1_curves_Point">curves::Point</a>&lt;G1&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_groth16_new_vk"></a>

## Function `new_vk`



<pre><code><b>public</b> <b>fun</b> <a href="groth16.md#0x1_groth16_new_vk">new_vk</a>&lt;G1, G2, Gt&gt;(alpha_g1: <a href="curves.md#0x1_curves_Point">curves::Point</a>&lt;G1&gt;, beta_g2: <a href="curves.md#0x1_curves_Point">curves::Point</a>&lt;G2&gt;, gamma_g2: <a href="curves.md#0x1_curves_Point">curves::Point</a>&lt;G2&gt;, delta_g2: <a href="curves.md#0x1_curves_Point">curves::Point</a>&lt;G2&gt;, gamma_abc_g1: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="curves.md#0x1_curves_Point">curves::Point</a>&lt;G1&gt;&gt;): <a href="groth16.md#0x1_groth16_VerifyingKey">groth16::VerifyingKey</a>&lt;G1, G2, Gt&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="groth16.md#0x1_groth16_new_vk">new_vk</a>&lt;G1,G2,Gt&gt;(alpha_g1: <a href="curves.md#0x1_curves_Point">curves::Point</a>&lt;G1&gt;, beta_g2: <a href="curves.md#0x1_curves_Point">curves::Point</a>&lt;G2&gt;, gamma_g2: <a href="curves.md#0x1_curves_Point">curves::Point</a>&lt;G2&gt;, delta_g2: <a href="curves.md#0x1_curves_Point">curves::Point</a>&lt;G2&gt;, gamma_abc_g1: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="curves.md#0x1_curves_Point">curves::Point</a>&lt;G1&gt;&gt;): <a href="groth16.md#0x1_groth16_VerifyingKey">VerifyingKey</a>&lt;G1,G2,Gt&gt; {
    <a href="groth16.md#0x1_groth16_VerifyingKey">VerifyingKey</a> {
        alpha_g1,
        beta_g2,
        gamma_g2,
        delta_g2,
        gamma_abc_g1,
    }
}
</code></pre>



</details>

<a name="0x1_groth16_new_proof"></a>

## Function `new_proof`



<pre><code><b>public</b> <b>fun</b> <a href="groth16.md#0x1_groth16_new_proof">new_proof</a>&lt;G1, G2, Gt&gt;(a: <a href="curves.md#0x1_curves_Point">curves::Point</a>&lt;G1&gt;, b: <a href="curves.md#0x1_curves_Point">curves::Point</a>&lt;G2&gt;, c: <a href="curves.md#0x1_curves_Point">curves::Point</a>&lt;G1&gt;): <a href="groth16.md#0x1_groth16_Proof">groth16::Proof</a>&lt;G1, G2, Gt&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="groth16.md#0x1_groth16_new_proof">new_proof</a>&lt;G1,G2,Gt&gt;(a: <a href="curves.md#0x1_curves_Point">curves::Point</a>&lt;G1&gt;, b: <a href="curves.md#0x1_curves_Point">curves::Point</a>&lt;G2&gt;, c: <a href="curves.md#0x1_curves_Point">curves::Point</a>&lt;G1&gt;): <a href="groth16.md#0x1_groth16_Proof">Proof</a>&lt;G1,G2,Gt&gt; {
    <a href="groth16.md#0x1_groth16_Proof">Proof</a> { a, b, c }
}
</code></pre>



</details>

<a name="0x1_groth16_verify_proof"></a>

## Function `verify_proof`



<pre><code><b>public</b> <b>fun</b> <a href="groth16.md#0x1_groth16_verify_proof">verify_proof</a>&lt;G1, G2, Gt&gt;(_vk: &<a href="groth16.md#0x1_groth16_VerifyingKey">groth16::VerifyingKey</a>&lt;G1, G2, Gt&gt;, _public_inputs: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="curves.md#0x1_curves_Scalar">curves::Scalar</a>&lt;G1&gt;&gt;, _proof: &<a href="groth16.md#0x1_groth16_Proof">groth16::Proof</a>&lt;G1, G2, Gt&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="groth16.md#0x1_groth16_verify_proof">verify_proof</a>&lt;G1,G2,Gt&gt;(_vk: &<a href="groth16.md#0x1_groth16_VerifyingKey">VerifyingKey</a>&lt;G1,G2,Gt&gt;, _public_inputs: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="curves.md#0x1_curves_Scalar">curves::Scalar</a>&lt;G1&gt;&gt;, _proof: &<a href="groth16.md#0x1_groth16_Proof">Proof</a>&lt;G1,G2,Gt&gt;): bool {
    <b>let</b> gamma_abc_g1_handles = <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];
    <b>let</b> gamma_abc_g1_count = std::vector::length(&_vk.gamma_abc_g1);
    <b>let</b> i = 0;
    <b>while</b> (i &lt; gamma_abc_g1_count) {
        <b>let</b> item = std::vector::borrow(&_vk.gamma_abc_g1, i);
        <b>let</b> handle = <a href="curves.md#0x1_curves_get_point_handle">curves::get_point_handle</a>(item);
        std::vector::push_back(&<b>mut</b> gamma_abc_g1_handles, (handle <b>as</b> u8));
        i = i + 1;
    };

    <b>let</b> public_input_handles: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];
    <b>let</b> public_input_count = std::vector::length(_public_inputs);
    <b>let</b> i = 0;
    <b>while</b> (i &lt; public_input_count) {
        <b>let</b> item = std::vector::borrow(_public_inputs, i);
        <b>let</b> handle = <a href="curves.md#0x1_curves_get_scalar_handle">curves::get_scalar_handle</a>(item);
        std::vector::push_back(&<b>mut</b> public_input_handles, (handle <b>as</b> u8));
        i = i + 1;
    };

    <a href="groth16.md#0x1_groth16_verify_proof_internal">verify_proof_internal</a>(
        <a href="curves.md#0x1_curves_get_point_handle">curves::get_point_handle</a>(&_vk.alpha_g1),
        <a href="curves.md#0x1_curves_get_point_handle">curves::get_point_handle</a>(&_vk.beta_g2),
        <a href="curves.md#0x1_curves_get_point_handle">curves::get_point_handle</a>(&_vk.gamma_g2),
        <a href="curves.md#0x1_curves_get_point_handle">curves::get_point_handle</a>(&_vk.delta_g2),
        gamma_abc_g1_handles,
        <a href="curves.md#0x1_curves_get_point_handle">curves::get_point_handle</a>(&_proof.a),
        <a href="curves.md#0x1_curves_get_point_handle">curves::get_point_handle</a>(&_proof.b),
        <a href="curves.md#0x1_curves_get_point_handle">curves::get_point_handle</a>(&_proof.c),
        public_input_handles,
        <a href="curves.md#0x1_curves_get_pairing_id">curves::get_pairing_id</a>&lt;G1,G2,Gt&gt;()
    )
}
</code></pre>



</details>

<a name="0x1_groth16_verify_proof_internal"></a>

## Function `verify_proof_internal`



<pre><code><b>fun</b> <a href="groth16.md#0x1_groth16_verify_proof_internal">verify_proof_internal</a>(vk_alpha_g1_handle: u8, vk_beta_g_handle: u8, vk_gamma_g2_handle: u8, vk_delta_g2_handle: u8, gamma_abc_g1_handles: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, proof_a_handle: u8, proof_b_handle: u8, proof_c_handle: u8, public_input_handle: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, pairing_id: u8): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="groth16.md#0x1_groth16_verify_proof_internal">verify_proof_internal</a>(
    vk_alpha_g1_handle: u8, vk_beta_g_handle: u8, vk_gamma_g2_handle: u8, vk_delta_g2_handle: u8, gamma_abc_g1_handles: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    proof_a_handle: u8, proof_b_handle: u8, proof_c_handle: u8,
    public_input_handle: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    pairing_id: u8
): bool;
</code></pre>



</details>


[move-book]: https://move-language.github.io/move/introduction.html
