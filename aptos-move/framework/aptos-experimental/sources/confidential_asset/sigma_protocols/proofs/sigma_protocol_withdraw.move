/// # The withdrawal NP relation ($\mathcal{R}^{-}_\mathsf{withdraw}$)
///
/// $\def\old#1{{\color{red}{\dot{#1}}}}\def\new#1{{\color{teal}{\widetilde{#1}}}}\def\opt#1{{\color{orange}{\boldsymbol{[}}} #1 {\color{orange}{\boldsymbol{]}}}}$
///
/// A ZKPoK of a correct balance update when publicly withdrawing amount $v$ from an old available balance.
/// Also used for normalization (where $v = 0$) with a different protocol ID.
///
/// ## Notation
///
/// - $\old{x}$ denotes a stale/old ciphertext component; $\new{x}$ denotes a fresh/new one.
/// - $\opt{\cdot}$ denotes components present only when an auditor is set.
///   Auditor components are placed at the **end** of the statement so that the common prefix is
///   identical in both cases. psi/f detect auditor presence by checking the statement length.
/// - $\langle \mathbf{x}, \mathbf{y} \rangle = \sum_i x_i \cdot y_i$ denotes the inner product.
/// - $\mathbf{B} = (B^0, B^1, \ldots)$ where $B = 2^{16}$ is the positional weight vector for chunk encoding.
/// - $\ell$: number of available balance chunks.
///
/// ## The relation
///
/// $$
/// \mathcal{R}^{-}_\mathsf{withdraw}\left(\begin{array}{l}
///     G, H, \mathsf{ek},
///       \old{\mathbf{P}}, \old{\mathbf{R}}, \new{\mathbf{P}}, \new{\mathbf{R}},
///       \opt{\mathsf{ek}^\mathsf{aid}, \new{\mathbf{R}}^\mathsf{aid}}
///       \textbf{;}\\
///     \mathsf{dk}, \new{\mathbf{a}}, \new{\mathbf{r}}
///       \textbf{;}\; v
/// \end{array}\right) = 1
/// \Leftrightarrow
/// \left\{\begin{array}{r@{\,\,}l@{\quad}l}
///     H &= \mathsf{dk} \cdot \mathsf{ek}\\
///     \new{P}_i &= \new{a}_i \cdot G + \new{r}_i \cdot H, &\forall i \in [\ell]\\
///     \new{R}_i &= \new{r}_i \cdot \mathsf{ek}, &\forall i \in [\ell]\\
///     \opt{\new{R}^\mathsf{aid}_i} &\opt{= \new{r}_i \cdot \mathsf{ek}^\mathsf{aid},}
///       &\opt{\forall i \in [\ell]}\\
///     \langle \mathbf{B}, \old{\mathbf{P}} \rangle - v \cdot G
///       &= \mathsf{dk} \cdot \langle \mathbf{B}, \old{\mathbf{R}} \rangle
///       + \langle \mathbf{B}, \new{\mathbf{a}} \rangle \cdot G\\
/// \end{array}\right.
/// $$
///
/// Note: $v$ is a **public** scalar in the statement (not in the witness). It appears in $f$ but not in $\psi$.
///
/// ## Homomorphism
///
/// This can be framed as a homomorphism check $\psi(\mathbf{w}) = f(\mathbf{X})$ where
/// $\mathbf{w} = (\mathsf{dk}, \new{\mathbf{a}}, \new{\mathbf{r}})$ is the witness
/// and $\mathbf{X}$ is the statement (including public scalar $v$).
///
///   1. The homomorphism $\psi$ is:
///
/// $$
/// \psi(\mathsf{dk}, \new{\mathbf{a}}, \new{\mathbf{r}}) = \begin{pmatrix}
///     \mathsf{dk} \cdot \mathsf{ek}\\
///     \new{a}_i \cdot G + \new{r}_i \cdot H, &\forall i \in [\ell]\\
///     \new{r}_i \cdot \mathsf{ek}, &\forall i \in [\ell]\\
///     \opt{\new{r}_i \cdot \mathsf{ek}^\mathsf{aid}, \;\forall i \in [\ell]}\\
///     \mathsf{dk} \cdot \langle \mathbf{B}, \old{\mathbf{R}} \rangle
///       + \langle \mathbf{B}, \new{\mathbf{a}} \rangle \cdot G\\
/// \end{pmatrix}
/// $$
///
///   2. The transformation function $f$ is:
///
/// $$
/// f(\mathbf{X}) = \begin{pmatrix}
///     H\\
///     \new{P}_i, &\forall i \in [\ell]\\
///     \new{R}_i, &\forall i \in [\ell]\\
///     \opt{\new{R}^\mathsf{aid}_i, \;\forall i \in [\ell]}\\
///     \langle \mathbf{B}, \old{\mathbf{P}} \rangle - v \cdot G\\
/// \end{pmatrix}
/// $$
///
module aptos_experimental::sigma_protocol_withdraw {
    use std::bcs;
    use std::error;
    use std::signer;
    use std::option::Option;
    use std::vector;
    use aptos_std::ristretto255::{Self, RistrettoPoint, Scalar, CompressedRistretto};
    use aptos_framework::fungible_asset::Metadata;
    use aptos_framework::object::Object;
    use aptos_experimental::sigma_protocol;
    use aptos_experimental::sigma_protocol_proof::Proof;
    use aptos_experimental::sigma_protocol_fiat_shamir::new_domain_separator;
    use aptos_experimental::sigma_protocol_witness::{Witness, new_secret_witness};
    use aptos_experimental::sigma_protocol_statement::{Statement, new_statement};
    use aptos_experimental::confidential_available_balance;
    use aptos_experimental::sigma_protocol_representation::new_representation;
    use aptos_experimental::sigma_protocol_representation_vec::{RepresentationVec, new_representation_vec};
    #[test_only]
    use aptos_framework::account;
    #[test_only]
    use aptos_framework::fungible_asset;
    #[test_only]
    use aptos_experimental::ristretto255_twisted_elgamal::{
        get_encryption_key_basepoint_compressed, generate_twisted_elgamal_keypair
    };
    #[test_only]
    use aptos_experimental::sigma_protocol_homomorphism::evaluate_psi;
    #[test_only]
    use aptos_experimental::sigma_protocol_proof;
    #[test_only]
    use aptos_experimental::sigma_protocol_utils::{compress_points, equal_vec_points, points_clone};

