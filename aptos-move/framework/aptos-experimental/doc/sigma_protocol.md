
<a id="0x7_sigma_protocol"></a>

# Module `0x7::sigma_protocol`



-  [Constants](#@Constants_0)
-  [Function `verify`](#0x7_sigma_protocol_verify)


<pre><code></code></pre>



<a id="@Constants_0"></a>

## Constants


<a id="0x7_sigma_protocol_E_INTERNAL_INVARIANT_FAILED"></a>

One of our internal invariants was broken. There is likely a logical error in the code.


<pre><code><b>const</b> <a href="sigma_protocol.md#0x7_sigma_protocol_E_INTERNAL_INVARIANT_FAILED">E_INTERNAL_INVARIANT_FAILED</a>: u64 = 2;
</code></pre>



<a id="0x7_sigma_protocol_E_PROOF_COMMITMENT_WRONG_LEN"></a>

The length of the <code>A</code> field in <code>Proof</code> did NOT match the homomorphism's output length


<pre><code><b>const</b> <a href="sigma_protocol.md#0x7_sigma_protocol_E_PROOF_COMMITMENT_WRONG_LEN">E_PROOF_COMMITMENT_WRONG_LEN</a>: u64 = 1;
</code></pre>



<a id="0x7_sigma_protocol_verify"></a>

## Function `verify`

Verifies a ZK <code>proof</code> that the prover knows a witness $w$ such that $f(X) = \psi(w)$ where $X$ is the
statement in <code>stmt</code>.

Optimized to perform a faster batched verification:
A + e f(X) - \psi(\sigma) = zero()
<=>
\forall i \in[m], A[i] + e f(X)[i] - \psi(\sigma)[i] = 0
<=>
\sum_{i \in [m]} \beta[i] A[i] + \beta[i] ( e f(X)[i] ) - \beta[i] ( \psi(\sigma)[i] ) = 0,
for random \beta[i]'s (picked via Fiat-Shamir)

Note: I don't think picking $\beta_i$'s via on-chain randomness will save that much gas. Plus, we do not want to
premise the security of confidential assets on the unpredictability of on-chain randomness.

@param  dst    application-specific domain separator
(e.g., "Aptos confidential assets protocol v2025.06 :: public withdrawal NP relation")

@param  psi    a homomorphism mapping a vector of scalars to a vector of $m$ group elements, except each group
element is returned as a <code>Representation</code> so that, later on, the main $\psi(\sigma) = A + e f(X)$
can be done efficiently in one MSM.

@param  f      transformation function takes takes in the public statement and outputs $m$ group elements, also
returned as a <code>RepresentationVec</code>.

@param  stmt   the public statement $X$ that satisfies $f(X) = \psi(w)$ for some secret witness $w$

@param  proof  the ZKP proving that the prover knows a $w$ s.t. $f(X) = \psi(w)$

Returns true if it succeeds and false otherwise.


<pre><code><b>public</b> <b>fun</b> <a href="sigma_protocol.md#0x7_sigma_protocol_verify">verify</a>(dst: <a href="sigma_protocol_fiat_shamir.md#0x7_sigma_protocol_fiat_shamir_DomainSeparator">sigma_protocol_fiat_shamir::DomainSeparator</a>, psi: <a href="sigma_protocol_homomorphism.md#0x7_sigma_protocol_homomorphism_Homomorphism">sigma_protocol_homomorphism::Homomorphism</a>, f: <a href="sigma_protocol_homomorphism.md#0x7_sigma_protocol_homomorphism_TransformationFunction">sigma_protocol_homomorphism::TransformationFunction</a>, stmt: &<a href="sigma_protocol_statement.md#0x7_sigma_protocol_statement_Statement">sigma_protocol_statement::Statement</a>, proof: &<a href="sigma_protocol_proof.md#0x7_sigma_protocol_proof_Proof">sigma_protocol_proof::Proof</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="sigma_protocol.md#0x7_sigma_protocol_verify">verify</a>(
    dst: DomainSeparator,
    psi: Homomorphism,
    f: TransformationFunction,
    stmt: &Statement,
    proof: &Proof,
): bool {
    // Step 1: Fiat-Shamir transform on `(dst, (psi, f), stmt)` <b>to</b> derive the random challenge `e`
    <b>let</b> _A = proof.get_commitment();
    <b>let</b> m = _A.length();
    <b>let</b> (e, betas) = fiat_shamir(dst, stmt, proof.get_compressed_commitment(), proof.get_response_length());

    // Step 2:
    <b>let</b> psi_sigma = psi(stmt, &proof.response_to_witness());
    <b>let</b> efx = f(stmt);

    <b>assert</b>!(m == psi_sigma.length(), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="sigma_protocol.md#0x7_sigma_protocol_E_PROOF_COMMITMENT_WRONG_LEN">E_PROOF_COMMITMENT_WRONG_LEN</a>));
    <b>assert</b>!(m == efx.length(), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="sigma_protocol.md#0x7_sigma_protocol_E_PROOF_COMMITMENT_WRONG_LEN">E_PROOF_COMMITMENT_WRONG_LEN</a>));

    // "Scale" all the representations in `f(stmt)` by `e`. (Implicit assumption here is that `f` is homomorphic:
    // i.e., `e f(X) = f(eX)`, which holds because our `f`'s are a `RepresentationVec`.)
    efx.scale_all(&e);

    // "Scale" the `i`th reprentation in `efx` by `\beta[i]`
    efx.scale_each(&betas);

    // "Scale" the `i`th reprentation in `\psi` by `-\beta[i]`
    // TODO(Perf): I think this could be sub-optimal: we will redo the same \beta[i] \sigma[j] multiplication several times
    //   when a `RepresentationVec`'s row reuses \sigma[j].
    psi_sigma.scale_each(&neg_scalars(&betas));

    // We start <b>with</b> an empty MSM: \sum_{i \in m} 0
    // ...and extend it <b>to</b>: \sum_{i \in [m]} A[i]^{\beta[i]}
    //                                          ^^^^^^^^^^^^^^^
    <b>let</b> bases = points_clone(_A);
    <b>let</b> scalars = betas;

    // These asserts will only fail when we have mis-implemented the cloning of `A` above
    <b>assert</b>!(bases.length() == m, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_internal">error::internal</a>(<a href="sigma_protocol.md#0x7_sigma_protocol_E_INTERNAL_INVARIANT_FAILED">E_INTERNAL_INVARIANT_FAILED</a>));
    <b>assert</b>!(scalars.length() == m, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_internal">error::internal</a>(<a href="sigma_protocol.md#0x7_sigma_protocol_E_INTERNAL_INVARIANT_FAILED">E_INTERNAL_INVARIANT_FAILED</a>));

    // Extend MSM <b>to</b>: be \sum_{i \in [m]} A[i]^\beta[i] + \beta[i] ( e f(stmt)[i] )
    //                                                    ^^^^^^^^^^^^^^^^^^^^^^^^^
    efx.for_each_ref(|repr| {
        bases.append(repr.to_points(stmt));
        scalars.append(*repr.get_scalars());
    });

    // Extend MSM <b>to</b>: be \sum_{i \in [m]} A[i]^\beta[i] + \beta[i] ( e f(stmt)[i] ) - \beta[i] (\psi(\sigma)[i])
    //                                                                                ^^^^^^^^^^^^^^^^^^^^^^^^^^
    psi_sigma.for_each_ref(|repr| {
        bases.append(repr.to_points(stmt));
        scalars.append(*repr.get_scalars());
    });

    // TODO(Perf): Could combine exponents for shared bases more aggresively? Or does the MSM <a href="../../aptos-framework/doc/code.md#0x1_code">code</a> do it implicitly?

    // Do the MSM and check it equals the (zero) identity
    point_equals(&multi_scalar_mul(&bases, &scalars), &point_identity())
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
