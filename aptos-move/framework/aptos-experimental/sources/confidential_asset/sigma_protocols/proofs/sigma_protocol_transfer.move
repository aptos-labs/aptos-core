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
module aptos_experimental::sigma_protocol_transfer {
    friend aptos_experimental::confidential_asset;

    use std::bcs;
    use std::error;
    use std::option::Option;
    use std::signer;
    use std::vector;
    use aptos_std::ristretto255::{Self, RistrettoPoint, CompressedRistretto};
    use aptos_framework::chain_id;
    use aptos_framework::fungible_asset::Metadata;
    use aptos_framework::object::Object;
    use aptos_experimental::confidential_balance::{Pending, Available, CompressedBalance, Balance,
        get_num_available_chunks, get_num_pending_chunks, get_b_powers, new_pending_from_p_and_r};
    use aptos_experimental::confidential_amount::CompressedAmount;
    use aptos_experimental::ristretto255_twisted_elgamal;
    use aptos_experimental::sigma_protocol;
    use aptos_experimental::sigma_protocol_proof::Proof;
    use aptos_experimental::sigma_protocol_fiat_shamir::new_domain_separator;
    use aptos_experimental::sigma_protocol_witness::Witness;
    #[test_only]
    use aptos_experimental::sigma_protocol_witness::new_secret_witness;
    use aptos_experimental::sigma_protocol_statement::Statement;
    use aptos_experimental::sigma_protocol_statement_builder::new_builder;
    use aptos_experimental::sigma_protocol_utils::{e_wrong_num_points, e_wrong_num_scalars, e_wrong_witness_len, e_wrong_output_len};
    use aptos_experimental::sigma_protocol_representation::{repr_point, repr_scaled, new_representation};
    use aptos_experimental::sigma_protocol_representation_vec::{RepresentationVec, new_representation_vec};
    #[test_only]
    use aptos_experimental::sigma_protocol_test_utils::setup_test_environment;
    #[test_only]
    use aptos_experimental::confidential_amount;
    #[test_only]
    use aptos_experimental::ristretto255_twisted_elgamal::{
        get_encryption_key_basepoint_compressed, generate_twisted_elgamal_keypair
    };
    #[test_only]
    use aptos_experimental::sigma_protocol_homomorphism::evaluate_psi;
    #[test_only]
    use aptos_experimental::sigma_protocol_proof;
    #[test_only]
    use aptos_experimental::sigma_protocol_utils::{equal_vec_points, points_clone};
    #[test_only]
    use aptos_std::ristretto255::{Scalar, double_scalar_mul, multi_scalar_mul, scalar_zero};
    #[test_only]
    use aptos_experimental::confidential_balance::{ConfidentialBalanceRandomness,
        generate_available_randomness, generate_pending_randomness,
        new_available_from_amount, split_available_into_chunks, split_pending_into_chunks};

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
    const E_INVALID_TRANSFER_PROOF: u64 = 5;
    /// The number of auditor R components does not match the expected auditor count.
    const E_AUDITOR_COUNT_MISMATCH: u64 = 6;

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
    ): (Statement<Transfer>, vector<RistrettoPoint>, Balance<Pending>) {
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

        let b = new_builder();
        b.add_point(ristretto255::basepoint_compressed());                                     // G
        b.add_point(ristretto255_twisted_elgamal::get_encryption_key_basepoint_compressed());  // H
        b.add_point(compressed_ek_sender);                                                      // ek_sender
        b.add_point(compressed_ek_recip);                                                       // ek_recip
        b.add_points(compressed_old_balance.get_compressed_P());                                // old_P
        b.add_points(compressed_old_balance.get_compressed_R());                                // old_R
        let (_, new_balance_P) = b.add_points_cloned(compressed_new_balance.get_compressed_P()); // new_P
        b.add_points(compressed_new_balance.get_compressed_R());                                // new_R
        let (_, amount_P) = b.add_points_cloned(compressed_amount.get_compressed_P());          // amount_P
        b.add_points(compressed_amount.get_compressed_R_sender());                              // amount_R_sender
        let (_, recip_R) = b.add_points_cloned(compressed_amount.get_compressed_R_recip());     // amount_R_recip

        // Effective auditor: ek, new_R[1..ℓ], amount_R[1..n]
        if (has_eff) {
            let ek_eff = *compressed_ek_eff_aud.borrow();
            b.add_point(ek_eff);                                                                // ek_eff_aud
            b.add_points(compressed_new_balance.get_compressed_R_aud());                        // new_R_eff_aud
            b.add_points(compressed_amount.get_compressed_R_eff_aud());                         // amount_R_eff_aud
        };

        // Voluntary auditors: for each, append [ek_volun_aud, amount_R_volun_aud[1..n]]
        let compressed_R_volun_auds = compressed_amount.get_compressed_R_volun_auds();
        vector::range(0, num_volun).for_each(|i| {
            let ek_volun = compressed_ek_volun_auds[i];
            b.add_point(ek_volun);                                                              // ek_volun_aud
            b.add_points(&compressed_R_volun_auds[i]);                                          // amount_R_volun_aud
        });

        let stmt = b.build();
        assert_transfer_statement_is_well_formed(&stmt, has_eff, num_volun);
        let recip_pending = new_pending_from_p_and_r(amount_P, recip_R);
        (stmt, new_balance_P, recip_pending)
    }

    #[test_only]
    /// Creates a transfer witness: (dk, new_a[1..ℓ], new_r[1..ℓ], v[1..n], r[1..n]).
    public fun new_transfer_witness(
        dk: Scalar, new_a: vector<Scalar>, new_r: vector<Scalar>,
        v: vector<Scalar>, r: vector<Scalar>,
    ): Witness {
        let w = vector[dk];
        w.append(new_a);
        w.append(new_r);
        w.append(v);
        w.append(r);
        new_secret_witness(w)
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

        let repr_vec = new_representation_vec(reprs);
        let expected_output_len = 2 + 2 * ell + 3 * n
            + if (has_effective_auditor) { ell + n } else { 0 }
            + num_volun_auditors * n;

        // WARNING: Crucial for security
        assert!(repr_vec.length() == expected_output_len, e_wrong_output_len());

        repr_vec
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
            new_domain_separator(@aptos_experimental, chain_id::get(), PROTOCOL_ID, bcs::to_bytes(self)),
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
    fun transfer_session_for_testing(
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
    /// Creates a transfer proof (for testing).
    public fun prove(self: &TransferSession, stmt: &Statement<Transfer>, witn: &Witness): Proof {
        let has_eff = self.has_effective_auditor;
        let num_volun = self.num_volun_auditors;
        let (proof, _) = sigma_protocol::prove(
            new_domain_separator(@aptos_experimental, chain_id::get(), PROTOCOL_ID, bcs::to_bytes(self)),
            |_X, w| psi(_X, w, has_eff, num_volun),
            stmt,
            witn
        );
        proof
    }

    #[test_only]
    /// Creates transfer ciphertexts under all keys, builds the transfer statement and witness.
    /// Callers provide randomness so they can access the scalars afterwards (e.g., for range proofs).
    public fun build_transfer_statement_and_witness(
        dk_sender: &Scalar,
        compressed_ek_sender: &CompressedRistretto,
        compressed_ek_recip: &CompressedRistretto,
        compressed_old_balance: &CompressedBalance<Available>,
        compressed_ek_eff_aud: &Option<CompressedRistretto>,
        compressed_ek_volun_auds: &vector<CompressedRistretto>,
        amount_u64: u64,
        new_balance_u128: u128,
        new_balance_randomness: &ConfidentialBalanceRandomness,
        amount_randomness: &ConfidentialBalanceRandomness,
    ): (
        Statement<Transfer>, Witness,
        Balance<Available>,
        confidential_amount::Amount,
    ) {
        let new_balance = new_available_from_amount(
            new_balance_u128, new_balance_randomness, compressed_ek_sender, compressed_ek_eff_aud
        );

        let amount = confidential_amount::new_from_amount(
            amount_u64, amount_randomness,
            compressed_ek_sender, compressed_ek_recip,
            compressed_ek_eff_aud, compressed_ek_volun_auds,
        );

        let compressed_new_balance = new_balance.compress();
        let compressed_amount = amount.compress();
        let (stmt, _, _) = new_transfer_statement(
            *compressed_ek_sender, *compressed_ek_recip,
            compressed_old_balance, &compressed_new_balance,
            &compressed_amount,
            compressed_ek_eff_aud, compressed_ek_volun_auds,
        );

        let new_a = split_available_into_chunks(new_balance_u128);
        let new_r = *new_balance_randomness.scalars();
        let v = split_pending_into_chunks(amount_u64 as u128);
        let r = *amount_randomness.scalars();
        let witn = new_transfer_witness(*dk_sender, new_a, new_r, v, r);

        (stmt, witn, new_balance, amount)
    }

    #[test_only]
    /// Generates random keys for the sender, recipient and auditor(s), and an old balance for our transfer tests.
    fun generate_keys_and_ciphertexts(
        has_effective_auditor: bool, num_volun_auditors: u64,
    ): (
        Scalar, CompressedRistretto, CompressedRistretto,
        Option<CompressedRistretto>, vector<CompressedRistretto>,
        Balance<Available>,
    ) {
        let (dk_sender, compressed_ek_sender) = generate_twisted_elgamal_keypair();
        let (_, compressed_ek_recip) = generate_twisted_elgamal_keypair();
        let compressed_ek_eff_aud = if (has_effective_auditor) {
            let (_, ek) = generate_twisted_elgamal_keypair();
            std::option::some(ek)
        } else { std::option::none() };
        let compressed_ek_volun_auds = vector::range(0, num_volun_auditors).map(|_| {
            let (_, ek) = generate_twisted_elgamal_keypair();
            ek
        });

        let old_balance_randomness = generate_available_randomness();
        let old_balance = new_available_from_amount(
            1000, &old_balance_randomness, &compressed_ek_sender, &compressed_ek_eff_aud
        );

        (dk_sender, compressed_ek_sender, compressed_ek_recip, compressed_ek_eff_aud, compressed_ek_volun_auds, old_balance)
    }

    #[test_only]
    /// Generates a random valid statement-witness pair for testing.
    /// Supports all auditor configurations.
    fun random_valid_statement_witness_pair(
        has_effective_auditor: bool, num_volun_auditors: u64,
    ): (Statement<Transfer>, Witness) {
        let (dk_sender, compressed_ek_sender, compressed_ek_recip,
             compressed_ek_eff_aud, compressed_ek_volun_auds, old_balance) =
            generate_keys_and_ciphertexts(has_effective_auditor, num_volun_auditors);

        let new_balance_randomness = generate_available_randomness();
        let amount_randomness = generate_pending_randomness();

        let compressed_old_balance = old_balance.compress();
        let (stmt, witn, _, _) = build_transfer_statement_and_witness(
            &dk_sender, &compressed_ek_sender, &compressed_ek_recip, &compressed_old_balance,
            &compressed_ek_eff_aud, &compressed_ek_volun_auds,
            100, 900, &new_balance_randomness, &amount_randomness,
        );

        (stmt, witn)
    }

    #[test_only]
    /// Verifies that `evaluate_psi` produces the same points as a manual computation using
    /// direct ristretto255 arithmetic, for all auditor configurations.
    ///
    /// The manual computation uses only raw points/scalars — no `IDX_*` constants — so it is
    /// completely independent of the statement layout.
    fun psi_correctness(has_eff: bool, num_volun: u64) {
        let (ell, n) = (get_ell(), get_n());
        let b_powers_ell = get_b_powers(ell);
        let b_powers_n = get_b_powers(n);

        let (dk_sender, compressed_ek_sender, compressed_ek_recip,
             compressed_ek_eff_aud, compressed_ek_volun_auds, old_balance) =
            generate_keys_and_ciphertexts(has_eff, num_volun);

        let new_balance_randomness = generate_available_randomness();
        let amount_randomness = generate_pending_randomness();

        let compressed_old_balance = old_balance.compress();
        let (stmt, witn, _, _) = build_transfer_statement_and_witness(
            &dk_sender, &compressed_ek_sender, &compressed_ek_recip, &compressed_old_balance,
            &compressed_ek_eff_aud, &compressed_ek_volun_auds,
            100, 900,
            &new_balance_randomness, &amount_randomness,
        );

        // Decompress keys for manual psi computation
        let _G = ristretto255::basepoint();
        let _H = get_encryption_key_basepoint_compressed().point_decompress();
        let ek_sender = compressed_ek_sender.point_decompress();
        let ek_recip = compressed_ek_recip.point_decompress();
        let ek_eff_aud = compressed_ek_eff_aud.map(|ek| ek.point_decompress());
        let ek_volun_auds = compressed_ek_volun_auds.map(|ek| ek.point_decompress());
        let old_R = points_clone(old_balance.get_R());
        let new_a = split_available_into_chunks(900);
        let new_r = *new_balance_randomness.scalars();
        let v = split_pending_into_chunks(100);
        let r = *amount_randomness.scalars();

        //
        // Manually compute the homomorphism using raw components (no IDX_* constants)
        //
        let manual_psi = vector[];

        // 1. dk_sender · ek_sender
        manual_psi.push_back(ek_sender.point_mul(&dk_sender));

        // 2. new_a[i] · G + new_r[i] · H, for i in [1..ell]
        vector::range(0, ell).for_each(|i| {
            manual_psi.push_back(double_scalar_mul(&new_a[i], &_G, &new_r[i], &_H));
        });

        // 3. new_r[i] · ek_sender, for i in [1..ell]
        vector::range(0, ell).for_each(|i| {
            manual_psi.push_back(ek_sender.point_mul(&new_r[i]));
        });

        // 3b. (effective auditor only) new_r[i] · ek_eff_aud, for i in [1..ell]
        if (ek_eff_aud.is_some()) {
            let ek_eff_aud_pt = ek_eff_aud.borrow();
            vector::range(0, ell).for_each(|i| {
                manual_psi.push_back(ek_eff_aud_pt.point_mul(&new_r[i]));
            });
        };

        // 4. dk_sender · ⟨B, old_R⟩ + (⟨B, new_a⟩ + ⟨B, v⟩) · G
        let dk_b_scalars: vector<Scalar> = vector::range(0, ell).map(|i| {
            dk_sender.scalar_mul(&b_powers_ell[i])
        });
        let dk_inner_b_old_R = multi_scalar_mul(&old_R, &dk_b_scalars);

        let inner_b_new_a = scalar_zero();
        vector::range(0, ell).for_each(|i| {
            inner_b_new_a = inner_b_new_a.scalar_add(&new_a[i].scalar_mul(&b_powers_ell[i]));
        });
        let inner_b_v = scalar_zero();
        vector::range(0, n).for_each(|j| {
            inner_b_v = inner_b_v.scalar_add(&v[j].scalar_mul(&b_powers_n[j]));
        });
        let sum_times_G = _G.point_mul(&inner_b_new_a.scalar_add(&inner_b_v));

        manual_psi.push_back(dk_inner_b_old_R.point_add(&sum_times_G));

        // 5. v[j] · G + r[j] · H, for j in [1..n]
        vector::range(0, n).for_each(|j| {
            manual_psi.push_back(double_scalar_mul(&v[j], &_G, &r[j], &_H));
        });

        // 6. r[j] · ek_sender, for j in [1..n]
        vector::range(0, n).for_each(|j| {
            manual_psi.push_back(ek_sender.point_mul(&r[j]));
        });

        // 7. r[j] · ek_recip, for j in [1..n]
        vector::range(0, n).for_each(|j| {
            manual_psi.push_back(ek_recip.point_mul(&r[j]));
        });

        // 7b. (effective auditor only) r[j] · ek_eff_aud, for j in [1..n]
        if (ek_eff_aud.is_some()) {
            let ek_eff_aud_pt = ek_eff_aud.borrow();
            vector::range(0, n).for_each(|j| {
                manual_psi.push_back(ek_eff_aud_pt.point_mul(&r[j]));
            });
        };

        // 7c. (voluntary auditors) r[j] · ek_volun_aud_t, for j in [1..n], for each voluntary auditor t
        vector::range(0, num_volun).for_each(|t| {
            vector::range(0, n).for_each(|j| {
                manual_psi.push_back(ek_volun_auds[t].point_mul(&r[j]));
            });
        });

        //
        // Compare: implemented_psi vs manual_psi computation
        //
        let implemented_psi = evaluate_psi(
            |_X, w| psi(_X, w, has_eff, num_volun), &stmt, &witn
        );

        assert!(equal_vec_points(&implemented_psi, &manual_psi), 1);
    }

    #[test]
    fun psi_correctness_0_volun() { psi_correctness(false, 0); }
    #[test]
    fun psi_correctness_1_volun() { psi_correctness(false, 1); }
    #[test]
    fun psi_correctness_2_volun() { psi_correctness(false, 2); }
    #[test]
    fun psi_correctness_effective_0_volun() { psi_correctness(true, 0); }
    #[test]
    fun psi_correctness_effective_1_volun() { psi_correctness(true, 1); }
    #[test]
    fun psi_correctness_effective_2_volun() { psi_correctness(true, 2); }

    #[test]
    fun proof_correctness() {
        let (sender, asset_type) = setup_test_environment();

        // Test all auditor configurations
        vector[false, true].for_each(|has_eff| {
            vector[0u64, 1, 2].for_each(|num_volun| {
                let ss = new_session(&sender, @0x2, asset_type, has_eff, num_volun);
                let (stmt, witn) = random_valid_statement_witness_pair(has_eff, num_volun);
                ss.assert_verifies(&stmt, &ss.prove(&stmt, &witn));
            });
        });
    }

    #[test]
    #[expected_failure(abort_code=65537, location=aptos_experimental::sigma_protocol_fiat_shamir)]
    fun proof_soundness_empty_proof() {
        let (stmt, _) = random_valid_statement_witness_pair(false, 0);
        transfer_session_for_testing(false, 0).assert_verifies(&stmt, &sigma_protocol_proof::empty());
    }
}
