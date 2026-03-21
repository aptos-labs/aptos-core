/// # The key rotation NP relation ($\mathcal{R}_\mathsf{keyrot}$)
///
/// $\def\old#1{{\color{red}{\dot{#1}}}}\def\new#1{{\color{teal}{\widetilde{#1}}}}$
///
/// A ZKPoK of having rotated an encryption key to a new one and re-encrypted (part of) a Twisted ElGamal ciphertext.
///
/// ## Notation
///
/// - $\old{x}$ denotes a stale/old ciphertext component; $\new{x}$ denotes a fresh/new one.
/// - $\ell$: number of available balance chunks.
///
/// ## The relation
///
/// $$
/// \mathcal{R}_\mathsf{keyrot}^\ell\left(\begin{array}{l}
///     H, \mathsf{ek}, \new{\mathsf{ek}},
///       \old{\mathbf{R}}, \new{\mathbf{R}}
///       \textbf{;}\\
///     \mathsf{dk}, \delta, \delta_\mathsf{inv}
/// \end{array}\right) = 1
/// \Leftrightarrow
/// \left\{\begin{array}{r@{\,\,}l@{\quad}l}
///     H &= \mathsf{dk} \cdot \mathsf{ek}\\
///     \new{\mathsf{ek}} &= \delta \cdot \mathsf{ek}\\
///     \mathsf{ek} &= \delta_\mathsf{inv} \cdot \new{\mathsf{ek}}\\
///     \new{R}_i &= \delta \cdot \old{R}_i, &\forall i \in [\ell]\\
/// \end{array}\right.
/// $$
///
/// ## Homomorphism
///
/// This can be framed as a homomorphism check $\psi(\mathbf{w}) = f(\mathbf{X})$ where
/// $\mathbf{w} = (\mathsf{dk}, \delta, \delta_\mathsf{inv})$ is the witness
/// and $\mathbf{X} = (H, \mathsf{ek}, \new{\mathsf{ek}}, \old{\mathbf{R}}, \new{\mathbf{R}})$ is the statement.
///
///   1. The homomorphism $\psi$ is:
///
/// $$
/// \psi(\mathsf{dk}, \delta, \delta_\mathsf{inv}) = \begin{pmatrix}
///     \mathsf{dk} \cdot \mathsf{ek}\\
///     \delta \cdot \mathsf{ek}\\
///     \delta_\mathsf{inv} \cdot \new{\mathsf{ek}}\\
///     \delta \cdot \old{R}_i, &\forall i \in [\ell]\\
/// \end{pmatrix}
/// $$
///
///   2. The transformation function $f$ is:
///
/// $$
/// f(\mathbf{X}) = \begin{pmatrix}
///     H\\
///     \new{\mathsf{ek}}\\
///     \mathsf{ek}\\
///     \new{R}_i, &\forall i \in [\ell]\\
/// \end{pmatrix}
/// $$
///
module aptos_experimental::sigma_protocol_key_rotation {
    friend aptos_experimental::confidential_asset;

    use std::bcs;
    use std::error;
    use std::signer;
    use std::vector;
    use aptos_std::ristretto255::CompressedRistretto;
    use aptos_framework::chain_id;
    use aptos_framework::fungible_asset::Metadata;
    use aptos_framework::object::Object;
    use aptos_experimental::confidential_balance::get_num_available_chunks;
    use aptos_experimental::sigma_protocol;
    use aptos_experimental::sigma_protocol_proof::Proof;
    use aptos_experimental::sigma_protocol_fiat_shamir::new_domain_separator;
    use aptos_experimental::sigma_protocol_witness::Witness;
    use aptos_experimental::sigma_protocol_statement::Statement;
    use aptos_experimental::sigma_protocol_statement_builder::new_builder;
    use aptos_experimental::sigma_protocol_utils::{e_wrong_num_points, e_wrong_num_scalars, e_wrong_witness_len, e_wrong_output_len};
    use aptos_experimental::sigma_protocol_representation::{repr_point, repr_scaled};
    use aptos_experimental::sigma_protocol_representation_vec::{RepresentationVec, new_representation_vec};
    use aptos_experimental::ristretto255_twisted_elgamal::get_encryption_key_basepoint_compressed;
    #[test_only]
    use aptos_std::ristretto255::{Scalar, random_scalar, random_point, point_identity_compressed};
    #[test_only]
    use aptos_experimental::sigma_protocol_witness::new_secret_witness;
    #[test_only]
    use aptos_experimental::sigma_protocol_utils::compressed_identity_points;
    #[test_only]
    use aptos_experimental::sigma_protocol_test_utils::setup_test_environment;
    #[test_only]
    use aptos_experimental::ristretto255_twisted_elgamal::generate_twisted_elgamal_keypair;
    #[test_only]
    use aptos_experimental::sigma_protocol_homomorphism::{evaluate_psi, evaluate_f};
    #[test_only]
    use aptos_experimental::sigma_protocol_proof;
    #[test_only]
    use aptos_experimental::sigma_protocol_utils::equal_vec_points;
    #[test_only]
    use aptos_experimental::sigma_protocol_utils::compress_points;
    #[test_only]
    use aptos_experimental::sigma_protocol_mutation_tests;

