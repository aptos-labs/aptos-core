/// # The key rotation NP relation
///
/// A ZKPoK of having rotated an encryption key to a new one and re-encrypted (part of) a Twisted ElGamal ciphertext.
///
/// The NP relation, parameterized by the # of available balance chunks $m$, is:
///
/// \begin{align}
///   \mathcal{R}_\mathsf{keyrot}^\ell(
///     H, \mathsf{ek}, \widetilde{\mathsf{ek}}, \dot{\mathbf{D}}, \widetilde{\mathbf{D}};
///     \mathsf{dk}, \delta, \delta_\mathsf{inv}
///   ) = 1 \Leftrightarrow
///   \begin{cases}
///     H &= \mathsf{dk} \cdot \mathsf{ek}\\
///     \widetilde{\mathsf{ek}} &= \delta \cdot \mathsf{ek}\\
///     \mathsf{ek} &= \delta_\mathsf{inv} \cdot \widetilde{\mathsf{ek}}\\
///     \widetilde{D}_i &= \delta \cdot \dot{D}_i, \forall i \in [\ell];
///   \end{cases}
/// \end{align}
///
/// This can be framed as a homomorphism check:
///
/// \begin{align}
///   \psi(\mathsf{dk}, \delta, \delta_\mathsf{inv})  =  f(H, \mathsf{ek}, \widetilde{\mathsf{ek}}, \dot{\mathbf{D}}, \widetilde{\mathbf{D}})
/// \end{align}
///
/// where:
///
///   1. The homomorphism $\psi$ is:
///
///     \begin{align}
///       \psi(\mathsf{dk}, \delta, \delta_\mathsf{inv}) = \begin{pmatrix}
///         \mathsf{dk} \cdot \mathsf{ek}\\
///         \delta \cdot \mathsf{ek}\\
///         \delta_\mathsf{inv} \cdot \widetilde{\mathsf{ek}}\\
///         \delta \cdot \dot{D}_i, \forall i \in [\ell]\\
///       \end{pmatrix}
///     \end{align}
///
///   2. The transformation function $f$ is:
///
///     \begin{align}
///       f(H, \mathsf{ek}, \widetilde{\mathsf{ek}}, \dot{\mathbf{D}}, \widetilde{\mathbf{D}}) = \begin{pmatrix}
///         H, \widetilde{\mathsf{ek}}, \mathsf{ek}, \widetilde{\mathbf{D}}
///       \end{pmatrix}
///     \end{align}
///
module aptos_experimental::sigma_protocol_key_rotation {
    use std::bcs;
    use std::error;
    use std::signer;
    use std::vector;
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
    use aptos_std::ristretto255::{basepoint_H, scalar_invert};
    #[test_only]
    use aptos_framework::account;
    #[test_only]
    use aptos_framework::fungible_asset;
    #[test_only]
    use aptos_experimental::ristretto255_twisted_elgamal::get_encryption_key_basepoint_compressed;
    #[test_only]
    use aptos_experimental::sigma_protocol_homomorphism::evaluate_psi;
    #[test_only]
    use aptos_experimental::sigma_protocol_proof;
    #[test_only]
    use aptos_experimental::sigma_protocol_utils::{equal_vec_points, points_clone};
    #[test_only]
    use aptos_experimental::sigma_protocol_utils::compress_points;

    //
    // Constants
    //

    /// Protocol ID used for domain separation
    const PROTOCOL_ID: vector<u8> = b"AptosConfidentialAsset/KeyRotationV1";

    //
    // Statement point indices (matches the order in the NP relation: H, ek, new_ek, old_D, new_D)
    //

    /// Index of $H$ in the statement's points vector.
    const IDX_H: u64 = 0;
    /// Index of $\mathsf{ek}$ (old encryption key) in the statement's points vector.
    const IDX_EK: u64 = 1;
    /// Index of $\widetilde{\mathsf{ek}}$ (new encryption key) in the statement's points vector.
    const IDX_EK_NEW: u64 = 2;
    /// The old D values ($\dot{D}_i$ ) occupy indices 3 to 3 + (num_chunks - 1), inclusive.
    ///
    /// Note: The new D values ($\widetilde{D}_i$) occupy indices 3 + num_chunks to 3 + (2*num_chunks - 1), inclusive.
    /// A `get_start_idx_for_new_D(num_chunks)` function can be used to fetch the 3 + num_chunks starting index.
    const START_IDX_OLD_D: u64 = 3;

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

