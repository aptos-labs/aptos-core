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
module aptos_experimental::sigma_protocol_registration {
    use std::bcs;
    use std::error;
    use std::signer;
    use aptos_std::ristretto255::{Self, RistrettoPoint, Scalar, CompressedRistretto};
    use aptos_framework::fungible_asset::Metadata;
    use aptos_framework::object::Object;
    use aptos_experimental::sigma_protocol;
    use aptos_experimental::sigma_protocol_proof::Proof;
    use aptos_experimental::sigma_protocol_fiat_shamir::new_domain_separator;
    use aptos_experimental::sigma_protocol_witness::{Witness, new_secret_witness};
    use aptos_experimental::sigma_protocol_statement::{Statement, new_statement};
    use aptos_experimental::sigma_protocol_representation::new_representation;
    use aptos_experimental::sigma_protocol_representation_vec::{RepresentationVec, new_representation_vec};
    #[test_only]
    use aptos_framework::account;
    #[test_only]
    use aptos_framework::fungible_asset;
    #[test_only]
    use aptos_experimental::ristretto255_twisted_elgamal::{get_encryption_key_basepoint_compressed, pubkey_from_secret_key};
    #[test_only]
    use aptos_experimental::sigma_protocol_homomorphism::evaluate_psi;
    #[test_only]
    use aptos_experimental::sigma_protocol_proof;
    #[test_only]
    use aptos_experimental::sigma_protocol_utils::equal_vec_points;

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

    /// The expected number of points in a registration statement is `N_1`.
    const E_WRONG_NUM_POINTS: u64 = 1;
    /// The expected number of scalars in a registration statement is `N_2`.
    const E_WRONG_NUM_SCALARS: u64 = 2;
    /// The expected number of scalars in a registration witness is `K`.
    const E_WRONG_WITNESS_LEN: u64 = 3;
    /// The expected number of points in the homomorphism & transformation function output is `M`.
    const E_WRONG_OUTPUT_LEN: u64 = 4;
    /// The registration proof was invalid
    const E_INVALID_REGISTRATION_PROOF: u64 = 5;

    //
    // Structs
    //

    /// Used for domain separation in the Fiat-Shamir transform.
    struct RegistrationSession has drop {
        sender: address,
        asset_type: Object<Metadata>,
    }

    //
    // Helper functions
    //

    /// Ensures the statement has `N_1` points and `N_2` scalars.
    fun assert_registration_statement_is_well_formed(stmt: &Statement) {
        assert!(stmt.get_points().length() == N_1, error::invalid_argument(E_WRONG_NUM_POINTS));
        assert!(stmt.get_scalars().length() == N_2, error::invalid_argument(E_WRONG_NUM_SCALARS));
    }

    //
    // Public functions
    //

    public fun new_session(sender: &signer, asset_type: Object<Metadata>): RegistrationSession {
        RegistrationSession {
            sender: signer::address_of(sender),
            asset_type,
        }
    }

    /// Creates a new registration statement: $(H, \mathsf{ek})$.
    public fun new_registration_statement(
        compressed_H: CompressedRistretto, _H: RistrettoPoint,
        compressed_ek: CompressedRistretto, ek: RistrettoPoint,
    ): Statement {
        let stmt = new_statement(
            vector[_H, ek],
            vector[compressed_H, compressed_ek],
            vector[]
        );
        assert_registration_statement_is_well_formed(&stmt);
        stmt
    }

    /// Creates a new registration witness: $(\mathsf{dk})$.
    public fun new_registration_witness(dk: Scalar): Witness {
        new_secret_witness(vector[dk])
    }

    /// The homomorphism $\psi_\mathsf{dl}(\mathsf{dk} \mid \mathsf{ek}) = \mathsf{dk} \cdot \mathsf{ek}$.
    public fun psi(stmt: &Statement, w: &Witness): RepresentationVec {
        // WARNING: Crucial for security
        assert_registration_statement_is_well_formed(stmt);
        // WARNING: Crucial for security
        assert!(w.length() == K, error::invalid_argument(E_WRONG_WITNESS_LEN));

        let dk = *w.get(IDX_DK);

        let output = new_representation_vec(vector[
            // dk * ek
            new_representation(vector[IDX_EK], vector[dk]),
        ]);

        // WARNING: Crucial for security
        assert!(output.length() == M, error::invalid_argument(E_WRONG_OUTPUT_LEN));

        output
    }