    //
    // Constants
    //

    /// Protocol ID for withdrawal proofs
    const WITHDRAWAL_PROTOCOL_ID: vector<u8> = b"AptosConfidentialAsset/WithdrawalV1";
    /// Protocol ID for normalization proofs (same psi/f, but distinct domain separation)
    const NORMALIZATION_PROTOCOL_ID: vector<u8> = b"AptosConfidentialAsset/NormalizationV1";

    //
    // Statement point indices (common prefix — auditor components appended at end)
    //

    /// Index of $G$ (the Ristretto255 basepoint) in the statement.
    const IDX_G: u64 = 0;
    /// Index of $H$ (the encryption key basepoint) in the statement.
    const IDX_H: u64 = 1;
    /// Index of $\mathsf{ek}$ (the sender's encryption key) in the statement.
    const IDX_EK: u64 = 2;
    /// old_P values start at index 3. old_R starts at 3 + ℓ. new_P at 3 + 2ℓ. new_R at 3 + 3ℓ.
    /// If auditor present: ek_aud at 3 + 4ℓ, then new_R_aud at 3 + 4ℓ + 1.
    const START_IDX_OLD_P: u64 = 3;

    //
    // Witness scalar indices
    //

    /// Index of $\mathsf{dk}$ in the witness.
    const IDX_DK: u64 = 0;
    /// new_a[0..ℓ-1] starts at index 1. new_r[0..ℓ-1] starts at 1 + ℓ.

