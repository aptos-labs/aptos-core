/// # The transfer NP relation ($\mathcal{R}^{-}_\mathsf{txfer}$)
///
/// $\def\old#1{{\color{red}{\dot{#1}}}}\def\new#1{{\color{teal}{\widetilde{#1}}}}\def\opt#1{{\color{orange}{\boldsymbol{[}}} #1 {\color{orange}{\boldsymbol{]}}}}$
///
/// A ZKPoK of a correct confidential transfer from sender to recipient. This is a composition of
/// $\mathcal{R}^\mathsf{veiled}_\mathsf{withdraw}$ (the sender's balance update with SECRET amount $\mathbf{v}$)
/// and $\mathcal{R}_\mathsf{eq}$ (the transfer amount encrypted identically under all parties' keys).
///
/// ## Notation
///
/// - $\old{x}$ denotes a stale/old ciphertext component; $\new{x}$ denotes a fresh/new one.
/// - $\opt{\cdot}$ denotes components present only when has\_effective\_auditor is true.
/// - $\langle \mathbf{x}, \mathbf{y} \rangle = \sum_i x_i \cdot y_i$ denotes the inner product.
/// - $\mathbf{B} = (B^0, B^1, \ldots)$ where $B = 2^{16}$ is the positional weight vector for chunk encoding.
/// - $\ell$: number of available balance chunks; $n$: number of transfer (pending balance) chunks.
/// - $T$: number of voluntary auditors ($T \ge 0$).
/// - The effective auditor (if present) sees the sender's new balance AND the transfer amount.
///   Extra auditors see only the transfer amount.
///
/// ## The relation
///
/// $$
/// \mathcal{R}^{-}_\mathsf{txfer}\left(\begin{array}{l}
///     G, H, \mathsf{ek}^\mathsf{sid}, \mathsf{ek}^\mathsf{rid},
///       \old{\mathbf{P}}, \old{\mathbf{R}}, \new{\mathbf{P}}, \new{\mathbf{R}},
///       \mathbf{P}, \mathbf{R}^\mathsf{sid}, \mathbf{R}^\mathsf{rid},\\
///     \opt{\mathsf{ek}^\mathsf{eff}, \new{\mathbf{R}}^\mathsf{eff}, \mathbf{R}^\mathsf{eff}},\;
///       (\mathsf{ek}^\mathsf{ex}_i, \mathbf{R}^\mathsf{ex}_i)_{i \in [T]}
///       \textbf{;}\\
///     \mathsf{dk}, \new{\mathbf{a}}, \new{\mathbf{r}}, \mathbf{v}, \mathbf{r}
/// \end{array}\right) = 1
/// \Leftrightarrow
/// \left\{\begin{array}{r@{\,\,}l@{\quad}l}
///     H &= \mathsf{dk} \cdot \mathsf{ek}^\mathsf{sid}\\
///     \new{P}_i &= \new{a}_i \cdot G + \new{r}_i \cdot H, &\forall i \in [\ell]\\
///     \new{R}_i &= \new{r}_i \cdot \mathsf{ek}^\mathsf{sid}, &\forall i \in [\ell]\\
///     \opt{\new{R}^\mathsf{eff}_i} &\opt{= \new{r}_i \cdot \mathsf{ek}^\mathsf{eff},}
///       &\opt{\forall i \in [\ell]}\\
///     \langle \mathbf{B}, \old{\mathbf{P}} \rangle &= \mathsf{dk} \cdot \langle \mathbf{B}, \old{\mathbf{R}} \rangle
///       + (\langle \mathbf{B}, \new{\mathbf{a}} \rangle + \langle \mathbf{B}, \mathbf{v} \rangle) \cdot G\\
///     P_j &= v_j \cdot G + r_j \cdot H, &\forall j \in [n]\\
///     R^\mathsf{sid}_j &= r_j \cdot \mathsf{ek}^\mathsf{sid}, &\forall j \in [n]\\
///     R^\mathsf{rid}_j &= r_j \cdot \mathsf{ek}^\mathsf{rid}, &\forall j \in [n]\\
///     \opt{R^\mathsf{eff}_j} &\opt{= r_j \cdot \mathsf{ek}^\mathsf{eff},}
///       &\opt{\forall j \in [n]}\\
///     R^\mathsf{ex}_{i,j} &= r_j \cdot \mathsf{ek}^\mathsf{ex}_i,
///       &\forall j \in [n],\; \forall i \in [T]\\
/// \end{array}\right.
/// $$
///
/// ## Homomorphism
///
/// This can be framed as a homomorphism check $\psi(\mathbf{w}) = f(\mathbf{X})$ where
/// $\mathbf{w} = (\mathsf{dk}, \new{\mathbf{a}}, \new{\mathbf{r}}, \mathbf{v}, \mathbf{r})$
/// is the witness and $\mathbf{X}$ is the statement.
///
///   1. The homomorphism $\psi$ is:
///
/// $$
/// \psi(\mathsf{dk}, \new{\mathbf{a}}, \new{\mathbf{r}}, \mathbf{v}, \mathbf{r}) = \begin{pmatrix}
///     \mathsf{dk} \cdot \mathsf{ek}^\mathsf{sid}\\
///     \new{a}_i \cdot G + \new{r}_i \cdot H, &\forall i \in [\ell]\\
///     \new{r}_i \cdot \mathsf{ek}^\mathsf{sid}, &\forall i \in [\ell]\\
///     \opt{\new{r}_i \cdot \mathsf{ek}^\mathsf{eff}, \;\forall i \in [\ell]}\\
///     \mathsf{dk} \cdot \langle \mathbf{B}, \old{\mathbf{R}} \rangle
///       + (\langle \mathbf{B}, \new{\mathbf{a}} \rangle + \langle \mathbf{B}, \mathbf{v} \rangle) \cdot G\\
///     v_j \cdot G + r_j \cdot H, &\forall j \in [n]\\
///     r_j \cdot \mathsf{ek}^\mathsf{sid}, &\forall j \in [n]\\
///     r_j \cdot \mathsf{ek}^\mathsf{rid}, &\forall j \in [n]\\
///     \opt{r_j \cdot \mathsf{ek}^\mathsf{eff}, \;\forall j \in [n]}\\
///     r_j \cdot \mathsf{ek}^\mathsf{ex}_i, &\forall j \in [n],\; \forall i \in [T]\\
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
///     \langle \mathbf{B}, \old{\mathbf{P}} \rangle\\
///     P_j, &\forall j \in [n]\\
///     R^\mathsf{sid}_j, &\forall j \in [n]\\
///     R^\mathsf{rid}_j, &\forall j \in [n]\\
///     \opt{R^\mathsf{eff}_j, \;\forall j \in [n]}\\
///     R^\mathsf{ex}_{i,j}, &\forall j \in [n],\; \forall i \in [T]\\
/// \end{pmatrix}
/// $$
///
module aptos_framework::sigma_protocol_transfer {
    friend aptos_framework::confidential_asset;
    #[test_only]
    friend aptos_framework::confidential_asset_tests;
    #[test_only]
    friend aptos_framework::sigma_protocol_proof_tests;