    //
    // Constants
    //

    /// Protocol ID used for domain separation
    const PROTOCOL_ID: vector<u8> = b"AptosConfidentialAsset/KeyRotationV1";

    //
    // Statement point indices (matches the order in the NP relation: H, ek, new_ek, old_R, new_R)
    //

    /// Index of $H$ in the statement's points vector.
    const IDX_H: u64 = 0;
    /// Index of $\mathsf{ek}$ (old encryption key) in the statement's points vector.
    const IDX_EK: u64 = 1;
    /// Index of $\widetilde{\mathsf{ek}}$ (new encryption key) in the statement's points vector.
    const IDX_EK_NEW: u64 = 2;
    /// The old R values ($\dot{R}_i$ ) occupy indices 3 to 3 + (num_chunks - 1), inclusive.
    ///
    /// Note: The new R values ($\widetilde{R}_i$) occupy indices 3 + num_chunks to 3 + (2*num_chunks - 1), inclusive.
    /// A `get_start_idx_for_new_R(num_chunks)` function can be used to fetch the 3 + num_chunks starting index.
    const START_IDX_OLD_R: u64 = 3;

    //
    // Witness scalar indices
    //

    /// Index of $\mathsf{dk}$ (old decryption key) in the witness's scalars vector.
    const IDX_DK: u64 = 0;
    /// Index of $\delta$ in the witness's scalars vector.
    const IDX_DELTA: u64 = 1;
    /// Index of $\delta_\mathsf{inv}$ in the witness's scalars vector.
    const IDX_DELTA_INV: u64 = 2;

    //
    // Error codes
    //

    /// The key rotation proof was invalid
    const E_INVALID_KEY_ROTATION_PROOF: u64 = 1;
    /// The homomorphism or transformation function implementation is not inserting points at the expected positions.
    const E_STATEMENT_BUILDER_INCONSISTENCY: u64 = 2;

    //
    // Structs
    //

    /// Phantom marker type for key rotation statements.
    struct KeyRotation has drop {}

    /// Used for domain separation
    struct KeyRotationSession has drop {
        sender: address,
        token_type: Object<Metadata>,
        num_chunks: u64,
    }

    //
    // Helper functions
    //

    /// Returns the starting index of new_R values.
    inline fun get_start_idx_for_new_R(): u64 {
        START_IDX_OLD_R + get_num_available_chunks()
    }

    /// Ensures the statement is of the form:
    /// $\left(
    ///     H, \mathsf{ek}, \widetilde{\mathsf{ek}},
    ///     (\dot{R}_i)_{i \in [\ell]}),
    ///     (\widetilde{R}_i)_{i \in [\ell]}
    /// \right)$
    fun assert_key_rotation_statement_is_well_formed(
        stmt: &Statement<KeyRotation>,
    ) {
        assert!(stmt.get_points().length() == 3 + 2 * get_num_available_chunks(), e_wrong_num_points());
        assert!(stmt.get_scalars().length() == 0, e_wrong_num_scalars());
    }

    //
    // Public functions
    //

    public(friend) fun new_session(sender: &signer, token_type: Object<Metadata>): KeyRotationSession {
        KeyRotationSession {
            sender: signer::address_of(sender),
            token_type,
            num_chunks: get_num_available_chunks(),
        }
    }

