/// # The withdrawal NP relation ($\mathcal{R}^{-}_\mathsf{withdraw}$)
///
/// $\def\old#1{{\color{red}{\dot{#1}}}}\def\new#1{{\color{teal}{\widetilde{#1}}}}\def\opt#1{{\color{orange}{\boldsymbol{[}}} #1 {\color{orange}{\boldsymbol{]}}}}$
///
/// A ZKPoK of a correct balance update when publicly withdrawing amount $v$ from an old available balance.
/// Also used for normalization (where $v = 0$).
///
/// ## Notation
///
/// - $\old{x}$ denotes a stale/old ciphertext component; $\new{x}$ denotes a fresh/new one.
/// - $\opt{\cdot}$ denotes components present only when an auditor is set.
///   Auditor components are placed at the **end** of the statement so that the common prefix is
///   identical in both cases. psi/f receive auditor presence via an explicit `has_auditor` flag.
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
///       \opt{\mathsf{ek}^\mathsf{eff}, \new{\mathbf{R}}^\mathsf{eff}}
///       \textbf{;}\\
///     \mathsf{dk}, \new{\mathbf{a}}, \new{\mathbf{r}}
///       \textbf{;}\; v
/// \end{array}\right) = 1
/// \Leftrightarrow
/// \left\{\begin{array}{r@{\,\,}l@{\quad}l}
///     H &= \mathsf{dk} \cdot \mathsf{ek}\\
///     \new{P}_i &= \new{a}_i \cdot G + \new{r}_i \cdot H, &\forall i \in [\ell]\\
///     \new{R}_i &= \new{r}_i \cdot \mathsf{ek}, &\forall i \in [\ell]\\
///     \opt{\new{R}^\mathsf{eff}_i} &\opt{= \new{r}_i \cdot \mathsf{ek}^\mathsf{eff},}
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
///     \opt{\new{r}_i \cdot \mathsf{ek}^\mathsf{eff}, \;\forall i \in [\ell]}\\
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
///     \opt{\new{R}^\mathsf{eff}_i, \;\forall i \in [\ell]}\\
///     \langle \mathbf{B}, \old{\mathbf{P}} \rangle - v \cdot G\\
/// \end{pmatrix}
/// $$
///
module aptos_experimental::sigma_protocol_withdraw {
    friend aptos_experimental::confidential_asset;

    use std::bcs;
    use std::error;
    use std::signer;
    use std::option::Option;
    use std::vector;
    use aptos_std::ristretto255::{Self, RistrettoPoint, Scalar, CompressedRistretto};
    use aptos_framework::chain_id;
    use aptos_framework::fungible_asset::Metadata;
    use aptos_framework::object::Object;
    use aptos_experimental::ristretto255_twisted_elgamal::get_encryption_key_basepoint_compressed;
    use aptos_experimental::confidential_balance::{Available, CompressedBalance, get_num_available_chunks, get_b_powers};
    use aptos_experimental::sigma_protocol;
    use aptos_experimental::sigma_protocol_proof::Proof;
    use aptos_experimental::sigma_protocol_fiat_shamir::new_domain_separator;
    use aptos_experimental::sigma_protocol_witness::Witness;
    use aptos_experimental::sigma_protocol_statement::Statement;
    use aptos_experimental::sigma_protocol_statement_builder::new_builder;
    use aptos_experimental::sigma_protocol_utils::{e_wrong_num_points, e_wrong_num_scalars, e_wrong_witness_len, e_wrong_output_len};
    use aptos_experimental::sigma_protocol_representation::{repr_point, repr_scaled, new_representation};
    use aptos_experimental::sigma_protocol_representation_vec::{RepresentationVec, new_representation_vec};
    #[test_only]
    use aptos_experimental::confidential_balance::{
        generate_available_randomness, new_available_from_amount, split_available_into_chunks, Balance};
    #[test_only]
    use aptos_experimental::sigma_protocol_test_utils::setup_test_environment;
    #[test_only]
    use aptos_experimental::ristretto255_twisted_elgamal::generate_twisted_elgamal_keypair;
    #[test_only]
    use aptos_experimental::sigma_protocol_homomorphism::evaluate_psi;
    #[test_only]
    use aptos_experimental::sigma_protocol_proof;
    #[test_only]
    use aptos_experimental::sigma_protocol_utils::{equal_vec_points, points_clone};
    #[test_only]
    use aptos_experimental::sigma_protocol_witness::new_secret_witness;
    #[test_only]
    use aptos_std::ristretto255::{new_scalar_from_u64, double_scalar_mul, multi_scalar_mul, scalar_zero};

