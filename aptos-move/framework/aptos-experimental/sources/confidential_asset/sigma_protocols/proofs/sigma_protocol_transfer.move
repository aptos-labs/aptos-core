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
/// - $T$: number of extra auditors ($T \ge 0$).
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
///     \opt{\mathsf{ek}^\mathsf{aid}, \new{\mathbf{R}}^\mathsf{aid}, \mathbf{R}^\mathsf{aid}},\;
///       \{\mathsf{ek}^\mathsf{ext}_t, \mathbf{R}^\mathsf{ext}_t\}_{t \in [T]}
///       \textbf{;}\\
///     \mathsf{dk}, \new{\mathbf{a}}, \new{\mathbf{r}}, \mathbf{v}, \mathbf{r}
/// \end{array}\right) = 1
/// \Leftrightarrow
/// \left\{\begin{array}{r@{\,\,}l@{\quad}l}
///     H &= \mathsf{dk} \cdot \mathsf{ek}^\mathsf{sid}\\
///     \new{P}_i &= \new{a}_i \cdot G + \new{r}_i \cdot H, &\forall i \in [\ell]\\
///     \new{R}_i &= \new{r}_i \cdot \mathsf{ek}^\mathsf{sid}, &\forall i \in [\ell]\\
///     \opt{\new{R}^\mathsf{aid}_i} &\opt{= \new{r}_i \cdot \mathsf{ek}^\mathsf{aid},}
///       &\opt{\forall i \in [\ell]}\\
///     \langle \mathbf{B}, \old{\mathbf{P}} \rangle &= \mathsf{dk} \cdot \langle \mathbf{B}, \old{\mathbf{R}} \rangle
///       + (\langle \mathbf{B}, \new{\mathbf{a}} \rangle + \langle \mathbf{B}, \mathbf{v} \rangle) \cdot G\\
///     P_j &= v_j \cdot G + r_j \cdot H, &\forall j \in [n]\\
///     R^\mathsf{sid}_j &= r_j \cdot \mathsf{ek}^\mathsf{sid}, &\forall j \in [n]\\
///     R^\mathsf{rid}_j &= r_j \cdot \mathsf{ek}^\mathsf{rid}, &\forall j \in [n]\\
///     \opt{R^\mathsf{aid}_j} &\opt{= r_j \cdot \mathsf{ek}^\mathsf{aid},}
///       &\opt{\forall j \in [n]}\\
///     R^\mathsf{ext}_{t,j} &= r_j \cdot \mathsf{ek}^\mathsf{ext}_t,
///       &\forall j \in [n],\; \forall t \in [T]\\
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
///     \opt{\new{r}_i \cdot \mathsf{ek}^\mathsf{aid}, \;\forall i \in [\ell]}\\
///     \mathsf{dk} \cdot \langle \mathbf{B}, \old{\mathbf{R}} \rangle
///       + (\langle \mathbf{B}, \new{\mathbf{a}} \rangle + \langle \mathbf{B}, \mathbf{v} \rangle) \cdot G\\
///     v_j \cdot G + r_j \cdot H, &\forall j \in [n]\\
///     r_j \cdot \mathsf{ek}^\mathsf{sid}, &\forall j \in [n]\\
///     r_j \cdot \mathsf{ek}^\mathsf{rid}, &\forall j \in [n]\\
///     \opt{r_j \cdot \mathsf{ek}^\mathsf{aid}, \;\forall j \in [n]}\\
///     r_j \cdot \mathsf{ek}^\mathsf{ext}_t, &\forall j \in [n],\; \forall t \in [T]\\
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
///     \langle \mathbf{B}, \old{\mathbf{P}} \rangle\\
///     P_j, &\forall j \in [n]\\
///     R^\mathsf{sid}_j, &\forall j \in [n]\\
///     R^\mathsf{rid}_j, &\forall j \in [n]\\
///     \opt{R^\mathsf{aid}_j, \;\forall j \in [n]}\\
///     R^\mathsf{ext}_{t,j}, &\forall j \in [n],\; \forall t \in [T]\\
/// \end{pmatrix}
/// $$
///
module aptos_experimental::sigma_protocol_transfer {
    use std::bcs;
    use std::error;
    use std::option::Option;
    use std::signer;
    use std::vector;
    use aptos_std::ristretto255::{Self, RistrettoPoint, Scalar, CompressedRistretto};
    use aptos_framework::fungible_asset::Metadata;
    use aptos_framework::object::Object;
    use aptos_experimental::confidential_pending_balance;
    use aptos_experimental::confidential_available_balance;
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
    /// For each extra auditor t: ..., ek_extra_auds[t], amount_R_extra_auds[t][1..n]
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

