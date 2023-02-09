
<a name="0x1_groth16"></a>

# Module `0x1::groth16`



-  [Struct `VerifyingKey`](#0x1_groth16_VerifyingKey)
-  [Struct `PreparedVerifyingKey`](#0x1_groth16_PreparedVerifyingKey)
-  [Struct `Proof`](#0x1_groth16_Proof)
-  [Function `new_vk`](#0x1_groth16_new_vk)
-  [Function `new_pvk`](#0x1_groth16_new_pvk)
-  [Function `prepare_verifying_key`](#0x1_groth16_prepare_verifying_key)
-  [Function `new_proof`](#0x1_groth16_new_proof)
-  [Function `triplet`](#0x1_groth16_triplet)


<pre><code><b>use</b> <a href="algebra.md#0x1_algebra">0x1::algebra</a>;
</code></pre>



<a name="0x1_groth16_VerifyingKey"></a>

## Struct `VerifyingKey`

A Groth16 verifying key.


<pre><code><b>struct</b> <a href="groth16.md#0x1_groth16_VerifyingKey">VerifyingKey</a>&lt;G1, G2, Gt&gt; <b>has</b> drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>alpha_g1: <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G1&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>beta_g2: <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G2&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>gamma_g2: <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G2&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>delta_g2: <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G2&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>gamma_abc_g1: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G1&gt;&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_groth16_PreparedVerifyingKey"></a>

## Struct `PreparedVerifyingKey`

A Groth16 verifying key pre-processed for faster verification.


<pre><code><b>struct</b> <a href="groth16.md#0x1_groth16_PreparedVerifyingKey">PreparedVerifyingKey</a>&lt;G1, G2, Gt&gt; <b>has</b> drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>alpha_g1_beta_g2: <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;Gt&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>gamma_g2_neg: <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G2&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>delta_g2_neg: <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G2&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>gamma_abc_g1: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G1&gt;&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_groth16_Proof"></a>

## Struct `Proof`

A Groth16 proof.


<pre><code><b>struct</b> <a href="groth16.md#0x1_groth16_Proof">Proof</a>&lt;G1, G2, Gt&gt; <b>has</b> drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>a: <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G1&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>b: <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G2&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>c: <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G1&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_groth16_new_vk"></a>

## Function `new_vk`

Create a new Groth16 verifying key.


<pre><code><b>public</b> <b>fun</b> <a href="groth16.md#0x1_groth16_new_vk">new_vk</a>&lt;G1, G2, Gt&gt;(alpha_g1: <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G1&gt;, beta_g2: <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G2&gt;, gamma_g2: <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G2&gt;, delta_g2: <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G2&gt;, gamma_abc_g1: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G1&gt;&gt;): <a href="groth16.md#0x1_groth16_VerifyingKey">groth16::VerifyingKey</a>&lt;G1, G2, Gt&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="groth16.md#0x1_groth16_new_vk">new_vk</a>&lt;G1,G2,Gt&gt;(alpha_g1: <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G1&gt;, beta_g2: <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G2&gt;, gamma_g2: <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G2&gt;, delta_g2: <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G2&gt;, gamma_abc_g1: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G1&gt;&gt;): <a href="groth16.md#0x1_groth16_VerifyingKey">VerifyingKey</a>&lt;G1,G2,Gt&gt; {
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

<a name="0x1_groth16_new_pvk"></a>

## Function `new_pvk`

Create a new pre-processed Groth16 verifying key.


<pre><code><b>public</b> <b>fun</b> <a href="groth16.md#0x1_groth16_new_pvk">new_pvk</a>&lt;G1, G2, Gt&gt;(alpha_g1_beta_g2: <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;Gt&gt;, gamma_g2_neg: <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G2&gt;, delta_g2_neg: <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G2&gt;, gamma_abc_g1: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G1&gt;&gt;): <a href="groth16.md#0x1_groth16_PreparedVerifyingKey">groth16::PreparedVerifyingKey</a>&lt;G1, G2, Gt&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="groth16.md#0x1_groth16_new_pvk">new_pvk</a>&lt;G1,G2,Gt&gt;(alpha_g1_beta_g2: <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;Gt&gt;, gamma_g2_neg: <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G2&gt;, delta_g2_neg: <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G2&gt;, gamma_abc_g1: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G1&gt;&gt;): <a href="groth16.md#0x1_groth16_PreparedVerifyingKey">PreparedVerifyingKey</a>&lt;G1,G2,Gt&gt; {
    <a href="groth16.md#0x1_groth16_PreparedVerifyingKey">PreparedVerifyingKey</a> {
        alpha_g1_beta_g2,
        gamma_g2_neg,
        delta_g2_neg,
        gamma_abc_g1,
    }
}
</code></pre>



</details>

<a name="0x1_groth16_prepare_verifying_key"></a>

## Function `prepare_verifying_key`

Pre-process a Groth16 verification key <code>vk</code> for faster verification.


<pre><code><b>public</b> <b>fun</b> <a href="groth16.md#0x1_groth16_prepare_verifying_key">prepare_verifying_key</a>&lt;G1, G2, Gt&gt;(vk: &<a href="groth16.md#0x1_groth16_VerifyingKey">groth16::VerifyingKey</a>&lt;G1, G2, Gt&gt;): <a href="groth16.md#0x1_groth16_PreparedVerifyingKey">groth16::PreparedVerifyingKey</a>&lt;G1, G2, Gt&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="groth16.md#0x1_groth16_prepare_verifying_key">prepare_verifying_key</a>&lt;G1,G2,Gt&gt;(vk: &<a href="groth16.md#0x1_groth16_VerifyingKey">VerifyingKey</a>&lt;G1,G2,Gt&gt;): <a href="groth16.md#0x1_groth16_PreparedVerifyingKey">PreparedVerifyingKey</a>&lt;G1,G2,Gt&gt; {
    <a href="groth16.md#0x1_groth16_PreparedVerifyingKey">PreparedVerifyingKey</a> {
        alpha_g1_beta_g2: <a href="algebra.md#0x1_algebra_pairing">algebra::pairing</a>&lt;G1,G2,Gt&gt;(&vk.alpha_g1, &vk.beta_g2),
        gamma_g2_neg: <a href="algebra.md#0x1_algebra_group_neg">algebra::group_neg</a>(&vk.gamma_g2),
        delta_g2_neg: <a href="algebra.md#0x1_algebra_group_neg">algebra::group_neg</a>(&vk.delta_g2),
        gamma_abc_g1: vk.gamma_abc_g1,
    }
}
</code></pre>



</details>

<a name="0x1_groth16_new_proof"></a>

## Function `new_proof`

Create a Groth16 proof.


<pre><code><b>public</b> <b>fun</b> <a href="groth16.md#0x1_groth16_new_proof">new_proof</a>&lt;G1, G2, Gt&gt;(a: <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G1&gt;, b: <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G2&gt;, c: <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G1&gt;): <a href="groth16.md#0x1_groth16_Proof">groth16::Proof</a>&lt;G1, G2, Gt&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="groth16.md#0x1_groth16_new_proof">new_proof</a>&lt;G1,G2,Gt&gt;(a: <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G1&gt;, b: <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G2&gt;, c: <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G1&gt;): <a href="groth16.md#0x1_groth16_Proof">Proof</a>&lt;G1,G2,Gt&gt; {
    <a href="groth16.md#0x1_groth16_Proof">Proof</a> { a, b, c }
}
</code></pre>



</details>

<a name="0x1_groth16_triplet"></a>

## Function `triplet`



<pre><code><b>fun</b> <a href="groth16.md#0x1_groth16_triplet">triplet</a>&lt;T&gt;(a: T, b: T, c: T): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;T&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="groth16.md#0x1_groth16_triplet">triplet</a>&lt;T&gt;(a: T, b: T, c: T): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;T&gt; {
    <b>let</b> ret = std::vector::empty();
    std::vector::push_back(&<b>mut</b> ret, a);
    std::vector::push_back(&<b>mut</b> ret, b);
    std::vector::push_back(&<b>mut</b> ret, c);
    ret
}
</code></pre>



</details>


[move-book]: https://move-language.github.io/move/introduction.html