    //
    // Constants
    //

    /// Protocol ID for withdrawal proofs (also used for normalization, which is withdrawal with v = 0)
    const WITHDRAWAL_PROTOCOL_ID: vector<u8> = b"AptosConfidentialAsset/WithdrawalV1";

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

    /// The withdrawal proof was invalid.
    const E_INVALID_PROOF: u64 = 5;
    /// The number of auditor R components does not match the expected auditor count.
    const E_AUDITOR_COUNT_MISMATCH: u64 = 6;

    /// An error occurred in one of our tests.
    const E_TEST_INTERNAL: u64 = 1_000;

    //
    // Structs
    //

    /// Phantom marker type for withdrawal statements.
    struct Withdrawal has drop {}

    /// Used for domain separation in the Fiat-Shamir transform.
    struct WithdrawSession has drop {
        sender: address,
        asset_type: Object<Metadata>,
        num_chunks: u64,
        has_auditor: bool,
    }

    //
    // Helper functions
    //

    /// Returns the fixed number of balance chunks ℓ (= AVAILABLE_BALANCE_CHUNKS).
    inline fun get_num_chunks(): u64 {
        get_num_available_chunks()
    }

    /// Validates that the statement has the correct structure for the given auditor flag.
    fun assert_withdraw_statement_is_well_formed(stmt: &Statement<Withdrawal>, has_auditor: bool) {
        let ell = get_num_chunks();
        let expected = 3 + 4 * ell + if (has_auditor) { 1 + ell } else { 0 };
        assert!(stmt.get_points().length() == expected,e_wrong_num_points());
        // i.e., the transferred amount v
        assert!(stmt.get_scalars().length() == 1, e_wrong_num_scalars());
    }

    //
    // Public functions
    //

    public(friend) fun new_session(sender: &signer, asset_type: Object<Metadata>, has_auditor: bool): WithdrawSession {
        WithdrawSession {
            sender: signer::address_of(sender),
            asset_type,
            num_chunks: get_num_chunks(),
            has_auditor,
        }
    }

    /// Creates a withdrawal statement, optionally including auditor components.
    ///
    /// Points (auditorless): [ G, H, ek, old_P[0..ℓ-1], old_R[0..ℓ-1], new_P[0..ℓ-1], new_R[0..ℓ-1] ]
    /// Points (w/ auditor):  [ ---------------------------- as above ------------------------------, ek_aud, new_R_aud]
    /// Scalars:              [ v ]
    ///
    /// For the auditorless case, pass `option::none()` for `compressed_ek_aud`
    /// and ensure `new_balance` / `compressed_new_balance` have empty R_aud.
    public(friend) fun new_withdrawal_statement(
        compressed_ek: CompressedRistretto,
        compressed_old_balance: &CompressedBalance<Available>,
        compressed_new_balance: &CompressedBalance<Available>,
        compressed_ek_aud: &Option<CompressedRistretto>,
        v: Scalar,
    ): (Statement<Withdrawal>, vector<RistrettoPoint>) {
        assert!(
            compressed_new_balance.get_compressed_R_aud().length() == if (compressed_ek_aud.is_some()) { get_num_chunks() } else { 0 },
            error::invalid_argument(E_AUDITOR_COUNT_MISMATCH)
        );

        let b = new_builder();
        b.add_point(ristretto255::basepoint_compressed());                                                 // G
        b.add_point(get_encryption_key_basepoint_compressed());                                            // H
        b.add_point(compressed_ek);                                                                           // ek
        b.add_points(compressed_old_balance.get_compressed_P());                                           // old_P
        b.add_points(compressed_old_balance.get_compressed_R());                                           // old_R
        let (_, new_P) = b.add_points_cloned(compressed_new_balance.get_compressed_P()); // new_P
        b.add_points(compressed_new_balance.get_compressed_R());                                           // new_R

        if (compressed_ek_aud.is_some()) {
            b.add_point(*compressed_ek_aud.borrow());                                                      // ek_aud
            b.add_points(compressed_new_balance.get_compressed_R_aud());                                   // new_R_aud
        };

        b.add_scalar(v);
        let stmt = b.build();
        assert_withdraw_statement_is_well_formed(&stmt, compressed_ek_aud.is_some());
        (stmt, new_P)
    }