    /// Statement has wrong number of points.
    const E_WRONG_NUM_POINTS: u64 = 1;
    /// Statement scalars vector must be empty (v is secret, in witness).
    const E_WRONG_NUM_SCALARS: u64 = 2;
    /// Witness has wrong length.
    const E_WRONG_WITNESS_LEN: u64 = 3;
    /// Homomorphism output has wrong length.
    const E_WRONG_OUTPUT_LEN: u64 = 4;
    /// The transfer proof was invalid.
    const E_INVALID_TRANSFER_PROOF: u64 = 5;

    //
    // Structs
    //

    /// Used for domain separation in the Fiat-Shamir transform.
    struct TransferSession has drop {
        sender: address,
        recipient: address,
        asset_type: Object<Metadata>,
        num_avail_chunks: u64,
        num_transfer_chunks: u64,
        has_effective_auditor: bool,
        num_extra_auditors: u64,
    }

    //
    // Helper functions
    //

    /// Returns the fixed number of available balance chunks ℓ.
    inline fun get_ell(): u64 { confidential_available_balance::get_num_chunks() }

    /// Returns the fixed number of transfer (pending) balance chunks n.
    inline fun get_n(): u64 { confidential_pending_balance::get_num_chunks() }

    /// Returns the B^i powers for the chunk weighted-sum: B = 2^16.
    fun get_b_powers(count: u64): vector<Scalar> {
        let b = ristretto255::new_scalar_from_u128(65536u128);
        let powers = vector[ristretto255::scalar_one()];
        let prev = ristretto255::scalar_one();
        let i = 1;
        while (i < count) {
            prev = prev.scalar_mul(&b);
            powers.push_back(prev);
            i = i + 1;
        };
        powers
    }

    /// Validates the statement structure.
    ///
    /// Expected point count: 4 + 4ℓ + 3n + (has_eff ? 1+ℓ+n : 0) + num_extra*(1+n)
    fun assert_transfer_statement_is_well_formed(
        stmt: &Statement, has_effective_auditor: bool, num_extra_auditors: u64,
    ) {
        let ell = get_ell();
        let n = get_n();
        let num_points = stmt.get_points().length();

        let expected_num_points = 4 + 4 * ell + 3 * n
            + if (has_effective_auditor) { 1 + ell + n } else { 0 }
            + num_extra_auditors * (1 + n);
        assert!(
            num_points == expected_num_points,
            error::invalid_argument(E_WRONG_NUM_POINTS)
        );
        assert!(stmt.get_scalars().length() == 0, error::invalid_argument(E_WRONG_NUM_SCALARS));
    }

    //
    // Public functions
    //

    public fun new_session(
        sender: &signer,
        recipient: address,
        asset_type: Object<Metadata>,
        has_effective_auditor: bool,
        num_extra_auditors: u64,
    ): TransferSession {
        TransferSession {
            sender: signer::address_of(sender),
            recipient,
            asset_type,
            num_avail_chunks: confidential_available_balance::get_num_chunks(),
            num_transfer_chunks: confidential_pending_balance::get_num_chunks(),
            has_effective_auditor,
            num_extra_auditors,
        }
    }

