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
module aptos_framework::sigma_protocol_withdraw {
    friend aptos_framework::confidential_asset;
    #[test_only]
    friend aptos_framework::confidential_asset_tests;
    #[test_only]
    friend aptos_framework::sigma_protocol_proof_tests;

    use std::bcs;
    use std::error;
    use std::signer;
    use std::option::Option;
    use std::vector;
    use aptos_std::ristretto255::{Self, RistrettoPoint, Scalar, CompressedRistretto};
    use aptos_framework::chain_id;
    use aptos_framework::fungible_asset::Metadata;
    use aptos_framework::object::Object;
    use aptos_framework::confidential_balance::{Available, CompressedBalance, get_num_available_chunks, get_b_powers, get_encryption_key_basepoint_compressed};
    use aptos_framework::sigma_protocol;
    use aptos_framework::sigma_protocol_proof::Proof;
    use aptos_framework::sigma_protocol_fiat_shamir::new_domain_separator;
    use aptos_framework::sigma_protocol_witness::Witness;
    use aptos_framework::sigma_protocol_statement::Statement;
    use aptos_framework::sigma_protocol_statement_builder::new_builder;
    use aptos_framework::sigma_protocol_utils::{e_wrong_num_points, e_wrong_num_scalars, e_wrong_witness_len, e_wrong_output_len};
    use aptos_framework::sigma_protocol_representation::{repr_point, repr_scaled, new_representation};
    use aptos_framework::sigma_protocol_representation_vec::{RepresentationVec, new_representation_vec};
    #[test_only]
    use aptos_framework::sigma_protocol_homomorphism::evaluate_psi;

    //
    // Constants
    //

    /// Protocol ID for withdrawal proofs (also used for normalization, which is withdrawal with v = 0)
    const WITHDRAWAL_PROTOCOL_ID: vector<u8> = b"AptosConfidentialAsset/WithdrawalV1";

    //
    // Statement point indices (common prefix — auditor components appended at end)
    //

    /// Index of $G$ (the Ristretto255 basepoint) in the statement's points.
    const IDX_G: u64 = 0;
    /// Index of $H$ (the encryption key basepoint) in the statement's points.
    const IDX_H: u64 = 1;
    /// Index of $\mathsf{ek}$ (the sender's encryption key) in the statement's points.
    const IDX_EK: u64 = 2;
    /// old_P values start at index 3. old_R starts at 3 + ℓ. new_P at 3 + 2ℓ. new_R at 3 + 3ℓ.
    /// If auditor present: ek_aud at 3 + 4ℓ, then new_R_aud at 3 + 4ℓ + 1.
    const START_IDX_OLD_P: u64 = 3;
    /// Index of $v$ (the withdrawn value) in the statement's scalars.
    const IDX_V: u64 = 0;

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
    const E_INVALID_PROOF: u64 = 5;  // other error codes in [1, 4] in sigma_protocol_utils.move
    /// The number of auditor R components does not match the expected auditor count.
    const E_AUDITOR_COUNT_MISMATCH: u64 = 6;
    /// The homomorphism or transformation function implementation is not inserting points (or scalars) at the expected positions.
    const E_STATEMENT_BUILDER_INCONSISTENCY: u64 = 7;

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

        let ell = get_num_chunks();
        let err = error::internal(E_STATEMENT_BUILDER_INCONSISTENCY);
        let b = new_builder();

        assert!(b.add_point(ristretto255::basepoint_compressed()) == IDX_G, err);                                  // G
        assert!(b.add_point(get_encryption_key_basepoint_compressed()) == IDX_H, err);                             // H
        assert!(b.add_point(compressed_ek) == IDX_EK, err);                                                           // ek
        assert!(b.add_points(compressed_old_balance.get_compressed_P()) == START_IDX_OLD_P, err);                  // old_P
        assert!(b.add_points(compressed_old_balance.get_compressed_R()) == START_IDX_OLD_P + ell, err);            // old_R
        let (idx, new_P) = b.add_points_cloned(compressed_new_balance.get_compressed_P()); // new_P
        assert!(idx == START_IDX_OLD_P + 2 * ell, err);
        assert!(b.add_points(compressed_new_balance.get_compressed_R()) == START_IDX_OLD_P + 3 * ell, err);        // new_R

        if (compressed_ek_aud.is_some()) {
            assert!(b.add_point(*compressed_ek_aud.borrow()) == START_IDX_OLD_P + 4 * ell, err);                        // ek_aud
            assert!(b.add_points(compressed_new_balance.get_compressed_R_aud()) == START_IDX_OLD_P + 4 * ell + 1, err); // new_R_aud
        };

        assert!(b.add_scalar(v) == IDX_V, err);
        let stmt = b.build();
        assert_withdraw_statement_is_well_formed(&stmt, compressed_ek_aud.is_some());
        (stmt, new_P)
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

        // WARNING: Crucial for security
        assert!(reprs.length() == expected_output_len(ell, has_auditor), e_wrong_output_len());
        new_representation_vec(reprs)
    }

    fun expected_output_len(ell: u64, has_auditor: bool): u64 {
        if (has_auditor) { 2 + 3 * ell } else { 2 + 2 * ell }
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

        // Note: Not needed for security, since a mismatched f(X) length will be caught in the verifier. But good practice
        // for catching mistakes *early* when implementing your f(X).
        assert!(reprs.length() == expected_output_len(ell, has_auditor), e_wrong_output_len());
        new_representation_vec(reprs)
    }

    /// Asserts that a withdrawal proof verifies.
    public(friend) fun assert_verifies(self: &WithdrawSession, stmt: &Statement<Withdrawal>, proof: &Proof) {
        assert_withdraw_statement_is_well_formed(stmt, self.has_auditor);

        let success = sigma_protocol::verify(
            new_domain_separator(@aptos_framework, chain_id::get(), WITHDRAWAL_PROTOCOL_ID, bcs::to_bytes(self)),
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
    /// Evaluates the withdrawal psi homomorphism for testing (wraps the private `psi` closure).
    public(friend) fun evaluate_psi_for_testing(
        stmt: &Statement<Withdrawal>, witn: &Witness, has_auditor: bool,
    ): vector<RistrettoPoint> {
        evaluate_psi(|_X, w| psi(_X, w, has_auditor), stmt, witn)
    }

    #[test_only]
    /// Creates a withdrawal proof (for testing).
    public(friend) fun prove(self: &WithdrawSession, stmt: &Statement<Withdrawal>, witn: &Witness): Proof {
        let (proof, _) = sigma_protocol::prove(
            new_domain_separator(@aptos_framework, chain_id::get(), WITHDRAWAL_PROTOCOL_ID, bcs::to_bytes(self)),
            |_X, w| psi(_X, w, self.has_auditor),
            stmt,
            witn
        );
        proof
    }

}
