module sigma_protocols::sigma_protocol {
    use std::error;
    use aptos_std::ristretto255::{point_identity, multi_scalar_mul, point_equals};
    use sigma_protocols::utils::{neg_scalars, points_clone};
    use sigma_protocols::proof::Proof;
    use sigma_protocols::representation_vec::RepresentationVec;
    use sigma_protocols::secret_witness::SecretWitness;
    use sigma_protocols::public_statement::PublicStatement;
    use sigma_protocols::fiat_shamir::{DomainSeparator, fiat_shamir};
    #[test_only]
    use sigma_protocols::homomorphism::{evaluate_f, evaluate_homomorphism, Homomorphism};
    #[test_only]
    use sigma_protocols::proof;
    #[test_only]
    use sigma_protocols::secret_witness;
    #[test_only]
    use sigma_protocols::utils::{equal_vec_points, add_vec_points, mul_points, mul_scalars, add_vec_scalars};

    //
    // Error codes
    //

    /// The length of the `A` field in `Proof` did NOT match the homomorphism's output length
    const E_PROOF_COMMITMENT_WRONG_LEN: u64 = 1;
    /// One of our internal invariants was broken. There is likely a logical error in the code.
    const E_INTERNAL_INVARIANT_FAILED: u64 = 2;

    #[test_only]
    /// Creates a proof and additionally returns the randomness $\alpha \in \mathbb{F}^k$ used to
    /// create the sigma protocol commitment $A = \psi(\alpha) \in \mathbb{G}^m$.
    public inline fun prove(
        dst: DomainSeparator,
        psi: Homomorphism,
        stmt: &PublicStatement,
        witn: &SecretWitness,
    ): (Proof, SecretWitness) {
        let k = witn.length();

        // Step 1: Pick a random \alpha \in \F^k
        let alpha = secret_witness::random(k);
        // debug::print(&string_utils::format1(&b"len(alpha) = k = {}", k));

        // Step 2: A <- \psi(\alpha) \in \Gr^m
        let _A = evaluate_homomorphism(|_X, w| psi(_X, w), stmt, &alpha);

        // Step 3: Derive a random-challenge `e` via Fiat-Shamir
        let (e, _) = fiat_shamir(dst, stmt, &_A, k);
        // debug::print(&string_utils::format1(&b"len(A) = m = {}", m));

        // Step 4: \sigma <- \alpha + e w
        let sigma = add_vec_scalars(
            alpha.get_scalars(),
            &mul_scalars(witn.get_scalars(), &e)
        );

        assert!(sigma.length() == k, error::internal(E_INTERNAL_INVARIANT_FAILED));

        (proof::new(_A, sigma), alpha)
    }

    #[test_only]
    /// This is implemented both as a "warm-up" *and* to test the faster `verify` implementation against it.
    ///
    /// Recall that, given a statement `stmt`, proof $(A, \sigma)$ and Fiat-Shamir challenge $e$, the verifier checks:
    ///   A + e f(stmt) = \psi(\sigma)
    ///         <=>
    ///   A + e f(stmt) - \psi(\sigma) = zero()
    ///         <=>
    ///   \forall i \in[m], A[i] + e f(stmt)[i] - \psi(\sigma)[i] = 0
    ///
    /// At a high level, this functions verifies the proof via:
    /// ```
    ///   vector_equals(
    ///      psi(stmt, &SecretWitness { w: proof.sigma }),
    ///      vector_add(
    ///           proof.A,
    ///           vector_scalar_mul(e, f(stmt))
    ///      );
    ///   );
    /// ```
    public inline fun verify_slow(
        dst: DomainSeparator,
        psi: |&PublicStatement, &SecretWitness|RepresentationVec,
        f: |&PublicStatement|RepresentationVec,
        stmt: &PublicStatement,
        proof: &Proof,
    ): bool {

        // Step 1: Fiat-Shamir transform on `(dst, (psi, f), stmt)` to derive the random challenge `e`
        let _A = proof.get_commitment();
        let m = _A.length();
        let sigma = proof.response_to_witness();
        let k = sigma.length();
        let (e, _) = fiat_shamir(dst, stmt, _A, k);

        // Step 3: Compute the `m` entries of `f(X)`
        let fx = evaluate_f(|_X| f(_X), stmt);
        assert!(fx.length() == m, error::invalid_argument(E_PROOF_COMMITMENT_WRONG_LEN));

        // Step 4: Compute the `m` entries of \psi(X, w)
        let psi_sigma = evaluate_homomorphism(|_X, w| psi(_X, w), stmt, &sigma);
        assert!(psi_sigma.length() == m, error::invalid_argument(E_PROOF_COMMITMENT_WRONG_LEN));

        equal_vec_points(
            &psi_sigma,
            &add_vec_points(
                _A,
                &mul_points(
                    &fx,
                    &e
                ),
            )
        )
    }

