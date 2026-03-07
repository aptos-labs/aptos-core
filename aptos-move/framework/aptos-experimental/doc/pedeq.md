
<a id="0x7_pedeq"></a>

# Module `0x7::pedeq`

A ZKPoK of $m, r_1, r_2$ such that $C_1 = m G + r_1 H$ and $C_2 = m G + r_2 H$.

The NP relation is:

R(G, H, C_1, C_2;
m, r_1, r_2)    =?= 1   <=>   {  C_1 =?= m G + r_1 H  } AND
{  C_2 =?= m G + r_2 H  }

This can be framed as a homomorphism check:

\psi(m, r_1, r_2)   =?=    f(G, H, C_1, C_2)

where:

1. The homomorphism $\psi$ is

\psi(m, r_1, r_2) := [
m G + r_1 H,
m G + r_2 H
]

2. The transformation function $f$ is:

f(G, H, C_1, C_2) := [
C_1,
C_2
]
^^^^^^^^^^^^^^
|
stmt.points


-  [Constants](#@Constants_0)
-  [Function `new_session`](#0x7_pedeq_new_session)
-  [Function `new_pedeq_statement`](#0x7_pedeq_new_pedeq_statement)
-  [Function `new_pedeq_witness`](#0x7_pedeq_new_pedeq_witness)
-  [Function `psi`](#0x7_pedeq_psi)
-  [Function `f`](#0x7_pedeq_f)


<pre><code><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255">0x1::ristretto255</a>;
<b>use</b> <a href="fiat_shamir.md#0x7_fiat_shamir">0x7::fiat_shamir</a>;
<b>use</b> <a href="public_statement.md#0x7_public_statement">0x7::public_statement</a>;
<b>use</b> <a href="representation.md#0x7_representation">0x7::representation</a>;
<b>use</b> <a href="representation_vec.md#0x7_representation_vec">0x7::representation_vec</a>;
<b>use</b> <a href="secret_witness.md#0x7_secret_witness">0x7::secret_witness</a>;
</code></pre>



<a id="@Constants_0"></a>

## Constants


<a id="0x7_pedeq_E_WRONG_K"></a>

The expected number of scalars $k$ in a PedEq witness is 3.


<pre><code><b>const</b> <a href="pedeq.md#0x7_pedeq_E_WRONG_K">E_WRONG_K</a>: u64 = 3;
</code></pre>



<a id="0x7_pedeq_E_WRONG_M"></a>

The expected number of points $m$ in the image of the PedEq homomorphism and transformation function is 2.


<pre><code><b>const</b> <a href="pedeq.md#0x7_pedeq_E_WRONG_M">E_WRONG_M</a>: u64 = 4;
</code></pre>



<a id="0x7_pedeq_E_WRONG_N_1"></a>

The expected number of points $n_1$ in a PedEq statement is 4.


<pre><code><b>const</b> <a href="pedeq.md#0x7_pedeq_E_WRONG_N_1">E_WRONG_N_1</a>: u64 = 1;
</code></pre>



<a id="0x7_pedeq_E_WRONG_N_2"></a>

The expected number of scalars $n_2$ in a PedEq statement is 0.


<pre><code><b>const</b> <a href="pedeq.md#0x7_pedeq_E_WRONG_N_2">E_WRONG_N_2</a>: u64 = 2;
</code></pre>



<a id="0x7_pedeq_IDX_C_1"></a>

Index of $C_1$ in the <code>PublicStatement::points</code> vector.


<pre><code><b>const</b> <a href="pedeq.md#0x7_pedeq_IDX_C_1">IDX_C_1</a>: u64 = 2;
</code></pre>



<a id="0x7_pedeq_IDX_C_2"></a>

Index of $C_2$ in the <code>PublicStatement::points</code> vector.


<pre><code><b>const</b> <a href="pedeq.md#0x7_pedeq_IDX_C_2">IDX_C_2</a>: u64 = 3;
</code></pre>



<a id="0x7_pedeq_IDX_G"></a>

Index of $G$ in the <code>PublicStatement::points</code> vector.


<pre><code><b>const</b> <a href="pedeq.md#0x7_pedeq_IDX_G">IDX_G</a>: u64 = 0;
</code></pre>



<a id="0x7_pedeq_IDX_H"></a>

Index of $H$ in the <code>PublicStatement::points</code> vector.


<pre><code><b>const</b> <a href="pedeq.md#0x7_pedeq_IDX_H">IDX_H</a>: u64 = 1;
</code></pre>



<a id="0x7_pedeq_IDX_m"></a>

Index of $m$ in the <code>SecretWitness::w</code> vector.


<pre><code><b>const</b> <a href="pedeq.md#0x7_pedeq_IDX_m">IDX_m</a>: u64 = 0;
</code></pre>



<a id="0x7_pedeq_IDX_r_1"></a>

Index of $r_1$ in the <code>SecretWitness::w</code> vector.


<pre><code><b>const</b> <a href="pedeq.md#0x7_pedeq_IDX_r_1">IDX_r_1</a>: u64 = 1;
</code></pre>



<a id="0x7_pedeq_IDX_r_2"></a>

Index of $r_2$ in the <code>SecretWitness::w</code> vector.


<pre><code><b>const</b> <a href="pedeq.md#0x7_pedeq_IDX_r_2">IDX_r_2</a>: u64 = 2;
</code></pre>



<a id="0x7_pedeq_PROTOCOL_ID"></a>

Protocol ID used for domain separation


<pre><code><b>const</b> <a href="pedeq.md#0x7_pedeq_PROTOCOL_ID">PROTOCOL_ID</a>: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [77, 121, 32, 80, 101, 100, 69, 113, 32, 116, 101, 115, 116, 32, 99, 97, 115, 101, 32, 97, 112, 112];
</code></pre>



<a id="0x7_pedeq_new_session"></a>

## Function `new_session`



<pre><code><b>fun</b> <a href="pedeq.md#0x7_pedeq_new_session">new_session</a>(session_id: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="fiat_shamir.md#0x7_fiat_shamir_DomainSeparator">fiat_shamir::DomainSeparator</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="pedeq.md#0x7_pedeq_new_session">new_session</a>(session_id: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): DomainSeparator {
    new_domain_separator(<a href="pedeq.md#0x7_pedeq_PROTOCOL_ID">PROTOCOL_ID</a>, session_id)
}
</code></pre>



</details>

<a id="0x7_pedeq_new_pedeq_statement"></a>

## Function `new_pedeq_statement`

Creates a new PedEq statement.


<pre><code><b>fun</b> <a href="pedeq.md#0x7_pedeq_new_pedeq_statement">new_pedeq_statement</a>(_G: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>, _H: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>, _C_1: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>, _C_2: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>): <a href="public_statement.md#0x7_public_statement_PublicStatement">public_statement::PublicStatement</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="pedeq.md#0x7_pedeq_new_pedeq_statement">new_pedeq_statement</a>(_G: RistrettoPoint, _H: RistrettoPoint,
                        _C_1: RistrettoPoint, _C_2: RistrettoPoint): PublicStatement {
    new_public_statement(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[_G, _H, _C_1, _C_2], <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[])
}
</code></pre>



</details>

<a id="0x7_pedeq_new_pedeq_witness"></a>

## Function `new_pedeq_witness`

Creates a new PedEq witness.


<pre><code><b>fun</b> <a href="pedeq.md#0x7_pedeq_new_pedeq_witness">new_pedeq_witness</a>(m: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>, r_1: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>, r_2: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>): <a href="secret_witness.md#0x7_secret_witness_SecretWitness">secret_witness::SecretWitness</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="pedeq.md#0x7_pedeq_new_pedeq_witness">new_pedeq_witness</a>(m: Scalar, r_1: Scalar, r_2: Scalar): SecretWitness {
    new_secret_witness(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[m, r_1, r_2])
}
</code></pre>



</details>

<a id="0x7_pedeq_psi"></a>

## Function `psi`

Note: It is good practice to assert your statement, your witness and the homomorphism's output have the right
sizes.

For the PedEq relation, $n_1, n_2, k, m$ are constants. But it is possible to implement relation "families"
which take a variable number of inputs (e.g., imagine this PedEq generalized to $n$ commitments).


<pre><code><b>fun</b> <a href="pedeq.md#0x7_pedeq_psi">psi</a>(_stmt: &<a href="public_statement.md#0x7_public_statement_PublicStatement">public_statement::PublicStatement</a>, w: &<a href="secret_witness.md#0x7_secret_witness_SecretWitness">secret_witness::SecretWitness</a>): <a href="representation_vec.md#0x7_representation_vec_RepresentationVec">representation_vec::RepresentationVec</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="pedeq.md#0x7_pedeq_psi">psi</a>(_stmt: &PublicStatement, w: &SecretWitness): RepresentationVec {
    <b>assert</b>!(_stmt.get_points().length() == 4, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="pedeq.md#0x7_pedeq_E_WRONG_N_1">E_WRONG_N_1</a>));
    <b>assert</b>!(_stmt.get_scalars().length() == 0, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="pedeq.md#0x7_pedeq_E_WRONG_N_2">E_WRONG_N_2</a>));
    <b>assert</b>!(w.length() == 3, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="pedeq.md#0x7_pedeq_E_WRONG_K">E_WRONG_K</a>));

    // [
    //   m G + r_1 H,
    //   m G + r_2 H
    // ]
    <b>let</b> repr_vec = new_representation_vec(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[
        new_representation(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[<a href="pedeq.md#0x7_pedeq_IDX_G">IDX_G</a>, <a href="pedeq.md#0x7_pedeq_IDX_H">IDX_H</a>], <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[*w.get(<a href="pedeq.md#0x7_pedeq_IDX_m">IDX_m</a>), *w.get(<a href="pedeq.md#0x7_pedeq_IDX_r_1">IDX_r_1</a>)]),
        new_representation(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[<a href="pedeq.md#0x7_pedeq_IDX_G">IDX_G</a>, <a href="pedeq.md#0x7_pedeq_IDX_H">IDX_H</a>], <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[*w.get(<a href="pedeq.md#0x7_pedeq_IDX_m">IDX_m</a>), *w.get(<a href="pedeq.md#0x7_pedeq_IDX_r_2">IDX_r_2</a>)]),
    ]);

    <b>assert</b>!(repr_vec.length() == 2, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="pedeq.md#0x7_pedeq_E_WRONG_M">E_WRONG_M</a>));

    repr_vec
}
</code></pre>



</details>

<a id="0x7_pedeq_f"></a>

## Function `f`

Note: It is good practice to assert your transformation function's output has the right # of group elements.


<pre><code><b>fun</b> <a href="pedeq.md#0x7_pedeq_f">f</a>(_stmt: &<a href="public_statement.md#0x7_public_statement_PublicStatement">public_statement::PublicStatement</a>): <a href="representation_vec.md#0x7_representation_vec_RepresentationVec">representation_vec::RepresentationVec</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="pedeq.md#0x7_pedeq_f">f</a>(_stmt: &PublicStatement): RepresentationVec {
    // [
    //   C_1,
    //   C_2
    // ]
    <b>let</b> repr_vec = new_representation_vec(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[
        new_representation(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[<a href="pedeq.md#0x7_pedeq_IDX_C_1">IDX_C_1</a>], <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[scalar_one()]),
        new_representation(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[<a href="pedeq.md#0x7_pedeq_IDX_C_2">IDX_C_2</a>], <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[scalar_one()])
    ]);

    <b>assert</b>!(repr_vec.length() == 2, <a href="pedeq.md#0x7_pedeq_E_WRONG_M">E_WRONG_M</a>);

    repr_vec
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
