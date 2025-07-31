/// This module can be used to build sigma protocols for proving knowledge of a pre-image on a homomorphism $\psi$.
///
/// Let $S = \mathbb{G}^{n_1} \times \mathbb{F}^{n_2}$ is the set of public statements.
///
/// This module helps you convince a verifier with $X\in S$ that you know a secret $w\in \mathbb{F}^k$ such that
/// $\psi(w) = f(X)$, where:
///
///    $\psi : \mathbb{F}^k \rightarrow \mathbb{G}^m$ is a *homomorphism*, and
///    $f : S \rightarrow \mathbb{G}^m$ is a *transformation function*.
///
/// Many useful statements can be proved in ZK by framing them as proving knowledge of a pre-image on a homomorphism:
///
/// e.g., a Schnorr signature is just proving knowledge of $x$ such that $\psi(x) = x G$ where the public key is $x G$.
///
/// e.g., a proof that $C_1, C_2$ both Pedersen-commit to the same $m$ is proving knowledge of $(m, r_1, r_2)$ such that
///        $\psi(m, r_1, r_2) = (m G + r_1 H, m G + r_2 H)$
///
/// The sigma protocol is very simple:
///
/// + ------------------  +                                         + ------------------------------------------------ +
/// | Prover has $(X, w)$ |                                         |    Verifier has                                  |
/// + ------------------  +                                         | $X \in \mathbb{G}^{n_1} \times \mathbb{F}^{n_2}$ |
///                                                                 + ------------------------------------------------ +
/// 1. Sample $\alpha \in \mathbb{F}^k
/// 2. Compute commitment $A \gets \psi(\alpha)$
///
///                                 3. send commitment $A$
///                            ------------------------------->
///
///                                                                  4. Assert $A \in \mathbb{G}^m$
///                                                                  5. Pick random challenge $e$
///                                                                     (via Fiat-Shamir on $X$ and a domain-separator)
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
        CompressedRistretto, scalar_neg_assign, point_clone, point_identity,
        multi_scalar_mul, point_equals, point_compress
    };
    use std::error;
    use aptos_std::aptos_hash::sha2_512;
    use sigma_protocols::public_statement::PublicStatement;
    use sigma_protocols::representation_vec::RepresentationVec;

    #[test_only]
    use std::vector::range;
    #[test_only]
    use aptos_std::ristretto255::{random_scalar, scalar_add};

    //
    // Error codes
    //

    /// The length of the `A` field in `Proof` did NOT match the homomorphism's output length
    const E_PROOF_COMMITMENT_WRONG_LEN: u64 = 1;

    /// The length of the output of the transformation function  did NOT match the homomorphism's output length
    const E_TRANSFORMATION_FUNC_WRONG_LEN: u64 = 2;

    /// The length of the `sigma` field in `Proof` did NOT match the homomorphism's domain length
    // const E_PROOF_RESPONSE_WRONG_LEN: u64 = 3;

    /// The length of the `A` field in `Proof` should NOT be zero
    const E_PROOF_COMMITMENT_EMPTY: u64 = 4;

    /// One of our internal invariants was broken. There is likely a logical error in the code.
    const E_INTERNAL_INVARIANT_FAILED: u64 = 5;

    //
    // Structs and their methods
    //

    /// A *secret witness* consists of a vector $w$ of $k$ scalars
    struct SecretWitness has drop {
        w: vector<Scalar>,
    }

    public fun new_secret_witness(w: vector<Scalar>): SecretWitness {
        SecretWitness {
            w
        }
    }

    public fun length(self: &SecretWitness): u64 {
        self.w.length()
    }

    public fun get_scalar(self: &SecretWitness, idx: u64): &Scalar {
        &self.w[idx]
    }

    #[test_only]
    /// Returns a size-$m$ random witness, used when creating a ZKP during testing.
    public fun random_witness(m: u64): SecretWitness {
        let w = vector[];

        range(0, m).for_each(|_|
            w.push_back(random_scalar())
        );

        new_secret_witness(
            w
        )
    }

    /// A sigma protocol *proof* always consists of:
    /// 1. a *commitment* $A \in \mathbb{G}^m$
    /// 2. a *response* $\sigma \in \mathbb{F}^k$
    struct Proof has drop {
        A: vector<RistrettoPoint>,
        sigma: vector<Scalar>,
    }

    public fun new_proof(_A: vector<RistrettoPoint>, sigma: vector<Scalar>): Proof {
        Proof {
            A: _A,
            sigma,
        }
    }

    /// Converts the proof's response $\sigma$ into a `SecretWitness` by setting the `w` field to $\sigma$.
    /// This is needed during proof verification, when calling the homomorphism on the `Proof`'s $\sigma$, but the
    /// homomorphism expects a `SecretWitness`.
    public fun response_to_witness(self: &Proof): SecretWitness {
        SecretWitness { w: self.sigma }
    }

    /// Returns the commitment component of the proof (i.e., $A$)
    public fun get_commitment(self: &Proof): &vector<RistrettoPoint> {
        &self.A
    }

    /// Unfortunately, we cannot directly use the `PublicStatement` struct here because its `vector<RistrettoPoint>`
    /// will not serialize correctly via `bcs::to_bytes`, since a `RistrettoPoint` stores a Move VM "handle" rather than
    /// an actual point.
    struct FiatShamirInputs has drop {
        dst: vector<u8>,
        name: vector<u8>,
        stmt_X: vector<CompressedRistretto>,
        stmt_x: vector<Scalar>,
        A: vector<CompressedRistretto>,
    }

    /// Needed for Fiat-Shamir hashing
    fun compress_ristretto_points(points: &vector<RistrettoPoint>): vector<CompressedRistretto> {
        let compressed = vector[];

        let i = 0;
        let len = points.length();
        while (i < len) {
            compressed.push_back(point_compress(&points[i]));
            i += 1;
        };

        compressed
    }

    /// Returns the Sigma protocol challenge $e$ and $1,\beta,\beta^2,\ldots, \beta^{m-1}$
    public fun fiat_shamir(dst: vector<u8>, name: vector<u8>, stmt: &PublicStatement, _A: &vector<RistrettoPoint>, m: u64): (Scalar, vector<Scalar>) {
        assert!(m != 0, error::invalid_argument(E_PROOF_COMMITMENT_EMPTY));

        // We will hash an application-specific domain separator, a protocol name and the (full) public statement,
        // which will include any public parameters like group generators $G$, $H$.
        let bytes = bcs::to_bytes(&FiatShamirInputs {
            dst,
            name,
            stmt_X: compress_ristretto_points(stmt.get_points()),
            stmt_x: *stmt.get_scalars(),
            A: compress_ristretto_points(_A)
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

        assert!(betas.length() == m, error::internal(E_INTERNAL_INVARIANT_FAILED));

        (e, betas)
    }

    /// Verifies a ZK `proof` that the prover knows a witness $w$ such that $f(X) = \psi(w)$ where $X$ is the
    /// statement in `stmt`.
    ///
    /// @param  dst  application-specific domain separator (e.g., "Aptos confidential assets protocol v2025.06")
    ///
    /// @param  name the name of the protocol being proved (e.g., "public withdrawal NP relation")
    ///
    /// @param  psi  a homomorphism mapping a vector of scalars to a vector of $m$ group elements, except each group
    /// element is returned as a `Representation` so that, later on, the main $\psi(\sigma) = A + e f(X)$ can be done
    /// efficiently in one MSM.
    ///
    /// @param f     transformation function takes takes in the public statement and outputs $m$ group elements, also
    /// returned as a `RepresentationVec`.
    ///
    /// @param stmt  the public statement $X$ that satisfies $f(X) = \psi(w)$ for some secret witness $w$
    ///
    /// @param proof the ZKP proving that the prover knows a $w$ s.t. $f(X) = \psi(w)$
    ///
    /// Returns true if it succeeds and false otherwise.
    public inline fun verify(
        dst: vector<u8>,
        name: vector<u8>,
        psi: |&PublicStatement, &SecretWitness|RepresentationVec,
        f: |&PublicStatement|RepresentationVec,
        stmt: &PublicStatement,
        proof: &Proof,
    ): bool {

        // Step 1: Fiat-Shamir transform on `(dst, (psi, f), stmt)` to derive the random challenge `e`
        let comm = proof.get_commitment();
        let m = comm.length();
        let (e, betas) = fiat_shamir(dst, name, stmt, comm, m);

        // Naively, we could do:
        // ```
        //   vector_equals(
        //      psi(stmt, &SecretWitness { w: proof.sigma }),
        //      vector_add(
        //           proof.A,
        //           vector_scalar_mul(e, f(stmt))
        //      );
        //   );
        // ```

        // Step 2:
        //   A + e f(stmt) - \psi(\sigma) = zero()
        //         <=>
        //   \forall i \in[m], A[i] + e f(stmt)[i] - \psi(\sigma)[i] = 0
        //         <=>
        //   \sum_{i \in [m]} \beta[i] A[i] + \beta[i] ( e f(stmt)[i] ) - \beta[i] ( \psi(\sigma)[i] ) = 0,
        //                             ^                   ^                        ^
        //                             |                   |                        |
        //                        \mathbb{G}         representation            representation
        //   for random \beta[i]'s (picked via on-chain randomness or more Fiat-Shamir)
        //
        // TODO(Perf): Silly, Move does not let us do anything better than cloning the response from the proof here, AFAICT.
        let psi_sigma = psi(stmt, &proof.response_to_witness());
        let efx = f(stmt);

        assert!(psi_sigma.length() == m, error::invalid_argument(E_PROOF_COMMITMENT_WRONG_LEN));
        assert!(psi_sigma.length() == efx.length(), error::invalid_argument(E_TRANSFORMATION_FUNC_WRONG_LEN));

        // "Scale" all the representations in `f(stmt)` by `e`. (Implicit assumption here is that `f` is homomorphic:
        // i.e., `e f(X) = f(eX)`, which holds because our `f`'s are a `RepresentationVec`.)
        efx.scale_all(&e);

        // "Scale" the `i`th reprentation in `efx` by `\beta[i]`
        efx.scale_each(&betas);

        // Negate all the beta[i]'s
        let neg_betas = betas;
        neg_betas.for_each_mut(|beta| {
            scalar_neg_assign(beta);
        });

        // "Scale" the `i`th reprentation in `\psi` by `-\beta[i]`
        // TODO(Perf): I think could be sub-optimal: we will redo the same \beta[i] \sigma[j] multiplication several times
        //   when a `RepresentationVec`'s row reuses \sigma[j].
        psi_sigma.scale_each(&neg_betas);

        // We start with an empty MSM: \sum_{i \in m} 0
        // ...and extend it to: \sum_{i \in [m]} A[i]^{\beta[i]}
        //                                          ^^^^^^^^^^^^^^^
        let scalars = betas;
        let bases = vector[];
        comm.for_each_ref(|p| {
            // TODO(Perf): Annoying limitation of our Ristretto255 module. (Should we "fix" it as per `crypto_algebra`?)
            bases.push_back(point_clone(p));
        });

        // Extend MSM to: be \sum_{i \in [m]} A[i]^\beta[i] + \beta[i] ( e f(stmt)[i] )
        //                                                    ^^^^^^^^^^^^^^^^^^^^^^^^^
        efx.for_each_ref(|repr| {
            repr.for_each(stmt,|pt, s| {
                bases.push_back(point_clone(pt));
                scalars.push_back(*s);
            });
        });

        // Extend MSM to: be \sum_{i \in [m]} A[i]^\beta[i] + \beta[i] ( e f(stmt)[i] ) - \beta[i] (\psi(\sigma)[i])
        //                                                                                ^^^^^^^^^^^^^^^^^^^^^^^^^^
        psi_sigma.for_each_ref(|repr| {
            repr.for_each(stmt,|pt, s| {
                bases.push_back(point_clone(pt));
                scalars.push_back(*s);
            });
        });

        assert!(bases.length() == 3 * m, error::internal(E_INTERNAL_INVARIANT_FAILED));
        assert!(scalars.length() == 3 * m, error::internal(E_INTERNAL_INVARIANT_FAILED));

        // Do the MSM and check it equals the (zero) identity
        point_equals(&multi_scalar_mul(&bases, &scalars), &point_identity())
    }

    //
    // Test only
    //

    #[test_only]
    public fun empty_proof(): Proof {
        Proof {
            A: vector[],
            sigma: vector[]
        }
    }

    #[test_only]
    /// Creates a proof and additionally returns the randomness $\alpha \in \mathbb{F}^k$ used to
    /// create the sigma protocol commitment $A = \psi(\alpha) \in \mathbb{G}^m$.
    public inline fun prove(
        dst: vector<u8>,
        name: vector<u8>,
        psi: |&PublicStatement, &SecretWitness|RepresentationVec,
        stmt: &PublicStatement,
        witn: &SecretWitness,
    ): (Proof, SecretWitness) {
        let k = witn.length();

        // Step 1: Pick a random \alpha \in \F^k
        let alpha = random_witness(k);

        // Step 2: A <- \psi(\alpha) \in \Gr^m
        let _A = vector[];
        psi(stmt, &alpha).for_each_ref(|repr| {
            let bases = vector[];
            let scalars = vector[];

            repr.for_each(stmt, |pt, s| {
                bases.push_back(point_clone(pt));
                scalars.push_back(*s);
            });

            _A.push_back(multi_scalar_mul(&bases, &scalars));
        });

        // Step 3: Derive a random-challenge `e` via Fiat-Shamir
        let m = _A.length();
        let (e, _) = fiat_shamir(dst, name, stmt, &_A, m);

        // Step 4: \sigma <- \alpha + e w
        let sigma = vector[];
        range(0, m).for_each(|i| {
            sigma.push_back(scalar_add(
                alpha.get_scalar(i),
                &scalar_mul(&e, witn.get_scalar(i))
            ));
        });

        (new_proof(_A, sigma), alpha)
    }
}