    /// The expected number of points in a key rotation statement is 3 + 2 * num_chunks, with num_chunks > 0.
    const E_WRONG_NUM_POINTS: u64 = 1;
    /// The expected number of scalars in a key rotation statement is 0.
    const E_WRONG_NUM_SCALARS: u64 = 2;
    /// The expected number of scalars in a key rotation witness is 3.
    const E_WRONG_WITNESS_LEN: u64 = 3;
    /// The expected number of points in the homomorphism & transformation function output is 3 + num_chunks, with num_chunks > 0.
    const E_WRONG_OUTPUT_LEN: u64 = 4;
    /// The key rotation proof was invalid
    const E_INVALID_KEY_ROTATION_PROOF: u64 = 5;

    //
    // Structs
    //

    /// Used for domain separation
    /// TODO(Security): It'd be nice to add more here (like some sort of account TXN counter). I suspect that the
    ///   ciphertext randomness in the public statement would act as enough of a "session ID", but I would prefer
    ///   to avoid reasoning about that.
    struct KeyRotationSession has drop {
        sender: address,
        token_type: Object<Metadata>,
        num_chunks: u64,
    }

    //
    // Helper functions
    //

    /// Returns the number of chunks encoded in the statement.
    /// The statement has 3 + 2*num_chunks points, so num_chunks = (num_points - 3) / 2.
    inline fun get_num_chunks(stmt: &Statement): u64 {
        let num_points = stmt.get_points().length();

        (num_points - 3) / 2
    }

    /// Returns the starting index of new_D values given the number of chunks.
    inline fun get_start_idx_for_new_D(num_chunks: u64): u64 {
        START_IDX_OLD_D + num_chunks
    }

    /// Ensures the statement is of the form:
    /// $\left(
    ///     H, \mathsf{ek}, \widetilde{\mathsf{ek}},
    ///     (\dot{D}_i)_{i \in [\ell]}),
    ///     (\widetilde{D}_i)_{i \in [\ell]}
    /// \right)$
    fun assert_key_rotation_statement_is_well_formed(
        stmt: &Statement,
    ) {
        let num_points = stmt.get_points().length();

        // The number of points must be odd and greater than 3
        assert!(num_points > 3, error::invalid_argument(E_WRONG_NUM_POINTS));
        assert!((num_points - 3) % 2 == 0, error::invalid_argument(E_WRONG_NUM_POINTS));

        // There should be no scalars
        assert!(stmt.get_scalars().length() == 0, error::invalid_argument(E_WRONG_NUM_SCALARS));
    }

    //
    // Public functions
    //

    public fun new_session(sender: &signer, token_type: Object<Metadata>, num_chunks: u64): KeyRotationSession {
        KeyRotationSession {
            sender: signer::address_of(sender),
            token_type,
            num_chunks,
        }
    }