    use std::bcs;
    use std::error;
    use std::option::Option;
    use std::signer;
    use std::vector;
    use aptos_std::ristretto255::{Self, CompressedRistretto};
    #[test_only]
    use aptos_std::ristretto255::RistrettoPoint;
    use aptos_framework::chain_id;
    use aptos_framework::fungible_asset::Metadata;
    use aptos_framework::object::Object;
    use aptos_framework::confidential_balance::{Self, Pending, Available, CompressedBalance, Balance,
        get_num_available_chunks, get_num_pending_chunks, get_b_powers, new_pending_from_p_and_r};
    use aptos_framework::confidential_amount::CompressedAmount;
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
    use aptos_framework::sigma_protocol_test_utils::setup_test_environment;
    #[test_only]
    use aptos_framework::sigma_protocol_homomorphism::evaluate_psi;

    //
    // Constants
    //

    /// Protocol ID used for domain separation
    const PROTOCOL_ID: vector<u8> = b"AptosConfidentialAsset/TransferV1";

    //
    // Statement point indices (common prefix — auditor components appended at end)
    //

    /// Index of $G$ (the Ristretto255 basepoint).
    const IDX_G: u64 = 0;
    /// Index of $H$ (the encryption key basepoint).
    const IDX_H: u64 = 1;
    /// Index of $\mathsf{ek}^\mathsf{sid}$ (the sender's encryption key).
    const IDX_EK_SENDER: u64 = 2;
    /// Index of $\mathsf{ek}^\mathsf{rid}$ (the recipient's encryption key).
    const IDX_EK_RECIP: u64 = 3;

