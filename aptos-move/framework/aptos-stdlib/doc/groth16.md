
<a name="0x1_groth16"></a>

# Module `0x1::groth16`



-  [Struct `VerifyingKey`](#0x1_groth16_VerifyingKey)
-  [Struct `Proof`](#0x1_groth16_Proof)
-  [Function `new_vk`](#0x1_groth16_new_vk)
-  [Function `new_proof`](#0x1_groth16_new_proof)
-  [Function `verify_proof`](#0x1_groth16_verify_proof)


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



<pre><code><b>public</b> <b>fun</b> <a href="groth16.md#0x1_groth16_verify_proof">verify_proof</a>&lt;G1, G2, Gt&gt;(vk: &<a href="groth16.md#0x1_groth16_VerifyingKey">groth16::VerifyingKey</a>&lt;G1, G2, Gt&gt;, public_inputs: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="curves.md#0x1_curves_Scalar">curves::Scalar</a>&lt;G1&gt;&gt;, proof: &<a href="groth16.md#0x1_groth16_Proof">groth16::Proof</a>&lt;G1, G2, Gt&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="groth16.md#0x1_groth16_verify_proof">verify_proof</a>&lt;G1,G2,Gt&gt;(vk: &<a href="groth16.md#0x1_groth16_VerifyingKey">VerifyingKey</a>&lt;G1,G2,Gt&gt;, public_inputs: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="curves.md#0x1_curves_Scalar">curves::Scalar</a>&lt;G1&gt;&gt;, proof: &<a href="groth16.md#0x1_groth16_Proof">Proof</a>&lt;G1,G2,Gt&gt;): bool {
    <b>let</b> left = <a href="curves.md#0x1_curves_pairing">curves::pairing</a>&lt;G1,G2,Gt&gt;(&proof.a, &proof.b);
    <b>let</b> right_1 = <a href="curves.md#0x1_curves_pairing">curves::pairing</a>&lt;G1,G2,Gt&gt;(&vk.alpha_g1, &vk.beta_g2);

    <b>let</b> n = std::vector::length(public_inputs);
    <b>let</b> i = 0;
    <b>let</b> acc = *std::vector::borrow(&vk.gamma_abc_g1, 0);
    <b>while</b> (i &lt; n) {
        <b>let</b> cur_scalar = std::vector::borrow(public_inputs, i);
        <b>let</b> cur_point = std::vector::borrow(&vk.gamma_abc_g1, i+1);
        acc = <a href="curves.md#0x1_curves_point_add">curves::point_add</a>(&acc, &<a href="curves.md#0x1_curves_point_mul">curves::point_mul</a>(cur_scalar, cur_point));
        i = i + 1;
    };

    <b>let</b> right_2 = <a href="curves.md#0x1_curves_pairing">curves::pairing</a>(&acc, &vk.gamma_g2);
    <b>let</b> right_3 = <a href="curves.md#0x1_curves_pairing">curves::pairing</a>(&proof.c, &vk.delta_g2);
    <b>let</b> right = <a href="curves.md#0x1_curves_point_add">curves::point_add</a>(&<a href="curves.md#0x1_curves_point_add">curves::point_add</a>(&right_1, &right_2), &right_3);
    <a href="curves.md#0x1_curves_point_eq">curves::point_eq</a>(&left, &right)
}
</code></pre>



</details>


[move-book]: https://move-language.github.io/move/introduction.html