    /// Creates a new key rotation statement.
    /// The order matches the NP relation: $(H, \mathsf{ek}, \widetilde{\mathsf{ek}}, \dot{\mathbf{D}}, \widetilde{\mathbf{D}})$.
    /// Note that the # of chunks is inferred from the sizes of the old and new balance ciphertexts.
    ///
    /// @param compressed_H: Compressed form of h
    /// @param _H: The hash-to-point base (= dk * ek)
    ///
    /// @param compressed_ek: Compressed form of ek
    /// @param ek: The old encryption key
    ///
    /// @param compressed_new_ek: Compressed form of new_ek
    /// @param new_ek: The new encryption key
    ///
    /// @param compressed_old_D: Compressed forms of old_D
    /// @param old_D: The old D values from the ciphertext (num_chunks elements)
    ///
    /// @param compressed_new_D: Compressed forms of d_new
    /// @param new_D: The new D values after re-encryption (num_chunks elements, must match old_D length)
    public fun new_key_rotation_statement(
        compressed_H: CompressedRistretto, _H: RistrettoPoint,
        compressed_ek: CompressedRistretto, ek: RistrettoPoint,
        compressed_new_ek: CompressedRistretto, new_ek: RistrettoPoint,
        compressed_old_D: vector<CompressedRistretto>, old_D: vector<RistrettoPoint>,
        compressed_new_D: vector<CompressedRistretto>, new_D: vector<RistrettoPoint>,
        num_chunks: u64,
    ): Statement {
        assert!(num_chunks > 0, error::invalid_argument(E_WRONG_NUM_POINTS));
        // assert all the D-component vectors are of equal size, matching num_chunks > 0
        assert!(compressed_old_D.length() == old_D.length(), error::invalid_argument(E_WRONG_NUM_POINTS));
        assert!(compressed_new_D.length() == new_D.length(), error::invalid_argument(E_WRONG_NUM_POINTS));
        assert!(old_D.length() == new_D.length(), error::invalid_argument(E_WRONG_NUM_POINTS));
        assert!(old_D.length() == num_chunks, error::invalid_argument(E_WRONG_NUM_POINTS));

        let points = vector[_H, ek, new_ek];
        points.append(old_D);
        points.append(new_D);

        let compressed_points = vector[compressed_H, compressed_ek, compressed_new_ek];
        compressed_points.append(compressed_old_D);
        compressed_points.append(compressed_new_D);

        let stmt = new_statement(points, compressed_points, vector[]);
        assert_key_rotation_statement_is_well_formed(&stmt);
        stmt
    }

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
    ///   delta * old_D_i,   // should equal new_D_i, for i in [num_chunks]
    /// ]
    /// ```
    public fun psi(_stmt: &Statement, w: &Witness): RepresentationVec {
        // WARNING: Crucial for security
        assert_key_rotation_statement_is_well_formed(_stmt);
        // WARNING: Crucial for security
        assert!(w.length() == 3, error::invalid_argument(E_WRONG_WITNESS_LEN));

        let dk = *w.get(IDX_DK);
        let delta = *w.get(IDX_DELTA);
        let delta_inv = *w.get(IDX_DELTA_INV);

        // Build the representation vector
        let reprs = vector[
            // dk * ek
            new_representation(vector[IDX_EK], vector[dk]),
            // delta * ek
            new_representation(vector[IDX_EK], vector[delta]),
            // delta_inv * new_ek
            new_representation(vector[IDX_EK_NEW], vector[delta_inv]),
        ];

        // delta * old_D_i for each chunk
        let num_chunks = get_num_chunks(_stmt);
        vector::range(0, num_chunks).for_each(|i| {
            reprs.push_back(
                new_representation(vector[START_IDX_OLD_D + i], vector[delta])
            );
        });

        let repr_vec = new_representation_vec(reprs);
        let expected_output_len = 3 + num_chunks;

        // WARNING: Crucial for security
        assert!(repr_vec.length() == expected_output_len, error::invalid_argument(E_WRONG_OUTPUT_LEN));

        repr_vec
    }

    /// The transformation function $f$ for the key rotation relation.
    ///
    /// Given the statement, outputs:
    /// ```
    /// [
    ///   H,
    ///   new_ek,
    ///   ek,
    ///   new_D_i for i in [num_chunks]
    /// ]
    /// ```
    public fun f(_stmt: &Statement): RepresentationVec {
        // WARNING: We do not re-assert the stmt is well-formed anymore here, since wherever the transformation function
        // is called, so is the homomorphism, so the check will be done.

        let num_chunks = get_num_chunks(_stmt);
        let idx_d_new_start = get_start_idx_for_new_D(num_chunks);

        let reprs = vector[
            // H
            new_representation(vector[IDX_H], vector[ristretto255::scalar_one()]),
            // new_ek
            new_representation(vector[IDX_EK_NEW], vector[ristretto255::scalar_one()]),
            // ek
            new_representation(vector[IDX_EK], vector[ristretto255::scalar_one()]),
        ];

        // new_D_i for each chunk
        vector::range(0, num_chunks).for_each(|i| {
            reprs.push_back(
                new_representation(vector[idx_d_new_start + i], vector[ristretto255::scalar_one()])
            );
        });

        let repr_vec = new_representation_vec(reprs);
        let expected_output_len = 3 + num_chunks;

        // WARNING: Crucial for security
        assert!(repr_vec.length() == expected_output_len, error::invalid_argument(E_WRONG_OUTPUT_LEN));

        repr_vec
    }