    /// old_P starts at index 4.
    /// Layout: old_P[1..ℓ], old_R[1..ℓ], new_P[1..ℓ], new_R[1..ℓ], amount_P[1..n], amount_R_sender[1..n], amount_R_recip[1..n]
    /// With effective auditor: ..., ek_eff_aud, new_R_eff_aud[1..ℓ], amount_R_eff_aud[1..n]
    /// For each voluntary auditor t: ..., ek_volun_auds[t], amount_R_volun_auds[t][1..n]
    const START_IDX_OLD_P: u64 = 4;

    //
    // Witness scalar indices
    //

    /// Index of dk (sender's decryption key).
    const IDX_DK: u64 = 0;
    /// new_a[0..ℓ-1] at 1..ℓ. new_r[0..ℓ-1] at 1+ℓ..2ℓ.
    /// v[0..n-1] at 1+2ℓ..1+2ℓ+n-1. r[0..n-1] at 1+2ℓ+n..1+2ℓ+2n-1.

    //
    // Error codes
    //

    /// The transfer proof was invalid.
    const E_INVALID_TRANSFER_PROOF: u64 = 5;  // other error codes in [1, 4] in sigma_protocol_utils.move
    /// The number of auditor R components does not match the expected auditor count.
    const E_AUDITOR_COUNT_MISMATCH: u64 = 6;
    /// The homomorphism or transformation function implementation is not inserting points at the expected positions.
    const E_STATEMENT_BUILDER_INCONSISTENCY: u64 = 7;

    //
    // Structs
    //

    /// Phantom marker type for transfer statements.
    struct Transfer has drop {}

    /// Used for domain separation in the Fiat-Shamir transform.
    struct TransferSession has drop {
        sender: address,
        recipient: address,
        asset_type: Object<Metadata>,
        num_avail_chunks: u64,
        num_transfer_chunks: u64,
        has_effective_auditor: bool,
        num_volun_auditors: u64,
    }

    //
    // Helper functions
    //

    /// Returns the fixed number of available balance chunks ℓ.
    inline fun get_ell(): u64 { get_num_available_chunks() }

    /// Returns the fixed number of transfer (pending) balance chunks n.
    inline fun get_n(): u64 { get_num_pending_chunks() }

    /// Validates the statement structure.
    ///
    /// Expected point count: 4 + 4ℓ + 3n + (has_eff ? 1+ℓ+n : 0) + num_volun*(1+n)
    fun assert_transfer_statement_is_well_formed(
        stmt: &Statement<Transfer>, has_effective_auditor: bool, num_volun_auditors: u64,
    ) {
        let ell = get_ell();
        let n = get_n();
        let num_points = stmt.get_points().length();

        let expected_num_points = 4 + 4 * ell + 3 * n
            + if (has_effective_auditor) { 1 + ell + n } else { 0 }
            + num_volun_auditors * (1 + n);
        assert!(num_points == expected_num_points, e_wrong_num_points());
        assert!(stmt.get_scalars().length() == 0, e_wrong_num_scalars());
    }

    //
    // Public functions
    //

    public(friend) fun new_session(
        sender: &signer,
        recipient: address,
        asset_type: Object<Metadata>,
        has_effective_auditor: bool,
        num_volun_auditors: u64,
    ): TransferSession {
        TransferSession {
            sender: signer::address_of(sender),
            recipient,
            asset_type,
            num_avail_chunks: get_num_available_chunks(),
            num_transfer_chunks: get_num_pending_chunks(),
            has_effective_auditor,
            num_volun_auditors,
        }
    }