    /// Creates a transfer statement, optionally including effective and extra auditor components.
    ///
    /// Points (base): [G, H, ek_sender, ek_recip, old_P[1..ℓ], old_R[1..ℓ], new_P[1..ℓ], new_R[1..ℓ], amount_P[1..n], amount_R_sender[1..n], amount_R_recip[1..n]]
    /// If effective: + [ek_eff_aud, new_R_eff_aud[1..ℓ], amount_R_eff_aud[1..n]]
    /// For each extra auditor t: + [ek_extra_auds[t], amount_R_extra_auds[t][1..n]]
    ///
    /// For no effective auditor, pass `option::none()` for `compressed_ek_eff_aud` and `ek_eff_aud`,
    /// and empty vectors for the effective auditor R components.
    /// For no extra auditors, pass empty vectors for the extra auditor components.
    public fun new_transfer_statement(
        compressed_G: CompressedRistretto, _G: RistrettoPoint,
        compressed_H: CompressedRistretto, _H: RistrettoPoint,
        compressed_ek_sender: CompressedRistretto, ek_sender: RistrettoPoint,
        compressed_ek_recip: CompressedRistretto, ek_recip: RistrettoPoint,
        compressed_old_P: vector<CompressedRistretto>, old_P: vector<RistrettoPoint>,
        compressed_old_R: vector<CompressedRistretto>, old_R: vector<RistrettoPoint>,
        compressed_new_P: vector<CompressedRistretto>, new_P: vector<RistrettoPoint>,
        compressed_new_R: vector<CompressedRistretto>, new_R: vector<RistrettoPoint>,
        compressed_amount_P: vector<CompressedRistretto>, amount_P: vector<RistrettoPoint>,
        compressed_amount_R_sender: vector<CompressedRistretto>, amount_R_sender: vector<RistrettoPoint>,
        compressed_amount_R_recip: vector<CompressedRistretto>, amount_R_recip: vector<RistrettoPoint>,
        compressed_ek_eff_aud: Option<CompressedRistretto>, ek_eff_aud: Option<RistrettoPoint>,
        compressed_new_R_eff_aud: vector<CompressedRistretto>, new_R_eff_aud: vector<RistrettoPoint>,
        compressed_amount_R_eff_aud: vector<CompressedRistretto>, amount_R_eff_aud: vector<RistrettoPoint>,
        compressed_ek_extra_auds: vector<CompressedRistretto>, ek_extra_auds: vector<RistrettoPoint>,
        compressed_amount_R_extra_auds: vector<vector<CompressedRistretto>>, amount_R_extra_auds: vector<vector<RistrettoPoint>>,
    ): Statement {
        let has_eff = ek_eff_aud.is_some();
        let num_extra = ek_extra_auds.length();

        let points = vector[_G, _H, ek_sender, ek_recip];
        points.append(old_P);
        points.append(old_R);
        points.append(new_P);
        points.append(new_R);
        points.append(amount_P);
        points.append(amount_R_sender);
        points.append(amount_R_recip);

        let compressed = vector[compressed_G, compressed_H, compressed_ek_sender, compressed_ek_recip];
        compressed.append(compressed_old_P);
        compressed.append(compressed_old_R);
        compressed.append(compressed_new_P);
        compressed.append(compressed_new_R);
        compressed.append(compressed_amount_P);
        compressed.append(compressed_amount_R_sender);
        compressed.append(compressed_amount_R_recip);

        // Effective auditor: ek, new_R[1..ℓ], amount_R[1..n]
        if (ek_eff_aud.is_some()) {
            points.push_back(ek_eff_aud.extract());
            points.append(new_R_eff_aud);
            points.append(amount_R_eff_aud);
            compressed.push_back(compressed_ek_eff_aud.extract());
            compressed.append(compressed_new_R_eff_aud);
            compressed.append(compressed_amount_R_eff_aud);
        };

        // Extra auditors: for each extra, append [ek_extra_aud, amount_R_extra_aud[1..n]]
        while (!ek_extra_auds.is_empty()) {
            points.push_back(ek_extra_auds.remove(0));
            points.append(amount_R_extra_auds.remove(0));
            compressed.push_back(compressed_ek_extra_auds.remove(0));
            compressed.append(compressed_amount_R_extra_auds.remove(0));
        };

        let stmt = new_statement(points, compressed, vector[]);
        assert_transfer_statement_is_well_formed(&stmt, has_eff, num_extra);
        stmt
    }

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
    ///
    /// $$
    /// \psi(\mathsf{dk}, \new{\mathbf{a}}, \new{\mathbf{r}}, \mathbf{v}, \mathbf{r}) = \begin{pmatrix}
    ///     \mathsf{dk} \cdot \mathsf{ek}^\mathsf{sid}\\
    ///     \new{a}_i \cdot G + \new{r}_i \cdot H, &\forall i \in [\ell]\\
    ///     \new{r}_i \cdot \mathsf{ek}^\mathsf{sid}, &\forall i \in [\ell]\\
    ///     \opt{\new{r}_i \cdot \mathsf{ek}^\mathsf{aid}, \;\forall i \in [\ell]}\\
    ///     \mathsf{dk} \cdot \langle \mathbf{B}, \old{\mathbf{R}} \rangle
    ///       + (\langle \mathbf{B}, \new{\mathbf{a}} \rangle + \langle \mathbf{B}, \mathbf{v} \rangle) \cdot G\\
    ///     v_j \cdot G + r_j \cdot H, &\forall j \in [n]\\
    ///     r_j \cdot \mathsf{ek}^\mathsf{sid}, &\forall j \in [n]\\
    ///     r_j \cdot \mathsf{ek}^\mathsf{rid}, &\forall j \in [n]\\
    ///     \opt{r_j \cdot \mathsf{ek}^\mathsf{aid}, \;\forall j \in [n]}\\
    ///     r_j \cdot \mathsf{ek}^\mathsf{ext}_t, &\forall j \in [n],\; \forall t \in [T]\\
    /// \end{pmatrix}
    /// $$
    public fun psi(
        stmt: &Statement, w: &Witness,
        has_effective_auditor: bool, num_extra_auditors: u64,
    ): RepresentationVec {
        // WARNING: Crucial for security
        assert_transfer_statement_is_well_formed(stmt, has_effective_auditor, num_extra_auditors);

        let ell = get_ell();
        let n = get_n();

        // WARNING: Crucial for security
        let expected_witness_len = 1 + 2 * ell + 2 * n;
        assert!(w.length() == expected_witness_len, error::invalid_argument(E_WRONG_WITNESS_LEN));

        let b_powers_ell = get_b_powers(ell);
        let b_powers_n = get_b_powers(n);

        let dk = *w.get(IDX_DK);

        let reprs = vector[];

        // === R^veiled_withdraw part ===

        // 1. dk · ek_sender
        reprs.push_back(new_representation(vector[IDX_EK_SENDER], vector[dk]));

        // 2. new_a[i] · G + new_r[i] · H
        vector::range(0, ell).for_each(|i| {
            let new_a_i = *w.get(1 + i);
            let new_r_i = *w.get(1 + ell + i);
            reprs.push_back(new_representation(vector[IDX_G, IDX_H], vector[new_a_i, new_r_i]));
        });

        // 3. new_r[i] · ek_sender
        vector::range(0, ell).for_each(|i| {
            let new_r_i = *w.get(1 + ell + i);
            reprs.push_back(new_representation(vector[IDX_EK_SENDER], vector[new_r_i]));
        });

        // 3b. (effective auditor only) new_r[i] · ek_eff_aud
        if (has_effective_auditor) {
            let idx_ek_eff_aud = START_IDX_OLD_P + 4 * ell + 3 * n;
            vector::range(0, ell).for_each(|i| {
                let new_r_i = *w.get(1 + ell + i);
                reprs.push_back(new_representation(vector[idx_ek_eff_aud], vector[new_r_i]));
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
            reprs.push_back(new_representation(vector[IDX_EK_SENDER], vector[r_j]));
        });

        // 7. r[j] · ek_recip
        vector::range(0, n).for_each(|j| {
            let r_j = *w.get(idx_r_start + j);
            reprs.push_back(new_representation(vector[IDX_EK_RECIP], vector[r_j]));
        });

        // 7b. (effective auditor only) r[j] · ek_eff_aud
        if (has_effective_auditor) {
            let idx_ek_eff_aud = START_IDX_OLD_P + 4 * ell + 3 * n;
            vector::range(0, n).for_each(|j| {
                let r_j = *w.get(idx_r_start + j);
                reprs.push_back(new_representation(vector[idx_ek_eff_aud], vector[r_j]));
            });
        };

        // 7c. (extra auditors) r[j] · ek_extra_aud_t, for each extra auditor t
        let idx_extra_auds_start = START_IDX_OLD_P + 4 * ell + 3 * n
            + if (has_effective_auditor) { 1 + ell + n } else { 0 };
        vector::range(0, num_extra_auditors).for_each(|i| {
            let idx_ek_extra_aud = idx_extra_auds_start + i * (1 + n);
            vector::range(0, n).for_each(|j| {
                let r_j = *w.get(idx_r_start + j);
                reprs.push_back(new_representation(vector[idx_ek_extra_aud], vector[r_j]));
            });
        });

        let repr_vec = new_representation_vec(reprs);
        let expected_output_len = 2 + 2 * ell + 3 * n
            + if (has_effective_auditor) { ell + n } else { 0 }
            + num_extra_auditors * n;

        // WARNING: Crucial for security
        assert!(repr_vec.length() == expected_output_len, error::invalid_argument(E_WRONG_OUTPUT_LEN));

        repr_vec
    }

    /// The transformation function $f$ for the transfer relation (see module-level doc for full definition).
    ///
    /// $$
    /// f(\mathbf{X}) = \begin{pmatrix}
    ///     H\\
    ///     \new{P}_i, &\forall i \in [\ell]\\
    ///     \new{R}_i, &\forall i \in [\ell]\\
    ///     \opt{\new{R}^\mathsf{aid}_i, \;\forall i \in [\ell]}\\
    ///     \langle \mathbf{B}, \old{\mathbf{P}} \rangle\\
    ///     P_j, &\forall j \in [n]\\
    ///     R^\mathsf{sid}_j, &\forall j \in [n]\\
    ///     R^\mathsf{rid}_j, &\forall j \in [n]\\
    ///     \opt{R^\mathsf{aid}_j, \;\forall j \in [n]}\\
    ///     R^\mathsf{ext}_{t,j}, &\forall j \in [n],\; \forall t \in [T]\\
    /// \end{pmatrix}
    /// $$
    public fun f(
        _stmt: &Statement,
        has_effective_auditor: bool, num_extra_auditors: u64,
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
        reprs.push_back(new_representation(vector[IDX_H], vector[ristretto255::scalar_one()]));

        // 2. new_P[i]
        vector::range(0, ell).for_each(|i| {
            reprs.push_back(new_representation(vector[idx_new_P_start + i], vector[ristretto255::scalar_one()]));
        });

        // 3. new_R[i]
        vector::range(0, ell).for_each(|i| {
            reprs.push_back(new_representation(vector[idx_new_R_start + i], vector[ristretto255::scalar_one()]));
        });

        // 3b. (effective auditor only) new_R_eff_aud[i]
        if (has_effective_auditor) {
            let idx_ek_eff_aud = START_IDX_OLD_P + 4 * ell + 3 * n;
            let idx_new_R_eff_aud_start = idx_ek_eff_aud + 1;
            vector::range(0, ell).for_each(|i| {
                reprs.push_back(new_representation(
                    vector[idx_new_R_eff_aud_start + i], vector[ristretto255::scalar_one()]
                ));
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
            reprs.push_back(new_representation(vector[idx_amount_P_start + j], vector[ristretto255::scalar_one()]));
        });

        // 6. amount_R_sender[j]
        vector::range(0, n).for_each(|j| {
            reprs.push_back(new_representation(vector[idx_amount_R_sender_start + j], vector[ristretto255::scalar_one()]));
        });

        // 7. amount_R_recip[j]
        vector::range(0, n).for_each(|j| {
            reprs.push_back(new_representation(vector[idx_amount_R_recip_start + j], vector[ristretto255::scalar_one()]));
        });

        // 7b. (effective auditor only) amount_R_eff_aud[j]
        if (has_effective_auditor) {
            let idx_amount_R_eff_aud_start = START_IDX_OLD_P + 4 * ell + 3 * n + 1 + ell;
            vector::range(0, n).for_each(|j| {
                reprs.push_back(new_representation(
                    vector[idx_amount_R_eff_aud_start + j], vector[ristretto255::scalar_one()]
                ));
            });
        };

        // 7c. (extra auditors) amount_R_extra_aud_t[j], for each extra auditor t
        let idx_extra_auds_start = START_IDX_OLD_P + 4 * ell + 3 * n
            + if (has_effective_auditor) { 1 + ell + n } else { 0 };
        vector::range(0, num_extra_auditors).for_each(|i| {
            let idx_amount_R_extra_aud_start = idx_extra_auds_start + i * (1 + n) + 1;
            vector::range(0, n).for_each(|j| {
                reprs.push_back(new_representation(
                    vector[idx_amount_R_extra_aud_start + j], vector[ristretto255::scalar_one()]
                ));
            });
        });

        new_representation_vec(reprs)
    }

    /// Asserts that a transfer proof verifies.
    public fun assert_verifies(
        session: &TransferSession, stmt: &Statement, proof: &Proof,
    ) {
        let has_eff = session.has_effective_auditor;
        let num_extra = session.num_extra_auditors;
        assert_transfer_statement_is_well_formed(stmt, has_eff, num_extra);

        let success = sigma_protocol::verify(
            new_domain_separator(PROTOCOL_ID, bcs::to_bytes(session)),
            |_X, w| psi(_X, w, has_eff, num_extra),
            |_X| f(_X, has_eff, num_extra),
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
        has_effective_auditor: bool, num_extra_auditors: u64,
    ): TransferSession {
        let sender = account::create_signer_for_test(@0x1);
        let (_, _, _, _, asset_type) = fungible_asset::create_fungible_asset(&sender);

        TransferSession {
            sender: signer::address_of(&sender),
            recipient: @0x2,
            asset_type,
            num_avail_chunks: get_ell(),
            num_transfer_chunks: get_n(),
            has_effective_auditor,
            num_extra_auditors,
        }
    }

    #[test_only]
    /// Creates a transfer proof (for testing).
    public fun prove(session: &TransferSession, stmt: &Statement, witn: &Witness): Proof {
        let has_eff = session.has_effective_auditor;
        let num_extra = session.num_extra_auditors;
        let (proof, _) = sigma_protocol::prove(
            new_domain_separator(PROTOCOL_ID, bcs::to_bytes(session)),
            |_X, w| psi(_X, w, has_eff, num_extra),
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
        has_effective_auditor: bool, num_extra_auditors: u64,
    ): (
        RistrettoPoint,                       // G
        RistrettoPoint,                       // H
        RistrettoPoint,                       // ek_sender
        RistrettoPoint,                       // ek_recip
        vector<RistrettoPoint>,               // old_P
        vector<RistrettoPoint>,               // old_R
        vector<RistrettoPoint>,               // new_P
        vector<RistrettoPoint>,               // new_R
        vector<RistrettoPoint>,               // amount_P
        vector<RistrettoPoint>,               // amount_R_sender
        vector<RistrettoPoint>,               // amount_R_recip
        Option<RistrettoPoint>,               // ek_eff_aud
        vector<RistrettoPoint>,               // new_R_eff_aud
        vector<RistrettoPoint>,               // amount_R_eff_aud
        vector<RistrettoPoint>,               // ek_extra_auds
        vector<vector<RistrettoPoint>>,       // amount_R_extra_auds
        Scalar,                               // dk_sender
        vector<Scalar>,                       // new_a
        vector<Scalar>,                       // new_r
        vector<Scalar>,                       // v
        vector<Scalar>,                       // r
    ) {
        // Generate keypairs
        let (dk_sender, compressed_ek_sender) = generate_twisted_elgamal_keypair();
        let (_, compressed_ek_recip) = generate_twisted_elgamal_keypair();

        let ek_aud_eff = if (has_effective_auditor) {
            let (_, compressed_ek_eff_aud) = generate_twisted_elgamal_keypair();
            std::option::some(compressed_ek_eff_aud)
        } else {
            std::option::none()
        };

        // Create sender's old and new balances (effective auditor sees balance R_aud)
        let amount = 100u128;
        let old_amount = 1000u128;
        let new_amount = old_amount - amount;

        let old_randomness = confidential_available_balance::generate_balance_randomness();
        let old_balance = confidential_available_balance::new_from_amount(
            old_amount, &old_randomness, &compressed_ek_sender, &ek_aud_eff
        );

        let new_randomness = confidential_available_balance::generate_balance_randomness();
        let new_balance = confidential_available_balance::new_from_amount(
            new_amount, &new_randomness, &compressed_ek_sender, &ek_aud_eff
        );

        // Create transfer amount ciphertexts as PendingBalances (C = P, D = R under each key)
        let amount_randomness = confidential_pending_balance::generate_balance_randomness();
        let amount_sender = confidential_pending_balance::new_from_amount(
            amount, &amount_randomness, &compressed_ek_sender
        );
        let amount_recip = confidential_pending_balance::new_from_amount(
            amount, &amount_randomness, &compressed_ek_recip
        );

        // Create effective auditor transfer amount ciphertexts (if set)
        let amount_R_eff_aud = if (ek_aud_eff.is_some()) {
            let amount_eff_aud = confidential_pending_balance::new_from_amount(
                amount, &amount_randomness, ek_aud_eff.borrow()
            );
            points_clone(amount_eff_aud.get_R())
        } else {
            vector[]
        };

        // Create extra auditor transfer amount ciphertexts
        let ek_extra_auds = vector[];
        let r_extra_auds = vector[];
        vector::range(0, num_extra_auditors).for_each(|_| {
            let (_, ek_extra_aud) = generate_twisted_elgamal_keypair();
            let amount_extra_aud = confidential_pending_balance::new_from_amount(
                amount, &amount_randomness, &ek_extra_aud
            );
            ek_extra_auds.push_back(ek_extra_aud.point_decompress());
            r_extra_auds.push_back(points_clone(amount_extra_aud.get_R()));
        });

        // Build raw points
        let _G = ristretto255::basepoint();
        let _H = get_encryption_key_basepoint_compressed().point_decompress();
        let ek_sender = compressed_ek_sender.point_decompress();
        let ek_recip = compressed_ek_recip.point_decompress();

        let ek_eff_aud = if (ek_aud_eff.is_some()) {
            std::option::some(ek_aud_eff.borrow().point_decompress())
        } else {
            std::option::none()
        };

        let old_P = points_clone(old_balance.get_P());
        let old_R = points_clone(old_balance.get_R());
        let new_P = points_clone(new_balance.get_P());
        let new_R = points_clone(new_balance.get_R());
        let amount_P = points_clone(amount_sender.get_P());
        let amount_R_sender = points_clone(amount_sender.get_R());
        let amount_R_recip = points_clone(amount_recip.get_R());
        let new_R_eff_aud = points_clone(new_balance.get_R_aud());

        // Build witness scalars
        let new_a = confidential_available_balance::split_into_chunks(new_amount);
        let new_r = *new_randomness.scalars();
        let v = confidential_pending_balance::split_into_chunks(amount);
        let r = *amount_randomness.scalars();

        (_G, _H, ek_sender, ek_recip,
         old_P, old_R, new_P, new_R,
            amount_P, amount_R_sender, amount_R_recip,
            ek_eff_aud, new_R_eff_aud, amount_R_eff_aud,
            ek_extra_auds, r_extra_auds,
            dk_sender, new_a, new_r, v, r)
    }

    #[test_only]
    /// Generates a random valid statement-witness pair for testing.
    /// Supports all auditor configurations:
    ///   - has_effective_auditor=false, num_extra=0: no auditors
    ///   - has_effective_auditor=true, num_extra=0: effective only
    ///   - has_effective_auditor=false, num_extra>0: extra only
    ///   - has_effective_auditor=true, num_extra>0: both
    fun random_valid_statement_witness_pair(
        has_effective_auditor: bool, num_extra_auditors: u64,
    ): (Statement, Witness) {
        let (_G, _H, ek_sender, ek_recip,
             old_P, old_R, new_P, new_R,
             amount_P, amount_R_sender, amount_R_recip,
             ek_eff_aud, new_R_eff_aud, amount_R_eff_aud,
             ek_extra_auds, amount_R_extra_auds,
             dk_sender, new_a, new_r, v, r,
        ) = random_valid_statement_witness_pair_internal(has_effective_auditor, num_extra_auditors);

        // Compress all points before moving originals into the statement
        let stmt = new_transfer_statement(
            _G.point_compress(), _G, _H.point_compress(), _H,
            ek_sender.point_compress(), ek_sender, ek_recip.point_compress(), ek_recip,
            compress_points(&old_P), old_P,
            compress_points(&old_R), old_R,
            compress_points(&new_P), new_P,
            compress_points(&new_R), new_R,
            compress_points(&amount_P), amount_P,
            compress_points(&amount_R_sender), amount_R_sender,
            compress_points(&amount_R_recip), amount_R_recip,
            if (ek_eff_aud.is_some()) { std::option::some(ek_eff_aud.borrow().point_compress()) }
            else { std::option::none() },
            ek_eff_aud,
            compress_points(&new_R_eff_aud), new_R_eff_aud,
            compress_points(&amount_R_eff_aud), amount_R_eff_aud,
            compress_points(&ek_extra_auds), ek_extra_auds,
            amount_R_extra_auds.map_ref(|r| compress_points(r)), amount_R_extra_auds,
        );

        let witn = new_transfer_witness(dk_sender, new_a, new_r, v, r);

        (stmt, witn)
    }

    #[test_only]
    /// Verifies that `evaluate_psi` produces the same points as a manual computation using
    /// direct ristretto255 arithmetic, for all auditor configurations.
    ///
    /// The manual computation uses only the raw points/scalars from `_internal` — no `IDX_*`
    /// constants — so it is completely independent of the statement layout.
    fun psi_correctness(has_eff: bool, num_extra: u64) {
        let (ell, n) = (get_ell(), get_n());
        let b_powers_ell = get_b_powers(ell);
        let b_powers_n = get_b_powers(n);

        let (_G, _H, ek_sender, ek_recip,
             old_P, old_R, new_P, new_R,
             amount_P, amount_R_sender, amount_R_recip,
             ek_eff_aud, new_R_eff_aud, amount_R_eff_aud,
             ek_extra_auds, amount_R_extra_auds,
             dk_sender, new_a, new_r, v, r,
        ) = random_valid_statement_witness_pair_internal(has_eff, num_extra);

        // Build statement + witness for evaluate_psi (cloning points that the manual computation also needs)
        let stmt = new_transfer_statement(
            _G.point_compress(), _G.point_clone(),
            _H.point_compress(), _H.point_clone(),
            ek_sender.point_compress(), ek_sender.point_clone(),
            ek_recip.point_compress(), ek_recip.point_clone(),
            compress_points(&old_P), old_P,
            compress_points(&old_R), points_clone(&old_R), // old_R needed for manual computation
            compress_points(&new_P), new_P,
            compress_points(&new_R), new_R,
            compress_points(&amount_P), amount_P,
            compress_points(&amount_R_sender), amount_R_sender,
            compress_points(&amount_R_recip), amount_R_recip,
            if (ek_eff_aud.is_some()) { std::option::some(ek_eff_aud.borrow().point_compress()) } else { std::option::none() },
            if (ek_eff_aud.is_some()) { std::option::some(ek_eff_aud.borrow().point_clone()) } else { std::option::none() },
            compress_points(&new_R_eff_aud), new_R_eff_aud,
            compress_points(&amount_R_eff_aud), amount_R_eff_aud,
            compress_points(&ek_extra_auds), points_clone(&ek_extra_auds), // ek_extra_auds needed for manual computation
            amount_R_extra_auds.map_ref(|points| compress_points(points)), amount_R_extra_auds,
        );
        let witn = new_transfer_witness(dk_sender, new_a, new_r, v, r);

        //
        // Manually compute the homomorphism using raw components (no IDX_* constants)
        //
        let manual_psi = vector[];

        // 1. dk_sender · ek_sender
        manual_psi.push_back(ek_sender.point_mul(&dk_sender));

        // 2. new_a[i] · G + new_r[i] · H, for i in [1..ell]
        vector::range(0, ell).for_each(|i| {
            manual_psi.push_back(ristretto255::double_scalar_mul(&new_a[i], &_G, &new_r[i], &_H));
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
        let dk_inner_b_old_R = ristretto255::multi_scalar_mul(&old_R, &dk_b_scalars);

        let inner_b_new_a = ristretto255::scalar_zero();
        vector::range(0, ell).for_each(|i| {
            inner_b_new_a = inner_b_new_a.scalar_add(&new_a[i].scalar_mul(&b_powers_ell[i]));
        });
        let inner_b_v = ristretto255::scalar_zero();
        vector::range(0, n).for_each(|j| {
            inner_b_v = inner_b_v.scalar_add(&v[j].scalar_mul(&b_powers_n[j]));
        });
        let sum_times_G = _G.point_mul(&inner_b_new_a.scalar_add(&inner_b_v));

        manual_psi.push_back(dk_inner_b_old_R.point_add(&sum_times_G));

        // 5. v[j] · G + r[j] · H, for j in [1..n]
        vector::range(0, n).for_each(|j| {
            manual_psi.push_back(ristretto255::double_scalar_mul(&v[j], &_G, &r[j], &_H));
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

        // 7c. (extra auditors) r[j] · ek_extra_aud_t, for j in [1..n], for each extra auditor t
        vector::range(0, num_extra).for_each(|t| {
            vector::range(0, n).for_each(|j| {
                manual_psi.push_back(ek_extra_auds[t].point_mul(&r[j]));
            });
        });

        //
        // Compare: implemented_psi vs manual_psi computation
        //
        let implemented_psi = evaluate_psi(
            |_X, w| psi(_X, w, has_eff, num_extra), &stmt, &witn
        );

        assert!(equal_vec_points(&implemented_psi, &manual_psi), 1);
    }

    #[test]
    fun psi_correctness_no_auditors() { psi_correctness(false, 0); }

    #[test]
    fun psi_correctness_effective_only() { psi_correctness(true, 0); }

    #[test]
    fun psi_correctness_extra_only() { psi_correctness(false, 2); }

    #[test]
    fun psi_correctness_both_auditors() { psi_correctness(true, 2); }

    #[test]
    fun proof_correctness() {
        // Create the asset once (cannot call create_fungible_asset multiple times in one test)
        let sender = account::create_signer_for_test(@0x1);
        let (_, _, _, _, asset_type) = fungible_asset::create_fungible_asset(&sender);

        // Test all auditor configurations
        vector[false, true].for_each(|has_eff| {
            vector[0u64, 1, 2].for_each(|num_extra| {
                let ss = new_session(&sender, @0x2, asset_type, has_eff, num_extra);
                let (stmt, witn) = random_valid_statement_witness_pair(has_eff, num_extra);
                let proof = prove(&ss, &stmt, &witn);
                assert_verifies(&ss, &stmt, &proof);
            });
        });
    }

    #[test]
    #[expected_failure(abort_code=65537, location=aptos_experimental::sigma_protocol_fiat_shamir)]
    fun proof_soundness_empty_proof() {
        let (stmt, _) = random_valid_statement_witness_pair(false, 0);
        let proof = sigma_protocol_proof::empty();
        assert_verifies(&transfer_session_for_testing(false, 0), &stmt, &proof);
    }
}