    /// Creates a new key rotation statement.
    /// The order matches the NP relation: $(H, \mathsf{ek}, \widetilde{\mathsf{ek}}, \dot{\mathbf{R}}, \widetilde{\mathbf{R}})$.
    /// Note that the # of chunks is inferred from the sizes of the old and new balance ciphertexts.
    ///
    /// All points are decompressed internally from their compressed forms by the `StatementBuilder`.
    ///
    /// @param compressed_ek: Compressed form of the old encryption key
    /// @param compressed_new_ek: Compressed form of the new encryption key
    /// @param compressed_old_R: Compressed forms of old_R (by reference; num_chunks elements)
    /// @param compressed_new_R: Compressed forms of new_R (by reference; num_chunks elements)
    public(friend) fun new_key_rotation_statement(
        compressed_ek: CompressedRistretto,
        compressed_new_ek: CompressedRistretto,
        compressed_old_R: &vector<CompressedRistretto>,
        compressed_new_R: &vector<CompressedRistretto>,
    ): Statement<KeyRotation> {
        let err = error::internal(E_STATEMENT_BUILDER_INCONSISTENCY);
        let b = new_builder();
        assert!(b.add_point(get_encryption_key_basepoint_compressed()) == IDX_H, err);                  // H
        assert!(b.add_point(compressed_ek) == IDX_EK, err);                                                // ek
        assert!(b.add_point(compressed_new_ek) == IDX_EK_NEW, err);                                        // new_ek
        assert!(b.add_points(compressed_old_R) == START_IDX_OLD_R, err);                                   // old_R
        assert!(b.add_points(compressed_new_R) == START_IDX_OLD_R + get_num_available_chunks(), err);      // new_R
        let stmt = b.build();
        assert_key_rotation_statement_is_well_formed(&stmt);
        stmt
    }

    #[test_only]
    /// Creates a new key rotation witness.
    ///
    /// @param dk: The old decryption key
    /// @param delta: The ratio new_dk / old_dk (i.e., new_dk * old_dk^{-1})
    /// @param delta_inv: The inverse of delta
    public fun new_key_rotation_witness(dk: Scalar, delta: Scalar, delta_inv: Scalar): Witness {
        new_secret_witness(vector[dk, delta, delta_inv])
    }

    /// The homomorphism $\psi$ for the key rotation relation.
    ///
    /// Given witness $(dk, \delta, \delta_{inv})$, outputs:
    /// ```
    /// [
    ///   dk * ek,           // should equal H
    ///   delta * ek,        // should equal new_ek
    ///   delta_inv * new_ek, // should equal ek
    ///   delta * old_R_i,   // should equal new_R_i, for i in [1..num_chunks]
    /// ]
    /// ```
    fun psi(_stmt: &Statement<KeyRotation>, w: &Witness): RepresentationVec {
        // WARNING: Crucial for security
        assert_key_rotation_statement_is_well_formed(_stmt);
        // WARNING: Crucial for security
        assert!(w.length() == 3, e_wrong_witness_len());

        let dk = *w.get(IDX_DK);
        let delta = *w.get(IDX_DELTA);
        let delta_inv = *w.get(IDX_DELTA_INV);

        // Build the representation vector
        let reprs = vector[
            // dk * ek
            repr_scaled(IDX_EK, dk),
            // delta * ek
            repr_scaled(IDX_EK, delta),
            // delta_inv * new_ek
            repr_scaled(IDX_EK_NEW, delta_inv),
        ];

        // delta * old_R_i for each chunk
        let ell = get_num_available_chunks();
        reprs.append(vector::range(0, ell).map(|i|
            repr_scaled(START_IDX_OLD_R + i, delta)
        ));

        // WARNING: Crucial for security
        assert!(reprs.length() == 3 + ell, e_wrong_output_len());
        new_representation_vec(reprs)
    }

    /// The transformation function $f$ for the key rotation relation.
    ///
    /// Given the statement, outputs:
    /// ```
    /// [
    ///   H,
    ///   new_ek,
    ///   ek,
    ///   new_R_i for i in [1..num_chunks]
    /// ]
    /// ```
    fun f(_stmt: &Statement<KeyRotation>): RepresentationVec {
        // WARNING: We do not re-assert the stmt is well-formed anymore here, since wherever the transformation function
        // is called, so is the homomorphism, so the check will be done.

        let ell = get_num_available_chunks();
        let idx_r_new_start = get_start_idx_for_new_R();

        let reprs = vector[
            // H
            repr_point(IDX_H),
            // new_ek
            repr_point(IDX_EK_NEW),
            // ek
            repr_point(IDX_EK),
        ];

        // new_R_i for each chunk
        reprs.append(vector::range(0, ell).map(|i|
            repr_point(idx_r_new_start + i)
        ));

        // Note: Not needed for security, since a mismatched f(X) length will be caught in the verifier. But good practice
        // for catching mistakes *early* when implementing your f(X).
        assert!(reprs.length() == 3 + ell, e_wrong_output_len());
        new_representation_vec(reprs)
    }