    #[test_only]
    /// Creates a withdrawal witness: $(\mathsf{dk}, \new{a}_0, \ldots, \new{a}_{\ell-1}, \new{r}_0, \ldots, \new{r}_{\ell-1})$.
    public fun new_withdrawal_witness(dk: Scalar, new_a: vector<Scalar>, new_r: vector<Scalar>): Witness {
        assert!(new_a.length() == new_r.length(), e_wrong_witness_len());

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
    fun psi(stmt: &Statement<Withdrawal>, w: &Witness, has_auditor: bool): RepresentationVec {
        // WARNING: Crucial for security
        assert_withdraw_statement_is_well_formed(stmt, has_auditor);

        let ell = get_num_chunks();
        let b_powers = get_b_powers(ell);

        // WARNING: Crucial for security
        let expected_witness_len = 1 + 2 * ell;
        assert!(w.length() == expected_witness_len, e_wrong_witness_len());

        let dk = *w.get(IDX_DK);

        let reprs = vector[];

        // 1. dk · ek
        reprs.push_back(repr_scaled(IDX_EK, dk));

        // 2. new_a[i] · G + new_r[i] · H, for i ∈ [1..ℓ]
        vector::range(0, ell).for_each(|i| {
            let new_a_i = *w.get(1 + i);
            let new_r_i = *w.get(1 + ell + i);
            reprs.push_back(new_representation(vector[IDX_G, IDX_H], vector[new_a_i, new_r_i]));
        });

        // 3. new_r[i] · ek, for i ∈ [1..ℓ]
        vector::range(0, ell).for_each(|i| {
            let new_r_i = *w.get(1 + ell + i);
            reprs.push_back(repr_scaled(IDX_EK, new_r_i));
        });

        // 3b. (auditor only) new_r[i] · ek_aud, for i ∈ [1..ℓ]
        if (has_auditor) {
            let idx_ek_aud = START_IDX_OLD_P + 4 * ell;
            vector::range(0, ell).for_each(|i| {
                let new_r_i = *w.get(1 + ell + i);
                reprs.push_back(repr_scaled(idx_ek_aud, new_r_i));
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
        let expected_output_len = if (has_auditor) { 2 + 3 * ell } else { 2 + 2 * ell };

        // WARNING: Crucial for security
        assert!(repr_vec.length() == expected_output_len, e_wrong_output_len());

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
    fun f(stmt: &Statement<Withdrawal>, has_auditor: bool): RepresentationVec {
        let ell = get_num_chunks();
        let b_powers = get_b_powers(ell);
        let v = stmt.get_scalars()[0];

        let idx_new_P_start = START_IDX_OLD_P + 2 * ell;
        let idx_new_R_start = START_IDX_OLD_P + 3 * ell;

        let reprs = vector[];

        // 1. H
        reprs.push_back(repr_point(IDX_H));

        // 2. new_P[i]
        vector::range(0, ell).for_each(|i| {
            reprs.push_back(repr_point(idx_new_P_start + i));
        });

        // 3. new_R[i]
        vector::range(0, ell).for_each(|i| {
            reprs.push_back(repr_point(idx_new_R_start + i));
        });

        // 3b. (auditor only) new_R_aud[i]
        if (has_auditor) {
            let idx_new_R_aud_start = START_IDX_OLD_P + 4 * ell + 1; // +1 for ek_aud
            vector::range(0, ell).for_each(|i| {
                reprs.push_back(repr_point(idx_new_R_aud_start + i));
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
    public(friend) fun assert_verifies(self: &WithdrawSession, stmt: &Statement<Withdrawal>, proof: &Proof) {
        assert_withdraw_statement_is_well_formed(stmt, self.has_auditor);

        let success = sigma_protocol::verify(
            new_domain_separator(@aptos_experimental, chain_id::get(), WITHDRAWAL_PROTOCOL_ID, bcs::to_bytes(self)),
            |_X, w| psi(_X, w, self.has_auditor),
            |_X| f(_X, self.has_auditor),
            stmt,
            proof
        );

        assert!(success, error::invalid_argument(E_INVALID_PROOF));
    }

    //
    // Tests
    //

    #[test_only]
    /// Creates a withdrawal proof (for testing).
    public fun prove(self: &WithdrawSession, stmt: &Statement<Withdrawal>, witn: &Witness): Proof {
        let (proof, _) = sigma_protocol::prove(
            new_domain_separator(@aptos_experimental, chain_id::get(), WITHDRAWAL_PROTOCOL_ID, bcs::to_bytes(self)),
            |_X, w| psi(_X, w, self.has_auditor),
            stmt,
            witn
        );
        proof
    }

    #[test_only]
    /// Returns the components of a random valid statement-witness pair.
    /// Used by `random_valid_statement_witness_pair` (which assembles Statement/Witness)
    /// and by `psi_correctness` (which needs the raw points for an independent manual computation).
    fun random_valid_statement_witness_pair_internal(
        amount: u64, with_auditor: bool
    ): (
        CompressedRistretto,           // compressed_ek
        Option<CompressedRistretto>,   // compressed_ek_aud
        Balance<Available>,              // old_balance
        Balance<Available>,              // new_balance
        Scalar,                        // v
        Scalar,                        // dk
        vector<Scalar>,                // new_a
        vector<Scalar>,                // new_r
    ) {
        // Generate sender keypair
        let (dk, compressed_ek) = generate_twisted_elgamal_keypair();

        // Generate optional auditor keypair
        let compressed_ek_aud = if (with_auditor) {
            let (_, ek_aud) = generate_twisted_elgamal_keypair();
            std::option::some(ek_aud)
        } else {
            std::option::none()
        };

        // Create old and new balances using the high-level API
        let old_balance_u128 = 1000u128;
        let new_balance_u128 = old_balance_u128 - (amount as u128);

        let old_randomness = generate_available_randomness();
        let old_balance = new_available_from_amount(
            old_balance_u128, &old_randomness, &compressed_ek, &compressed_ek_aud
        );
        let new_randomness = generate_available_randomness();
        let new_balance = new_available_from_amount(
            new_balance_u128, &new_randomness, &compressed_ek, &compressed_ek_aud
        );

        let v = new_scalar_from_u64(amount);
        let new_a = split_available_into_chunks(new_balance_u128);
        let new_r = *new_randomness.scalars();

        (compressed_ek, compressed_ek_aud, old_balance, new_balance, v, dk, new_a, new_r)
    }

    #[test_only]
    /// Generates a random valid statement-witness pair for testing.
    /// When `with_auditor` is true, includes auditor components in the statement.
    fun random_valid_statement_witness_pair(amount: u64, with_auditor: bool): (Statement<Withdrawal>, Witness) {
        let (compressed_ek, compressed_ek_aud, old_balance, new_balance,
            v, dk, new_a, new_r
        ) = random_valid_statement_witness_pair_internal(amount, with_auditor);

        let compressed_old_balance = old_balance.compress();
        let compressed_new_balance = new_balance.compress();

        let (stmt, _) = new_withdrawal_statement(
            compressed_ek, &compressed_old_balance, &compressed_new_balance, &compressed_ek_aud, v,
        );
        let witn = new_withdrawal_witness(dk, new_a, new_r);

        (stmt, witn)
    }

    #[test_only]
    /// Verifies that `evaluate_psi` produces the same points as a manual computation using
    /// direct ristretto255 arithmetic, for both auditor and auditorless cases.
    ///
    /// The manual computation uses only raw points/scalars — no `IDX_*` constants — so it is
    /// completely independent of the statement layout.
    fun psi_correctness(with_auditor: bool) {
        let ell = get_num_chunks();
        let (compressed_ek, compressed_ek_aud, old_balance, new_balance,
            v, dk, new_a, new_r
        ) = random_valid_statement_witness_pair_internal(100, with_auditor);
        let b_powers = get_b_powers(ell);

        // Derive raw points from compressed keys and balances
        let _G = ristretto255::basepoint();
        let _H = get_encryption_key_basepoint_compressed().point_decompress();
        let ek = compressed_ek.point_decompress();
        let ek_aud = compressed_ek_aud.map(|ek| ek.point_decompress());

        // Sanity check: ek = dk^{-1} * H
        assert!(_H.point_equals(&ek.point_mul(&dk)), error::internal(E_TEST_INTERNAL));

        // Clone old_R for the manual computation below (before old_balance is consumed)
        let old_R_clone = points_clone(old_balance.get_R());

        // Build statement + witness for evaluate_psi
        let (stmt, _) = new_withdrawal_statement(
            compressed_ek, &old_balance.compress(), &new_balance.compress(), &compressed_ek_aud, v,
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
            manual_psi.push_back(double_scalar_mul(&new_a[i], &_G, &new_r[i], &_H));
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
        let dk_inner_b_old_R = multi_scalar_mul(&old_R_clone, &dk_b_scalars);

        let inner_b_new_a = scalar_zero();
        vector::range(0, ell).for_each(|i| {
            inner_b_new_a = inner_b_new_a.scalar_add(&new_a[i].scalar_mul(&b_powers[i]));
        });
        let inner_b_new_a_times_G = _G.point_mul(&inner_b_new_a);

        manual_psi.push_back(dk_inner_b_old_R.point_add(&inner_b_new_a_times_G));

        //
        // Compare: implemented_psi vs manual_psi computation
        //
        let implemented_psi = evaluate_psi(|_X, w| psi(_X, w, with_auditor), &stmt, &witn);

        assert!(equal_vec_points(&implemented_psi, &manual_psi), 1);
    }

    #[test]
    fun psi_correctness_no_auditor() { psi_correctness(false); }

    #[test]
    fun psi_correctness_with_auditor() { psi_correctness(true); }

    #[test]
    fun proof_correctness() {
        let (sender, asset_type) = setup_test_environment();

        // Test both auditor configurations and both withdrawal (v=100) and normalization (v=0)
        vector[false, true].for_each(|with_auditor| {
            vector[0u64, 100].for_each(|amount| {
                let ss = new_session(&sender, asset_type, with_auditor);
                let (stmt, witn) = random_valid_statement_witness_pair(amount, with_auditor);
                ss.assert_verifies(&stmt, &ss.prove(&stmt, &witn));
            });
        });
    }

    #[test]
    #[expected_failure(abort_code=65537, location=aptos_experimental::sigma_protocol_fiat_shamir)]
    fun proof_soundness_empty_proof() {
        let (sender, asset_type) = setup_test_environment();

        let (stmt, _) = random_valid_statement_witness_pair(100, false);
        new_session(&sender, asset_type, false).assert_verifies(&stmt, &sigma_protocol_proof::empty());
    }
}
