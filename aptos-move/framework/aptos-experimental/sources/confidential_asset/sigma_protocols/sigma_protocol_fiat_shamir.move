module aptos_experimental::sigma_protocol_fiat_shamir {
    use std::bcs;
    use std::error;
    use std::string::String;
    use aptos_std::aptos_hash::sha2_512;
    use aptos_std::ristretto255::{CompressedRistretto, Scalar, new_scalar_uniform_from_64_bytes,
        scalar_one, scalar_mul
    };
    use aptos_std::type_info;
    use aptos_experimental::sigma_protocol_statement::Statement;

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
    ///
    /// TODO(Security): We may want to add more here (like some sort of account TXN counter). I suspect that the
    ///   ciphertext randomness in the public statement would act as enough of a "session ID", but I would prefer
    ///   to avoid reasoning about that.
    enum DomainSeparator has drop, copy {
        V1 {
            contract_address: address,
            chain_id: u8,
            protocol_id: vector<u8>,
            session_id: vector<u8>,
        }
    }

    public fun new_domain_separator(contract_address: address, chain_id: u8, protocol_id: vector<u8>, session_id: vector<u8>): DomainSeparator {
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
    public fun fiat_shamir<P>(
        dst: DomainSeparator,
        stmt: &Statement<P>,
        compressed_A: &vector<CompressedRistretto>,
        k: u64): (Scalar, vector<Scalar>)
    {
        let m = compressed_A.length();
        assert!(m != 0, error::invalid_argument(E_PROOF_COMMITMENT_EMPTY));

        // We will hash an application-specific domain separator and the (full) public statement,
        // which will include any public parameters like group generators $G$, $H$.

        // Note: A hardcodes $m$, the statement hardcodes $n_1$ and $n_2$, and $k$ is specified manually!
        let bytes = bcs::to_bytes(&FiatShamirInputs {
            dst,
            type_name: type_info::type_name<P>(),
            k,
            stmt_X: *stmt.get_compressed_points(),
            stmt_x: *stmt.get_scalars(),
            proof_A: *compressed_A
        });

        // TODO(Security): A bit ad-hoc.
        let seed = sha2_512(bytes);

        // e = SHA2-512(SHA2-512(bytes) || 0x00)
        seed.push_back(0u8);
        let e_hash = sha2_512(seed);

        // beta = SHA2-512(SHA2-512(bytes) || 0x01)
        let len = seed.length();
        seed[len - 1] = 1u8;
        let beta_hash = sha2_512(seed);

        let e = new_scalar_uniform_from_64_bytes(e_hash).extract();
        let beta = new_scalar_uniform_from_64_bytes(beta_hash).extract();

        let betas = vector[];
        let prev_beta = scalar_one();
        betas.push_back(prev_beta);
        for (_i in 1..m) {
            prev_beta = scalar_mul(&prev_beta, &beta);
            betas.push_back(prev_beta);
        };

        // This will only fail when our logic above for generating the `\beta_i`'s is broken
        assert!(betas.length() == m, error::internal(E_INTERNAL_INVARIANT_FAILED));

        (e, betas)
    }
}