    /// Verifies a ZK `proof` that the prover knows a witness $w$ such that $f(X) = \psi(w)$ where $X$ is the
    /// statement in `stmt`.
    ///
    /// Optimized to perform a faster batched verification:
    ///   A + e f(X) - \psi(\sigma) = zero()
    ///         <=>
    ///   \forall i \in[m], A[i] + e f(X)[i] - \psi(\sigma)[i] = 0
    ///         <=>
    ///   \sum_{i \in [m]} \beta[i] A[i] + \beta[i] ( e f(X)[i] ) - \beta[i] ( \psi(\sigma)[i] ) = 0,
    ///   for random \beta[i]'s (picked via Fiat-Shamir)
    ///
    /// Note: I don't think picking $\beta_i$'s via on-chain randomness will save that much gas. Plus, we do not want to
    /// premise the security of confidential assets on the unpredictability of on-chain randomness.
    ///
    /// @param  dst    application-specific domain separator
    ///                (e.g., "Aptos confidential assets protocol v2025.06 :: public withdrawal NP relation")
    ///
    /// @param  psi    a homomorphism mapping a vector of scalars to a vector of $m$ group elements, except each group
    ///                element is returned as a `Representation` so that, later on, the main $\psi(\sigma) = A + e f(X)$
    ///                can be done efficiently in one MSM.
    ///
    /// @param  f      transformation function takes takes in the public statement and outputs $m$ group elements, also
    ///                returned as a `RepresentationVec`.
    ///
    /// @param  stmt   the public statement $X$ that satisfies $f(X) = \psi(w)$ for some secret witness $w$
    ///
    /// @param  proof  the ZKP proving that the prover knows a $w$ s.t. $f(X) = \psi(w)$
    ///
    /// Returns true if it succeeds and false otherwise.
    public inline fun verify(
        dst: DomainSeparator,
        psi: |&PublicStatement, &SecretWitness|RepresentationVec,
        f: |&PublicStatement|RepresentationVec,
        stmt: &PublicStatement,
        proof: &Proof,
    ): bool {
        // Step 1: Fiat-Shamir transform on `(dst, (psi, f), stmt)` to derive the random challenge `e`
        let _A = proof.get_commitment();
        let m = _A.length();
        let (e, betas) = fiat_shamir(dst, stmt, _A, proof.get_response_length());

        // Step 2:
        let psi_sigma = psi(stmt, &proof.response_to_witness());
        let efx = f(stmt);

        assert!(m == psi_sigma.length(), error::invalid_argument(E_PROOF_COMMITMENT_WRONG_LEN));
        assert!(m == efx.length(), error::invalid_argument(E_PROOF_COMMITMENT_WRONG_LEN));

        // "Scale" all the representations in `f(stmt)` by `e`. (Implicit assumption here is that `f` is homomorphic:
        // i.e., `e f(X) = f(eX)`, which holds because our `f`'s are a `RepresentationVec`.)
        efx.scale_all(&e);

        // "Scale" the `i`th reprentation in `efx` by `\beta[i]`
        efx.scale_each(&betas);

        // "Scale" the `i`th reprentation in `\psi` by `-\beta[i]`
        // TODO(Perf): I think this could be sub-optimal: we will redo the same \beta[i] \sigma[j] multiplication several times
        //   when a `RepresentationVec`'s row reuses \sigma[j].
        psi_sigma.scale_each(&neg_scalars(&betas));

        // We start with an empty MSM: \sum_{i \in m} 0
        // ...and extend it to: \sum_{i \in [m]} A[i]^{\beta[i]}
        //                                          ^^^^^^^^^^^^^^^
        let bases = points_clone(_A);
        let scalars = betas;

        // These asserts will only fail when we have mis-implemented the cloning of `A` above
        assert!(bases.length() == m, error::internal(E_INTERNAL_INVARIANT_FAILED));
        assert!(scalars.length() == m, error::internal(E_INTERNAL_INVARIANT_FAILED));

        // Extend MSM to: be \sum_{i \in [m]} A[i]^\beta[i] + \beta[i] ( e f(stmt)[i] )
        //                                                    ^^^^^^^^^^^^^^^^^^^^^^^^^
        efx.for_each_ref(|repr| {
            bases.append(repr.to_points(stmt));
            scalars.append(*repr.get_scalars());
        });

        // Extend MSM to: be \sum_{i \in [m]} A[i]^\beta[i] + \beta[i] ( e f(stmt)[i] ) - \beta[i] (\psi(\sigma)[i])
        //                                                                                ^^^^^^^^^^^^^^^^^^^^^^^^^^
        psi_sigma.for_each_ref(|repr| {
            bases.append(repr.to_points(stmt));
            scalars.append(*repr.get_scalars());
        });

        // TODO(Perf): Could combine exponents for shared bases more aggresively? Or does the MSM code do it implicitly?

        // Do the MSM and check it equals the (zero) identity
        point_equals(&multi_scalar_mul(&bases, &scalars), &point_identity())
    }