    /// Creates a transfer statement, optionally including effective and voluntary auditor components.
    ///
    /// Points (base): [G, H, ek_sender, ek_recip, old_P[1..ℓ], old_R[1..ℓ], new_P[1..ℓ], new_R[1..ℓ], amount_P[1..n], amount_R_sender[1..n], amount_R_recip[1..n]]
    /// If effective: + [ek_eff_aud, new_R_eff_aud[1..ℓ], amount_R_eff_aud[1..n]]
    /// For each voluntary auditor t: + [ek_volun_auds[t], amount_R_volun_auds[t][1..n]]
    ///
    /// For no effective auditor, pass `option::none()` for `compressed_ek_eff_aud`
    /// and ensure `amount` / `new_balance` have empty effective-auditor R components.
    /// For no voluntary auditors, pass an empty vector for `compressed_ek_volun_auds`.
    public(friend) fun new_transfer_statement(
        compressed_ek_sender: CompressedRistretto,
        compressed_ek_recip: CompressedRistretto,
        compressed_old_balance: &CompressedBalance<Available>,
        compressed_new_balance: &CompressedBalance<Available>,
        compressed_amount: &CompressedAmount,
        compressed_ek_eff_aud: &Option<CompressedRistretto>,
        compressed_ek_volun_auds: &vector<CompressedRistretto>,
    ): (Statement<Transfer>, Balance<Pending>) {
        let has_eff = compressed_ek_eff_aud.is_some();
        let num_volun = compressed_ek_volun_auds.length();

        // Validate auditor counts before expensive statement construction
        assert!(
            compressed_amount.num_volun_auditors_compressed() == num_volun,
            error::invalid_argument(E_AUDITOR_COUNT_MISMATCH)
        );
        assert!(
            compressed_amount.has_effective_auditor_compressed() == has_eff,
            error::invalid_argument(E_AUDITOR_COUNT_MISMATCH)
        );
        assert!(
            compressed_new_balance.get_compressed_R_aud().length() == if (has_eff) { get_num_available_chunks() } else { 0 },
            error::invalid_argument(E_AUDITOR_COUNT_MISMATCH)
        );
        assert!(
            compressed_amount.get_compressed_R_volun_auds().all(|r| r.length() == get_n()),
            error::invalid_argument(E_AUDITOR_COUNT_MISMATCH)
        );

        let ell = get_ell();
        let n = get_n();
        let e = error::internal(E_STATEMENT_BUILDER_INCONSISTENCY);

        let b = new_builder();
        assert!(b.add_point(ristretto255::basepoint_compressed()) == IDX_G, e);                                            // G
        assert!(b.add_point(confidential_balance::get_encryption_key_basepoint_compressed()) == IDX_H, e);         // H
        assert!(b.add_point(compressed_ek_sender) == IDX_EK_SENDER, e);                                                       // ek_sender
        assert!(b.add_point(compressed_ek_recip) == IDX_EK_RECIP, e);                                                         // ek_recip
        assert!(b.add_points(compressed_old_balance.get_compressed_P()) == START_IDX_OLD_P, e);                            // old_P
        assert!(b.add_points(compressed_old_balance.get_compressed_R()) == START_IDX_OLD_P + ell, e);                      // old_R
        assert!(b.add_points(compressed_new_balance.get_compressed_P()) == START_IDX_OLD_P + 2 * ell, e); // new_P
        assert!(b.add_points(compressed_new_balance.get_compressed_R()) == START_IDX_OLD_P + 3 * ell, e);                  // new_R
        let (idx, amount_P) = b.add_points_cloned(compressed_amount.get_compressed_P());           // amount_P
        assert!(idx == START_IDX_OLD_P + 4 * ell, e);
        assert!(b.add_points(compressed_amount.get_compressed_R_sender()) == START_IDX_OLD_P + 4 * ell + n, e);            // amount_R_sender
        let (idx, recip_R) = b.add_points_cloned(compressed_amount.get_compressed_R_recip());      // amount_R_recip
        assert!(idx == START_IDX_OLD_P + 4 * ell + 2 * n, e);

        // Effective auditor: ek, new_R[1..ℓ], amount_R[1..n]
        let idx_eff_start = START_IDX_OLD_P + 4 * ell + 3 * n;
        if (has_eff) {
            let ek_eff = *compressed_ek_eff_aud.borrow();
            assert!(b.add_point(ek_eff) == idx_eff_start, e);                                                     // ek_eff_aud
            assert!(b.add_points(compressed_new_balance.get_compressed_R_aud()) == idx_eff_start + 1, e);      // new_R_eff_aud
            assert!(b.add_points(compressed_amount.get_compressed_R_eff_aud()) == idx_eff_start + 1 + ell, e); // amount_R_eff_aud
        };

        // Voluntary auditors: for each, append [ek_volun_aud, amount_R_volun_aud[1..n]]
        let idx_volun_start = idx_eff_start + if (has_eff) { 1 + ell + n } else { 0 };
        let compressed_R_volun_auds = compressed_amount.get_compressed_R_volun_auds();
        vector::range(0, num_volun).for_each(|i| {
            let expected_idx = idx_volun_start + i * (1 + n);
            let ek_volun = compressed_ek_volun_auds[i];
            assert!(b.add_point(ek_volun) == expected_idx, e);                             // ek_volun_aud
            assert!(b.add_points(&compressed_R_volun_auds[i]) == expected_idx + 1, e);  // amount_R_volun_aud
        });

        let stmt = b.build();
        assert_transfer_statement_is_well_formed(&stmt, has_eff, num_volun);
        let amount = new_pending_from_p_and_r(amount_P, recip_R);
        (stmt, amount)
    }

