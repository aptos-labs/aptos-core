
<a id="0x7_sigma_protocol_homomorphism"></a>

# Module `0x7::sigma_protocol_homomorphism`

This module can be used to build $\Sigma$-protocols for proving knowledge of a pre-image on a homomorphism $\psi$.

Let $\mathbb{G}^{n_1} \times \mathbb{F}^{n_2}$ denote the set of public statements.

This module helps you convince a verifier with $X\in S$ that you know a secret $w\in \mathbb{F}^k$ such that
$\psi(w) = f(X)$, where:

$\psi : \mathbb{F}^k \rightarrow \mathbb{G}^m$ is a *homomorphism*, and
$f : \mathbb{G}^{n_1} \times \mathbb{F}^{n_2} \rightarrow \mathbb{G}^m$ is a *transformation function*.

Many useful statements can be proved in ZK by framing them as proving knowledge of a pre-image on a homomorphism:

e.g., a Schnorr signature is just proving knowledge of $x$ such that $\psi(x) = x G$, where the PK is $x G$.

e.g., a proof that $C_1, C_2$ both Pedersen-commit to the same $m$ is proving knowledge of $(m, r_1, r_2)$ s.t.
$\psi(m, r_1, r_2) = (m G + r_1 H, m G + r_2 H)$

The sigma protocol is very simple:

+ ------------------  +                                         + ------------------------------------------------ +
| Prover has $(X, w)$ |                                         | Verifier has                                     |
+ ------------------  +                                         | $X \in \mathbb{G}^{n_1} \times \mathbb{F}^{n_2}$ |
+ ------------------------------------------------ +
1. Sample $\alpha \in \mathbb{F}^k
2. Compute *commitment* $A \gets \psi(\alpha)$

3. send commitment $A$
------------------------------->

4. Assert $A \in \mathbb{G}^m$
5. Pick *random challenge* $e$
(via Fiat-Shamir on: $(X, A)$ a protocol
identifier and a session identifier)
6. send challenge $e$
<-------------------------------

7. Compute response $\sigma = \alpha + e \cdot w$

8. send response $\sigma$
------------------------------->

9. Check $\psi(\sigma) = A + e f(X)$


-  [Struct `TransformationFunction`](#0x7_sigma_protocol_homomorphism_TransformationFunction)
-  [Struct `Homomorphism`](#0x7_sigma_protocol_homomorphism_Homomorphism)
-  [Function `evaluate_psi`](#0x7_sigma_protocol_homomorphism_evaluate_psi)
-  [Function `evaluate_f`](#0x7_sigma_protocol_homomorphism_evaluate_f)


<pre><code><b>use</b> <a href="sigma_protocol_representation_vec.md#0x7_sigma_protocol_representation_vec">0x7::sigma_protocol_representation_vec</a>;
<b>use</b> <a href="sigma_protocol_statement.md#0x7_sigma_protocol_statement">0x7::sigma_protocol_statement</a>;
<b>use</b> <a href="sigma_protocol_witness.md#0x7_sigma_protocol_witness">0x7::sigma_protocol_witness</a>;
</code></pre>



<a id="0x7_sigma_protocol_homomorphism_TransformationFunction"></a>

## Struct `TransformationFunction`

The transformation function  $f : \mathbb{G}^{n_1} \times \mathbb{F}^{n_2} \rightarrow \mathbb{G}^m$


<pre><code><b>struct</b> <a href="sigma_protocol_homomorphism.md#0x7_sigma_protocol_homomorphism_TransformationFunction">TransformationFunction</a>
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>0: |&<a href="sigma_protocol_statement.md#0x7_sigma_protocol_statement_Statement">sigma_protocol_statement::Statement</a>|<a href="sigma_protocol_representation_vec.md#0x7_sigma_protocol_representation_vec_RepresentationVec">sigma_protocol_representation_vec::RepresentationVec</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x7_sigma_protocol_homomorphism_Homomorphism"></a>

## Struct `Homomorphism`

The homomorphism $\psi : \mathbb{F}^k \rightarrow \mathbb{G}^m$


<pre><code><b>struct</b> <a href="sigma_protocol_homomorphism.md#0x7_sigma_protocol_homomorphism_Homomorphism">Homomorphism</a>
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>0: |&<a href="sigma_protocol_statement.md#0x7_sigma_protocol_statement_Statement">sigma_protocol_statement::Statement</a>, &<a href="sigma_protocol_witness.md#0x7_sigma_protocol_witness_Witness">sigma_protocol_witness::Witness</a>|<a href="sigma_protocol_representation_vec.md#0x7_sigma_protocol_representation_vec_RepresentationVec">sigma_protocol_representation_vec::RepresentationVec</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x7_sigma_protocol_homomorphism_evaluate_psi"></a>

## Function `evaluate_psi`

Computes and returns $\psi(X, w) \in \mathbb{G}^m$ given the public statement $X$ and the secret witness $w$.


<pre><code><b>public</b> <b>fun</b> <a href="sigma_protocol_homomorphism.md#0x7_sigma_protocol_homomorphism_evaluate_psi">evaluate_psi</a>(psi: <a href="sigma_protocol_homomorphism.md#0x7_sigma_protocol_homomorphism_Homomorphism">sigma_protocol_homomorphism::Homomorphism</a>, stmt: &<a href="sigma_protocol_statement.md#0x7_sigma_protocol_statement_Statement">sigma_protocol_statement::Statement</a>, witn: &<a href="sigma_protocol_witness.md#0x7_sigma_protocol_witness_Witness">sigma_protocol_witness::Witness</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">0x1::ristretto255::RistrettoPoint</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="sigma_protocol_homomorphism.md#0x7_sigma_protocol_homomorphism_evaluate_psi">evaluate_psi</a>(psi: <a href="sigma_protocol_homomorphism.md#0x7_sigma_protocol_homomorphism_Homomorphism">Homomorphism</a>,
                               stmt: &Statement,
                               witn: &Witness): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;RistrettoPoint&gt; {
    psi(stmt, witn).map_ref(|repr| multi_scalar_mul(&repr.to_points(stmt), repr.get_scalars()))
}
</code></pre>



</details>

<a id="0x7_sigma_protocol_homomorphism_evaluate_f"></a>

## Function `evaluate_f`

Returns $f(X) \in \mathbb{G}^m$ given the public statement $X$.


<pre><code><b>public</b> <b>fun</b> <a href="sigma_protocol_homomorphism.md#0x7_sigma_protocol_homomorphism_evaluate_f">evaluate_f</a>(f: <a href="sigma_protocol_homomorphism.md#0x7_sigma_protocol_homomorphism_TransformationFunction">sigma_protocol_homomorphism::TransformationFunction</a>, stmt: &<a href="sigma_protocol_statement.md#0x7_sigma_protocol_statement_Statement">sigma_protocol_statement::Statement</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">0x1::ristretto255::RistrettoPoint</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="sigma_protocol_homomorphism.md#0x7_sigma_protocol_homomorphism_evaluate_f">evaluate_f</a>(f: <a href="sigma_protocol_homomorphism.md#0x7_sigma_protocol_homomorphism_TransformationFunction">TransformationFunction</a>,
                             stmt: &Statement): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;RistrettoPoint&gt; {
    f(stmt).map_ref(|repr| multi_scalar_mul(&repr.to_points(stmt), repr.get_scalars()))
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
