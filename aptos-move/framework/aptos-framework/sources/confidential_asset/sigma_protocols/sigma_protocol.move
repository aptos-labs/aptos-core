module aptos_framework::sigma_protocol {
    friend aptos_framework::sigma_protocol_registration;
    friend aptos_framework::sigma_protocol_withdraw;
    friend aptos_framework::sigma_protocol_transfer;
    friend aptos_framework::sigma_protocol_key_rotation;
    #[test_only]
    friend aptos_framework::sigma_protocol_pedeq_example;
    #[test_only]
    friend aptos_framework::sigma_protocol_schnorr_example;

    use std::error;
    use std::vector;
    use aptos_std::ristretto255::{RistrettoPoint, Scalar};
    use aptos_framework::sigma_protocol_proof::Proof;
    use aptos_framework::sigma_protocol_statement::Statement;
    use aptos_framework::sigma_protocol_fiat_shamir::{DomainSeparator, fiat_shamir};
    use aptos_framework::sigma_protocol_homomorphism::{Homomorphism, TransformationFunction};
    use aptos_framework::sigma_protocol_homomorphism::{evaluate_f, evaluate_psi};
    #[test_only]
    use aptos_framework::sigma_protocol_proof;
    #[test_only]
    use aptos_framework::sigma_protocol_witness::Witness;

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
    public(friend) inline fun prove<P>(
        dst: DomainSeparator,
        psi: Homomorphism<P>,
        stmt: &Statement<P>,
        witn: &Witness,
    ): (Proof, Witness) {
        let k = witn.length();

        // Step 1: Pick a random \alpha \in \F^k
        let alpha = random_witness(k);

        // Step 2: A <- \psi(\alpha) \in \Gr^m
        let _A = evaluate_psi(|_X, w| psi(_X, w), stmt, &alpha);

        // Step 3: Derive a random-challenge `e` via Fiat-Shamir
        let compressed_A = compress_points(&_A);
        let (e, _) = fiat_shamir(dst, stmt, &compressed_A, &vector[], k);

        // Step 4: \sigma <- \alpha + e w
        let sigma = add_vec_scalars(
            alpha.get_scalars(),
            &mul_scalars(witn.get_scalars(), &e)
        );

        assert!(sigma.length() == k, error::internal(E_INTERNAL_INVARIANT_FAILED));

        (sigma_protocol_proof::new_proof(_A, compressed_A, sigma), alpha)
    }

    /// Verifies a ZK `proof` that the prover knows a witness $w$ such that $f(X) = \psi(w)$ where $X$ is the
    /// statement in `stmt`.
    ///
    /// Given a statement `stmt`, proof $(A, \sigma)$ and Fiat-Shamir challenge $e$, the verifier checks:
    ///   A + e f(stmt) = \psi(\sigma)
    ///         <=>
    ///   A + e f(stmt) - \psi(\sigma) = zero()
    ///         <=>
    ///   \forall i \in[m], A[i] + e f(stmt)[i] - \psi(\sigma)[i] = 0
    ///
    /// At a high level, this functions verifies the proof via:
    /// ```
    ///   vector_equals(
    ///      psi(stmt, &Witness { w: proof.sigma }),
    ///      vector_add(
    ///           proof.A,
    ///           vector_scalar_mul(e, f(stmt))
    ///      );
    ///   );
    /// ```
    public(friend) inline fun verify<P>(
        dst: DomainSeparator,
        psi: Homomorphism<P>,
        f: TransformationFunction<P>,
        stmt: &Statement<P>,
        proof: &Proof,
    ): bool {

        // Step 1: Fiat-Shamir transform on `(dst, (psi, f), stmt)` to derive the random challenge `e`
        let _A = proof.get_commitment();
        let sigma = proof.response_to_witness();
        let (e, _) = fiat_shamir(dst, stmt, proof.get_compressed_commitment(), &vector[], sigma.length());

        // Step 3: Compute the `m` entries of `f(X)`
        let fx = evaluate_f(|_X| f(_X), stmt);
        assert!(fx.length() == _A.length(), error::invalid_argument(E_PROOF_COMMITMENT_WRONG_LEN));

        // Step 4: Compute the `m` entries of \psi(X, w)
        let psi_sigma = evaluate_psi(|_X, w| psi(_X, w), stmt, &sigma);
        assert!(psi_sigma.length() == _A.length(), error::invalid_argument(E_PROOF_COMMITMENT_WRONG_LEN));

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

    //
    // Test-only error codes
    //

    #[test_only]
    /// Verification of the sigma protocol proof failed (instead of succeeding) in one of the tests.
    const E_VERIFICATION_FAILED: u64 = 3;

    #[test_only]
    /// A generic correctness test that takes the DST, the public statement, the secret witness, and the $\psi$ and $f$
    /// lambdas.
    public(friend) inline fun assert_correctly_computed_proof_verifies<P>(
        dst: DomainSeparator,
        stmt: Statement<P>,
        witn: Witness,
        psi: Homomorphism<P>,
        f: TransformationFunction<P>,
    ): (Proof, Witness) {
        let (proof, alpha) = prove(
            dst,
            |_X, w| psi(_X, w),
            &stmt,
            &witn
        );

        // Make sure the sigma protocol proof verifies
        assert!(
            verify(
                dst,
                |_X, w| psi(_X, w),
                |_X| f(_X),
                &stmt,
                &proof
            ), error::invalid_argument(E_VERIFICATION_FAILED));

        (proof, alpha)
    }

    #[test_only]
    /// Returns `true` if the empty proof does not verify for the specific statement. Otherwise, returns `false`.
    public(friend) inline fun empty_proof_verifies<P>(
        dst: DomainSeparator,
        psi: Homomorphism<P>,
        f: TransformationFunction<P>,
        stmt: Statement<P>,
    ): bool {
        let proof = sigma_protocol_proof::empty();

        !verify(
            dst,
            |_X, w| psi(_X, w),
            |_X| f(_X),
            &stmt,
            &proof
        )
    }

    /// Adds up two vectors of points `a` and `b` returning a new vector `c` where `c[i] = a[i] + b[i]`.
    public fun add_vec_points(a: &vector<RistrettoPoint>, b: &vector<RistrettoPoint>): vector<RistrettoPoint> {
        assert!(a.length() == b.length(), error::internal(E_INTERNAL_INVARIANT_FAILED));

        let r = vector[];
        a.enumerate_ref(|i, pt| {
            r.push_back(pt.point_add(&b[i]));
        });

        r
    }

    /// Given a vector of Ristretto255 points `a` and a scalar `e`, returns a new vector `c` where `c[i] = e * a[i]`.
    public fun mul_points(a: &vector<RistrettoPoint>, e: &Scalar): vector<RistrettoPoint> {
        a.map_ref(|pt| pt.point_mul(e))
    }

    /// Ensures two vectors of Ristretto255 points are equal.
    public fun equal_vec_points(a: &vector<RistrettoPoint>, b: &vector<RistrettoPoint>): bool {
        let m = a.length();
        assert!(m == b.length(), error::internal(E_INTERNAL_INVARIANT_FAILED));

        vector::range(0, m).all(|i| a[*i].point_equals(&b[*i]))
    }
}