    /// The combined homomorphism $\psi$ for the transfer relation (see module-level doc for full definition).
    fun psi(
        stmt: &Statement<Transfer>, w: &Witness,
        has_effective_auditor: bool, num_volun_auditors: u64,
    ): RepresentationVec {
        // WARNING: Crucial for security
        assert_transfer_statement_is_well_formed(stmt, has_effective_auditor, num_volun_auditors);

        let ell = get_ell();
        let n = get_n();

        // WARNING: Crucial for security
        let expected_witness_len = 1 + 2 * ell + 2 * n;
        assert!(w.length() == expected_witness_len, e_wrong_witness_len());

        let b_powers_ell = get_b_powers(ell);
        let b_powers_n = get_b_powers(n);

        let dk = *w.get(IDX_DK);

        let reprs = vector[];

        // === R^veiled_withdraw part ===

        // 1. dk · ek_sender
        reprs.push_back(repr_scaled(IDX_EK_SENDER, dk));

        // 2. new_a[i] · G + new_r[i] · H
        vector::range(0, ell).for_each(|i| {
            let new_a_i = *w.get(1 + i);
            let new_r_i = *w.get(1 + ell + i);
            reprs.push_back(new_representation(vector[IDX_G, IDX_H], vector[new_a_i, new_r_i]));
        });

        // 3. new_r[i] · ek_sender
        vector::range(0, ell).for_each(|i| {
            let new_r_i = *w.get(1 + ell + i);
            reprs.push_back(repr_scaled(IDX_EK_SENDER, new_r_i));
        });

        // 3b. (effective auditor only) new_r[i] · ek_eff_aud
        if (has_effective_auditor) {
            let idx_ek_eff_aud = START_IDX_OLD_P + 4 * ell + 3 * n;
            vector::range(0, ell).for_each(|i| {
                let new_r_i = *w.get(1 + ell + i);
                reprs.push_back(repr_scaled(idx_ek_eff_aud, new_r_i));
            });
        };

        // 4. Balance equation: dk · ⟨B, old_R⟩ + (⟨B, new_a⟩ + ⟨B, v⟩) · G
        let idx_old_R_start = START_IDX_OLD_P + ell;
        let point_idxs = vector[];
        let scalars = vector[];

        // dk · B^i · old_R[i]
        vector::range(0, ell).for_each(|i| {
            point_idxs.push_back(idx_old_R_start + i);
            scalars.push_back(dk.scalar_mul(&b_powers_ell[i]));
        });

        // new_a[i] · B^i · G
        vector::range(0, ell).for_each(|i| {
            let new_a_i = *w.get(1 + i);
            point_idxs.push_back(IDX_G);
            scalars.push_back(new_a_i.scalar_mul(&b_powers_ell[i]));
        });

        // v[j] · B^j · G (the secret transfer amount)
        vector::range(0, n).for_each(|j| {
            let v_j = *w.get(1 + 2 * ell + j);
            point_idxs.push_back(IDX_G);
            scalars.push_back(v_j.scalar_mul(&b_powers_n[j]));
        });

        reprs.push_back(new_representation(point_idxs, scalars));

        // === R_eq part ===

        let idx_v_start = 1 + 2 * ell;
        let idx_r_start = 1 + 2 * ell + n;

        // 5. v[j] · G + r[j] · H
        vector::range(0, n).for_each(|j| {
            let v_j = *w.get(idx_v_start + j);
            let r_j = *w.get(idx_r_start + j);
            reprs.push_back(new_representation(vector[IDX_G, IDX_H], vector[v_j, r_j]));
        });

        // 6. r[j] · ek_sender
        vector::range(0, n).for_each(|j| {
            let r_j = *w.get(idx_r_start + j);
            reprs.push_back(repr_scaled(IDX_EK_SENDER, r_j));
        });

        // 7. r[j] · ek_recip
        vector::range(0, n).for_each(|j| {
            let r_j = *w.get(idx_r_start + j);
            reprs.push_back(repr_scaled(IDX_EK_RECIP, r_j));
        });

        // 7b. (effective auditor only) r[j] · ek_eff_aud
        if (has_effective_auditor) {
            let idx_ek_eff_aud = START_IDX_OLD_P + 4 * ell + 3 * n;
            vector::range(0, n).for_each(|j| {
                let r_j = *w.get(idx_r_start + j);
                reprs.push_back(repr_scaled(idx_ek_eff_aud, r_j));
            });
        };

        // 7c. (voluntary auditors) r[j] · ek_volun_aud_t, for each voluntary auditor t
        let idx_volun_auds_start = START_IDX_OLD_P + 4 * ell + 3 * n
            + if (has_effective_auditor) { 1 + ell + n } else { 0 };
        vector::range(0, num_volun_auditors).for_each(|i| {
            let idx_ek_volun_aud = idx_volun_auds_start + i * (1 + n);
            vector::range(0, n).for_each(|j| {
                let r_j = *w.get(idx_r_start + j);
                reprs.push_back(repr_scaled(idx_ek_volun_aud, r_j));
            });
        });

        // WARNING: Crucial for security
        assert!(reprs.length() == expected_output_len(ell, n, has_effective_auditor, num_volun_auditors), e_wrong_output_len());
        new_representation_vec(reprs)
    }