    /// Returns true if the proof verifies, false otherwise.
    fun verify(self: &KeyRotationSession, stmt: &Statement<KeyRotation>, proof: &Proof): bool {
        assert_key_rotation_statement_is_well_formed(stmt);

        sigma_protocol::verify(
            new_domain_separator(@aptos_experimental, chain_id::get(), PROTOCOL_ID, bcs::to_bytes(self)),
            |_X, w| psi(_X, w),
            |_X| f(_X),
            stmt,
            proof
        )
    }

    /// Asserts that a key rotation proof verifies
    public(friend) fun assert_verifies(self: &KeyRotationSession, stmt: &Statement<KeyRotation>, proof: &Proof) {
        assert!(self.verify(stmt, proof), error::invalid_argument(E_INVALID_KEY_ROTATION_PROOF));
    }

    //
    // Tests
    //

    #[test_only]
    fun key_rotation_session_for_testing(): KeyRotationSession {
        let (sender, token_type) = setup_test_environment();
        KeyRotationSession { sender: signer::address_of(&sender), token_type, num_chunks: get_num_available_chunks() }
    }

    #[test_only]
    /// Creates a key rotation proof (for testing)
    public fun prove(self: &KeyRotationSession, stmt: &Statement<KeyRotation>, witn: &Witness): Proof {
        let (proof, _) = sigma_protocol::prove(
            new_domain_separator(@aptos_experimental, chain_id::get(), PROTOCOL_ID, bcs::to_bytes(self)),
            |_X, w| psi(_X, w),
            stmt,
            witn
        );

        proof
    }

    #[test_only]
    /// Computes the key rotation statement and witness from actual keys and balance.
    /// Returns (statement, witness, compressed_new_ek, compressed_new_R).
    public fun compute_statement_and_witness_from_keys_and_old_ctxt(
        old_dk: &Scalar,
        new_dk: &Scalar,
        compressed_old_ek: CompressedRistretto,
        compressed_old_R: &vector<CompressedRistretto>,
    ): (Statement<KeyRotation>, Witness, CompressedRistretto, vector<CompressedRistretto>) {
        let old_ek = compressed_old_ek.point_decompress();

        // Compute delta = old_dk * new_dk^{-1} (since ek = dk^{-1} * H, new_ek = delta * old_ek)
        let new_dk_inv = new_dk.scalar_invert().extract();
        let delta = old_dk.scalar_mul(&new_dk_inv);
        let delta_inv = delta.scalar_invert().extract();

        // Compute new_ek = delta * old_ek
        let new_ek = old_ek.point_mul(&delta);
        let compressed_new_ek = new_ek.point_compress();

        // Compute new_R = delta * old_R
        let old_R = compressed_old_R.map_ref(|r| r.point_decompress());
        let new_R = old_R.map_ref(|r| r.point_mul(&delta));
        let compressed_new_R = compress_points(&new_R);

        let stmt = new_key_rotation_statement(
            compressed_old_ek,
            compressed_new_ek,
            compressed_old_R,
            &compressed_new_R,
        );
        let witn = new_key_rotation_witness(*old_dk, delta, delta_inv);

        (stmt, witn, compressed_new_ek, compressed_new_R)
    }

    #[test_only]
    /// Generates a random valid statement-witness pair for testing.
    fun random_valid_statement_witness_pair(): (Statement<KeyRotation>, Witness) {
        let ell = get_num_available_chunks();
        let (dk, compressed_ek) = generate_twisted_elgamal_keypair();
        let new_dk = random_scalar();
        let old_R = vector::range(0, ell).map(|_| random_point());
        let compressed_old_R = compress_points(&old_R);

        let (stmt, witn, _, _) = compute_statement_and_witness_from_keys_and_old_ctxt(
            &dk, &new_dk,
            compressed_ek,
            &compressed_old_R,
        );

        (stmt, witn)
    }