    //
    // Error codes
    //

    /// Statement has wrong number of points.
    const E_WRONG_NUM_POINTS: u64 = 1;
    /// Statement scalars vector must have exactly 1 element (the withdrawal amount v).
    const E_WRONG_NUM_SCALARS: u64 = 2;
    /// Witness has wrong length.
    const E_WRONG_WITNESS_LEN: u64 = 3;
    /// Homomorphism output has wrong length.
    const E_WRONG_OUTPUT_LEN: u64 = 4;
    /// The withdrawal/normalization proof was invalid.
    const E_INVALID_PROOF: u64 = 5;

    /// An error occurred in one of our tests.
    const E_TEST_INTERNAL: u64 = 1_000;

    //
    // Structs
    //

    /// Used for domain separation in the Fiat-Shamir transform.
    struct WithdrawSession has drop {
        sender: address,
        asset_type: Object<Metadata>,
        num_chunks: u64,
    }

    //
    // Helper functions
    //

    /// Returns the fixed number of balance chunks ℓ (= AVAILABLE_BALANCE_CHUNKS).
    inline fun get_num_chunks(): u64 {
        confidential_available_balance::get_num_chunks()
    }

    /// Determines whether the statement includes auditor components based on point count.
    /// Auditorless: 3 + 4ℓ points
    /// With auditor: 4 + 5ℓ points
    inline fun has_auditor(stmt: &Statement): bool {
        let ell = get_num_chunks();
        stmt.get_points().length() > 3 + 4 * ell
    }

    /// Returns the B^i powers for the chunk weighted-sum: B = 2^16.
    fun get_b_powers(ell: u64): vector<Scalar> {
        let b = ristretto255::new_scalar_from_u128(65536u128); // 2^16
        let powers = vector[ristretto255::scalar_one()]; // B^0 = 1
        let prev = ristretto255::scalar_one();
        let i = 1;
        while (i < ell) {
            prev = prev.scalar_mul(&b);
            powers.push_back(prev);
            i = i + 1;
        };
        powers
    }

    /// Validates that the statement has the correct structure.
    fun assert_withdraw_statement_is_well_formed(stmt: &Statement) {
        let num_points = stmt.get_points().length();
        let ell = get_num_chunks();

        let expected_num_points_no_aud = 3 + 4 * ell; // i.e., G, H, ek, old balance (2\ell), new balance (2\ell)
        let expected_num_points_with_aud = expected_num_points_no_aud + 1 + ell; // + auditor's EK and R-component
        assert!(
            num_points == expected_num_points_no_aud || num_points == expected_num_points_with_aud,
            error::invalid_argument(E_WRONG_NUM_POINTS)
        );

        // i.e., the transferred amount v
        assert!(stmt.get_scalars().length() == 1, error::invalid_argument(E_WRONG_NUM_SCALARS));
    }

    //
    // Public functions
    //

    public fun new_session(sender: &signer, asset_type: Object<Metadata>): WithdrawSession {
        WithdrawSession {
            sender: signer::address_of(sender),
            asset_type,
            num_chunks: get_num_chunks(),
        }
    }

