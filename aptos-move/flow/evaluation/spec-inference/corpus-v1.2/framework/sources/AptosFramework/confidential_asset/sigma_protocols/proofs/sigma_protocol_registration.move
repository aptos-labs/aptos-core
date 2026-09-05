/// # The registration NP relation ($\mathcal{R}_\mathsf{dl}$)
///
/// A ZKPoK that the user knows the decryption key corresponding to their encryption key.
///
/// \begin{align}
///   \mathcal{R}_\mathsf{dl}\left(\mathsf{ek}; \mathsf{dk}\right) = 1
///   \Leftrightarrow H = \mathsf{dk} \cdot \mathsf{ek}
/// \end{align}
///
/// This is a Schnorr-like proof framed as a homomorphism check:
///
/// \begin{align}
///   \underbrace{H}_{\mathsf{f}_\mathsf{dl}(\mathsf{ek})}
///   =
///   \underbrace{\mathsf{dk} \cdot \mathsf{ek}}_{\psi_\mathsf{dl}(\mathsf{dk} \mid \mathsf{ek})}
/// \end{align}
///
module aptos_framework::sigma_protocol_registration {
    friend aptos_framework::confidential_asset;
    #[test_only]
    friend aptos_framework::confidential_asset_tests;
    #[test_only]
    friend aptos_framework::sigma_protocol_proof_tests;

    use std::bcs;
    use std::error;
    use std::signer;
    use aptos_std::ristretto255::CompressedRistretto;
    use aptos_framework::chain_id;
    use aptos_framework::fungible_asset::Metadata;
    use aptos_framework::object::Object;
    use aptos_framework::sigma_protocol;
    use aptos_framework::sigma_protocol_proof::Proof;
    use aptos_framework::sigma_protocol_fiat_shamir::new_domain_separator;
    use aptos_framework::sigma_protocol_witness::Witness;
    use aptos_framework::sigma_protocol_statement::Statement;
    use aptos_framework::sigma_protocol_statement_builder::new_builder;
    use aptos_framework::sigma_protocol_representation::{repr_point, repr_scaled};
    use aptos_framework::sigma_protocol_representation_vec::{RepresentationVec, new_representation_vec};
    use aptos_framework::sigma_protocol_utils::{e_wrong_num_points, e_wrong_num_scalars, e_wrong_witness_len, e_wrong_output_len};
    use aptos_framework::confidential_balance::get_encryption_key_basepoint_compressed;
    #[test_only]
    use aptos_std::ristretto255::{Scalar, random_scalar};
    #[test_only]
    use aptos_framework::sigma_protocol_test_utils::setup_test_environment;
    #[test_only]
    use aptos_framework::confidential_crypto_test_utils::{pubkey_from_secret_key, equal_vec_points, new_registration_witness};
    #[test_only]
    use aptos_framework::sigma_protocol_homomorphism::evaluate_psi;

    //
    // Constants
    //

    /// Protocol ID used for domain separation
    const PROTOCOL_ID: vector<u8> = b"AptosConfidentialAsset/RegistrationV1";

    /// The number of points $n_1$ in a $\mathcal{R}_\mathsf{dl}$ public statement.
    /// WARNING: Crucial for security.
    const N_1: u64 = 2;
    /// The number of scalars $n_2$ in a $\mathcal{R}_\mathsf{dl}$ public statement.
    /// WARNING: Crucial for security.
    const N_2: u64 = 0;
    /// The number of scalars $k$ in a $\mathcal{R}_\mathsf{dl}$ secret witness.
    /// WARNING: Crucial for security.
    const K: u64 = 1;
    /// The number of points $m$ in the image of the $\mathcal{R}_\mathsf{dl}$ homomorphism and transformation function.
    /// WARNING: Crucial for security.
    const M: u64 = 1;

    //
    // Statement point indices
    //

    /// Index of $H$ (the encryption key basepoint) in the statement's points vector.
    const IDX_H: u64 = 0;
    /// Index of $\mathsf{ek}$ (the user's encryption key) in the statement's points vector.
    const IDX_EK: u64 = 1;

    //
    // Witness scalar indices
    //

    /// Index of $\mathsf{dk}$ (the user's decryption key) in the witness's scalars vector.
    const IDX_DK: u64 = 0;

    //
    // Error codes
    //

    /// The registration proof was invalid
    const E_INVALID_REGISTRATION_PROOF: u64 = 5;  // other error codes in [1, 4] in sigma_protocol_utils.move
    /// The homomorphism or transformation function implementation is not inserting points at the expected positions.
    const E_STATEMENT_BUILDER_INCONSISTENCY: u64 = 6;

    //
    // Structs
    //

    /// Phantom marker type for registration statements.
    struct Registration has drop {}

    /// Used for domain separation in the Fiat-Shamir transform.
    struct RegistrationSession has drop {
        sender: address,
        asset_type: Object<Metadata>,
    }

