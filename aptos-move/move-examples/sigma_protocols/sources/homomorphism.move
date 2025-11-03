/// This module can be used to build $\Sigma$-protocols for proving knowledge of a pre-image on a homomorphism $\psi$.
///
/// Let $\mathbb{G}^{n_1} \times \mathbb{F}^{n_2}$ denote the set of public statements.
///
/// This module helps you convince a verifier with $X\in S$ that you know a secret $w\in \mathbb{F}^k$ such that
/// $\psi(w) = f(X)$, where:
///
///    $\psi : \mathbb{F}^k \rightarrow \mathbb{G}^m$ is a *homomorphism*, and
///    $f : \mathbb{G}^{n_1} \times \mathbb{F}^{n_2} \rightarrow \mathbb{G}^m$ is a *transformation function*.
///
/// Many useful statements can be proved in ZK by framing them as proving knowledge of a pre-image on a homomorphism:
///
///    e.g., a Schnorr signature is just proving knowledge of $x$ such that $\psi(x) = x G$, where the PK is $x G$.
///
///    e.g., a proof that $C_1, C_2$ both Pedersen-commit to the same $m$ is proving knowledge of $(m, r_1, r_2)$ s.t.
///          $\psi(m, r_1, r_2) = (m G + r_1 H, m G + r_2 H)$
///
/// The sigma protocol is very simple:
///
/// + ------------------  +                                         + ------------------------------------------------ +
/// | Prover has $(X, w)$ |                                         | Verifier has                                     |
/// + ------------------  +                                         | $X \in \mathbb{G}^{n_1} \times \mathbb{F}^{n_2}$ |
///                                                                 + ------------------------------------------------ +
/// 1. Sample $\alpha \in \mathbb{F}^k
/// 2. Compute *commitment* $A \gets \psi(\alpha)$
///
///                                 3. send commitment $A$
///                            ------------------------------->
///
///                                                                  4. Assert $A \in \mathbb{G}^m$
///                                                                  5. Pick *random challenge* $e$
///                                                                     (via Fiat-Shamir on: $(X, A)$ a protocol
///                                                                      identifier and a session identifier)
///                                  6. send challenge $e$
///                            <-------------------------------
///
/// 7. Compute response $\sigma = \alpha + e \cdot w$
///
///                               8. send response $\sigma$
///                            ------------------------------->
///
///                                                                  9. Check $\psi(\sigma) = A + e f(X)$
module sigma_protocols::homomorphism {
    use std::bcs;
    use aptos_std::ristretto255::{RistrettoPoint, Scalar, scalar_one, scalar_mul, new_scalar_uniform_from_64_bytes,
        CompressedRistretto, point_identity,
        multi_scalar_mul, point_equals
    };
    use std::error;
    use aptos_std::aptos_hash::sha2_512;
    use sigma_protocols::proof::Proof;
    use sigma_protocols::secret_witness::SecretWitness;
    use sigma_protocols::public_statement::PublicStatement;
    use sigma_protocols::representation_vec::RepresentationVec;
    use sigma_protocols::utils::{compress_points, points_clone, neg_scalars};

    #[test_only]
    use sigma_protocols::proof::{new_proof, empty_proof};
    #[test_only]
    use sigma_protocols::secret_witness::random_witness;
    #[test_only]
    use sigma_protocols::utils::{add_vec_points, add_vec_scalars, equal_vec_points, mul_points, mul_scalars};

    //
    // Error codes
    //

    /// The length of the `A` field in `Proof` did NOT match the homomorphism's output length
    const E_PROOF_COMMITMENT_WRONG_LEN: u64 = 1;
    /// The length of the `A` field in `Proof` should NOT be zero
    const E_PROOF_COMMITMENT_EMPTY: u64 = 2;
    /// One of our internal invariants was broken. There is likely a logical error in the code.
    const E_INTERNAL_INVARIANT_FAILED: u64 = 3;

    //
    // Test-only error codes
    //

    #[test_only]
    /// The slow verification of the sigma protocol proof failed (instead of succeeding) in one of the tests.
    const E_SLOW_VERIFICATION_FAILED: u64 = 4;
    #[test_only]
    /// The fast verification of the sigma protocol proof failed (instead of succeeding) in one of the tests.
    const E_FAST_VERIFICATION_FAILED: u64 = 5;