    /// Creates a withdrawal statement, optionally including auditor components.
    ///
    /// Points (auditorless): [ G, H, ek, old_P[0..ℓ-1], old_R[0..ℓ-1], new_P[0..ℓ-1], new_R[0..ℓ-1] ]
    /// Points (w/ auditor):  [ ---------------------------- as above ------------------------------, ek_aud, new_R_aud]
    /// Scalars:              [ v ]
    ///
    /// For the auditorless case, pass `option::none()` for `compressed_ek_aud` and `ek_aud`,
    /// and empty vectors for `compressed_new_R_aud` and `new_R_aud`.
    public fun new_withdrawal_statement(
        compressed_G: CompressedRistretto, _G: RistrettoPoint,
        compressed_H: CompressedRistretto, _H: RistrettoPoint,
        compressed_ek: CompressedRistretto, ek: RistrettoPoint,
        compressed_old_P: vector<CompressedRistretto>, old_P: vector<RistrettoPoint>,
        compressed_old_R: vector<CompressedRistretto>, old_R: vector<RistrettoPoint>,
        compressed_new_P: vector<CompressedRistretto>, new_P: vector<RistrettoPoint>,
        compressed_new_R: vector<CompressedRistretto>, new_R: vector<RistrettoPoint>,
        compressed_ek_aud: Option<CompressedRistretto>, ek_aud: Option<RistrettoPoint>,
        compressed_new_R_aud: vector<CompressedRistretto>, new_R_aud: vector<RistrettoPoint>,
        v: Scalar,
    ): Statement {
        let points = vector[_G, _H, ek];
        points.append(old_P);
        points.append(old_R);
        points.append(new_P);
        points.append(new_R);

        let compressed_points = vector[compressed_G, compressed_H, compressed_ek];
        compressed_points.append(compressed_old_P);
        compressed_points.append(compressed_old_R);
        compressed_points.append(compressed_new_P);
        compressed_points.append(compressed_new_R);

        if (ek_aud.is_some()) {
            points.push_back(ek_aud.extract());
            points.append(new_R_aud);
            compressed_points.push_back(compressed_ek_aud.extract());
            compressed_points.append(compressed_new_R_aud);
        };

        let stmt = new_statement(points, compressed_points, vector[v]);
        assert_withdraw_statement_is_well_formed(&stmt);
        stmt
    }

    /// Creates a withdrawal witness: $(\mathsf{dk}, \new{a}_0, \ldots, \new{a}_{\ell-1}, \new{r}_0, \ldots, \new{r}_{\ell-1})$.
    public fun new_withdrawal_witness(dk: Scalar, new_a: vector<Scalar>, new_r: vector<Scalar>): Witness {
        assert!(new_a.length() == new_r.length(), error::invalid_argument(E_WRONG_WITNESS_LEN));

        let w = vector[dk];
        w.append(new_a);
        w.append(new_r);
        new_secret_witness(w)
    }

    /// The homomorphism $\psi$ for the withdrawal relation.
    ///
    /// Here, B = (B^0, B^1, …, B^{ℓ-1}) with B = 2^16 is the chunk weight vector (see module doc).
    ///
    /// Outputs (auditorless, m = 2 + 2ℓ):
    ///   1. dk · ek
    ///   2. new_a[i] · G + new_r[i] · H, for i ∈ [1..ℓ]
    ///   3. new_r[i] · ek, for i ∈ [1..ℓ]
    ///   4. dk · ⟨B, old_R⟩ + ⟨B, new_a⟩ · G
    ///
    /// With auditor (m = 2 + 3ℓ), inserts between 3 and 4:
    ///   3b. new_r[i] · ek_aud, for i ∈ [1..ℓ]
    public fun psi(stmt: &Statement, w: &Witness): RepresentationVec {
        // WARNING: Crucial for security
        assert_withdraw_statement_is_well_formed(stmt);

        let ell = get_num_chunks();
        let has_aud = has_auditor(stmt);
        let b_powers = get_b_powers(ell);

        // WARNING: Crucial for security
        let expected_witness_len = 1 + 2 * ell;
        assert!(w.length() == expected_witness_len, error::invalid_argument(E_WRONG_WITNESS_LEN));

        let dk = *w.get(IDX_DK);

        let reprs = vector[];

        // 1. dk · ek
        reprs.push_back(new_representation(vector[IDX_EK], vector[dk]));

        // 2. new_a[i] · G + new_r[i] · H, for i ∈ [1..ℓ]
        vector::range(0, ell).for_each(|i| {
            let new_a_i = *w.get(1 + i);
            let new_r_i = *w.get(1 + ell + i);
            reprs.push_back(new_representation(vector[IDX_G, IDX_H], vector[new_a_i, new_r_i]));
        });

        // 3. new_r[i] · ek, for i ∈ [1..ℓ]
        vector::range(0, ell).for_each(|i| {
            let new_r_i = *w.get(1 + ell + i);
            reprs.push_back(new_representation(vector[IDX_EK], vector[new_r_i]));
        });

        // 3b. (auditor only) new_r[i] · ek_aud, for i ∈ [1..ℓ]
        if (has_aud) {
            let idx_ek_aud = START_IDX_OLD_P + 4 * ell;
            vector::range(0, ell).for_each(|i| {
                let new_r_i = *w.get(1 + ell + i);
                reprs.push_back(new_representation(vector[idx_ek_aud], vector[new_r_i]));
            });
        };

        // 4. Balance equation: dk · ⟨B, old_R⟩ + ⟨B, new_a⟩ · G
        let idx_old_R_start = START_IDX_OLD_P + ell;

        let point_idxs = vector[];
        let scalars = vector[];

        // dk · B^i · old_R[i]
        vector::range(0, ell).for_each(|i| {
            point_idxs.push_back(idx_old_R_start + i);
            scalars.push_back(dk.scalar_mul(&b_powers[i]));
        });

        // new_a[i] · B^i · G
        vector::range(0, ell).for_each(|i| {
            let new_a_i = *w.get(1 + i);
            point_idxs.push_back(IDX_G);
            scalars.push_back(new_a_i.scalar_mul(&b_powers[i]));
        });

        reprs.push_back(new_representation(point_idxs, scalars));

        let repr_vec = new_representation_vec(reprs);
        let expected_output_len = if (has_aud) { 2 + 3 * ell } else { 2 + 2 * ell };

        // WARNING: Crucial for security
        assert!(repr_vec.length() == expected_output_len, error::invalid_argument(E_WRONG_OUTPUT_LEN));

        repr_vec
    }