    /// Asserts that a key rotation proof verifies
    public fun assert_verifies(session: &KeyRotationSession, stmt: &Statement, proof: &Proof, num_chunks: u64) {
        // TODO: Ideally, we should do this via a type-safe KeyRotationStatement?
        assert!(get_num_chunks(stmt) == num_chunks, error::invalid_argument(E_INVALID_KEY_ROTATION_PROOF));

        let success = sigma_protocol::verify(
            new_domain_separator(PROTOCOL_ID, bcs::to_bytes(session)),
            // TODO(Ugly): Change `|_X, w| psi(_X, w)` to `psi` and `|_X| f(_X)` to `f` when public structs ship and allow this.
            |_X, w| psi(_X, w),
            |_X| f(_X),
            stmt,
            proof
        );

        assert!(success, error::invalid_argument(E_INVALID_KEY_ROTATION_PROOF));
    }

    //
    // Tests
    //

    #[test_only]
    /// Returns a dummy session used for testing
    /// WARNING: Can only be called once because it calls `create_fungible_asset`!
    fun key_rotation_session_for_testing(num_chunks: u64): KeyRotationSession {
        let sender = account::create_signer_for_test(@0x1);
        let (_, _, _, _, token_type) = fungible_asset::create_fungible_asset(&sender);

        KeyRotationSession {
            sender: signer::address_of(&sender),
            token_type,
            num_chunks,
        }
    }

    #[test_only]
    /// Creates a key rotation proof (for testing)
    public fun prove(session: &KeyRotationSession, stmt: &Statement, witn: &Witness): Proof {
        let (proof, _) = sigma_protocol::prove(
            new_domain_separator(PROTOCOL_ID, bcs::to_bytes(session)),
            // TODO(Ugly): Change `|_X, w| psi(_X, w)` to `psi` when public structs ship and allow this.
            |_X, w| psi(_X, w),
            stmt,
            witn
        );

        proof
    }

    #[test_only]
    /// Computes the key rotation statement and witness from actual keys and balance.
    /// Returns (statement, witness, compressed_new_ek, new_D, compressed_new_D).
    public fun compute_statement_and_witness_from_keys_and_old_ctxt(
        old_dk: &Scalar,
        new_dk: &Scalar,
        compressed_old_ek: CompressedRistretto,
        old_ek: RistrettoPoint,
        compressed_old_D: vector<CompressedRistretto>,
        old_D: vector<RistrettoPoint>,
        num_chunks: u64,
    ): (Statement, Witness, CompressedRistretto, vector<RistrettoPoint>, vector<CompressedRistretto>) {
        let compressed_gen_H = get_encryption_key_basepoint_compressed();
        let gen_H = ristretto255::point_decompress(&compressed_gen_H);

        // Compute delta = old_dk * new_dk^{-1} (since ek = dk^{-1} * H, new_ek = delta * old_ek)
        let new_dk_inv = ristretto255::scalar_invert(new_dk).extract();
        let delta = ristretto255::scalar_mul(old_dk, &new_dk_inv);
        let delta_inv = ristretto255::scalar_invert(&delta).extract();

        // Compute new_ek = delta * old_ek
        let new_ek = ristretto255::point_mul(&old_ek, &delta);
        let compressed_new_ek = ristretto255::point_compress(&new_ek);

        // Compute new_D = delta * old_D
        let new_D = old_D.map_ref(|d| ristretto255::point_mul(d, &delta));
        let compressed_new_D = compress_points(&new_D);

        let stmt = new_key_rotation_statement(
            compressed_gen_H, gen_H,
            compressed_old_ek, old_ek,
            compressed_new_ek, new_ek,
            compressed_old_D, old_D,
            compressed_new_D, points_clone(&new_D),
            num_chunks,
        );
        let witn = new_key_rotation_witness(*old_dk, delta, delta_inv);

        (stmt, witn, compressed_new_ek, new_D, compressed_new_D)
    }

    #[test_only]
    /// Generates a random valid statement-witness pair for testing.
    fun random_valid_statement_witness_pair(num_chunks: u64): (Statement, Witness) {
        let dk = ristretto255::random_scalar();
        let new_dk = ristretto255::random_scalar();
        let ek = ristretto255::point_mul(&basepoint_H(), &scalar_invert(&dk).extract());
        let old_D = vector::range(0, num_chunks).map(|_| ristretto255::random_point());

        let (stmt, witn, _, _, _) = compute_statement_and_witness_from_keys_and_old_ctxt(
            &dk, &new_dk,
            ristretto255::point_compress(&ek), ek,
            compress_points(&old_D), old_D,
            num_chunks,
        );

        (stmt, witn)
    }