    /// A domain separator prevents replay attacks in $\Sigma$ protocols and consists of 3 things.
    ///
    /// 1. A protocol identifier, which is typically split up into two things:
    ///    - A higher-level protocol: "Confidential Assets v1 on Aptos"
    ///    - A lower-level relation identifier (e.g., "PedEq", "Schnorr", "DLEQ", etc.)
    ///
    /// 2. Statement (i.e., the public statement in the NP relation being proved)
    ///    - This is captured implicitly in our `prove` and `verify` functions ==> it is not part of this struct.
    ///
    /// 3. Session identifier
    ///    - Chosen by user
    ///    - specifies the "context" in which this proof is valid
    ///    - e.g., "Alice (0x1) is paying Bob (0x2) at time `t`
    ///    - together with the protocol identifier, prevents replay attacks across the same protocol or different protocols
    ///
    /// Note: The session identifier can be tricky, since in some settings the "session" accumulates implicitly in the
    /// statement being proven. For confidential assets, it does not AFAICT: the "session" is represented at least by
    /// the confidential balances of the users & their addresses.
    struct DomainSeparator has drop, copy {
        protocol_id: vector<u8>,
        session_id: vector<u8>,
    }

    public fun new_domain_separator(protocol_id: vector<u8>, session_id: vector<u8>): DomainSeparator {
        DomainSeparator {
            protocol_id,
            session_id
        }
    }

    /// Unfortunately, we cannot directly use the `PublicStatement` struct here because its `vector<RistrettoPoint>`
    /// will not serialize correctly via `bcs::to_bytes`, since a `RistrettoPoint` stores a Move VM "handle" rather than
    /// an actual point.
    struct FiatShamirInputs has drop {
        dst: DomainSeparator,
        k: u64,
        stmt_X: vector<CompressedRistretto>,
        stmt_x: vector<Scalar>,
        A: vector<CompressedRistretto>,
    }

    /// Returns the Sigma protocol challenge $e$ and $1,\beta,\beta^2,\ldots, \beta^{m-1}$
    public fun fiat_shamir(
        dst: DomainSeparator,
        stmt: &PublicStatement,
        _A: &vector<RistrettoPoint>,
        k: u64): (Scalar, vector<Scalar>)
    {
        let m = _A.length();
        assert!(m != 0, error::invalid_argument(E_PROOF_COMMITMENT_EMPTY));

        // We will hash an application-specific domain separator and the (full) public statement,
        // which will include any public parameters like group generators $G$, $H$.

        // Note: A hardcodes $m$, the statement hardcodes $n_1$ and $n_2$, and $k$ is specified manually!
        let bytes = bcs::to_bytes(&FiatShamirInputs {
            dst,
            k,
            stmt_X: compress_points(stmt.get_points()),
            stmt_x: *stmt.get_scalars(),
            A: compress_points(_A)
        });

        // TODO(Security): A bit ad-hoc.
        let e_hash = sha2_512(bytes);
        let beta_hash = sha2_512(e_hash);

        let e = new_scalar_uniform_from_64_bytes(e_hash).extract();
        let beta = new_scalar_uniform_from_64_bytes(beta_hash).extract();

        let betas = vector[];
        let prev_beta = scalar_one();
        betas.push_back(prev_beta);
        let i = 1;
        while (i < m) {
            let new_beta = scalar_mul(&prev_beta, &beta);

            // \beta^i <- \beta^{i-1} * \beta
            betas.push_back(new_beta);

            prev_beta = new_beta;
            i += 1;
        };

        // This will only fail when our logic above for generating the `\beta_i`'s is broken
        assert!(betas.length() == m, error::internal(E_INTERNAL_INVARIANT_FAILED));

        (e, betas)
    }

    /// Returns $\psi(X, w) \in \mathbb{G}^m$ given the public statement $X$ and the secret witness $w$.
    public inline fun evaluate_homomorphism(psi: |&PublicStatement, &SecretWitness|RepresentationVec,
                                            stmt: &PublicStatement,
                                            witn: &SecretWitness): vector<RistrettoPoint> {
        let evals = vector[];

        psi(stmt, witn).for_each_ref(|repr| {
            evals.push_back(multi_scalar_mul(&repr.to_points(stmt), repr.get_scalars()));
        });

        evals
    }

    /// Returns $f(X) \in \mathbb{G}^m$ given the public statement $X$.
    public inline fun evaluate_f(f: |&PublicStatement|RepresentationVec,
                                 stmt: &PublicStatement): vector<RistrettoPoint> {
        let evals = vector[];

        f(stmt).for_each_ref(|repr| {
            evals.push_back(multi_scalar_mul(&repr.to_points(stmt), repr.get_scalars()));
        });

        evals
    }

    #[test_only]
    /// Creates a proof and additionally returns the randomness $\alpha \in \mathbb{F}^k$ used to
    /// create the sigma protocol commitment $A = \psi(\alpha) \in \mathbb{G}^m$.
    public inline fun prove(
        dst: DomainSeparator,
        psi: |&PublicStatement, &SecretWitness|RepresentationVec,
        stmt: &PublicStatement,
        witn: &SecretWitness,
    ): (Proof, SecretWitness) {
        let k = witn.length();

        // Step 1: Pick a random \alpha \in \F^k
        let alpha = random_witness(k);
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

        (new_proof(_A, sigma), alpha)
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
        let proof = empty_proof();

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