    //
    // Test-only error codes
    //

    #[test_only]
    /// The slow verification of the sigma protocol proof failed (instead of succeeding) in one of the tests.
    const E_SLOW_VERIFICATION_FAILED: u64 = 4;
    #[test_only]
    /// The fast verification of the sigma protocol proof failed (instead of succeeding) in one of the tests.
    const E_FAST_VERIFICATION_FAILED: u64 = 5;

    #[test_only]
    /// A generic correctness test that takes the DST, the public statement, the secret witness, and the $\psi$ and $f$
    /// lambdas.
    public inline fun assert_correctly_computed_proof_verifies(
        dst: DomainSeparator,
        stmt: PublicStatement,
        witn: SecretWitness,
        psi: |&PublicStatement, &SecretWitness|RepresentationVec,
        f: |&PublicStatement|RepresentationVec,
    ): (Proof, SecretWitness) {
        let (proof, alpha) = prove(
            dst,
            |_X, w| psi(_X, w),
            &stmt,
            &witn
        );

        // Make sure the sigma protocol proof verifies (slowly)
        assert!(
            verify_slow(
                dst,
                |_X, w| psi(_X, w),
                |_X| f(_X),
                &stmt,
                &proof
            ), error::invalid_argument(E_SLOW_VERIFICATION_FAILED));

        // Make sure the sigma protocol proof verifies (quickly)
        assert!(
            verify(
                dst,
                |_X, w| psi(_X, w),
                |_X| f(_X),
                &stmt,
                &proof
            ), error::invalid_argument(E_FAST_VERIFICATION_FAILED));

        (proof, alpha)
    }

    #[test_only]
    /// Returns `true` if the empty proof does not verify for the specific statement. Otherwise, returns `false`.
    public inline fun empty_proof_verifies(
        dst: DomainSeparator,
        psi: |&PublicStatement, &SecretWitness|RepresentationVec,
        f: |&PublicStatement|RepresentationVec,
        stmt: PublicStatement,
    ): bool {
        let proof = proof::empty();

        let r1 = !verify_slow(
            dst,
            |_X, w| psi(_X, w),
            |_X| f(_X),
            &stmt,
            &proof
        );

        let r2 = !verify(
            dst,
            |_X, w| psi(_X, w),
            |_X| f(_X),
            &stmt,
            &proof
        );

        r1 && r2
    }
}