    /// The transformation function $f$ for the withdrawal relation.
    ///
    /// Outputs (auditorless, m = 2 + 2ℓ):
    ///   1. H
    ///   2. new_P[i], for i ∈ [1..ℓ]
    ///   3. new_R[i], for i ∈ [1..ℓ]
    ///   4. ⟨B, old_P⟩ − v · G
    ///
    /// With auditor (m = 2 + 3ℓ), inserts between 3 and 4:
    ///   3b. new_R_aud[i], for i ∈ [1..ℓ]
    public fun f(stmt: &Statement): RepresentationVec {
        let ell = get_num_chunks();
        let has_aud = has_auditor(stmt);
        let b_powers = get_b_powers(ell);
        let v = stmt.get_scalars()[0];

        let idx_new_P_start = START_IDX_OLD_P + 2 * ell;
        let idx_new_R_start = START_IDX_OLD_P + 3 * ell;

        let reprs = vector[];

        // 1. H
        reprs.push_back(new_representation(vector[IDX_H], vector[ristretto255::scalar_one()]));

        // 2. new_P[i]
        vector::range(0, ell).for_each(|i| {
            reprs.push_back(new_representation(vector[idx_new_P_start + i], vector[ristretto255::scalar_one()]));
        });

        // 3. new_R[i]
        vector::range(0, ell).for_each(|i| {
            reprs.push_back(new_representation(vector[idx_new_R_start + i], vector[ristretto255::scalar_one()]));
        });

        // 3b. (auditor only) new_R_aud[i]
        if (has_aud) {
            let idx_new_R_aud_start = START_IDX_OLD_P + 4 * ell + 1; // +1 for ek_aud
            vector::range(0, ell).for_each(|i| {
                reprs.push_back(new_representation(
                    vector[idx_new_R_aud_start + i], vector[ristretto255::scalar_one()]
                ));
            });
        };

        // 4. ⟨B, old_P⟩ − v · G
        let point_idxs = vector[];
        let scalars = vector[];

        vector::range(0, ell).for_each(|i| {
            point_idxs.push_back(START_IDX_OLD_P + i);
            scalars.push_back(b_powers[i]);
        });

        point_idxs.push_back(IDX_G);
        scalars.push_back(v.scalar_neg());

        reprs.push_back(new_representation(point_idxs, scalars));

        new_representation_vec(reprs)
    }

