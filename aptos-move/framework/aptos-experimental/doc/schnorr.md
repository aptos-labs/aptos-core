
<a id="0x7_schnorr"></a>

# Module `0x7::schnorr`

A Schnorr ZKPoK of $x$ such that $Y = x G$.

The Schnorr NP relation is:

R(G, Y; x) =?= 1   <=>   Y =?= x G

This can be framed as a homomorphism check:

\psi(x)   =?=    f(G, Y)

where:

1. The homomorphism $\psi$ is

\psi(x) := [ x G ]

2. The transformation function $f$ is:

f(G, Y) := [ Y ]
^^^^
|
stmt.points


-  [Constants](#@Constants_0)
-  [Function `new_session`](#0x7_schnorr_new_session)
-  [Function `new_schnorr_statement`](#0x7_schnorr_new_schnorr_statement)
-  [Function `new_schnorr_witness`](#0x7_schnorr_new_schnorr_witness)
-  [Function `psi`](#0x7_schnorr_psi)
-  [Function `f`](#0x7_schnorr_f)


<pre><code><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255">0x1::ristretto255</a>;
<b>use</b> <a href="fiat_shamir.md#0x7_fiat_shamir">0x7::fiat_shamir</a>;
<b>use</b> <a href="public_statement.md#0x7_public_statement">0x7::public_statement</a>;
<b>use</b> <a href="representation.md#0x7_representation">0x7::representation</a>;
<b>use</b> <a href="representation_vec.md#0x7_representation_vec">0x7::representation_vec</a>;
<b>use</b> <a href="secret_witness.md#0x7_secret_witness">0x7::secret_witness</a>;
</code></pre>



<a id="@Constants_0"></a>

## Constants


<a id="0x7_schnorr_E_WRONG_K"></a>

The expected number of scalars $k$ in a PedEq witness is 3.


<pre><code><b>const</b> <a href="schnorr.md#0x7_schnorr_E_WRONG_K">E_WRONG_K</a>: u64 = 3;
</code></pre>



<a id="0x7_schnorr_E_WRONG_M"></a>

The expected number of points $m$ in the image of the PedEq homomorphism and transformation function is 2.


<pre><code><b>const</b> <a href="schnorr.md#0x7_schnorr_E_WRONG_M">E_WRONG_M</a>: u64 = 4;
</code></pre>



<a id="0x7_schnorr_E_WRONG_N_1"></a>

The expected number of points $n_1$ in a PedEq statement is 4.


<pre><code><b>const</b> <a href="schnorr.md#0x7_schnorr_E_WRONG_N_1">E_WRONG_N_1</a>: u64 = 1;
</code></pre>



<a id="0x7_schnorr_E_WRONG_N_2"></a>

The expected number of scalars $n_2$ in a PedEq statement is 0.


<pre><code><b>const</b> <a href="schnorr.md#0x7_schnorr_E_WRONG_N_2">E_WRONG_N_2</a>: u64 = 2;
</code></pre>



<a id="0x7_schnorr_IDX_G"></a>

Index of $G$ in the <code>PublicStatement::points</code> vector.


<pre><code><b>const</b> <a href="schnorr.md#0x7_schnorr_IDX_G">IDX_G</a>: u64 = 0;
</code></pre>



<a id="0x7_schnorr_PROTOCOL_ID"></a>

Protocol ID used for domain separation


<pre><code><b>const</b> <a href="schnorr.md#0x7_schnorr_PROTOCOL_ID">PROTOCOL_ID</a>: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [77, 121, 32, 83, 99, 104, 110, 111, 114, 114, 32, 116, 101, 115, 116, 32, 99, 97, 115, 101, 32, 97, 112, 112];
</code></pre>



<a id="0x7_schnorr_IDX_Y"></a>

Index of $Y$ in the <code>PublicStatement::points</code> vector.


<pre><code><b>const</b> <a href="schnorr.md#0x7_schnorr_IDX_Y">IDX_Y</a>: u64 = 1;
</code></pre>



<a id="0x7_schnorr_IDX_x"></a>

Index of $x$ in the <code>SecretWitness::w</code> vector.


<pre><code><b>const</b> <a href="schnorr.md#0x7_schnorr_IDX_x">IDX_x</a>: u64 = 0;
</code></pre>



<a id="0x7_schnorr_new_session"></a>

## Function `new_session`



<pre><code><b>fun</b> <a href="schnorr.md#0x7_schnorr_new_session">new_session</a>(session_id: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="fiat_shamir.md#0x7_fiat_shamir_DomainSeparator">fiat_shamir::DomainSeparator</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="schnorr.md#0x7_schnorr_new_session">new_session</a>(session_id: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): DomainSeparator {
    new_domain_separator(<a href="schnorr.md#0x7_schnorr_PROTOCOL_ID">PROTOCOL_ID</a>, session_id)
}
</code></pre>



</details>

<a id="0x7_schnorr_new_schnorr_statement"></a>

## Function `new_schnorr_statement`



<pre><code><b>fun</b> <a href="schnorr.md#0x7_schnorr_new_schnorr_statement">new_schnorr_statement</a>(_G: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>, _Y: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>): <a href="public_statement.md#0x7_public_statement_PublicStatement">public_statement::PublicStatement</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="schnorr.md#0x7_schnorr_new_schnorr_statement">new_schnorr_statement</a>(_G: RistrettoPoint, _Y: RistrettoPoint): PublicStatement {
    new_public_statement(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[_G, _Y], <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[])
}
</code></pre>



</details>

<a id="0x7_schnorr_new_schnorr_witness"></a>

## Function `new_schnorr_witness`



<pre><code><b>fun</b> <a href="schnorr.md#0x7_schnorr_new_schnorr_witness">new_schnorr_witness</a>(x: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>): <a href="secret_witness.md#0x7_secret_witness_SecretWitness">secret_witness::SecretWitness</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="schnorr.md#0x7_schnorr_new_schnorr_witness">new_schnorr_witness</a>(x: Scalar): SecretWitness {
    new_secret_witness(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[x])
}
</code></pre>



</details>

<a id="0x7_schnorr_psi"></a>

## Function `psi`



<pre><code><b>fun</b> <a href="schnorr.md#0x7_schnorr_psi">psi</a>(_stmt: &<a href="public_statement.md#0x7_public_statement_PublicStatement">public_statement::PublicStatement</a>, w: &<a href="secret_witness.md#0x7_secret_witness_SecretWitness">secret_witness::SecretWitness</a>): <a href="representation_vec.md#0x7_representation_vec_RepresentationVec">representation_vec::RepresentationVec</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="schnorr.md#0x7_schnorr_psi">psi</a>(_stmt: &PublicStatement, w: &SecretWitness): RepresentationVec {
    <b>assert</b>!(_stmt.get_points().length() == 2, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="schnorr.md#0x7_schnorr_E_WRONG_N_1">E_WRONG_N_1</a>));
    <b>assert</b>!(_stmt.get_scalars().length() == 0, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="schnorr.md#0x7_schnorr_E_WRONG_N_2">E_WRONG_N_2</a>));
    <b>assert</b>!(w.length() == 1, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="schnorr.md#0x7_schnorr_E_WRONG_K">E_WRONG_K</a>));
    // [
    //   x G
    // ]
    <b>let</b> repr_vec = new_representation_vec(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[
        new_representation(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[<a href="schnorr.md#0x7_schnorr_IDX_G">IDX_G</a>], <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[*w.get(<a href="schnorr.md#0x7_schnorr_IDX_x">IDX_x</a>)])
    ]);

    <b>assert</b>!(repr_vec.length() == 1, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="schnorr.md#0x7_schnorr_E_WRONG_M">E_WRONG_M</a>));

    repr_vec
}
</code></pre>



</details>

<a id="0x7_schnorr_f"></a>

## Function `f`



<pre><code><b>fun</b> <a href="schnorr.md#0x7_schnorr_f">f</a>(_stmt: &<a href="public_statement.md#0x7_public_statement_PublicStatement">public_statement::PublicStatement</a>): <a href="representation_vec.md#0x7_representation_vec_RepresentationVec">representation_vec::RepresentationVec</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="schnorr.md#0x7_schnorr_f">f</a>(_stmt: &PublicStatement): RepresentationVec {
    // [
    //   Y
    // ]
    <b>let</b> repr_vec = new_representation_vec(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[
        new_representation(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[<a href="schnorr.md#0x7_schnorr_IDX_Y">IDX_Y</a>], <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[scalar_one()])
    ]);

    <b>assert</b>!(repr_vec.length() == 1, <a href="schnorr.md#0x7_schnorr_E_WRONG_M">E_WRONG_M</a>);

    repr_vec
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
