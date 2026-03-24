#[test_only]
/// Tests for all four sigma protocol proof modules (registration, key rotation, withdrawal, transfer).
/// Moved from the source modules to reduce source code size.
module aptos_framework::sigma_protocol_proof_tests {
    use std::error;
    use std::option;
    use std::vector;
    use aptos_std::ristretto255::{Self, Scalar, CompressedRistretto,
        random_scalar, new_scalar_from_u64, point_identity_compressed,
        double_scalar_mul, multi_scalar_mul, scalar_zero};
    use aptos_framework::confidential_balance::{Self,
        Available, Balance, ConfidentialBalanceRandomness};
    use aptos_framework::sigma_protocol_proof;
    use aptos_framework::sigma_protocol_statement::Statement;
    use aptos_framework::sigma_protocol_statement_builder::new_builder;
    use aptos_framework::sigma_protocol_witness::Witness;
    use aptos_framework::sigma_protocol_utils::points_clone;
    use aptos_framework::confidential_crypto_test_utils::{
        generate_twisted_elgamal_keypair, generate_available_randomness,
        generate_pending_randomness, new_available_from_amount, equal_vec_points,
        compressed_identity_points, new_amount_from_u64,
    };
    use aptos_framework::sigma_protocol_witness::new_secret_witness;
    use aptos_framework::sigma_protocol_test_utils::setup_test_environment;
    use aptos_framework::confidential_balance::get_encryption_key_basepoint_compressed;

    // ========================================= //
    //      Registration proof tests              //
    // ========================================= //

    use aptos_framework::sigma_protocol_registration::{Registration,
        registration_session_for_testing as reg_session,
        compute_statement_and_witness as reg_stmt_witn,
    };

    #[test]
    /// Verifies that a correctly computed proof verifies.
    fun registration_proof_correctness() {
        let dk = random_scalar();
        let (stmt, witn) = reg_stmt_witn(&dk);

        let ss = reg_session();
        let proof = ss.prove(&stmt, &witn);

        ss.assert_verifies(&stmt, &proof);
    }

    #[test]
    #[expected_failure(abort_code=65537, location=aptos_framework::sigma_protocol_fiat_shamir)]
    /// Verifies that an empty proof does not verify for a random statement.
    fun registration_proof_soundness_against_random_statement() {
        let dk = random_scalar();
        let (stmt, _) = reg_stmt_witn(&dk);

        let proof = sigma_protocol_proof::empty();

        reg_session().assert_verifies(&stmt, &proof);
    }

    #[test]
    #[expected_failure(abort_code=65537, location=aptos_framework::sigma_protocol_fiat_shamir)]
    /// Verifies that an empty proof does not verify for a "zero" statement (all identity points).
    fun registration_proof_soundness_against_zero_statement() {
        let b = new_builder();
        b.add_point(point_identity_compressed()); // H
        b.add_point(point_identity_compressed()); // ek
        let stmt: Statement<Registration> = b.build();

        let proof = sigma_protocol_proof::empty();

        reg_session().assert_verifies(&stmt, &proof);
    }

    // ========================================= //
    //      Key rotation proof tests              //
    // ========================================= //

    use aptos_framework::sigma_protocol_key_rotation::{
        key_rotation_session_for_testing as keyrot_session,
        random_valid_statement_witness_pair as keyrot_stmt_witn,
        new_key_rotation_statement,
    };

    #[test]
    /// Verifies that a correctly computed proof verifies.
    fun key_rotation_proof_correctness() {
        let (stmt, witn) = keyrot_stmt_witn();
        let ss = keyrot_session();
        ss.assert_verifies(&stmt, &ss.prove(&stmt, &witn));
    }

    #[test]
    #[expected_failure(abort_code=65537, location=aptos_framework::sigma_protocol_fiat_shamir)]
    /// Verifies that an empty proof does not verify for a random statement.
    fun key_rotation_proof_soundness_against_random_statement() {
        let (stmt, _) = keyrot_stmt_witn();
        keyrot_session().assert_verifies(&stmt, &sigma_protocol_proof::empty());
    }

    #[test]
    #[expected_failure(abort_code=65537, location=aptos_framework::sigma_protocol_fiat_shamir)]
    /// Verifies that an empty proof does not verify for a "zero" statement (all identity points).
    fun key_rotation_proof_soundness_against_zero_statement() {
        let ell = confidential_balance::get_num_available_chunks();

        let stmt = new_key_rotation_statement(
            point_identity_compressed(),
            point_identity_compressed(),
            &compressed_identity_points(ell),
            &compressed_identity_points(ell),
        );

        keyrot_session().assert_verifies(&stmt, &sigma_protocol_proof::empty());
    }

    // ========================================= //
    //      Withdrawal proof tests                //
    // ========================================= //

    use aptos_framework::sigma_protocol_withdraw::{Withdrawal,
        evaluate_psi_for_testing as withdraw_evaluate_psi,
        new_withdrawal_statement,
    };

    const E_TEST_WITHDRAW: u64 = 1;