    #[test]
    /// Verifies that the homomorphism $\psi$ is implemented correctly by comparing
    /// against a manually computed evaluation.
    fun psi_correctness() {
        let (_X, w) = random_valid_statement_witness_pair();

        // Get statement components
        let ek = _X.get_point(IDX_EK);
        let new_ek = _X.get_point(IDX_EK_NEW);

        // Get witness components
        let dk = w.get(IDX_DK);
        let delta = w.get(IDX_DELTA);
        let delta_inv = w.get(IDX_DELTA_INV);

        // Compute expected psi output manually
        let expected_psi = vector[
            ek.point_mul(dk),           // dk * ek
            ek.point_mul(delta),        // delta * ek
            new_ek.point_mul(delta_inv), // delta_inv * new_ek
        ];

        // Add delta * old_R_i for each chunk
        let ell = get_num_available_chunks();
        vector::range(0, ell).for_each(|i| {
            let old_R_i = _X.get_point(START_IDX_OLD_R + i);
            expected_psi.push_back(old_R_i.point_mul(delta));
        });

        // Compute actual psi output via our implementation
        let actual_psi = evaluate_psi(|_X, w| psi(_X, w), &_X, &w);

        assert!(actual_psi.length() == 3 + ell, 2);
        assert!(equal_vec_points(&actual_psi, &expected_psi), 1);
    }

    #[test]
    /// Verifies that a correctly computed proof verifies.
    fun proof_correctness() {
        let (stmt, witn) = random_valid_statement_witness_pair();
        let ss = key_rotation_session_for_testing();
        ss.assert_verifies(&stmt, &ss.prove(&stmt, &witn));
    }

    #[test]
    #[expected_failure(abort_code=65537, location=aptos_experimental::sigma_protocol_fiat_shamir)]
    /// Verifies that an empty proof does not verify for a random statement.
    fun proof_soundness_against_random_statement() {
        let (stmt, _) = random_valid_statement_witness_pair();
        key_rotation_session_for_testing().assert_verifies(&stmt, &sigma_protocol_proof::empty());
    }

    #[test]
    #[expected_failure(abort_code=65537, location=aptos_experimental::sigma_protocol_fiat_shamir)]
    /// Verifies that an empty proof does not verify for a "zero" statement (all identity points).
    fun proof_soundness_against_zero_statement_and_empty_proof() {
        let ell = get_num_available_chunks();

        let stmt = new_key_rotation_statement(
            point_identity_compressed(),
            point_identity_compressed(),
            &compressed_identity_points(ell),
            &compressed_identity_points(ell),
        );

        key_rotation_session_for_testing().assert_verifies(&stmt, &sigma_protocol_proof::empty());
    }

    // ──────────────────────────────────────────────────────────────────
    // Transformation function f correctness
    // ──────────────────────────────────────────────────────────────────

    #[test]
    /// Verifies that the transformation function f is implemented correctly.
    fun f_correctness() {
        let ell = get_num_available_chunks();
        let (dk, compressed_ek) = generate_twisted_elgamal_keypair();
        let new_dk = random_scalar();
        let old_R = vector::range(0, ell).map(|_| random_point());
        let compressed_old_R = compress_points(&old_R);

        let (stmt, _witn, compressed_new_ek, compressed_new_R) =
            compute_statement_and_witness_from_keys_and_old_ctxt(
                &dk, &new_dk, compressed_ek, &compressed_old_R,
            );

        let _H = get_encryption_key_basepoint_compressed().point_decompress();
        let ek = compressed_ek.point_decompress();
        let new_ek = compressed_new_ek.point_decompress();
        let new_R = compressed_new_R.map(|r| r.point_decompress());

        let expected_f = vector[_H, new_ek, ek];
        expected_f.append(new_R);

        let actual_f = evaluate_f(|_X| f(_X), &stmt);
        assert!(actual_f.length() == 3 + ell, 2);
        assert!(equal_vec_points(&actual_f, &expected_f), 1);
    }

    // ──────────────────────────────────────────────────────────────────
    // Witness-dimension tests
    // ──────────────────────────────────────────────────────────────────

    #[test]
    /// Verify witness length matches the paper's k=3 for R_keyrot.
    fun witness_dimension_matches_paper() {
        let (stmt, witn) = random_valid_statement_witness_pair();
        let ell = get_num_available_chunks();

        assert!(witn.length() == 3, 1);
        assert!(stmt.get_points().length() == 3 + 2 * ell, 2);
    }

    // ──────────────────────────────────────────────────────────────────
    // Tamper-one-field soundness tests (witness)
    // ──────────────────────────────────────────────────────────────────

