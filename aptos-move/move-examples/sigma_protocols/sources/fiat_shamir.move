module sigma_protocols::fiat_shamir {
    use std::bcs;
    use std::error;
    use aptos_std::aptos_hash::sha2_512;
    use aptos_std::ristretto255::{CompressedRistretto, Scalar, RistrettoPoint, new_scalar_uniform_from_64_bytes,
        scalar_one, scalar_mul
    };
    use sigma_protocols::public_statement::PublicStatement;
    use sigma_protocols::utils::compress_points;

    /// The length of the `A` field in `Proof` should NOT be zero
    const E_PROOF_COMMITMENT_EMPTY: u64 = 1;
    /// One of our internal invariants was broken. There is likely a logical error in the code.
    const E_INTERNAL_INVARIANT_FAILED: u64 = 2;

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
}