    /// Asserts that a withdrawal proof verifies.
    public fun assert_verifies_withdrawal(session: &WithdrawSession, stmt: &Statement, proof: &Proof) {
        assert_withdraw_statement_is_well_formed(stmt);

        let success = sigma_protocol::verify(
            new_domain_separator(WITHDRAWAL_PROTOCOL_ID, bcs::to_bytes(session)),
            |_X, w| psi(_X, w),
            |_X| f(_X),
            stmt,
            proof
        );

        assert!(success, error::invalid_argument(E_INVALID_PROOF));
    }

    /// Asserts that a normalization proof verifies (same psi/f as withdrawal, different protocol ID).
    public fun assert_verifies_normalization(session: &WithdrawSession, stmt: &Statement, proof: &Proof) {
        assert_withdraw_statement_is_well_formed(stmt);

        let success = sigma_protocol::verify(
            new_domain_separator(NORMALIZATION_PROTOCOL_ID, bcs::to_bytes(session)),
            |_X, w| psi(_X, w),
            |_X| f(_X),
            stmt,
            proof
        );

        assert!(success, error::invalid_argument(E_INVALID_PROOF));
    }

    //
    // Tests
    //

    #[test_only]
    /// Returns a dummy session used for testing.
    fun withdraw_session_for_testing(): WithdrawSession {
        let sender = account::create_signer_for_test(@0x1);
        let (_, _, _, _, asset_type) = fungible_asset::create_fungible_asset(&sender);

        WithdrawSession {
            sender: signer::address_of(&sender),
            asset_type,
            num_chunks: get_num_chunks(),
        }
    }

    #[test_only]
    /// Creates a withdrawal proof (for testing).
    public fun prove_withdrawal(session: &WithdrawSession, stmt: &Statement, witn: &Witness): Proof {
        let (proof, _) = sigma_protocol::prove(
            new_domain_separator(WITHDRAWAL_PROTOCOL_ID, bcs::to_bytes(session)),
            |_X, w| psi(_X, w),
            stmt,
            witn
        );
        proof
    }

    #[test_only]
    /// Creates a normalization proof (for testing).
    public fun prove_normalization(session: &WithdrawSession, stmt: &Statement, witn: &Witness): Proof {
        let (proof, _) = sigma_protocol::prove(
            new_domain_separator(NORMALIZATION_PROTOCOL_ID, bcs::to_bytes(session)),
            |_X, w| psi(_X, w),
            stmt,
            witn
        );
        proof
    }