    fun expected_output_len(ell: u64, n: u64, has_effective_auditor: bool, num_volun_auditors: u64): u64 {
        2 + 2 * ell + 3 * n + (if (has_effective_auditor) { ell + n } else { 0 }) + num_volun_auditors * n
    }

    /// The transformation function $f$ for the transfer relation (see module-level doc for full definition).
    fun f(
        _stmt: &Statement<Transfer>,
        has_effective_auditor: bool, num_volun_auditors: u64,
    ): RepresentationVec {
        let ell = get_ell();
        let n = get_n();
        let b_powers_ell = get_b_powers(ell);

        let idx_new_P_start = START_IDX_OLD_P + 2 * ell;
        let idx_new_R_start = START_IDX_OLD_P + 3 * ell;
        let idx_amount_P_start = START_IDX_OLD_P + 4 * ell;
        let idx_amount_R_sender_start = idx_amount_P_start + n;
        let idx_amount_R_recip_start = idx_amount_R_sender_start + n;

        let reprs = vector[];

        // === R^veiled_withdraw part ===

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

        // 3b. (effective auditor only) new_R_eff_aud[i]
        if (has_effective_auditor) {
            let idx_ek_eff_aud = START_IDX_OLD_P + 4 * ell + 3 * n;
            let idx_new_R_eff_aud_start = idx_ek_eff_aud + 1;
            vector::range(0, ell).for_each(|i| {
                reprs.push_back(repr_point(idx_new_R_eff_aud_start + i));
            });
        };

        // 4. ⟨B, old_P⟩ (no -v·G because v is secret)
        let point_idxs = vector[];
        let scalars = vector[];
        vector::range(0, ell).for_each(|i| {
            point_idxs.push_back(START_IDX_OLD_P + i);
            scalars.push_back(b_powers_ell[i]);
        });
        reprs.push_back(new_representation(point_idxs, scalars));

        // === R_eq part ===

        // 5. amount_P[j]
        vector::range(0, n).for_each(|j| {
            reprs.push_back(repr_point(idx_amount_P_start + j));
        });

        // 6. amount_R_sender[j]
        vector::range(0, n).for_each(|j| {
            reprs.push_back(repr_point(idx_amount_R_sender_start + j));
        });

        // 7. amount_R_recip[j]
        vector::range(0, n).for_each(|j| {
            reprs.push_back(repr_point(idx_amount_R_recip_start + j));
        });

        // 7b. (effective auditor only) amount_R_eff_aud[j]
        if (has_effective_auditor) {
            let idx_amount_R_eff_aud_start = START_IDX_OLD_P + 4 * ell + 3 * n + 1 + ell;
            vector::range(0, n).for_each(|j| {
                reprs.push_back(repr_point(idx_amount_R_eff_aud_start + j));
            });
        };

        // 7c. (voluntary auditors) amount_R_volun_aud_t[j], for each voluntary auditor t
        let idx_volun_auds_start = START_IDX_OLD_P + 4 * ell + 3 * n
            + if (has_effective_auditor) { 1 + ell + n } else { 0 };
        vector::range(0, num_volun_auditors).for_each(|i| {
            let idx_amount_R_volun_aud_start = idx_volun_auds_start + i * (1 + n) + 1;
            vector::range(0, n).for_each(|j| {
                reprs.push_back(repr_point(idx_amount_R_volun_aud_start + j));
            });
        });

        // Note: Not needed for security, since a mismatched f(X) length will be caught in the verifier. But good practice
        // for catching mistakes *early* when implementing your f(X).
        assert!(reprs.length() == expected_output_len(ell, n, has_effective_auditor, num_volun_auditors), e_wrong_output_len());
        new_representation_vec(reprs)
    }