    //
    // Helper functions
    //

    /// Ensures the statement has `N_1` points and `N_2` scalars.
    fun assert_registration_statement_is_well_formed(stmt: &Statement<Registration>) {
        assert!(stmt.get_points().length() == N_1, e_wrong_num_points());
        assert!(stmt.get_scalars().length() == N_2, e_wrong_num_scalars());
    }

    //
    // Public functions
    //

    public(friend) fun new_session(sender: &signer, asset_type: Object<Metadata>): RegistrationSession {
        RegistrationSession {
            sender: signer::address_of(sender),
            asset_type,
        }
    }

    /// Creates a new registration statement: $(H, \mathsf{ek})$.
    ///
    /// H is computed internally via `get_encryption_key_basepoint_compressed()`.
    /// ek is decompressed internally from `compressed_ek`.
    public(friend) fun new_registration_statement(
        compressed_ek: CompressedRistretto,
    ): Statement<Registration> {
        let b = new_builder();
        assert!(b.add_point(get_encryption_key_basepoint_compressed()) == IDX_H, error::internal(E_STATEMENT_BUILDER_INCONSISTENCY)); // H
        assert!(b.add_point(compressed_ek) == IDX_EK, error::internal(E_STATEMENT_BUILDER_INCONSISTENCY)); // ek
        let stmt = b.build();
        assert_registration_statement_is_well_formed(&stmt);
        stmt
    }

    /// The homomorphism $\psi_\mathsf{dl}(\mathsf{dk} \mid \mathsf{ek}) = \mathsf{dk} \cdot \mathsf{ek}$.
    fun psi(stmt: &Statement<Registration>, w: &Witness): RepresentationVec {
        // WARNING: Crucial for security
        assert_registration_statement_is_well_formed(stmt);
        // WARNING: Crucial for security
        assert!(w.length() == K, e_wrong_witness_len());

        let dk = *w.get(IDX_DK);

        let reprs = (vector[
            // dk * ek
            repr_scaled(IDX_EK, dk),
        ]);

        // WARNING: Crucial for security
        assert!(reprs.length() == M, e_wrong_output_len());

        new_representation_vec(reprs)
    }

    /// The transformation function $\mathsf{f}_\mathsf{dl}(\mathsf{ek}) = H$.
    fun f(_stmt: &Statement<Registration>): RepresentationVec {
        // We do not re-assert well-formedness since wherever f is called, psi is also called.
        new_representation_vec(vector[
            // H
            repr_point(IDX_H),
        ])
    }

    /// Asserts that a registration proof verifies.
    public(friend) fun assert_verifies(self: &RegistrationSession, stmt: &Statement<Registration>, proof: &Proof) {
        let success = sigma_protocol::verify(
            new_domain_separator(@aptos_framework, chain_id::get(), PROTOCOL_ID, bcs::to_bytes(self)),
            |_X, w| psi(_X, w),
            |_X| f(_X),
            stmt,
            proof
        );

        assert!(success, error::invalid_argument(E_INVALID_REGISTRATION_PROOF));
    }

    //
    // Tests
    //

    #[test_only]
    public(friend) fun registration_session_for_testing(): RegistrationSession {
        let (sender, asset_type) = setup_test_environment();
        RegistrationSession { sender: signer::address_of(&sender), asset_type }
    }

    #[test_only]
    /// Creates a registration proof (for testing).
    public(friend) fun prove(self: &RegistrationSession, stmt: &Statement<Registration>, witn: &Witness): Proof {
        let (proof, _) = sigma_protocol::prove(
            new_domain_separator(@aptos_framework, chain_id::get(), PROTOCOL_ID, bcs::to_bytes(self)),
            |_X, w| psi(_X, w),
            stmt,
            witn
        );

        proof
    }

    #[test_only]
    /// Computes the statement and witness from a decryption key.
    public(friend) fun compute_statement_and_witness(
        dk: &Scalar,
    ): (Statement<Registration>, Witness) {
        let compressed_ek = pubkey_from_secret_key(dk).extract();

        let stmt = new_registration_statement(compressed_ek);
        let witn = new_registration_witness(*dk);

        (stmt, witn)
    }

    #[test]
    /// Verifies that the homomorphism $\psi$ is implemented correctly.
    fun psi_correctness() {
        let dk = random_scalar();
        let (_X, w) = compute_statement_and_witness(&dk);

        // Get statement components
        let ek = _X.get_point(IDX_EK);

        // Compute expected psi output manually: dk * ek
        let expected_psi = vector[
            ek.point_mul(&dk),
        ];

        // Compute actual psi output via our implementation
        let actual_psi = evaluate_psi(|_X, w| psi(_X, w), &_X, &w);

        assert!(equal_vec_points(&actual_psi, &expected_psi), 1);
    }

}