    /// The transformation function $\mathsf{f}_\mathsf{dl}(\mathsf{ek}) = H$.
    public fun f(_stmt: &Statement): RepresentationVec {
        // We do not re-assert well-formedness since wherever f is called, psi is also called.
        new_representation_vec(vector[
            // H
            new_representation(vector[IDX_H], vector[ristretto255::scalar_one()]),
        ])
    }

    /// Asserts that a registration proof verifies.
    public fun assert_verifies(session: &RegistrationSession, stmt: &Statement, proof: &Proof) {
        let success = sigma_protocol::verify(
            new_domain_separator(PROTOCOL_ID, bcs::to_bytes(session)),
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
    /// Returns a dummy session used for testing.
    /// WARNING: Can only be called once because it calls `create_fungible_asset`!
    fun registration_session_for_testing(): RegistrationSession {
        let sender = account::create_signer_for_test(@0x1);
        let (_, _, _, _, asset_type) = fungible_asset::create_fungible_asset(&sender);

        RegistrationSession {
            sender: signer::address_of(&sender),
            asset_type,
        }
    }

    #[test_only]
    /// Creates a registration proof (for testing).
    public fun prove(session: &RegistrationSession, stmt: &Statement, witn: &Witness): Proof {
        let (proof, _) = sigma_protocol::prove(
            new_domain_separator(PROTOCOL_ID, bcs::to_bytes(session)),
            |_X, w| psi(_X, w),
            stmt,
            witn
        );

        proof
    }

    #[test_only]
    /// Computes the statement and witness from a decryption key.
    public fun compute_statement_and_witness(
        dk: &Scalar,
    ): (Statement, Witness) {
        let compressed_H = get_encryption_key_basepoint_compressed();
        let _H = compressed_H.point_decompress();

        let compressed_ek = pubkey_from_secret_key(dk).extract();
        let ek = compressed_ek.point_decompress();

        let stmt = new_registration_statement(compressed_H, _H, compressed_ek, ek);
        let witn = new_registration_witness(*dk);

        (stmt, witn)
    }

    #[test]
    /// Verifies that the homomorphism $\psi$ is implemented correctly.
    fun psi_correctness() {
        let dk = ristretto255::random_scalar();
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

    #[test]
    /// Verifies that a correctly computed proof verifies.
    fun proof_correctness() {
        let dk = ristretto255::random_scalar();
        let (stmt, witn) = compute_statement_and_witness(&dk);

        let ss = registration_session_for_testing();
        let proof = prove(&ss, &stmt, &witn);

        assert_verifies(&ss, &stmt, &proof);
    }

    #[test]
    #[expected_failure(abort_code=65537, location=aptos_experimental::sigma_protocol_fiat_shamir)]
    /// Verifies that an empty proof does not verify for a random statement.
    fun proof_soundness_against_random_statement() {
        let dk = ristretto255::random_scalar();
        let (stmt, _) = compute_statement_and_witness(&dk);

        let proof = sigma_protocol_proof::empty();

        assert_verifies(&registration_session_for_testing(), &stmt, &proof);
    }

    #[test]
    #[expected_failure(abort_code=65537, location=aptos_experimental::sigma_protocol_fiat_shamir)]
    /// Verifies that an empty proof does not verify for an "empty" statement (all identity points).
    fun proof_soundness_against_empty_statement_and_empty_proof() {
        let _H = ristretto255::point_identity();
        let compressed_H = ristretto255::point_identity_compressed();
        let ek = ristretto255::point_identity();
        let compressed_ek = ristretto255::point_identity_compressed();

        let stmt = new_registration_statement(compressed_H, _H, compressed_ek, ek);
        let proof = sigma_protocol_proof::empty();

        assert_verifies(&registration_session_for_testing(), &stmt, &proof);
    }
}