    /// Asserts that a transfer proof verifies.
    public(friend) fun assert_verifies(
        self: &TransferSession, stmt: &Statement<Transfer>, proof: &Proof,
    ) {
        let has_eff = self.has_effective_auditor;
        let num_volun = self.num_volun_auditors;
        assert_transfer_statement_is_well_formed(stmt, has_eff, num_volun);

        let success = sigma_protocol::verify(
            new_domain_separator(@aptos_framework, chain_id::get(), PROTOCOL_ID, bcs::to_bytes(self)),
            |_X, w| psi(_X, w, has_eff, num_volun),
            |_X| f(_X, has_eff, num_volun),
            stmt,
            proof
        );

        assert!(success, error::invalid_argument(E_INVALID_TRANSFER_PROOF));
    }

    //
    // Tests
    //

    #[test_only]
    public(friend) fun transfer_session_for_testing(
        has_effective_auditor: bool, num_volun_auditors: u64,
    ): TransferSession {
        let (sender, asset_type) = setup_test_environment();
        TransferSession {
            sender: signer::address_of(&sender), recipient: @0x2, asset_type,
            num_avail_chunks: get_ell(), num_transfer_chunks: get_n(),
            has_effective_auditor, num_volun_auditors,
        }
    }

    #[test_only]
    /// Evaluates the transfer psi homomorphism for testing (wraps the private `psi` closure).
    public(friend) fun evaluate_psi_for_testing(
        stmt: &Statement<Transfer>, witn: &Witness,
        has_effective_auditor: bool, num_volun_auditors: u64,
    ): vector<RistrettoPoint> {
        evaluate_psi(|_X, w| psi(_X, w, has_effective_auditor, num_volun_auditors), stmt, witn)
    }

    #[test_only]
    /// Creates a transfer proof (for testing).
    public(friend) fun prove(self: &TransferSession, stmt: &Statement<Transfer>, witn: &Witness): Proof {
        let has_eff = self.has_effective_auditor;
        let num_volun = self.num_volun_auditors;
        let (proof, _) = sigma_protocol::prove(
            new_domain_separator(@aptos_framework, chain_id::get(), PROTOCOL_ID, bcs::to_bytes(self)),
            |_X, w| psi(_X, w, has_eff, num_volun),
            stmt,
            witn
        );
        proof
    }

}