    #[test_only]
    /// Returns the raw components of a random valid statement-witness pair.
    /// Used by `random_valid_statement_witness_pair` (which assembles Statement/Witness)
    /// and by `psi_correctness` (which needs the raw points for an independent manual computation).
    fun random_valid_statement_witness_pair_internal(
        amount: u64, with_auditor: bool
    ): (
        RistrettoPoint,           // G
        RistrettoPoint,           // H
        RistrettoPoint,           // ek
        vector<RistrettoPoint>,   // old_P
        vector<RistrettoPoint>,   // old_R
        vector<RistrettoPoint>,   // new_P
        vector<RistrettoPoint>,   // new_R
        Option<RistrettoPoint>,   // ek_aud
        vector<RistrettoPoint>,   // new_R_aud
        Scalar,                   // v
        Scalar,                   // dk
        vector<Scalar>,           // new_a
        vector<Scalar>,           // new_r
    ) {
        let ell = get_num_chunks();

        // Generate sender keypair
        let (dk, compressed_ek) = generate_twisted_elgamal_keypair();

        // Generate optional auditor keypair
        let compressed_ek_aud = if (with_auditor) {
            let (_, aud_ek) = generate_twisted_elgamal_keypair();
            std::option::some(aud_ek)
        } else {
            std::option::none()
        };

        // Create old and new balances using the high-level API
        let old_amount = 1000u128;
        let new_amount = old_amount - (amount as u128);

        let old_randomness = confidential_available_balance::generate_balance_randomness();
        let old_balance = confidential_available_balance::new_from_amount(
            old_amount, &old_randomness, &compressed_ek, &compressed_ek_aud
        );
        let new_randomness = confidential_available_balance::generate_balance_randomness();
        let new_balance = confidential_available_balance::new_from_amount(
            new_amount, &new_randomness, &compressed_ek, &compressed_ek_aud
        );

        // Build raw points
        let _G = ristretto255::basepoint();
        let _H = get_encryption_key_basepoint_compressed().point_decompress();
        let ek = compressed_ek.point_decompress();

        let ek_aud = if (compressed_ek_aud.is_some()) {
            std::option::some(compressed_ek_aud.borrow().point_decompress())
        } else {
            std::option::none()
        };

        let old_P = points_clone(old_balance.get_P());
        let old_R = points_clone(old_balance.get_R());
        let new_P = points_clone(new_balance.get_P());
        let new_R = points_clone(new_balance.get_R());
        let new_R_aud = points_clone(new_balance.get_R_aud());

        let v = ristretto255::new_scalar_from_u64(amount);

        // Build witness scalars
        let new_a = confidential_available_balance::split_into_chunks(new_amount);
        let new_r = *new_randomness.scalars();

        (_G, _H, ek, old_P, old_R, new_P, new_R, ek_aud, new_R_aud, v, dk, new_a, new_r)
    }

    #[test_only]
    /// Generates a random valid statement-witness pair for testing.
    /// When `with_auditor` is true, includes auditor components in the statement.
    fun random_valid_statement_witness_pair(amount: u64, with_auditor: bool): (Statement, Witness) {
        let (_G, _H, ek,
            old_P, old_R,
            new_P, new_R,
            ek_aud, new_R_aud,
            v, dk, new_a, new_r
        ) = random_valid_statement_witness_pair_internal(amount, with_auditor);

        // Compress all points before moving originals into the statement
        let stmt = new_withdrawal_statement(
            _G.point_compress(), _G,
            _H.point_compress(), _H,
            ek.point_compress(), ek,
            compress_points(&old_P), old_P,
            compress_points(&old_R), old_R,
            compress_points(&new_P), new_P,
            compress_points(&new_R), new_R,
            if (ek_aud.is_some()) { std::option::some(ek_aud.borrow().point_compress()) } else { std::option::none() }, ek_aud,
            compress_points(&new_R_aud), new_R_aud,
            v,
        );

        let witn = new_withdrawal_witness(dk, new_a, new_r);

        (stmt, witn)
    }