    #[test]
    #[expected_failure(abort_code=65537, location=aptos_experimental::sigma_protocol_key_rotation)]
    /// Tamper dk: proof generated with dk+1 must not verify.
    fun tamper_witness_dk() {
        let (stmt, witn) = random_valid_statement_witness_pair();
        let ss = key_rotation_session_for_testing();

        sigma_protocol_mutation_tests::tamper_witness(&mut witn, IDX_DK);
        let bad_proof = ss.prove(&stmt, &witn);

        ss.assert_verifies(&stmt, &bad_proof);
    }

    #[test]
    #[expected_failure(abort_code=65537, location=aptos_experimental::sigma_protocol_key_rotation)]
    /// Tamper delta: proof generated with delta+1 must not verify.
    fun tamper_witness_delta() {
        let (stmt, witn) = random_valid_statement_witness_pair();
        let ss = key_rotation_session_for_testing();

        sigma_protocol_mutation_tests::tamper_witness(&mut witn, IDX_DELTA);
        let bad_proof = ss.prove(&stmt, &witn);

        ss.assert_verifies(&stmt, &bad_proof);
    }

    #[test]
    #[expected_failure(abort_code=65537, location=aptos_experimental::sigma_protocol_key_rotation)]
    /// Tamper delta_inv: proof generated with delta_inv+1 must not verify.
    fun tamper_witness_delta_inv() {
        let (stmt, witn) = random_valid_statement_witness_pair();
        let ss = key_rotation_session_for_testing();

        sigma_protocol_mutation_tests::tamper_witness(&mut witn, IDX_DELTA_INV);
        let bad_proof = ss.prove(&stmt, &witn);

        ss.assert_verifies(&stmt, &bad_proof);
    }

    // ──────────────────────────────────────────────────────────────────
    // Homomorphism linearity test
    // ──────────────────────────────────────────────────────────────────

    #[test]
    /// Dynamic check that ψ is a homomorphism: ψ(w₁ + w₂) == ψ(w₁) + ψ(w₂).
    fun psi_is_homomorphism() {
        let (stmt, _) = random_valid_statement_witness_pair();
        let k = 3u64;

        let s1: vector<Scalar> = vector::range(0, k).map(|_| random_scalar());
        let s2: vector<Scalar> = vector::range(0, k).map(|_| random_scalar());
        let s_sum: vector<Scalar> = vector::range(0, k).map(|i| s1[i].scalar_add(&s2[i]));

        let w1 = new_secret_witness(s1);
        let w2 = new_secret_witness(s2);
        let w_sum = new_secret_witness(s_sum);

        let psi_w1 = evaluate_psi(|_X, w| psi(_X, w), &stmt, &w1);
        let psi_w2 = evaluate_psi(|_X, w| psi(_X, w), &stmt, &w2);
        let psi_sum = evaluate_psi(|_X, w| psi(_X, w), &stmt, &w_sum);

        vector::range(0, psi_sum.length()).for_each(|i| {
            assert!(psi_w1[i].point_add(&psi_w2[i]).point_equals(&psi_sum[i]), 1);
        });
    }

    // ──────────────────────────────────────────────────────────────────
    // Cross-statement replay tests
    // ──────────────────────────────────────────────────────────────────

    #[test]
    #[expected_failure(abort_code=65537, location=aptos_experimental::sigma_protocol_key_rotation)]
    /// Cross-statement replay: a proof for one key rotation must not verify against a different one.
    fun cross_statement_replay() {
        let (stmt_a, witn_a) = random_valid_statement_witness_pair();
        let (stmt_b, _) = random_valid_statement_witness_pair();

        let ss = key_rotation_session_for_testing();
        let proof_a = ss.prove(&stmt_a, &witn_a);

        ss.assert_verifies(&stmt_b, &proof_a);
    }

    // ──────────────────────────────────────────────────────────────────
    // Exhaustive mutation tests
    // ──────────────────────────────────────────────────────────────────

    #[test]
    /// Tamper each statement point individually; every mutation must cause rejection.
    fun mutate_all_statement_points() {
        let (stmt, witn) = random_valid_statement_witness_pair();
        let ss = key_rotation_session_for_testing();
        let proof = ss.prove(&stmt, &witn);

        let n = stmt.get_compressed_points().length();
        vector::range(0, n).for_each(|i| {
            let saved = sigma_protocol_mutation_tests::tamper_statement_point(&mut stmt, i);
            assert!(!ss.verify(&stmt, &proof), i);
            sigma_protocol_mutation_tests::restore_statement_point(&mut stmt, i, saved);
        });
    }

