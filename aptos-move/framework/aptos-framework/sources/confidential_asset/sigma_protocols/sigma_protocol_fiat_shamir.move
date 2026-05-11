module aptos_framework::sigma_protocol_fiat_shamir {
    friend aptos_framework::sigma_protocol;
    friend aptos_framework::sigma_protocol_registration;
    friend aptos_framework::sigma_protocol_withdraw;
    friend aptos_framework::sigma_protocol_transfer;
    friend aptos_framework::sigma_protocol_key_rotation;
    #[test_only]
    friend aptos_framework::sigma_protocol_pedeq_example;
    #[test_only]
    friend aptos_framework::sigma_protocol_schnorr_example;

    use std::bcs;
    use std::error;
    use std::string::String;
    use aptos_std::aptos_hash::{sha2_512, sha2_512_value};
    use aptos_std::ristretto255::{CompressedRistretto, Scalar, new_scalar_uniform_from_64_bytes, scalar_one};
    use aptos_std::type_info;
    use aptos_framework::sigma_protocol_statement::Statement;

    /// The length of the `A` field in `Proof` should NOT be zero
    const E_PROOF_COMMITMENT_EMPTY: u64 = 1;
    /// One of our internal invariants was broken. There is likely a logical error in the code.
    const E_INTERNAL_INVARIANT_FAILED: u64 = 2;

    /// A domain separator prevents replay attacks in $\Sigma$ protocols and consists of 5 things.
    ///
    /// 1. The contract address (defense in depth: binds the proof to a specific deployed contract)
    ///
    /// 2. The chain ID (defense in depth: binds the proof to a specific Aptos network)
    ///
    /// 3. A protocol identifier, which is typically split up into two things:
    ///    - A higher-level protocol: "Confidential Assets v1 on Aptos"
    ///    - A lower-level relation identifier (e.g., "PedEq", "Schnorr", "DLEQ", etc.)
    ///
    /// 4. Statement (i.e., the public statement in the NP relation being proved)
    ///    - This is captured implicitly in our `prove` and `verify` functions ==> it is not part of this struct.
    ///
    /// 5. Session identifier
    ///    - Chosen by user
    ///    - specifies the "context" in which this proof is valid
    ///    - e.g., "Alice (0x1) is paying Bob (0x2) at time `t`
    ///    - together with the protocol identifier, prevents replay attacks across the same protocol or different protocols
    ///
    /// Note: The session identifier can be tricky, since in some settings the "session" accumulates implicitly in the
    /// statement being proven. For confidential assets, it does not AFAICT: the "session" is represented at least by
    /// the confidential balances of the users & their addresses.
    enum DomainSeparator has drop, copy {
        V1 {
            contract_address: address,
            chain_id: u8,
            protocol_id: vector<u8>,
            session_id: vector<u8>,
        }
    }

    public(friend) fun new_domain_separator(contract_address: address, chain_id: u8, protocol_id: vector<u8>, session_id: vector<u8>): DomainSeparator {
        DomainSeparator::V1 {
            contract_address,
            chain_id,
            protocol_id,
            session_id
        }
    }

    /// Unfortunately, we cannot directly use the `Statement` struct here because its `vector<RistrettoPoint>`
    /// will not serialize correctly via `bcs::to_bytes`, since a `RistrettoPoint` stores a Move VM "handle" rather than
    /// an actual point.
    struct FiatShamirInputs has drop {
        dst: DomainSeparator,
        /// The fully-qualified type name of the phantom marker type `P` in `Statement<P>`.
        /// E.g., `"0x7::sigma_protocol_registration::Registration"`.
        /// This binds the Fiat-Shamir challenge to the specific protocol type for defense in depth.
        type_name: String,
        k: u64,
        stmt_X: vector<CompressedRistretto>,
        stmt_x: vector<Scalar>,
        proof_A: vector<CompressedRistretto>,
    }

    /// Returns the Sigma protocol challenge $e$ and $1,\beta,\beta^2,\ldots, \beta^{m-1}$
    public(friend) fun fiat_shamir<P>(
        dst: DomainSeparator,
        stmt: &Statement<P>,
        compressed_A: &vector<CompressedRistretto>,
        sigmas: &vector<Scalar>,
        k: u64): (Scalar, vector<Scalar>)
    {
        let m = compressed_A.length();
        assert!(m != 0, error::invalid_argument(E_PROOF_COMMITMENT_EMPTY));

        // We will hash an application-specific domain separator and the (full) public statement,
        // which will include any public parameters like group generators $G$, $H$.

        // Note: A more principled `Merlin` / `spongefish`-like approach would have been preferred, but... more code.

        // Note: A hardcodes $m$, the statement hardcodes $n_1$ and $n_2$, and $k$ is specified manually!;
        let seed = sha2_512_value(&FiatShamirInputs {
            dst,
            type_name: type_info::type_name<P>(),
            k,
            stmt_X: *stmt.get_compressed_points(),
            stmt_x: *stmt.get_scalars(),
            proof_A: *compressed_A
        });
        seed.push_back(0u8);
        assert!(*seed.last() == 0, error::internal(E_INTERNAL_INVARIANT_FAILED));

        // i.e., SHA2-512(
        //         SHA2-512(BCS{ dst, type_name, k, stmt_X, stmt_x, proof_A })
        //         || 0x00
        //       )
        let e = new_scalar_uniform_from_64_bytes(sha2_512(seed)).extract();

        *seed.last_mut() += 1;
        assert!(*seed.last() == 1, error::internal(E_INTERNAL_INVARIANT_FAILED));
        seed.append(bcs::to_bytes(sigmas));

        // i.e., SHA2-512(
        //         SHA2-512(BCS{ dst, type_name, k, stmt_X, stmt_x, proof_A })
        //         || 0x01
        //         || BCS{ sigmas }
        //       )
        let beta = new_scalar_uniform_from_64_bytes(sha2_512(seed)).extract();

        let betas = vector[];
        let prev_beta = scalar_one();
        betas.push_back(prev_beta);
        for (_i in 1..m) {
            prev_beta = prev_beta.scalar_mul(&beta);
            betas.push_back(prev_beta);
        };

        // This will only fail when our logic above for generating the `\beta_i`'s is broken
        assert!(betas.length() == m, error::internal(E_INTERNAL_INVARIANT_FAILED));

        (e, betas)
    }

    #[test_only]
    use aptos_framework::sigma_protocol_statement::new_statement;
    #[test_only]
    use aptos_std::ristretto255::{point_identity_compressed, new_scalar_from_u64, scalar_equals, basepoint_H,
        basepoint_H_compressed, basepoint, basepoint_compressed
    };

    #[test_only]
    /// Phantom marker used for tests that need a `Statement<P>` but do not
    /// care about a specific protocol.
    struct TestProtocol has drop {}

    #[test]
    /// Regression test pinning every binding of the Fiat-Shamir transcript.
    ///
    /// Soundness of the aggregated check
    ///
    /// $$\sum_i \beta^i \cdot (\psi_i(\sigma) - A_i - e \cdot f_i(X)) = 0$$
    ///
    /// requires $\beta$ to be unpredictable to the prover at the moment $\sigma$ is committed. That requires $\sigma$
    /// to be part of the Fiat-Shamir transcript that derives $\beta$. Conversely, $e$ MUST NOT depend on $\sigma$ —
    /// the honest prover computes $\sigma = \alpha + e \cdot w$, so $e$ must be fixed before $\sigma$ exists.
    /// $e$ MUST, however, depend on the rest of the public transcript (the statement and the prover's commitment
    /// $A$); otherwise the verifier would be replaying a fixed challenge across distinct statements.
    ///
    /// Holding the rest of the transcript fixed and varying one input at a time, this test pins:
    ///   - changing $\sigma$ MUST change $\beta$ and MUST NOT change $e$ (the bounty regression);
    ///   - changing $A$ MUST change both $e$ and $\beta$;
    ///   - changing the statement MUST change both $e$ and $\beta$.
    fun beta_changes_with_sigma_e_does_not() {
        let dst = new_domain_separator(@aptos_framework, 4u8, b"fs regression", b"session");
        let stmt = new_statement<TestProtocol>(vector[basepoint(), basepoint_H()], vector[basepoint_compressed(), basepoint_H_compressed()], vector[]);

        // $m = 2$ $\Rightarrow$ `betas` = $[1, \beta]$; `betas[1]` is the raw $\beta$ value.
        let _A = vector[point_identity_compressed(), point_identity_compressed()];
        let k = 1;

        let sigmas_a = vector[new_scalar_from_u64(7)];
        let sigmas_b = vector[new_scalar_from_u64(8)];

        let (e_A, betas_A) = fiat_shamir<TestProtocol>(dst, &stmt, &_A, &sigmas_a, k);
        let (e_b, betas_b) = fiat_shamir<TestProtocol>(dst, &stmt, &_A, &sigmas_b, k);

        // (1) $\sigma$-binding.
        // $e$: derived from a transcript that excludes $\sigma$ — must be invariant.
        assert!(e_A.scalar_equals(&e_b), 1);
        // $\beta$: derived from a transcript that includes $\sigma$ — must change with $\sigma$.
        assert!(!betas_A[1].scalar_equals(&betas_b[1]), 2);

        // (2) $A$-binding: flipping any component of $A$ MUST change both $e$ and $\beta$.
        let alt_A = vector[basepoint_compressed(), point_identity_compressed()];
        let (e_alt_A, betas_alt_A) = fiat_shamir<TestProtocol>(dst, &stmt, &alt_A, &sigmas_a, k);
        assert!(!e_A.scalar_equals(&e_alt_A), 3);
        assert!(!betas_A[1].scalar_equals(&betas_alt_A[1]), 4);

        // (3) Statement-binding: any change to the statement MUST change both $e$ and $\beta$. Adding a scalar suffices
        // since `stmt_x` is part of the $e$-transcript (and the inner seed feeds into the $\beta$-transcript).
        let alt_stmt = new_statement<TestProtocol>(
            vector[basepoint(), basepoint_H()],
            vector[basepoint_compressed(), basepoint_H_compressed()],
            vector[new_scalar_from_u64(1)],
        );
        let (e_alt_stmt, betas_alt_stmt) = fiat_shamir<TestProtocol>(dst, &alt_stmt, &_A, &sigmas_a, k);
        assert!(!e_A.scalar_equals(&e_alt_stmt), 5);
        assert!(!betas_A[1].scalar_equals(&betas_alt_stmt[1]), 6);
    }
}