    #[test_only]
    /// Verifies that `evaluate_psi` produces the same points as a manual computation using
    /// direct ristretto255 arithmetic, for both auditor and auditorless cases.
    ///
    /// The manual computation uses only the raw points/scalars from `_internal` — no `IDX_*`
    /// constants — so it is completely independent of the statement layout.
    fun psi_correctness(with_auditor: bool) {
        let ell = get_num_chunks();
        let (_G, _H, ek,
            old_P, old_R,
            new_P, new_R,
            ek_aud, new_R_aud,
            v, dk, new_a, new_r
        ) = random_valid_statement_witness_pair_internal(100, with_auditor);
        let b_powers = get_b_powers(ell);

        // Sanity check: ek = dk^{-1} * H
        assert!(_H.point_equals(&ek.point_mul(&dk)), error::internal(E_TEST_INTERNAL));

        // Build statement + witness for evaluate_psi (cloning points that the manual computation also needs)
        let stmt = new_withdrawal_statement(
            _G.point_compress(), _G.point_clone(),
            _H.point_compress(), _H.point_clone(),
            ek.point_compress(), ek.point_clone(),
            compress_points(&old_P), old_P,
            compress_points(&old_R), points_clone(&old_R), // old_R needed for manual computation
            compress_points(&new_P), new_P,
            compress_points(&new_R), new_R,
            if (ek_aud.is_some()) { std::option::some(ek_aud.borrow().point_compress()) }
            else { std::option::none() },
            if (ek_aud.is_some()) { std::option::some(ek_aud.borrow().point_clone()) }
            else { std::option::none() },
            compress_points(&new_R_aud), new_R_aud,
            v,
        );
        let witn = new_withdrawal_witness(dk, new_a, new_r);

        //
        // Manually compute the homomorphism using raw components (no IDX_* constants)
        //
        let manual_psi = vector[];

        // 1. dk · ek
        manual_psi.push_back(ek.point_mul(&dk));

        // 2. new_a[i] · G + new_r[i] · H, for i in [1..ell]
        vector::range(0, ell).for_each(|i| {
            manual_psi.push_back(ristretto255::double_scalar_mul(&new_a[i], &_G, &new_r[i], &_H));
        });

        // 3. new_r[i] · ek, for i in [1..ell]
        vector::range(0, ell).for_each(|i| {
            manual_psi.push_back(ek.point_mul(&new_r[i]));
        });

        // 3b. (auditor only) new_r[i] · ek_aud, for i in [1..ell]
        if (ek_aud.is_some()) {
            let ek_aud_pt = ek_aud.borrow();
            vector::range(0, ell).for_each(|i| {
                manual_psi.push_back(ek_aud_pt.point_mul(&new_r[i]));
            });
        };

        // 4. dk · ⟨B, old_R⟩ + ⟨B, new_a⟩ · G
        let dk_b_scalars: vector<Scalar> = vector::range(0, ell).map(|i| {
            dk.scalar_mul(&b_powers[i])
        });
        let dk_inner_b_old_R = ristretto255::multi_scalar_mul(&old_R, &dk_b_scalars);

        let inner_b_new_a = ristretto255::scalar_zero();
        vector::range(0, ell).for_each(|i| {
            inner_b_new_a = inner_b_new_a.scalar_add(&new_a[i].scalar_mul(&b_powers[i]));
        });
        let inner_b_new_a_times_G = _G.point_mul(&inner_b_new_a);

        manual_psi.push_back(dk_inner_b_old_R.point_add(&inner_b_new_a_times_G));

        //
        // Compare: implemented_psi vs manual_psi computation
        //
        let implemented_psi = evaluate_psi(|_X, w| psi(_X, w), &stmt, &witn);

        assert!(equal_vec_points(&implemented_psi, &manual_psi), 1);
    }

    #[test]
    fun psi_correctness_no_auditor() { psi_correctness(false); }

    #[test]
    fun psi_correctness_with_auditor() { psi_correctness(true); }

    #[test]
    fun proof_correctness_withdrawal() {
        let ss = withdraw_session_for_testing();
        vector[false, true].for_each(|with_auditor| {
            let (stmt, witn) = random_valid_statement_witness_pair(100, with_auditor);
            let proof = prove_withdrawal(&ss, &stmt, &witn);
            assert_verifies_withdrawal(&ss, &stmt, &proof);
        });
    }

    #[test]
    fun proof_correctness_normalization() {
        // Normalization is withdrawal with v=0
        let ss = withdraw_session_for_testing();
        vector[false, true].for_each(|with_auditor| {
            let (stmt, witn) = random_valid_statement_witness_pair(0, with_auditor);
            let proof = prove_normalization(&ss, &stmt, &witn);
            assert_verifies_normalization(&ss, &stmt, &proof);
        });
    }

    #[test]
    #[expected_failure(abort_code=65537, location=aptos_experimental::sigma_protocol_fiat_shamir)]
    fun proof_soundness_empty_proof() {
        let (stmt, _) = random_valid_statement_witness_pair(100, false);
        let proof = sigma_protocol_proof::empty();
        assert_verifies_withdrawal(&withdraw_session_for_testing(), &stmt, &proof);
    }
}