    #[test]
    /// Verifies that the homomorphism $\psi$ is implemented correctly by comparing
    /// against a manually computed evaluation.
    fun psi_correctness() {
        let (_X, w) = random_valid_statement_witness_pair(8);

        // Get statement components
        let ek = _X.get_point(IDX_EK);
        let new_ek = _X.get_point(IDX_EK_NEW);

        // Get witness components
        let dk = w.get(IDX_DK);
        let delta = w.get(IDX_DELTA);
        let delta_inv = w.get(IDX_DELTA_INV);

        // Compute expected psi output manually
        let expected_psi = vector[
            ristretto255::point_mul(ek, dk),           // dk * ek
            ristretto255::point_mul(ek, delta),        // delta * ek
            ristretto255::point_mul(new_ek, delta_inv), // delta_inv * new_ek
        ];

        // Add delta * old_D_i for each chunk
        let num_chunks = get_num_chunks(&_X);
        vector::range(0, num_chunks).for_each(|i| {
            let old_D_i = _X.get_point(START_IDX_OLD_D + i);
            expected_psi.push_back(ristretto255::point_mul(old_D_i, delta));
        });

        // Compute actual psi output via our implementation
        // TODO(Ugly): Change `|_X, w| psi(_X, w)` to `psi` when public structs ship and allow this.
        let actual_psi = evaluate_psi(|_X, w| psi(_X, w), &_X, &w);

        assert!(equal_vec_points(&actual_psi, &expected_psi), 1);
    }

    #[test]
    /// Verifies that a correctly computed proof verifies.
    fun proof_correctness() {
        let num_chunks = 8;
        let (stmt, witn) = random_valid_statement_witness_pair(num_chunks);

        let ss = key_rotation_session_for_testing(num_chunks);

        let proof = prove(&ss, &stmt, &witn);

        assert_verifies(&ss, &stmt, &proof, num_chunks);
    }

    #[test]
    #[expected_failure(abort_code=65537, location=aptos_experimental::sigma_protocol_fiat_shamir)]
    /// Verifies that an empty proof does not verify for a random statement.
    fun proof_soundness_against_random_statement() {
        let num_chunks = 8;
        let (stmt, _) = random_valid_statement_witness_pair(num_chunks);

        let proof = sigma_protocol_proof::empty();

        assert_verifies(&key_rotation_session_for_testing(num_chunks), &stmt, &proof, num_chunks);
    }

    #[test]
    #[expected_failure(abort_code=65537, location=aptos_experimental::sigma_protocol_fiat_shamir)]
    /// Verifies that an empty proof does not verify for an "empty" statement (all identity points).
    fun proof_soundness_against_empty_statement_and_empty_proof() {
        let num_chunks = 8;

        // Create identity points for old_D and d_new
        let old_D = vector::range(0, num_chunks).map(|_| ristretto255::point_identity());
        let compressed_old_D = vector::range(0, num_chunks).map(|_| ristretto255::point_identity_compressed());
        let new_D = vector::range(0, num_chunks).map(|_| ristretto255::point_identity());
        let compressed_new_D = vector::range(0, num_chunks).map(|_| ristretto255::point_identity_compressed());

        let _H = ristretto255::point_identity();
        let compressed_H = ristretto255::point_identity_compressed();
        let ek = ristretto255::point_identity();
        let compressed_ek = ristretto255::point_identity_compressed();
        let new_ek = ristretto255::point_identity();
        let compressed_new_ek = ristretto255::point_identity_compressed();

        let stmt = new_key_rotation_statement(
            compressed_H, _H,
            compressed_ek, ek,
            compressed_new_ek, new_ek,
            compressed_old_D, old_D,
            compressed_new_D, new_D,
            num_chunks
        );

        let proof = sigma_protocol_proof::empty();

        assert_verifies(&key_rotation_session_for_testing(num_chunks), &stmt, &proof, num_chunks);
    }
}