    public fun new_withdrawal_witness(dk: Scalar, new_a: vector<Scalar>, new_r: vector<Scalar>): Witness {
        assert!(new_a.length() == new_r.length(), error::internal(E_TEST_WITHDRAW));

        let w = vector[dk];
        w.append(new_a);
        w.append(new_r);
        new_secret_witness(w)
    }

    /// Returns raw components for building a random valid withdrawal statement-witness pair.
    fun random_withdrawal_components(
        amount: u64, with_auditor: bool,
    ): (
        CompressedRistretto, option::Option<CompressedRistretto>,
        Balance<Available>, Balance<Available>,
        Scalar, Scalar, vector<Scalar>, vector<Scalar>,
    ) {
        let (dk, compressed_ek) = generate_twisted_elgamal_keypair();

        let compressed_ek_aud = if (with_auditor) {
            let (_, ek_aud) = generate_twisted_elgamal_keypair();
            option::some(ek_aud)
        } else {
            option::none()
        };

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
        let new_a = confidential_balance::split_available_into_chunks(new_balance_u128);
        let new_r = *new_randomness.scalars();

        (compressed_ek, compressed_ek_aud, old_balance, new_balance, v, dk, new_a, new_r)
    }

    fun random_withdrawal_stmt_witn(amount: u64, with_auditor: bool): (Statement<Withdrawal>, Witness) {
        let (compressed_ek, compressed_ek_aud, old_balance, new_balance,
            v, dk, new_a, new_r
        ) = random_withdrawal_components(amount, with_auditor);

        let compressed_old_balance = old_balance.compress();
        let compressed_new_balance = new_balance.compress();

        let (stmt, _) = new_withdrawal_statement(
            compressed_ek, &compressed_old_balance, &compressed_new_balance, &compressed_ek_aud, v,
        );
        let witn = new_withdrawal_witness(dk, new_a, new_r);

        (stmt, witn)
    }

    /// Verifies that `evaluate_psi` produces the same points as a manual computation.
    fun withdraw_psi_correctness(with_auditor: bool) {
        let ell = confidential_balance::get_num_available_chunks();
        let (compressed_ek, compressed_ek_aud, old_balance, new_balance,
            v, dk, new_a, new_r
        ) = random_withdrawal_components(100, with_auditor);
        let b_powers = confidential_balance::get_b_powers(ell);

        let _G = ristretto255::basepoint();
        let _H = get_encryption_key_basepoint_compressed().point_decompress();
        let ek = compressed_ek.point_decompress();
        let ek_aud = compressed_ek_aud.map(|ek| ek.point_decompress());

        assert!(_H.point_equals(&ek.point_mul(&dk)), error::internal(E_TEST_WITHDRAW));

        let old_R_clone = points_clone(old_balance.get_R());

        let (stmt, _) = new_withdrawal_statement(
            compressed_ek, &old_balance.compress(), &new_balance.compress(), &compressed_ek_aud, v,
        );
        let witn = new_withdrawal_witness(dk, new_a, new_r);

        // Manually compute the homomorphism using raw components (no IDX_* constants)
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

        // Compare: implemented_psi vs manual_psi computation
        let implemented_psi = withdraw_evaluate_psi(&stmt, &witn, with_auditor);

        assert!(equal_vec_points(&implemented_psi, &manual_psi), 1);
    }

    #[test]
    fun withdraw_psi_correctness_no_auditor() { withdraw_psi_correctness(false); }

    #[test]
    fun withdraw_psi_correctness_with_auditor() { withdraw_psi_correctness(true); }

    #[test]
    fun withdraw_proof_correctness() {
        let (sender, asset_type) = setup_test_environment();

        vector[false, true].for_each(|with_auditor| {
            vector[0u64, 100].for_each(|amount| {
                let ss = aptos_framework::sigma_protocol_withdraw::new_session(
                    &sender, asset_type, with_auditor
                );
                let (stmt, witn) = random_withdrawal_stmt_witn(amount, with_auditor);
                ss.assert_verifies(&stmt, &ss.prove(&stmt, &witn));
            });
        });
    }

    #[test]
    #[expected_failure(abort_code=65537, location=aptos_framework::sigma_protocol_fiat_shamir)]
    fun withdraw_proof_soundness_empty_proof() {
        let (sender, asset_type) = setup_test_environment();

        let (stmt, _) = random_withdrawal_stmt_witn(100, false);
        aptos_framework::sigma_protocol_withdraw::new_session(
            &sender, asset_type, false
        ).assert_verifies(&stmt, &sigma_protocol_proof::empty());
    }

    // ========================================= //
    //      Transfer proof tests                  //
    // ========================================= //

    use aptos_framework::sigma_protocol_transfer::{Transfer,
        evaluate_psi_for_testing as transfer_evaluate_psi,
        transfer_session_for_testing,
        new_transfer_statement,
    };
    use aptos_framework::confidential_amount;