    #[test]
    /// Tamper each proof commitment A[i] individually; every mutation must cause rejection.
    fun mutate_every_commitment() {
        let (stmt, witn) = random_valid_statement_witness_pair();
        let ss = key_rotation_session_for_testing();
        let proof = ss.prove(&stmt, &witn);

        let m = proof.get_commitment().length();
        vector::range(0, m).for_each(|i| {
            let saved = sigma_protocol_mutation_tests::tamper_proof_commitment(&mut proof, i);
            assert!(!ss.verify(&stmt, &proof), i);
            sigma_protocol_mutation_tests::restore_proof_commitment(&mut proof, i, saved);
        });
    }

    #[test]
    /// Tamper each proof response sigma[j] individually; every mutation must cause rejection.
    fun mutate_every_response() {
        let (stmt, witn) = random_valid_statement_witness_pair();
        let ss = key_rotation_session_for_testing();
        let proof = ss.prove(&stmt, &witn);

        let k = proof.get_response_length();
        vector::range(0, k).for_each(|j| {
            let saved = sigma_protocol_mutation_tests::tamper_proof_response(&mut proof, j);
            assert!(!ss.verify(&stmt, &proof), j);
            sigma_protocol_mutation_tests::restore_proof_response(&mut proof, j, saved);
        });
    }

    #[test]
    /// Swap every adjacent pair of statement points; every swap must cause rejection.
    fun mutate_swap_every_adjacent_point_pair() {
        let (stmt, witn) = random_valid_statement_witness_pair();
        let ss = key_rotation_session_for_testing();
        let proof = ss.prove(&stmt, &witn);

        let n = stmt.get_compressed_points().length();
        vector::range(0, n - 1).for_each(|i| {
            sigma_protocol_mutation_tests::swap_statement_points(&mut stmt, i, i + 1);
            assert!(!ss.verify(&stmt, &proof), i);
            sigma_protocol_mutation_tests::swap_statement_points(&mut stmt, i, i + 1);
        });
    }

    // ──────────────────────────────────────────────────────────────────
    // Dimension mutation tests
    // ──────────────────────────────────────────────────────────────────

    #[test]
    #[expected_failure(abort_code=65537, location=aptos_experimental::sigma_protocol_key_rotation)]
    /// Extra commitment: psi output length mismatch.
    fun mutate_dimension_extra_commitment() {
        let (stmt, witn) = random_valid_statement_witness_pair();
        let ss = key_rotation_session_for_testing();
        let proof = ss.prove(&stmt, &witn);

        sigma_protocol_mutation_tests::append_to_proof_commitments(&mut proof);
        ss.assert_verifies(&stmt, &proof);
    }

    #[test]
    #[expected_failure(abort_code=65537, location=aptos_experimental::sigma_protocol_key_rotation)]
    /// Fewer commitments: psi output length mismatch.
    fun mutate_dimension_fewer_commitments() {
        let (stmt, witn) = random_valid_statement_witness_pair();
        let ss = key_rotation_session_for_testing();
        let proof = ss.prove(&stmt, &witn);

        sigma_protocol_mutation_tests::pop_from_proof_commitments(&mut proof);
        ss.assert_verifies(&stmt, &proof);
    }

    #[test]
    #[expected_failure(abort_code=65539, location=aptos_experimental::sigma_protocol_key_rotation)]
    /// Extra response: psi aborts on witness length.
    fun mutate_dimension_extra_response() {
        let (stmt, witn) = random_valid_statement_witness_pair();
        let ss = key_rotation_session_for_testing();
        let proof = ss.prove(&stmt, &witn);

        sigma_protocol_mutation_tests::append_to_proof_responses(&mut proof);
        ss.assert_verifies(&stmt, &proof);
    }

    #[test]
    #[expected_failure(abort_code=65539, location=aptos_experimental::sigma_protocol_key_rotation)]
    /// Fewer responses: psi aborts on witness length.
    fun mutate_dimension_fewer_responses() {
        let (stmt, witn) = random_valid_statement_witness_pair();
        let ss = key_rotation_session_for_testing();
        let proof = ss.prove(&stmt, &witn);

        sigma_protocol_mutation_tests::pop_from_proof_responses(&mut proof);
        ss.assert_verifies(&stmt, &proof);
    }
}