    fun new_transfer_witness(
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

    /// Creates transfer ciphertexts under all keys, builds the transfer statement and witness.
    public fun build_transfer_statement_and_witness(
        dk_sender: &Scalar,
        compressed_ek_sender: &CompressedRistretto,
        compressed_ek_recip: &CompressedRistretto,
        compressed_old_balance: &confidential_balance::CompressedBalance<Available>,
        compressed_ek_eff_aud: &option::Option<CompressedRistretto>,
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

        let amount = new_amount_from_u64(
            amount_u64, amount_randomness,
            compressed_ek_sender, compressed_ek_recip,
            compressed_ek_eff_aud, compressed_ek_volun_auds,
        );

        let compressed_new_balance = new_balance.compress();
        let compressed_amount = confidential_amount::compress(&amount);
        let (stmt, _, _) = new_transfer_statement(
            *compressed_ek_sender, *compressed_ek_recip,
            compressed_old_balance, &compressed_new_balance,
            &compressed_amount,
            compressed_ek_eff_aud, compressed_ek_volun_auds,
        );

        let new_a = confidential_balance::split_available_into_chunks(new_balance_u128);
        let new_r = *new_balance_randomness.scalars();
        let v = confidential_balance::split_pending_into_chunks(amount_u64 as u128);
        let r = *amount_randomness.scalars();
        let witn = new_transfer_witness(*dk_sender, new_a, new_r, v, r);

        (stmt, witn, new_balance, amount)
    }

    /// Generates random keys for sender, recipient, auditor(s), and an old balance.
    fun generate_transfer_keys_and_ciphertexts(
        has_effective_auditor: bool, num_volun_auditors: u64,
    ): (
        Scalar, CompressedRistretto, CompressedRistretto,
        option::Option<CompressedRistretto>, vector<CompressedRistretto>,
        Balance<Available>,
    ) {
        let (dk_sender, compressed_ek_sender) = generate_twisted_elgamal_keypair();
        let (_, compressed_ek_recip) = generate_twisted_elgamal_keypair();
        let compressed_ek_eff_aud = if (has_effective_auditor) {
            let (_, ek) = generate_twisted_elgamal_keypair();
            option::some(ek)
        } else { option::none() };
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

    fun random_transfer_stmt_witn(
        has_effective_auditor: bool, num_volun_auditors: u64,
    ): (Statement<Transfer>, Witness) {
        let (dk_sender, compressed_ek_sender, compressed_ek_recip,
             compressed_ek_eff_aud, compressed_ek_volun_auds, old_balance) =
            generate_transfer_keys_and_ciphertexts(has_effective_auditor, num_volun_auditors);

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

    /// Verifies that `evaluate_psi` produces the same points as a manual computation.
    fun transfer_psi_correctness(has_eff: bool, num_volun: u64) {
        let ell = confidential_balance::get_num_available_chunks();
        let n = confidential_balance::get_num_pending_chunks();
        let b_powers_ell = confidential_balance::get_b_powers(ell);
        let b_powers_n = confidential_balance::get_b_powers(n);

        let (dk_sender, compressed_ek_sender, compressed_ek_recip,
             compressed_ek_eff_aud, compressed_ek_volun_auds, old_balance) =
            generate_transfer_keys_and_ciphertexts(has_eff, num_volun);

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
        let new_a = confidential_balance::split_available_into_chunks(900);
        let new_r = *new_balance_randomness.scalars();
        let v = confidential_balance::split_pending_into_chunks(100);
        let r = *amount_randomness.scalars();

        // Manually compute the homomorphism using raw components (no IDX_* constants)
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

        // Compare: implemented_psi vs manual_psi computation
        let implemented_psi = transfer_evaluate_psi(&stmt, &witn, has_eff, num_volun);

        assert!(equal_vec_points(&implemented_psi, &manual_psi), 1);
    }

    #[test]
    fun transfer_psi_correctness_0_volun() { transfer_psi_correctness(false, 0); }
    #[test]
    fun transfer_psi_correctness_1_volun() { transfer_psi_correctness(false, 1); }
    #[test]
    fun transfer_psi_correctness_2_volun() { transfer_psi_correctness(false, 2); }
    #[test]
    fun transfer_psi_correctness_effective_0_volun() { transfer_psi_correctness(true, 0); }
    #[test]
    fun transfer_psi_correctness_effective_1_volun() { transfer_psi_correctness(true, 1); }
    #[test]
    fun transfer_psi_correctness_effective_2_volun() { transfer_psi_correctness(true, 2); }

    #[test]
    fun transfer_proof_correctness() {
        let (sender, asset_type) = setup_test_environment();

        vector[false, true].for_each(|has_eff| {
            vector[0u64, 1, 2].for_each(|num_volun| {
                let ss = aptos_framework::sigma_protocol_transfer::new_session(
                    &sender, @0x2, asset_type, has_eff, num_volun
                );
                let (stmt, witn) = random_transfer_stmt_witn(has_eff, num_volun);
                ss.assert_verifies(&stmt, &ss.prove(&stmt, &witn));
            });
        });
    }

    #[test]
    #[expected_failure(abort_code=65537, location=aptos_framework::sigma_protocol_fiat_shamir)]
    fun transfer_proof_soundness_empty_proof() {
        let (stmt, _) = random_transfer_stmt_witn(false, 0);
        transfer_session_for_testing(false, 0).assert_verifies(&stmt, &sigma_protocol_proof::empty());
    }
}
