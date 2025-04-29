/// The `confidential_proof` module provides the infrastructure for verifying zero-knowledge proofs used in the Confidential Asset protocol.
/// These proofs ensure correctness for operations such as `confidential_transfer`, `withdraw`, `rotate_encryption_key`, and `normalize`.
module aptos_experimental::confidential_proof {
    use std::error;
    use std::option;
    use std::option::Option;
    use std::vector;
    use aptos_std::ristretto255::{Self, CompressedRistretto, Scalar};
    use aptos_std::ristretto255_bulletproofs::{Self as bulletproofs, RangeProof};

    use aptos_experimental::confidential_balance;
    use aptos_experimental::ristretto255_twisted_elgamal as twisted_elgamal;

    friend aptos_experimental::confidential_asset;

    //
    // Errors
    //

    const ESIGMA_PROTOCOL_VERIFY_FAILED: u64 = 1;
    const ERANGE_PROOF_VERIFICATION_FAILED: u64 = 2;

    //
    // Constants
    //

    const FIAT_SHAMIR_WITHDRAWAL_SIGMA_DST: vector<u8> = b"AptosConfidentialAsset/WithdrawalProofFiatShamir";
    const FIAT_SHAMIR_TRANSFER_SIGMA_DST: vector<u8> = b"AptosConfidentialAsset/TransferProofFiatShamir";
    const FIAT_SHAMIR_ROTATION_SIGMA_DST: vector<u8> = b"AptosConfidentialAsset/RotationProofFiatShamir";
    const FIAT_SHAMIR_NORMALIZATION_SIGMA_DST: vector<u8> = b"AptosConfidentialAsset/NormalizationProofFiatShamir";

    const BULLETPROOFS_DST: vector<u8> = b"AptosConfidentialAsset/BulletproofRangeProof";
    const BULLETPROOFS_NUM_BITS: u64 = 16;

    //
    // Structs
    //

    /// Represents the proof structure for validating a withdrawal operation.
    struct WithdrawalProof has drop {
        /// Sigma proof ensuring that the withdrawal operation maintains balance integrity.
        sigma_proof: WithdrawalSigmaProof,
        /// Range proof ensuring that the resulting balance chunks are normalized (i.e., within the 16-bit limit).
        zkrp_new_balance: RangeProof,
    }

    /// Represents the proof structure for validating a transfer operation.
    struct TransferProof has drop {
        /// Sigma proof ensuring that the transfer operation maintains balance integrity and correctness.
        sigma_proof: TransferSigmaProof,
        /// Range proof ensuring that the resulting balance chunks for the sender are normalized (i.e., within the 16-bit limit).
        zkrp_new_balance: RangeProof,
        /// Range proof ensuring that the transferred amount chunks are normalized (i.e., within the 16-bit limit).
        zkrp_transfer_amount: RangeProof,
    }

    /// Represents the proof structure for validating a normalization operation.
    struct NormalizationProof has drop {
        /// Sigma proof ensuring that the normalization operation maintains balance integrity.
        sigma_proof: NormalizationSigmaProof,
        /// Range proof ensuring that the resulting balance chunks are normalized (i.e., within the 16-bit limit).
        zkrp_new_balance: RangeProof,
    }

    /// Represents the proof structure for validating a key rotation operation.
    struct RotationProof has drop {
        /// Sigma proof ensuring that the key rotation operation preserves balance integrity.
        sigma_proof: RotationSigmaProof,
        /// Range proof ensuring that the resulting balance chunks after key rotation are normalized (i.e., within the 16-bit limit).
        zkrp_new_balance: RangeProof,
    }

    //
    // Helper structs
    //

    struct WithdrawalSigmaProofXs has drop {
        x1: CompressedRistretto,
        x2: CompressedRistretto,
        x3s: vector<CompressedRistretto>,
        x4s: vector<CompressedRistretto>,
    }

    struct WithdrawalSigmaProofAlphas has drop {
        a1s: vector<Scalar>,
        a2: Scalar,
        a3: Scalar,
        a4s: vector<Scalar>,
    }

    struct WithdrawalSigmaProofGammas has drop {
        g1: Scalar,
        g2: Scalar,
        g3s: vector<Scalar>,
        g4s: vector<Scalar>,
    }

    struct WithdrawalSigmaProof has drop {
        alphas: WithdrawalSigmaProofAlphas,
        xs: WithdrawalSigmaProofXs,
    }

    struct TransferSigmaProofXs has drop {
        x1: CompressedRistretto,
        x2s: vector<CompressedRistretto>,
        x3s: vector<CompressedRistretto>,
        x4s: vector<CompressedRistretto>,
        x5: CompressedRistretto,
        x6s: vector<CompressedRistretto>,
        x7s: vector<vector<CompressedRistretto>>,
    }

    struct TransferSigmaProofAlphas has drop {
        a1s: vector<Scalar>,
        a2: Scalar,
        a3s: vector<Scalar>,
        a4s: vector<Scalar>,
        a5: Scalar,
    }

    struct TransferSigmaProofGammas has drop {
        g1: Scalar,
        g2s: vector<Scalar>,
        g3s: vector<Scalar>,
        g4s: vector<Scalar>,
        g5: Scalar,
        g6s: vector<Scalar>,
        g7s: vector<vector<Scalar>>,
    }

    struct TransferSigmaProof has drop {
        alphas: TransferSigmaProofAlphas,
        xs: TransferSigmaProofXs,
    }

    struct NormalizationSigmaProofXs has drop {
        x1: CompressedRistretto,
        x2: CompressedRistretto,
        x3s: vector<CompressedRistretto>,
        x4s: vector<CompressedRistretto>,
    }

    struct NormalizationSigmaProofAlphas has drop {
        a1s: vector<Scalar>,
        a2: Scalar,
        a3: Scalar,
        a4s: vector<Scalar>,
    }

    struct NormalizationSigmaProofGammas has drop {
        g1: Scalar,
        g2: Scalar,
        g3s: vector<Scalar>,
        g4s: vector<Scalar>,
    }

    struct NormalizationSigmaProof has drop {
        alphas: NormalizationSigmaProofAlphas,
        xs: NormalizationSigmaProofXs,
    }

    struct RotationSigmaProofXs has drop {
        x1: CompressedRistretto,
        x2: CompressedRistretto,
        x3: CompressedRistretto,
        x4s: vector<CompressedRistretto>,
        x5s: vector<CompressedRistretto>,
    }

    struct RotationSigmaProofAlphas has drop {
        a1s: vector<Scalar>,
        a2: Scalar,
        a3: Scalar,
        a4: Scalar,
        a5s: vector<Scalar>,
    }

    struct RotationSigmaProofGammas has drop {
        g1: Scalar,
        g2: Scalar,
        g3: Scalar,
        g4s: vector<Scalar>,
        g5s: vector<Scalar>,
    }

    struct RotationSigmaProof has drop {
        alphas: RotationSigmaProofAlphas,
        xs: RotationSigmaProofXs,
    }

    //
    // Proof verification functions
    //

    /// Verifies the validity of the `withdraw` operation.
    ///
    /// This function ensures that the provided proof (`WithdrawalProof`) meets the following conditions:
    /// 1. The current balance (`current_balance`) and new balance (`new_balance`) encrypt the corresponding values
    ///    under the same encryption key (`ek`) before and after the withdrawal of the specified amount (`amount`), respectively.
    /// 2. The relationship `new_balance = current_balance - amount` holds, verifying that the withdrawal amount is deducted correctly.
    /// 3. The new balance (`new_balance`) is normalized, with each chunk adhering to the range [0, 2^16).
    ///
    /// If all conditions are satisfied, the proof validates the withdrawal; otherwise, the function causes an error.
    public fun verify_withdrawal_proof(
        ek: &twisted_elgamal::CompressedPubkey,
        amount: u64,
        current_balance: &confidential_balance::ConfidentialBalance,
        new_balance: &confidential_balance::ConfidentialBalance,
        proof: &WithdrawalProof)
    {
        verify_withdrawal_sigma_proof(ek, amount, current_balance, new_balance, &proof.sigma_proof);
        verify_new_balance_range_proof(new_balance, &proof.zkrp_new_balance);
    }

    /// Verifies the validity of the `confidential_transfer` operation.
    ///
    /// This function ensures that the provided proof (`TransferProof`) meets the following conditions:
    /// 1. The transferred amount (`transfer_amount`) and the auditor's balances (`auditor_amounts`), if provided,
    ///    encrypt the same transfer value under the recipient's encryption key (`recipient_ek`) and the auditor's
    ///    encryption keys (`auditor_eks`), respectively.
    /// 2. The sender's current balance (`current_balance`) and new balance (`new_balance`) encrypt the corresponding values
    ///    under the sender's encryption key (`sender_ek`) before and after the transfer, respectively.
    /// 3. The relationship `new_balance = current_balance - transfer_amount` is maintained, ensuring balance integrity.
    /// 4. The transferred value is properly normalized, with each chunk in both `transfer_amount` and the `auditor_amounts`
    ///    balance adhering to the range [0, 2^16).
    /// 5. The sender's new balance is normalized, with each chunk in `new_balance` also adhering to the range [0, 2^16).
    ///
    /// If all conditions are satisfied, the proof validates the transfer; otherwise, the function causes an error.
    public fun verify_transfer_proof(
        sender_ek: &twisted_elgamal::CompressedPubkey,
        recipient_ek: &twisted_elgamal::CompressedPubkey,
        current_balance: &confidential_balance::ConfidentialBalance,
        new_balance: &confidential_balance::ConfidentialBalance,
        transfer_amount: &confidential_balance::ConfidentialBalance,
        auditor_eks: &vector<twisted_elgamal::CompressedPubkey>,
        auditor_amounts: &vector<confidential_balance::ConfidentialBalance>,
        proof: &TransferProof)
    {
        verify_transfer_sigma_proof(
            sender_ek,
            recipient_ek,
            current_balance,
            new_balance,
            transfer_amount,
            auditor_eks,
            auditor_amounts,
            &proof.sigma_proof
        );
        verify_new_balance_range_proof(new_balance, &proof.zkrp_new_balance);
        verify_transfer_amount_range_proof(transfer_amount, &proof.zkrp_transfer_amount);
    }

    /// Verifies the validity of the `normalize` operation.
    ///
    /// This function ensures that the provided proof (`NormalizationProof`) meets the following conditions:
    /// 1. The current balance (`current_balance`) and new balance (`new_balance`) encrypt the same value
    ///    under the same provided encryption key (`ek`), verifying that the normalization process preserves the balance value.
    /// 2. The new balance (`new_balance`) is properly normalized, with each chunk adhering to the range [0, 2^16),
    ///    as verified through the range proof in the normalization process.
    ///
    /// If all conditions are satisfied, the proof validates the normalization; otherwise, the function causes an error.
    public fun verify_normalization_proof(
        ek: &twisted_elgamal::CompressedPubkey,
        current_balance: &confidential_balance::ConfidentialBalance,
        new_balance: &confidential_balance::ConfidentialBalance,
        proof: &NormalizationProof)
    {
        verify_normalization_sigma_proof(ek, current_balance, new_balance, &proof.sigma_proof);
        verify_new_balance_range_proof(new_balance, &proof.zkrp_new_balance);
    }

    /// Verifies the validity of the `rotate_encryption_key` operation.
    ///
    /// This function ensures that the provided proof (`RotationProof`) meets the following conditions:
    /// 1. The current balance (`current_balance`) and new balance (`new_balance`) encrypt the same value under the
    ///    current encryption key (`current_ek`) and the new encryption key (`new_ek`), respectively, verifying
    ///    that the key rotation preserves the balance value.
    /// 2. The new balance (`new_balance`) is properly normalized, with each chunk adhering to the range [0, 2^16),
    ///    ensuring balance integrity after the key rotation.
    ///
    /// If all conditions are satisfied, the proof validates the key rotation; otherwise, the function causes an error.
    public fun verify_rotation_proof(
        current_ek: &twisted_elgamal::CompressedPubkey,
        new_ek: &twisted_elgamal::CompressedPubkey,
        current_balance: &confidential_balance::ConfidentialBalance,
        new_balance: &confidential_balance::ConfidentialBalance,
        proof: &RotationProof)
    {
        verify_rotation_sigma_proof(current_ek, new_ek, current_balance, new_balance, &proof.sigma_proof);
        verify_new_balance_range_proof(new_balance, &proof.zkrp_new_balance);
    }

    //
    // Verification functions implementations
    //

    /// Verifies the validity of the `WithdrawalSigmaProof`.
    fun verify_withdrawal_sigma_proof(
        ek: &twisted_elgamal::CompressedPubkey,
        amount: u64,
        current_balance: &confidential_balance::ConfidentialBalance,
        new_balance: &confidential_balance::ConfidentialBalance,
        proof: &WithdrawalSigmaProof)
    {
        let amount_chunks = confidential_balance::split_into_chunks_u64(amount);
        let amount = ristretto255::new_scalar_from_u64(amount);

        let rho = fiat_shamir_withdrawal_sigma_proof_challenge(ek, &amount_chunks, current_balance, &proof.xs);

        let gammas = msm_withdrawal_gammas(&rho);

        let scalars_lhs = vector[gammas.g1, gammas.g2];
        scalars_lhs.append(gammas.g3s);
        scalars_lhs.append(gammas.g4s);

        let points_lhs = vector[
            ristretto255::point_decompress(&proof.xs.x1),
            ristretto255::point_decompress(&proof.xs.x2)
        ];
        points_lhs.append(proof.xs.x3s.map_ref(|x| ristretto255::point_decompress(x)));
        points_lhs.append(proof.xs.x4s.map_ref(|x| ristretto255::point_decompress(x)));

        let scalar_g = scalar_linear_combination(
            &proof.alphas.a1s,
            &vector::range(0, 8).map(|i| new_scalar_from_pow2(i * 16))
        );
        ristretto255::scalar_mul_assign(&mut scalar_g, &gammas.g1);
        ristretto255::scalar_add_assign(
            &mut scalar_g,
            &scalar_linear_combination(&gammas.g3s, &proof.alphas.a1s)
        );
        ristretto255::scalar_sub_assign(&mut scalar_g, &scalar_mul_3(&gammas.g1, &rho, &amount));

        let scalar_h = ristretto255::scalar_mul(&gammas.g2, &proof.alphas.a3);
        ristretto255::scalar_add_assign(
            &mut scalar_h,
            &scalar_linear_combination(&gammas.g3s, &proof.alphas.a4s)
        );

        let scalar_ek = ristretto255::scalar_mul(&gammas.g2, &rho);
        ristretto255::scalar_add_assign(
            &mut scalar_ek,
            &scalar_linear_combination(&gammas.g4s, &proof.alphas.a4s)
        );

        let scalars_current_balance_d = vector::range(0, 8).map(|i| {
            scalar_mul_3(&gammas.g1, &proof.alphas.a2, &new_scalar_from_pow2(i * 16))
        });

        let scalars_new_balance_d = vector::range(0, 8).map(|i| {
            ristretto255::scalar_mul(&gammas.g4s[i], &rho)
        });

        let scalars_current_balance_c = vector::range(0, 8).map(|i| {
            scalar_mul_3(&gammas.g1, &rho, &new_scalar_from_pow2(i * 16))
        });

        let scalars_new_balance_c = vector::range(0, 8).map(|i| {
            ristretto255::scalar_mul(&gammas.g3s[i], &rho)
        });

        let scalars_rhs = vector[scalar_g, scalar_h, scalar_ek];
        scalars_rhs.append(scalars_current_balance_d);
        scalars_rhs.append(scalars_new_balance_d);
        scalars_rhs.append(scalars_current_balance_c);
        scalars_rhs.append(scalars_new_balance_c);

        let points_rhs = vector[
            ristretto255::basepoint(),
            ristretto255::hash_to_point_base(),
            twisted_elgamal::pubkey_to_point(ek)
        ];
        points_rhs.append(confidential_balance::balance_to_points_d(current_balance));
        points_rhs.append(confidential_balance::balance_to_points_d(new_balance));
        points_rhs.append(confidential_balance::balance_to_points_c(current_balance));
        points_rhs.append(confidential_balance::balance_to_points_c(new_balance));

        let lhs = ristretto255::multi_scalar_mul(&points_lhs, &scalars_lhs);
        let rhs = ristretto255::multi_scalar_mul(&points_rhs, &scalars_rhs);

        assert!(
            ristretto255::point_equals(&lhs, &rhs),
            error::invalid_argument(ESIGMA_PROTOCOL_VERIFY_FAILED)
        );
    }

    /// Verifies the validity of the `TransferSigmaProof`.
    fun verify_transfer_sigma_proof(
        sender_ek: &twisted_elgamal::CompressedPubkey,
        recipient_ek: &twisted_elgamal::CompressedPubkey,
        current_balance: &confidential_balance::ConfidentialBalance,
        new_balance: &confidential_balance::ConfidentialBalance,
        transfer_amount: &confidential_balance::ConfidentialBalance,
        auditor_eks: &vector<twisted_elgamal::CompressedPubkey>,
        auditor_amounts: &vector<confidential_balance::ConfidentialBalance>,
        proof: &TransferSigmaProof)
    {
        let rho = fiat_shamir_transfer_sigma_proof_challenge(
            sender_ek,
            recipient_ek,
            current_balance,
            new_balance,
            transfer_amount,
            auditor_eks,
            auditor_amounts,
            &proof.xs
        );

        let gammas = msm_transfer_gammas(&rho, proof.xs.x7s.length());

        let scalars_lhs = vector[gammas.g1];
        scalars_lhs.append(gammas.g2s);
        scalars_lhs.append(gammas.g3s);
        scalars_lhs.append(gammas.g4s);
        scalars_lhs.push_back(gammas.g5);
        scalars_lhs.append(gammas.g6s);
        gammas.g7s.for_each(|gamma| scalars_lhs.append(gamma));

        let points_lhs = vector[
            ristretto255::point_decompress(&proof.xs.x1),
        ];
        points_lhs.append(proof.xs.x2s.map_ref(|x| ristretto255::point_decompress(x)));
        points_lhs.append(proof.xs.x3s.map_ref(|x| ristretto255::point_decompress(x)));
        points_lhs.append(proof.xs.x4s.map_ref(|x| ristretto255::point_decompress(x)));
        points_lhs.push_back(ristretto255::point_decompress(&proof.xs.x5));
        points_lhs.append(proof.xs.x6s.map_ref(|x| ristretto255::point_decompress(x)));
        proof.xs.x7s.for_each_ref(|xs| {
            points_lhs.append(xs.map_ref(|x| ristretto255::point_decompress(x)));
        });

        let scalar_g = scalar_linear_combination(
            &proof.alphas.a1s,
            &vector::range(0, 8).map(|i| new_scalar_from_pow2(i * 16))
        );
        ristretto255::scalar_mul_assign(&mut scalar_g, &gammas.g1);
        vector::range(0, 4).for_each(|i| {
            ristretto255::scalar_add_assign(
                &mut scalar_g,
                &ristretto255::scalar_mul(&gammas.g4s[i], &proof.alphas.a4s[i])
            );
        });
        ristretto255::scalar_add_assign(
            &mut scalar_g,
            &scalar_linear_combination(&gammas.g6s, &proof.alphas.a1s)
        );

        let scalar_h = ristretto255::scalar_mul(&gammas.g5, &proof.alphas.a5);
        vector::range(0, 4).for_each(|i| {
            ristretto255::scalar_add_assign(
                &mut scalar_h,
                &ristretto255::scalar_mul(&gammas.g4s[i], &proof.alphas.a3s[i])
            );
        });
        ristretto255::scalar_add_assign(
            &mut scalar_h,
            &scalar_linear_combination(&gammas.g6s, &proof.alphas.a3s)
        );
        vector::range(4, 8).for_each(|i| {
            ristretto255::scalar_add_assign(
                &mut scalar_h,
                &scalar_mul_3(&gammas.g1, &proof.alphas.a3s[i], &new_scalar_from_pow2(i * 16))
            );
        });

        let scalar_sender_ek = scalar_linear_combination(&gammas.g2s, &proof.alphas.a3s);
        ristretto255::scalar_add_assign(&mut scalar_sender_ek, &ristretto255::scalar_mul(&gammas.g5, &rho));

        let scalar_recipient_ek = ristretto255::scalar_zero();
        vector::range(0, 4).for_each(|i| {
            ristretto255::scalar_add_assign(
                &mut scalar_recipient_ek,
                &ristretto255::scalar_mul(&gammas.g3s[i], &proof.alphas.a3s[i])
            );
        });

        let scalar_ek_auditors = gammas.g7s.map_ref(|gamma: &vector<Scalar>| {
            let scalar_auditor_ek = ristretto255::scalar_zero();
            vector::range(0, 4).for_each(|i| {
                ristretto255::scalar_add_assign(
                    &mut scalar_auditor_ek,
                    &ristretto255::scalar_mul(&gamma[i], &proof.alphas.a3s[i])
                );
            });
            scalar_auditor_ek
        });

        let scalars_new_balance_d = vector::range(0, 8).map(|i| {
            let scalar = ristretto255::scalar_mul(&gammas.g2s[i], &rho);
            ristretto255::scalar_sub_assign(
                &mut scalar,
                &scalar_mul_3(&gammas.g1, &proof.alphas.a2, &new_scalar_from_pow2(i * 16))
            );
            scalar
        });

        let scalars_transfer_amount_d = vector::range(0, 4).map(|i| {
            ristretto255::scalar_mul(&gammas.g3s[i], &rho)
        });

        let scalars_current_balance_d = vector::range(0, 8).map(|i| {
            scalar_mul_3(&gammas.g1, &proof.alphas.a2, &new_scalar_from_pow2(i * 16))
        });

        let scalars_auditor_amount_d = gammas.g7s.map_ref(|gamma| {
            gamma.map_ref(|gamma| ristretto255::scalar_mul(gamma, &rho))
        });

        let scalars_current_balance_c = vector::range(0, 8).map(|i| {
            scalar_mul_3(&gammas.g1, &rho, &new_scalar_from_pow2(i * 16))
        });

        let scalars_transfer_amount_c = vector::range(0, 4).map(|i| {
            let scalar = ristretto255::scalar_mul(&gammas.g4s[i], &rho);
            ristretto255::scalar_sub_assign(
                &mut scalar,
                &scalar_mul_3(&gammas.g1, &rho, &new_scalar_from_pow2(i * 16))
            );
            scalar
        });

        let scalars_new_balance_c = vector::range(0, 8).map(|i| {
            ristretto255::scalar_mul(&gammas.g6s[i], &rho)
        });

        let scalars_rhs = vector[scalar_g, scalar_h, scalar_sender_ek, scalar_recipient_ek];
        scalars_rhs.append(scalar_ek_auditors);
        scalars_rhs.append(scalars_new_balance_d);
        scalars_rhs.append(scalars_transfer_amount_d);
        scalars_rhs.append(scalars_current_balance_d);
        scalars_auditor_amount_d.for_each(|scalars| scalars_rhs.append(scalars));
        scalars_rhs.append(scalars_current_balance_c);
        scalars_rhs.append(scalars_transfer_amount_c);
        scalars_rhs.append(scalars_new_balance_c);

        let points_rhs = vector[
            ristretto255::basepoint(),
            ristretto255::hash_to_point_base(),
            twisted_elgamal::pubkey_to_point(sender_ek),
            twisted_elgamal::pubkey_to_point(recipient_ek)
        ];
        points_rhs.append(auditor_eks.map_ref(|ek| twisted_elgamal::pubkey_to_point(ek)));
        points_rhs.append(confidential_balance::balance_to_points_d(new_balance));
        points_rhs.append(confidential_balance::balance_to_points_d(transfer_amount));
        points_rhs.append(confidential_balance::balance_to_points_d(current_balance));
        auditor_amounts.for_each_ref(|balance| {
            points_rhs.append(confidential_balance::balance_to_points_d(balance));
        });
        points_rhs.append(confidential_balance::balance_to_points_c(current_balance));
        points_rhs.append(confidential_balance::balance_to_points_c(transfer_amount));
        points_rhs.append(confidential_balance::balance_to_points_c(new_balance));

        let lhs = ristretto255::multi_scalar_mul(&points_lhs, &scalars_lhs);
        let rhs = ristretto255::multi_scalar_mul(&points_rhs, &scalars_rhs);

        assert!(
            ristretto255::point_equals(&lhs, &rhs),
            error::invalid_argument(ESIGMA_PROTOCOL_VERIFY_FAILED)
        );
    }

    /// Verifies the validity of the `NormalizationSigmaProof`.
    fun verify_normalization_sigma_proof(
        ek: &twisted_elgamal::CompressedPubkey,
        current_balance: &confidential_balance::ConfidentialBalance,
        new_balance: &confidential_balance::ConfidentialBalance,
        proof: &NormalizationSigmaProof)
    {
        let rho = fiat_shamir_normalization_sigma_proof_challenge(ek, current_balance, new_balance, &proof.xs);
        let gammas = msm_normalization_gammas(&rho);

        let scalars_lhs = vector[gammas.g1, gammas.g2];
        scalars_lhs.append(gammas.g3s);
        scalars_lhs.append(gammas.g4s);

        let points_lhs = vector[
            ristretto255::point_decompress(&proof.xs.x1),
            ristretto255::point_decompress(&proof.xs.x2)
        ];
        points_lhs.append(proof.xs.x3s.map_ref(|x| ristretto255::point_decompress(x)));
        points_lhs.append(proof.xs.x4s.map_ref(|x| ristretto255::point_decompress(x)));

        let scalar_g = scalar_linear_combination(
            &proof.alphas.a1s,
            &vector::range(0, 8).map(|i| new_scalar_from_pow2(i * 16))
        );
        ristretto255::scalar_mul_assign(&mut scalar_g, &gammas.g1);
        ristretto255::scalar_add_assign(
            &mut scalar_g,
            &scalar_linear_combination(&gammas.g3s, &proof.alphas.a1s)
        );

        let scalar_h = ristretto255::scalar_mul(&gammas.g2, &proof.alphas.a3);
        ristretto255::scalar_add_assign(
            &mut scalar_h,
            &scalar_linear_combination(&gammas.g3s, &proof.alphas.a4s)
        );

        let scalar_ek = ristretto255::scalar_mul(&gammas.g2, &rho);
        ristretto255::scalar_add_assign(
            &mut scalar_ek,
            &scalar_linear_combination(&gammas.g4s, &proof.alphas.a4s)
        );

        let scalars_current_balance_d = vector::range(0, 8).map(|i| {
            scalar_mul_3(&gammas.g1, &proof.alphas.a2, &new_scalar_from_pow2(i * 16))
        });

        let scalars_new_balance_d = vector::range(0, 8).map(|i| {
            ristretto255::scalar_mul(&gammas.g4s[i], &rho)
        });

        let scalars_current_balance_c = vector::range(0, 8).map(|i| {
            scalar_mul_3(&gammas.g1, &rho, &new_scalar_from_pow2(i * 16))
        });

        let scalars_new_balance_c = vector::range(0, 8).map(|i| {
            ristretto255::scalar_mul(&gammas.g3s[i], &rho)
        });

        let scalars_rhs = vector[scalar_g, scalar_h, scalar_ek];
        scalars_rhs.append(scalars_current_balance_d);
        scalars_rhs.append(scalars_new_balance_d);
        scalars_rhs.append(scalars_current_balance_c);
        scalars_rhs.append(scalars_new_balance_c);

        let points_rhs = vector[
            ristretto255::basepoint(),
            ristretto255::hash_to_point_base(),
            twisted_elgamal::pubkey_to_point(ek)
        ];
        points_rhs.append(confidential_balance::balance_to_points_d(current_balance));
        points_rhs.append(confidential_balance::balance_to_points_d(new_balance));
        points_rhs.append(confidential_balance::balance_to_points_c(current_balance));
        points_rhs.append(confidential_balance::balance_to_points_c(new_balance));

        let lhs = ristretto255::multi_scalar_mul(&points_lhs, &scalars_lhs);
        let rhs = ristretto255::multi_scalar_mul(&points_rhs, &scalars_rhs);

        assert!(
            ristretto255::point_equals(&lhs, &rhs),
            error::invalid_argument(ESIGMA_PROTOCOL_VERIFY_FAILED)
        );
    }

    /// Verifies the validity of the `RotationSigmaProof`.
    fun verify_rotation_sigma_proof(
        current_ek: &twisted_elgamal::CompressedPubkey,
        new_ek: &twisted_elgamal::CompressedPubkey,
        current_balance: &confidential_balance::ConfidentialBalance,
        new_balance: &confidential_balance::ConfidentialBalance,
        proof: &RotationSigmaProof)
    {
        let rho = fiat_shamir_rotation_sigma_proof_challenge(
            current_ek,
            new_ek,
            current_balance,
            new_balance,
            &proof.xs
        );
        let gammas = msm_rotation_gammas(&rho);

        let scalars_lhs = vector[gammas.g1, gammas.g2, gammas.g3];
        scalars_lhs.append(gammas.g4s);
        scalars_lhs.append(gammas.g5s);

        let points_lhs = vector[
            ristretto255::point_decompress(&proof.xs.x1),
            ristretto255::point_decompress(&proof.xs.x2),
            ristretto255::point_decompress(&proof.xs.x3)
        ];
        points_lhs.append(proof.xs.x4s.map_ref(|x| ristretto255::point_decompress(x)));
        points_lhs.append(proof.xs.x5s.map_ref(|x| ristretto255::point_decompress(x)));

        let scalar_g = scalar_linear_combination(
            &proof.alphas.a1s,
            &vector::range(0, 8).map(|i| new_scalar_from_pow2(i * 16))
        );
        ristretto255::scalar_mul_assign(&mut scalar_g, &gammas.g1);
        ristretto255::scalar_add_assign(
            &mut scalar_g,
            &scalar_linear_combination(&gammas.g4s, &proof.alphas.a1s)
        );

        let scalar_h = ristretto255::scalar_mul(&gammas.g2, &proof.alphas.a3);
        ristretto255::scalar_add_assign(&mut scalar_h, &ristretto255::scalar_mul(&gammas.g3, &proof.alphas.a4));
        ristretto255::scalar_add_assign(
            &mut scalar_h,
            &scalar_linear_combination(&gammas.g4s, &proof.alphas.a5s)
        );

        let scalar_ek_cur = ristretto255::scalar_mul(&gammas.g2, &rho);

        let scalar_ek_new = ristretto255::scalar_mul(&gammas.g3, &rho);
        ristretto255::scalar_add_assign(
            &mut scalar_ek_new,
            &scalar_linear_combination(&gammas.g5s, &proof.alphas.a5s)
        );

        let scalars_current_balance_d = vector::range(0, 8).map(|i| {
            scalar_mul_3(&gammas.g1, &proof.alphas.a2, &new_scalar_from_pow2(i * 16))
        });

        let scalars_new_balance_d = vector::range(0, 8).map(|i| {
            ristretto255::scalar_mul(&gammas.g5s[i], &rho)
        });

        let scalars_current_balance_c = vector::range(0, 8).map(|i| {
            scalar_mul_3(&gammas.g1, &rho, &new_scalar_from_pow2(i * 16))
        });

        let scalars_new_balance_c = vector::range(0, 8).map(|i| {
            ristretto255::scalar_mul(&gammas.g4s[i], &rho)
        });

        let scalars_rhs = vector[scalar_g, scalar_h, scalar_ek_cur, scalar_ek_new];
        scalars_rhs.append(scalars_current_balance_d);
        scalars_rhs.append(scalars_new_balance_d);
        scalars_rhs.append(scalars_current_balance_c);
        scalars_rhs.append(scalars_new_balance_c);

        let points_rhs = vector[
            ristretto255::basepoint(),
            ristretto255::hash_to_point_base(),
            twisted_elgamal::pubkey_to_point(current_ek),
            twisted_elgamal::pubkey_to_point(new_ek)
        ];
        points_rhs.append(confidential_balance::balance_to_points_d(current_balance));
        points_rhs.append(confidential_balance::balance_to_points_d(new_balance));
        points_rhs.append(confidential_balance::balance_to_points_c(current_balance));
        points_rhs.append(confidential_balance::balance_to_points_c(new_balance));

        let lhs = ristretto255::multi_scalar_mul(&points_lhs, &scalars_lhs);
        let rhs = ristretto255::multi_scalar_mul(&points_rhs, &scalars_rhs);

        assert!(
            ristretto255::point_equals(&lhs, &rhs),
            error::invalid_argument(ESIGMA_PROTOCOL_VERIFY_FAILED)
        );
    }

    /// Verifies the validity of the `NewBalanceRangeProof`.
    fun verify_new_balance_range_proof(
        new_balance: &confidential_balance::ConfidentialBalance,
        zkrp_new_balance: &RangeProof)
    {
        let balance_c = confidential_balance::balance_to_points_c(new_balance);

        assert!(
            bulletproofs::verify_batch_range_proof(
                &balance_c,
                &ristretto255::basepoint(),
                &ristretto255::hash_to_point_base(),
                zkrp_new_balance,
                BULLETPROOFS_NUM_BITS,
                BULLETPROOFS_DST
            ),
            error::out_of_range(ERANGE_PROOF_VERIFICATION_FAILED)
        );
    }

    /// Verifies the validity of the `TransferBalanceRangeProof`.
    fun verify_transfer_amount_range_proof(
        transfer_amount: &confidential_balance::ConfidentialBalance,
        zkrp_transfer_amount: &RangeProof)
    {
        let balance_c = confidential_balance::balance_to_points_c(transfer_amount);

        assert!(
            bulletproofs::verify_batch_range_proof(
                &balance_c,
                &ristretto255::basepoint(),
                &ristretto255::hash_to_point_base(),
                zkrp_transfer_amount,
                BULLETPROOFS_NUM_BITS,
                BULLETPROOFS_DST
            ),
            error::out_of_range(ERANGE_PROOF_VERIFICATION_FAILED)
        );
    }

    //
    // Friend public functions
    //

    /// Returns the number of range proofs in the provided `WithdrawalProof`.
    /// Used in the `confidential_asset` module to validate input parameters of the `confidential_transfer` function.
    public(friend) fun auditors_count_in_transfer_proof(proof: &TransferProof): u64 {
        proof.sigma_proof.xs.x7s.length()
    }

    //
    // Deserialization functions
    //

    /// Deserializes the `WithdrawalProof` from the byte array.
    /// Returns `Some(WithdrawalProof)` if the deserialization is successful; otherwise, returns `None`.
    public fun deserialize_withdrawal_proof(
        sigma_proof_bytes: vector<u8>,
        zkrp_new_balance_bytes: vector<u8>): Option<WithdrawalProof>
    {
        let sigma_proof = deserialize_withdrawal_sigma_proof(sigma_proof_bytes);
        let zkrp_new_balance = bulletproofs::range_proof_from_bytes(zkrp_new_balance_bytes);

        if (sigma_proof.is_none()) {
            return option::none()
        };

        option::some(
            WithdrawalProof {
                sigma_proof: sigma_proof.extract(),
                zkrp_new_balance,
            }
        )
    }

    /// Deserializes the `TransferProof` from the byte array.
    /// Returns `Some(TransferProof)` if the deserialization is successful; otherwise, returns `None`.
    public fun deserialize_transfer_proof(
        sigma_proof_bytes: vector<u8>,
        zkrp_new_balance_bytes: vector<u8>,
        zkrp_transfer_amount_bytes: vector<u8>): Option<TransferProof>
    {
        let sigma_proof = deserialize_transfer_sigma_proof(sigma_proof_bytes);
        let zkrp_new_balance = bulletproofs::range_proof_from_bytes(zkrp_new_balance_bytes);
        let zkrp_transfer_amount = bulletproofs::range_proof_from_bytes(zkrp_transfer_amount_bytes);

        if (sigma_proof.is_none()) {
            return option::none()
        };

        option::some(
            TransferProof {
                sigma_proof: sigma_proof.extract(),
                zkrp_new_balance,
                zkrp_transfer_amount,
            }
        )
    }

    /// Deserializes the `NormalizationProof` from the byte array.
    /// Returns `Some(NormalizationProof)` if the deserialization is successful; otherwise, returns `None`.
    public fun deserialize_normalization_proof(
        sigma_proof_bytes: vector<u8>,
        zkrp_new_balance_bytes: vector<u8>): Option<NormalizationProof>
    {
        let sigma_proof = deserialize_normalization_sigma_proof(sigma_proof_bytes);
        let zkrp_new_balance = bulletproofs::range_proof_from_bytes(zkrp_new_balance_bytes);

        if (sigma_proof.is_none()) {
            return option::none()
        };

        option::some(
            NormalizationProof {
                sigma_proof: sigma_proof.extract(),
                zkrp_new_balance,
            }
        )
    }

    /// Deserializes the `RotationProof` from the byte array.
    /// Returns `Some(RotationProof)` if the deserialization is successful; otherwise, returns `None`.
    public fun deserialize_rotation_proof(
        sigma_proof_bytes: vector<u8>,
        zkrp_new_balance_bytes: vector<u8>): Option<RotationProof>
    {
        let sigma_proof = deserialize_rotation_sigma_proof(sigma_proof_bytes);
        let zkrp_new_balance = bulletproofs::range_proof_from_bytes(zkrp_new_balance_bytes);

        if (sigma_proof.is_none()) {
            return option::none()
        };

        option::some(
            RotationProof {
                sigma_proof: sigma_proof.extract(),
                zkrp_new_balance,
            }
        )
    }

    //
    // Deserialization functions implementations
    //

    /// Deserializes the `WithdrawalSigmaProof` from the byte array.
    /// Returns `Some(WithdrawalSigmaProof)` if the deserialization is successful; otherwise, returns `None`.
    fun deserialize_withdrawal_sigma_proof(proof_bytes: vector<u8>): Option<WithdrawalSigmaProof> {
        let alphas_count = 18;
        let xs_count = 18;

        if (proof_bytes.length() != 32 * xs_count + 32 * alphas_count) {
            return option::none()
        };

        let alphas = vector::range(0, alphas_count).map(|i| {
            ristretto255::new_scalar_from_bytes(proof_bytes.slice(i * 32, (i + 1) * 32))
        });
        let xs = vector::range(alphas_count, alphas_count + xs_count).map(|i| {
            ristretto255::new_compressed_point_from_bytes(proof_bytes.slice(i * 32, (i + 1) * 32))
        });

        if (alphas.any(|alpha| alpha.is_none()) || xs.any(|x| x.is_none())) {
            return option::none()
        };

        option::some(
            WithdrawalSigmaProof {
                alphas: WithdrawalSigmaProofAlphas {
                    a1s: alphas.slice(0, 8).map(|alpha| alpha.extract()),
                    a2: alphas[8].extract(),
                    a3: alphas[9].extract(),
                    a4s: alphas.slice(10, 18).map(|alpha| alpha.extract()),
                },
                xs: WithdrawalSigmaProofXs {
                    x1: xs[0].extract(),
                    x2: xs[1].extract(),
                    x3s: xs.slice(2, 10).map(|x| x.extract()),
                    x4s: xs.slice(10, 18).map(|x| x.extract()),
                },
            }
        )
    }

    /// Deserializes the `TransferSigmaProof` from the byte array.
    /// Returns `Some(TransferSigmaProof)` if the deserialization is successful; otherwise, returns `None`.
    fun deserialize_transfer_sigma_proof(proof_bytes: vector<u8>): Option<TransferSigmaProof> {
        let alphas_count = 22;
        let xs_count = 26;

        if (proof_bytes.length() < 32 * xs_count + 32 * alphas_count) {
            return option::none()
        };

        // Transfer proof may contain additional four Xs for each auditor.
        let auditor_xs = proof_bytes.length() - (32 * xs_count + 32 * alphas_count);

        if (auditor_xs % 128 != 0) {
            return option::none()
        };

        xs_count += auditor_xs / 32;

        let alphas = vector::range(0, alphas_count).map(|i| {
            ristretto255::new_scalar_from_bytes(proof_bytes.slice(i * 32, (i + 1) * 32))
        });
        let xs = vector::range(alphas_count, alphas_count + xs_count).map(|i| {
            ristretto255::new_compressed_point_from_bytes(proof_bytes.slice(i * 32, (i + 1) * 32))
        });

        if (alphas.any(|alpha| alpha.is_none()) || xs.any(|x| x.is_none())) {
            return option::none()
        };

        option::some(
            TransferSigmaProof {
                alphas: TransferSigmaProofAlphas {
                    a1s: alphas.slice(0, 8).map(|alpha| alpha.extract()),
                    a2: alphas[8].extract(),
                    a3s: alphas.slice(9, 17).map(|alpha| alpha.extract()),
                    a4s: alphas.slice(17, 21).map(|alpha| alpha.extract()),
                    a5: alphas[21].extract(),
                },
                xs: TransferSigmaProofXs {
                    x1: xs[0].extract(),
                    x2s: xs.slice(1, 9).map(|x| x.extract()),
                    x3s: xs.slice(9, 13).map(|x| x.extract()),
                    x4s: xs.slice(13, 17).map(|x| x.extract()),
                    x5: xs[17].extract(),
                    x6s: xs.slice(18, 26).map(|x| x.extract()),
                    x7s: vector::range_with_step(26, xs_count, 4).map(|i| {
                        vector::range(i, i + 4).map(|j| xs[j].extract())
                    }),
                },
            }
        )
    }

    /// Deserializes the `NormalizationSigmaProof` from the byte array.
    /// Returns `Some(NormalizationSigmaProof)` if the deserialization is successful; otherwise, returns `None`.
    fun deserialize_normalization_sigma_proof(proof_bytes: vector<u8>): Option<NormalizationSigmaProof> {
        let alphas_count = 18;
        let xs_count = 18;

        if (proof_bytes.length() != 32 * xs_count + 32 * alphas_count) {
            return option::none()
        };

        let alphas = vector::range(0, alphas_count).map(|i| {
            ristretto255::new_scalar_from_bytes(proof_bytes.slice(i * 32, (i + 1) * 32))
        });
        let xs = vector::range(alphas_count, alphas_count + xs_count).map(|i| {
            ristretto255::new_compressed_point_from_bytes(proof_bytes.slice(i * 32, (i + 1) * 32))
        });

        if (alphas.any(|alpha| alpha.is_none()) || xs.any(|x| x.is_none())) {
            return option::none()
        };

        option::some(
            NormalizationSigmaProof {
                alphas: NormalizationSigmaProofAlphas {
                    a1s: alphas.slice(0, 8).map(|alpha| alpha.extract()),
                    a2: alphas[8].extract(),
                    a3: alphas[9].extract(),
                    a4s: alphas.slice(10, 18).map(|alpha| alpha.extract()),
                },
                xs: NormalizationSigmaProofXs {
                    x1: xs[0].extract(),
                    x2: xs[1].extract(),
                    x3s: xs.slice(2, 10).map(|x| x.extract()),
                    x4s: xs.slice(10, 18).map(|x| x.extract()),
                },
            }
        )
    }

    /// Deserializes the `RotationSigmaProof` from the byte array.
    /// Returns `Some(RotationSigmaProof)` if the deserialization is successful; otherwise, returns `None`.
    fun deserialize_rotation_sigma_proof(proof_bytes: vector<u8>): Option<RotationSigmaProof> {
        let alphas_count = 19;
        let xs_count = 19;

        if (proof_bytes.length() != 32 * xs_count + 32 * alphas_count) {
            return option::none()
        };

        let alphas = vector::range(0, alphas_count).map(|i| {
            ristretto255::new_scalar_from_bytes(proof_bytes.slice(i * 32, (i + 1) * 32))
        });
        let xs = vector::range(alphas_count, alphas_count + xs_count).map(|i| {
            ristretto255::new_compressed_point_from_bytes(proof_bytes.slice(i * 32, (i + 1) * 32))
        });

        if (alphas.any(|alpha| alpha.is_none()) || xs.any(|x| x.is_none())) {
            return option::none()
        };

        option::some(
            RotationSigmaProof {
                alphas: RotationSigmaProofAlphas {
                    a1s: alphas.slice(0, 8).map(|alpha| alpha.extract()),
                    a2: alphas[8].extract(),
                    a3: alphas[9].extract(),
                    a4: alphas[10].extract(),
                    a5s: alphas.slice(11, 19).map(|alpha| alpha.extract()),
                },
                xs: RotationSigmaProofXs {
                    x1: xs[0].extract(),
                    x2: xs[1].extract(),
                    x3: xs[2].extract(),
                    x4s: xs.slice(3, 11).map(|x| x.extract()),
                    x5s: xs.slice(11, 19).map(|x| x.extract()),
                },
            }
        )
    }

    //
    // Public view functions
    //

    #[view]
    /// Returns the Fiat Shamir DST for the `WithdrawalSigmaProof`.
    public fun get_fiat_shamir_withdrawal_sigma_dst(): vector<u8> {
        FIAT_SHAMIR_WITHDRAWAL_SIGMA_DST
    }

    #[view]
    /// Returns the Fiat Shamir DST for the `TransferSigmaProof`.
    public fun get_fiat_shamir_transfer_sigma_dst(): vector<u8> {
        FIAT_SHAMIR_TRANSFER_SIGMA_DST
    }

    #[view]
    /// Returns the Fiat Shamir DST for the `NormalizationSigmaProof`.
    public fun get_fiat_shamir_normalization_sigma_dst(): vector<u8> {
        FIAT_SHAMIR_NORMALIZATION_SIGMA_DST
    }

    #[view]
    /// Returns the Fiat Shamir DST for the `RotationSigmaProof`.
    public fun get_fiat_shamir_rotation_sigma_dst(): vector<u8> {
        FIAT_SHAMIR_ROTATION_SIGMA_DST
    }

    #[view]
    /// Returns the DST for the range proofs.
    public fun get_bulletproofs_dst(): vector<u8> {
        BULLETPROOFS_DST
    }

    #[view]
    /// Returns the maximum number of bits of the normalized chunk for the range proofs.
    public fun get_bulletproofs_num_bits(): u64 {
        BULLETPROOFS_NUM_BITS
    }

    //
    // Private functions for Fiat-Shamir challenge derivation.
    // The Fiat Shamir is used to make the proofs non-interactive.
    // The challenge has the same for the proof generation and verification and is derived from the public parameters.
    //

    /// Derives the Fiat-Shamir challenge for the `WithdrawalSigmaProof`.
    fun fiat_shamir_withdrawal_sigma_proof_challenge(
        ek: &twisted_elgamal::CompressedPubkey,
        amount_chunks: &vector<Scalar>,
        current_balance: &confidential_balance::ConfidentialBalance,
        proof_xs: &WithdrawalSigmaProofXs): Scalar
    {
        // rho = H(DST, v_{1..4}, P, (C_cur, D_cur)_{1..8}, G, H, X_{1..18})
        let bytes = FIAT_SHAMIR_WITHDRAWAL_SIGMA_DST;

        amount_chunks.for_each_ref(|chunk| {
            bytes.append(ristretto255::scalar_to_bytes(chunk));
        });
        bytes.append(twisted_elgamal::pubkey_to_bytes(ek));
        bytes.append(confidential_balance::balance_to_bytes(current_balance));
        bytes.append(ristretto255::compressed_point_to_bytes(ristretto255::basepoint_compressed()));
        bytes.append(
            ristretto255::compressed_point_to_bytes(ristretto255::point_compress(&ristretto255::hash_to_point_base()))
        );
        bytes.append(ristretto255::point_to_bytes(&proof_xs.x1));
        bytes.append(ristretto255::point_to_bytes(&proof_xs.x2));
        proof_xs.x3s.for_each_ref(|x| {
            bytes.append(ristretto255::point_to_bytes(x));
        });
        proof_xs.x4s.for_each_ref(|x| {
            bytes.append(ristretto255::point_to_bytes(x));
        });

        ristretto255::new_scalar_from_sha2_512(bytes)
    }

    /// Derives the Fiat-Shamir challenge for the `TransferSigmaProof`.
    fun fiat_shamir_transfer_sigma_proof_challenge(
        sender_ek: &twisted_elgamal::CompressedPubkey,
        recipient_ek: &twisted_elgamal::CompressedPubkey,
        current_balance: &confidential_balance::ConfidentialBalance,
        new_balance: &confidential_balance::ConfidentialBalance,
        transfer_amount: &confidential_balance::ConfidentialBalance,
        auditor_eks: &vector<twisted_elgamal::CompressedPubkey>,
        auditor_amounts: &vector<confidential_balance::ConfidentialBalance>,
        proof_xs: &TransferSigmaProofXs): Scalar
    {
        // rho = H(DST, G, H, P_s, P_r, P_a_{1..n}, (C_cur, D_cur)_{1..8}, (C_new, D_new)_{1..8}, (C_v, D_v)_{1..8}, D_a_{1..n}, X_{1..26 + 4n})
        let bytes = FIAT_SHAMIR_TRANSFER_SIGMA_DST;

        bytes.append(ristretto255::compressed_point_to_bytes(ristretto255::basepoint_compressed()));
        bytes.append(
            ristretto255::compressed_point_to_bytes(ristretto255::point_compress(&ristretto255::hash_to_point_base()))
        );
        bytes.append(twisted_elgamal::pubkey_to_bytes(sender_ek));
        bytes.append(twisted_elgamal::pubkey_to_bytes(recipient_ek));
        auditor_eks.for_each_ref(|ek| {
            bytes.append(twisted_elgamal::pubkey_to_bytes(ek));
        });
        bytes.append(confidential_balance::balance_to_bytes(current_balance));
        bytes.append(confidential_balance::balance_to_bytes(new_balance));
        bytes.append(confidential_balance::balance_to_bytes(transfer_amount));
        auditor_amounts.for_each_ref(|balance| {
            confidential_balance::balance_to_points_d(balance).for_each_ref(|d| {
                bytes.append(ristretto255::compressed_point_to_bytes(ristretto255::point_compress(d)));
            });
        });
        bytes.append(ristretto255::point_to_bytes(&proof_xs.x1));
        proof_xs.x2s.for_each_ref(|x| {
            bytes.append(ristretto255::point_to_bytes(x));
        });
        proof_xs.x3s.for_each_ref(|x| {
            bytes.append(ristretto255::point_to_bytes(x));
        });
        proof_xs.x4s.for_each_ref(|x| {
            bytes.append(ristretto255::point_to_bytes(x));
        });
        bytes.append(ristretto255::point_to_bytes(&proof_xs.x5));
        proof_xs.x6s.for_each_ref(|x| {
            bytes.append(ristretto255::point_to_bytes(x));
        });
        proof_xs.x7s.for_each_ref(|xs| {
            xs.for_each_ref(|x| {
                bytes.append(ristretto255::point_to_bytes(x));
            });
        });

        ristretto255::new_scalar_from_sha2_512(bytes)
    }

    /// Derives the Fiat-Shamir challenge for the `NormalizationSigmaProof`.
    fun fiat_shamir_normalization_sigma_proof_challenge(
        ek: &twisted_elgamal::CompressedPubkey,
        current_balance: &confidential_balance::ConfidentialBalance,
        new_balance: &confidential_balance::ConfidentialBalance,
        proof_xs: &NormalizationSigmaProofXs): Scalar
    {
        // rho = H(DST, G, H, P, (C_cur, D_cur)_{1..8}, (C_new, D_new)_{1..8}, X_{1..18})
        let bytes = FIAT_SHAMIR_NORMALIZATION_SIGMA_DST;

        bytes.append(ristretto255::compressed_point_to_bytes(ristretto255::basepoint_compressed()));
        bytes.append(
            ristretto255::compressed_point_to_bytes(ristretto255::point_compress(&ristretto255::hash_to_point_base()))
        );
        bytes.append(twisted_elgamal::pubkey_to_bytes(ek));
        bytes.append(confidential_balance::balance_to_bytes(current_balance));
        bytes.append(confidential_balance::balance_to_bytes(new_balance));
        bytes.append(ristretto255::point_to_bytes(&proof_xs.x1));
        bytes.append(ristretto255::point_to_bytes(&proof_xs.x2));
        proof_xs.x3s.for_each_ref(|x| {
            bytes.append(ristretto255::point_to_bytes(x));
        });
        proof_xs.x4s.for_each_ref(|x| {
            bytes.append(ristretto255::point_to_bytes(x));
        });

        ristretto255::new_scalar_from_sha2_512(bytes)
    }

    /// Derives the Fiat-Shamir challenge for the `RotationSigmaProof`.
    fun fiat_shamir_rotation_sigma_proof_challenge(
        current_ek: &twisted_elgamal::CompressedPubkey,
        new_ek: &twisted_elgamal::CompressedPubkey,
        current_balance: &confidential_balance::ConfidentialBalance,
        new_balance: &confidential_balance::ConfidentialBalance,
        proof_xs: &RotationSigmaProofXs): Scalar
    {
        // rho = H(DST, G, H, P_cur, P_new, (C_cur, D_cur)_{1..8}, (C_new, D_new)_{1..8}, X_{1..19})
        let bytes = FIAT_SHAMIR_ROTATION_SIGMA_DST;

        bytes.append(ristretto255::compressed_point_to_bytes(ristretto255::basepoint_compressed()));
        bytes.append(
            ristretto255::compressed_point_to_bytes(ristretto255::point_compress(&ristretto255::hash_to_point_base()))
        );
        bytes.append(twisted_elgamal::pubkey_to_bytes(current_ek));
        bytes.append(twisted_elgamal::pubkey_to_bytes(new_ek));
        bytes.append(confidential_balance::balance_to_bytes(current_balance));
        bytes.append(confidential_balance::balance_to_bytes(new_balance));
        bytes.append(ristretto255::point_to_bytes(&proof_xs.x1));
        bytes.append(ristretto255::point_to_bytes(&proof_xs.x2));
        bytes.append(ristretto255::point_to_bytes(&proof_xs.x3));
        proof_xs.x4s.for_each_ref(|x| {
            bytes.append(ristretto255::point_to_bytes(x));
        });
        proof_xs.x5s.for_each_ref(|x| {
            bytes.append(ristretto255::point_to_bytes(x));
        });

        ristretto255::new_scalar_from_sha2_512(bytes)
    }

    //
    // Private functions for constructing the scalar multipliers (`gammas`) used for uniting multiple proof relations
    // into a single multi-scalar multiplication (MSM) equation
    //

    /// Returns the scalar multipliers for the `WithdrawalSigmaProof`.
    fun msm_withdrawal_gammas(rho: &Scalar): WithdrawalSigmaProofGammas {
        WithdrawalSigmaProofGammas {
            g1: ristretto255::new_scalar_from_sha2_512(msm_gamma_1(rho, 1)),
            g2: ristretto255::new_scalar_from_sha2_512(msm_gamma_1(rho, 2)),
            g3s: vector::range(0, 8).map(|i| {
                ristretto255::new_scalar_from_sha2_512(msm_gamma_2(rho, 3, (i as u8)))
            }),
            g4s: vector::range(0, 8).map(|i| {
                ristretto255::new_scalar_from_sha2_512(msm_gamma_2(rho, 4, (i as u8)))
            }),
        }
    }

    /// Returns the scalar multipliers for the `TransferSigmaProof`.
    fun msm_transfer_gammas(rho: &Scalar, auditors_count: u64): TransferSigmaProofGammas {
        TransferSigmaProofGammas {
            g1: ristretto255::new_scalar_from_sha2_512(msm_gamma_1(rho, 1)),
            g2s: vector::range(0, 8).map(|i| {
                ristretto255::new_scalar_from_sha2_512(msm_gamma_2(rho, 2, (i as u8)))
            }),
            g3s: vector::range(0, 4).map(|i| {
                ristretto255::new_scalar_from_sha2_512(msm_gamma_2(rho, 3, (i as u8)))
            }),
            g4s: vector::range(0, 4).map(|i| {
                ristretto255::new_scalar_from_sha2_512(msm_gamma_2(rho, 4, (i as u8)))
            }),
            g5: ristretto255::new_scalar_from_sha2_512(msm_gamma_1(rho, 5)),
            g6s: vector::range(0, 8).map(|i| {
                ristretto255::new_scalar_from_sha2_512(msm_gamma_2(rho, 6, (i as u8)))
            }),
            g7s: vector::range(0, auditors_count).map(|i| {
                vector::range(0, 4).map(|j| {
                    ristretto255::new_scalar_from_sha2_512(msm_gamma_2(rho, (i + 7 as u8), (j as u8)))
                })
            }),
        }
    }

    /// Returns the scalar multipliers for the `NormalizationSigmaProof`.
    fun msm_normalization_gammas(rho: &Scalar): NormalizationSigmaProofGammas {
        NormalizationSigmaProofGammas {
            g1: ristretto255::new_scalar_from_sha2_512(msm_gamma_1(rho, 1)),
            g2: ristretto255::new_scalar_from_sha2_512(msm_gamma_1(rho, 2)),
            g3s: vector::range(0, 8).map(|i| {
                ristretto255::new_scalar_from_sha2_512(msm_gamma_2(rho, 3, (i as u8)))
            }),
            g4s: vector::range(0, 8).map(|i| {
                ristretto255::new_scalar_from_sha2_512(msm_gamma_2(rho, 4, (i as u8)))
            }),
        }
    }

    /// Returns the scalar multipliers for the `RotationSigmaProof`.
    fun msm_rotation_gammas(rho: &Scalar): RotationSigmaProofGammas {
        RotationSigmaProofGammas {
            g1: ristretto255::new_scalar_from_sha2_512(msm_gamma_1(rho, 1)),
            g2: ristretto255::new_scalar_from_sha2_512(msm_gamma_1(rho, 2)),
            g3: ristretto255::new_scalar_from_sha2_512(msm_gamma_1(rho, 3)),
            g4s: vector::range(0, 8).map(|i| {
                ristretto255::new_scalar_from_sha2_512(msm_gamma_2(rho, 4, (i as u8)))
            }),
            g5s: vector::range(0, 8).map(|i| {
                ristretto255::new_scalar_from_sha2_512(msm_gamma_2(rho, 5, (i as u8)))
            }),
        }
    }

    /// Returns the scalar multiplier computed as a hash of the provided `rho` and corresponding `gamma` index.
    fun msm_gamma_1(rho: &Scalar, i: u8): vector<u8> {
        let bytes = ristretto255::scalar_to_bytes(rho);
        bytes.push_back(i);
        bytes
    }

    /// Returns the scalar multiplier computed as a hash of the provided `rho` and corresponding `gamma` indices.
    fun msm_gamma_2(rho: &Scalar, i: u8, j: u8): vector<u8> {
        let bytes = ristretto255::scalar_to_bytes(rho);
        bytes.push_back(i);
        bytes.push_back(j);
        bytes
    }

    /// Calculates the product of the provided scalars.
    fun scalar_mul_3(scalar1: &Scalar, scalar2: &Scalar, scalar3: &Scalar): Scalar {
        let result = *scalar1;

        ristretto255::scalar_mul_assign(&mut result, scalar2);
        ristretto255::scalar_mul_assign(&mut result, scalar3);

        result
    }

    /// Calculates the linear combination of the provided scalars.
    fun scalar_linear_combination(lhs: &vector<Scalar>, rhs: &vector<Scalar>): Scalar {
        let result = ristretto255::scalar_zero();

        lhs.zip_ref(rhs, |l, r| {
            ristretto255::scalar_add_assign(&mut result, &ristretto255::scalar_mul(l, r));
        });

        result
    }

    /// Raises 2 to the power of the provided exponent and returns the result as a scalar.
    fun new_scalar_from_pow2(exp: u64): Scalar {
        ristretto255::new_scalar_from_u128(1 << (exp as u8))
    }

    //
    // Test-only structs
    //

    #[test_only] struct WithdrawalSigmaProofRandomness has drop {
        x1s: vector<Scalar>,
        x2: Scalar,
        x3: Scalar,
        x4s: vector<Scalar>,
    }

    #[test_only] struct TransferSigmaProofRandomness has drop {
        x1s: vector<Scalar>,
        x2: Scalar,
        x3s: vector<Scalar>,
        x4s: vector<Scalar>,
        x5: Scalar,
    }

    #[test_only] struct NormalizationSigmaProofRandomness has drop {
        x1s: vector<Scalar>,
        x2: Scalar,
        x3: Scalar,
        x4s: vector<Scalar>,
    }

    #[test_only] struct RotationSigmaProofRandomness has drop {
        x1s: vector<Scalar>,
        x2: Scalar,
        x3: Scalar,
        x4: Scalar,
        x5s: vector<Scalar>,
    }

    //
    // Test-only prove functions
    //

    #[test_only]
    public fun prove_withdrawal(
        dk: &Scalar,
        ek: &twisted_elgamal::CompressedPubkey,
        amount: u64,
        new_amount: u128,
        current_balance: &confidential_balance::ConfidentialBalance
    ): (WithdrawalProof, confidential_balance::ConfidentialBalance)
    {
        let new_balance_r = confidential_balance::generate_balance_randomness();
        let new_balance = confidential_balance::new_actual_balance_from_u128(new_amount, &new_balance_r, ek);

        let new_balance_r = confidential_balance::balance_randomness_as_scalars(&new_balance_r);

        let sigma_r = generate_withdrawal_sigma_proof_randomness();

        let zkrp_new_balance = prove_new_balance_range(new_amount, new_balance_r);

        let x1 = ristretto255::basepoint_mul(
            &scalar_linear_combination(&sigma_r.x1s, &vector::range(0, 8).map(|i| new_scalar_from_pow2(i * 16)))
        );
        ristretto255::point_add_assign(
            &mut x1,
            &ristretto255::point_mul(
                &ristretto255::multi_scalar_mul(
                    &confidential_balance::balance_to_points_d(current_balance),
                    &vector::range(0, 8).map(|i| new_scalar_from_pow2(i * 16))
                ),
                &sigma_r.x2
            )
        );

        let x2 = ristretto255::point_mul(&ristretto255::hash_to_point_base(), &sigma_r.x3);
        let x3s = vector::range(0, 8).map(|i| {
            let x3i = ristretto255::basepoint_mul(&sigma_r.x1s[i]);
            ristretto255::point_add_assign(
                &mut x3i,
                &ristretto255::point_mul(&ristretto255::hash_to_point_base(), &sigma_r.x4s[i])
            );
            x3i
        });
        let x4s = vector::range(0, 8).map(|i| {
            ristretto255::point_mul(&twisted_elgamal::pubkey_to_point(ek), &sigma_r.x4s[i])
        });

        let proof_xs = WithdrawalSigmaProofXs {
            x1: ristretto255::point_compress(&x1),
            x2: ristretto255::point_compress(&x2),
            x3s: x3s.map(|x| ristretto255::point_compress(&x)),
            x4s: x4s.map(|x| ristretto255::point_compress(&x)),
        };

        let amount_chunks = confidential_balance::split_into_chunks_u64(amount);

        let rho = fiat_shamir_withdrawal_sigma_proof_challenge(ek, &amount_chunks, current_balance, &proof_xs);

        let new_amount_chunks = confidential_balance::split_into_chunks_u128(new_amount);

        let a1s = vector::range(0, 8).map(|i| {
            ristretto255::scalar_sub(&sigma_r.x1s[i], &ristretto255::scalar_mul(&rho, &new_amount_chunks[i]))
        });
        let a2 = ristretto255::scalar_sub(&sigma_r.x2, &ristretto255::scalar_mul(&rho, dk));
        let a3 = ristretto255::scalar_sub(
            &sigma_r.x3,
            &ristretto255::scalar_mul(&rho, &ristretto255::scalar_invert(dk).extract())
        );
        let a4s = vector::range(0, 8).map(|i| {
            ristretto255::scalar_sub(&sigma_r.x4s[i], &ristretto255::scalar_mul(&rho, &new_balance_r[i]))
        });

        (
            WithdrawalProof {
                sigma_proof: WithdrawalSigmaProof {
                    xs: proof_xs,
                    alphas: WithdrawalSigmaProofAlphas { a1s, a2, a3, a4s }
                },
                zkrp_new_balance
            },
            new_balance
        )
    }

    #[test_only]
    public fun prove_transfer(
        sender_dk: &Scalar,
        sender_ek: &twisted_elgamal::CompressedPubkey,
        recipient_ek: &twisted_elgamal::CompressedPubkey,
        amount: u64,
        new_amount: u128,
        current_balance: &confidential_balance::ConfidentialBalance,
        auditor_eks: &vector<twisted_elgamal::CompressedPubkey>
    ): (TransferProof, confidential_balance::ConfidentialBalance, confidential_balance::ConfidentialBalance, vector<confidential_balance::ConfidentialBalance>)
    {
        let balance_r = confidential_balance::generate_balance_randomness();
        let new_balance = confidential_balance::new_actual_balance_from_u128(new_amount, &balance_r, sender_ek);
        let transfer_amount = confidential_balance::new_pending_balance_from_u64(amount, &balance_r, recipient_ek);
        let auditor_amounts = auditor_eks.map_ref(|ek| {
            confidential_balance::new_pending_balance_from_u64(amount, &balance_r, ek)
        });

        let balance_r = confidential_balance::balance_randomness_as_scalars(&balance_r);

        let sigma_r = generate_transfer_sigma_proof_randomness();

        let zkrp_new_balance = prove_new_balance_range(new_amount, balance_r);
        let zkrp_transfer_amount = prove_transfer_amount_range(amount, &balance_r.slice(0, 4));

        let x1 = ristretto255::basepoint_mul(
            &scalar_linear_combination(&sigma_r.x1s, &vector::range(0, 8).map(|i| new_scalar_from_pow2(i * 16)))
        );

        let current_balance_d = confidential_balance::balance_to_points_d(current_balance);
        let new_balance_d = confidential_balance::balance_to_points_d(&new_balance);

        vector::range(0, 8).for_each(|i| {
            ristretto255::point_add_assign(
                &mut x1,
                &ristretto255::point_mul(
                    &current_balance_d[i],
                    &ristretto255::scalar_mul(&sigma_r.x2, &new_scalar_from_pow2(i * 16))
                )
            );
            ristretto255::point_sub_assign(
                &mut x1,
                &ristretto255::point_mul(
                    &new_balance_d[i],
                    &ristretto255::scalar_mul(&sigma_r.x2, &new_scalar_from_pow2(i * 16))
                )
            );

            if (i > 3) {
                ristretto255::point_add_assign(
                    &mut x1,
                    &ristretto255::point_mul(
                        &ristretto255::hash_to_point_base(),
                        &ristretto255::scalar_mul(&sigma_r.x3s[i], &new_scalar_from_pow2(i * 16))
                    )
                );
            }
        });

        let x2s = vector::range(0, 8).map(|i| {
            ristretto255::point_mul(&twisted_elgamal::pubkey_to_point(sender_ek), &sigma_r.x3s[i])
        });
        let x3s = vector::range(0, 4).map(|i| {
            ristretto255::point_mul(&twisted_elgamal::pubkey_to_point(recipient_ek), &sigma_r.x3s[i])
        });
        let x4s = vector::range(0, 4).map(|i| {
            let x4i = ristretto255::basepoint_mul(&sigma_r.x4s[i]);
            ristretto255::point_add_assign(
                &mut x4i,
                &ristretto255::point_mul(&ristretto255::hash_to_point_base(), &sigma_r.x3s[i])
            );
            x4i
        });
        let x5 = ristretto255::point_mul(&ristretto255::hash_to_point_base(), &sigma_r.x5);
        let x6s = vector::range(0, 8).map(|i| {
            let x6i = ristretto255::basepoint_mul(&sigma_r.x1s[i]);
            ristretto255::point_add_assign(
                &mut x6i,
                &ristretto255::point_mul(&ristretto255::hash_to_point_base(), &sigma_r.x3s[i])
            );
            x6i
        });
        let x7s = auditor_eks.map_ref(|ek| {
            vector::range(0, 4).map(|i| {
                ristretto255::point_mul(&twisted_elgamal::pubkey_to_point(ek), &sigma_r.x3s[i])
            })
        });

        let proof_xs = TransferSigmaProofXs {
            x1: ristretto255::point_compress(&x1),
            x2s: x2s.map(|x| ristretto255::point_compress(&x)),
            x3s: x3s.map(|x| ristretto255::point_compress(&x)),
            x4s: x4s.map(|x| ristretto255::point_compress(&x)),
            x5: ristretto255::point_compress(&x5),
            x6s: x6s.map(|x| ristretto255::point_compress(&x)),
            x7s: x7s.map(|xs| {
                xs.map(|x| ristretto255::point_compress(&x))
            }),
        };

        let rho = fiat_shamir_transfer_sigma_proof_challenge(
            sender_ek,
            recipient_ek,
            current_balance,
            &new_balance,
            &transfer_amount,
            auditor_eks,
            &auditor_amounts,
            &proof_xs
        );

        let amount_chunks = confidential_balance::split_into_chunks_u64(amount);
        let new_amount_chunks = confidential_balance::split_into_chunks_u128(new_amount);

        let a1s = vector::range(0, 8).map(|i| {
            ristretto255::scalar_sub(&sigma_r.x1s[i], &ristretto255::scalar_mul(&rho, &new_amount_chunks[i]))
        });
        let a2 = ristretto255::scalar_sub(&sigma_r.x2, &ristretto255::scalar_mul(&rho, sender_dk));
        let a3s = vector::range(0, 8).map(|i| {
            ristretto255::scalar_sub(&sigma_r.x3s[i], &ristretto255::scalar_mul(&rho, &balance_r[i]))
        });
        let a4s = vector::range(0, 4).map(|i| {
            ristretto255::scalar_sub(&sigma_r.x4s[i], &ristretto255::scalar_mul(&rho, &amount_chunks[i]))
        });
        let a5 = ristretto255::scalar_sub(
            &sigma_r.x5,
            &ristretto255::scalar_mul(&rho, &ristretto255::scalar_invert(sender_dk).extract())
        );

        (
            TransferProof {
                sigma_proof: TransferSigmaProof {
                    xs: proof_xs,
                    alphas: TransferSigmaProofAlphas { a1s, a2, a3s, a4s, a5 }
                },
                zkrp_new_balance,
                zkrp_transfer_amount
            },
            new_balance,
            transfer_amount,
            auditor_amounts,
        )
    }

    #[test_only]
    public fun prove_normalization(
        dk: &Scalar,
        ek: &twisted_elgamal::CompressedPubkey,
        amount: u128,
        current_balance: &confidential_balance::ConfidentialBalance
    ): (NormalizationProof, confidential_balance::ConfidentialBalance)
    {
        let new_balance_r = confidential_balance::generate_balance_randomness();
        let new_balance = confidential_balance::new_actual_balance_from_u128(amount, &new_balance_r, ek);

        let new_balance_r = confidential_balance::balance_randomness_as_scalars(&new_balance_r);

        let sigma_r = generate_normalization_sigma_proof_randomness();

        let zkrp_new_balance = prove_new_balance_range(amount, new_balance_r);

        let x1 = ristretto255::basepoint_mul(
            &scalar_linear_combination(&sigma_r.x1s, &vector::range(0, 8).map(|i| new_scalar_from_pow2(i * 16)))
        );

        let current_balance_d = confidential_balance::balance_to_points_d(current_balance);

        vector::range(0, 8).for_each(|i| {
            ristretto255::point_add_assign(
                &mut x1,
                &ristretto255::point_mul(
                    &current_balance_d[i],
                    &ristretto255::scalar_mul(&sigma_r.x2, &new_scalar_from_pow2(i * 16))
                )
            );
        });

        let x2 = ristretto255::point_mul(&ristretto255::hash_to_point_base(), &sigma_r.x3);
        let x3s = vector::range(0, 8).map(|i| {
            let x3i = ristretto255::basepoint_mul(&sigma_r.x1s[i]);
            ristretto255::point_add_assign(
                &mut x3i,
                &ristretto255::point_mul(&ristretto255::hash_to_point_base(), &sigma_r.x4s[i])
            );
            x3i
        });
        let x4s = vector::range(0, 8).map(|i| {
            ristretto255::point_mul(&twisted_elgamal::pubkey_to_point(ek), &sigma_r.x4s[i])
        });

        let proof_xs = NormalizationSigmaProofXs {
            x1: ristretto255::point_compress(&x1),
            x2: ristretto255::point_compress(&x2),
            x3s: x3s.map(|x| ristretto255::point_compress(&x)),
            x4s: x4s.map(|x| ristretto255::point_compress(&x)),
        };

        let rho = fiat_shamir_normalization_sigma_proof_challenge(
            ek,
            current_balance,
            &new_balance,
            &proof_xs
        );

        let amount_chunks = confidential_balance::split_into_chunks_u128(amount);

        let a1s = vector::range(0, 8).map(|i| {
            ristretto255::scalar_sub(&sigma_r.x1s[i], &ristretto255::scalar_mul(&rho, &amount_chunks[i]))
        });
        let a2 = ristretto255::scalar_sub(&sigma_r.x2, &ristretto255::scalar_mul(&rho, dk));
        let a3 = ristretto255::scalar_sub(
            &sigma_r.x3,
            &ristretto255::scalar_mul(&rho, &ristretto255::scalar_invert(dk).extract())
        );
        let a4s = vector::range(0, 8).map(|i| {
            ristretto255::scalar_sub(&sigma_r.x4s[i], &ristretto255::scalar_mul(&rho, &new_balance_r[i]))
        });

        (
            NormalizationProof {
                sigma_proof: NormalizationSigmaProof {
                    xs: proof_xs,
                    alphas: NormalizationSigmaProofAlphas { a1s, a2, a3, a4s }
                },
                zkrp_new_balance
            },
            new_balance
        )
    }

    #[test_only]
    public fun prove_rotation(
        current_dk: &Scalar,
        new_dk: &Scalar,
        current_ek: &twisted_elgamal::CompressedPubkey,
        new_ek: &twisted_elgamal::CompressedPubkey,
        amount: u128,
        current_balance: &confidential_balance::ConfidentialBalance
    ): (RotationProof, confidential_balance::ConfidentialBalance)
    {
        let new_balance_r = confidential_balance::generate_balance_randomness();
        let new_balance = confidential_balance::new_actual_balance_from_u128(amount, &new_balance_r, new_ek);

        let new_balance_r = confidential_balance::balance_randomness_as_scalars(&new_balance_r);

        let sigma_r = generate_rotation_sigma_proof_randomness();

        let zkrp_new_balance = prove_new_balance_range(amount, new_balance_r);

        let x1 = ristretto255::basepoint_mul(
            &scalar_linear_combination(&sigma_r.x1s, &vector::range(0, 8).map(|i| new_scalar_from_pow2(i * 16)))
        );
        let current_balance_d = confidential_balance::balance_to_points_d(current_balance);

        vector::range(0, 8).for_each(|i| {
            ristretto255::point_add_assign(
                &mut x1,
                &ristretto255::point_mul(
                    &current_balance_d[i],
                    &ristretto255::scalar_mul(&sigma_r.x2, &new_scalar_from_pow2(i * 16))
                )
            );
        });

        let x2 = ristretto255::point_mul(&ristretto255::hash_to_point_base(), &sigma_r.x3);
        let x3 = ristretto255::point_mul(&ristretto255::hash_to_point_base(), &sigma_r.x4);
        let x4s = vector::range(0, 8).map(|i| {
            let x4i = ristretto255::basepoint_mul(&sigma_r.x1s[i]);
            ristretto255::point_add_assign(
                &mut x4i,
                &ristretto255::point_mul(&ristretto255::hash_to_point_base(), &sigma_r.x5s[i])
            );
            x4i
        });
        let x5s = vector::range(0, 8).map(|i| {
            ristretto255::point_mul(&twisted_elgamal::pubkey_to_point(new_ek), &sigma_r.x5s[i])
        });

        let proof_xs = RotationSigmaProofXs {
            x1: ristretto255::point_compress(&x1),
            x2: ristretto255::point_compress(&x2),
            x3: ristretto255::point_compress(&x3),
            x4s: x4s.map(|x| ristretto255::point_compress(&x)),
            x5s: x5s.map(|x| ristretto255::point_compress(&x)),
        };

        let rho = fiat_shamir_rotation_sigma_proof_challenge(
            current_ek,
            new_ek,
            current_balance,
            &new_balance,
            &proof_xs
        );

        let amount_chunks = confidential_balance::split_into_chunks_u128(amount);

        let a1s = vector::range(0, 8).map(|i| {
            ristretto255::scalar_sub(&sigma_r.x1s[i], &ristretto255::scalar_mul(&rho, &amount_chunks[i]))
        });
        let a2 = ristretto255::scalar_sub(&sigma_r.x2, &ristretto255::scalar_mul(&rho, current_dk));
        let a3 = ristretto255::scalar_sub(
            &sigma_r.x3,
            &ristretto255::scalar_mul(&rho, &ristretto255::scalar_invert(current_dk).extract())
        );
        let a4 = ristretto255::scalar_sub(
            &sigma_r.x4,
            &ristretto255::scalar_mul(&rho, &ristretto255::scalar_invert(new_dk).extract())
        );
        let a5s = vector::range(0, 8).map(|i| {
            ristretto255::scalar_sub(&sigma_r.x5s[i], &ristretto255::scalar_mul(&rho, &new_balance_r[i]))
        });

        (
            RotationProof {
                sigma_proof: RotationSigmaProof {
                    xs: proof_xs,
                    alphas: RotationSigmaProofAlphas { a1s, a2, a3, a4, a5s }
                },
                zkrp_new_balance
            },
            new_balance
        )
    }

    //
    // Test-only serialization functions
    //

    #[test_only]
    public fun serialize_withdrawal_proof(proof: &WithdrawalProof): (vector<u8>, vector<u8>) {
        (
            serialize_withdrawal_sigma_proof(&proof.sigma_proof),
            bulletproofs::range_proof_to_bytes(&proof.zkrp_new_balance)
        )
    }

    #[test_only]
    public fun serialize_transfer_proof(proof: &TransferProof): (vector<u8>, vector<u8>, vector<u8>) {
        (
            serialize_transfer_sigma_proof(&proof.sigma_proof),
            bulletproofs::range_proof_to_bytes(&proof.zkrp_new_balance),
            bulletproofs::range_proof_to_bytes(&proof.zkrp_transfer_amount)
        )
    }

    #[test_only]
    public fun serialize_normalization_proof(proof: &NormalizationProof): (vector<u8>, vector<u8>) {
        (
            serialize_normalization_sigma_proof(&proof.sigma_proof),
            bulletproofs::range_proof_to_bytes(&proof.zkrp_new_balance)
        )
    }

    #[test_only]
    public fun serialize_rotation_proof(proof: &RotationProof): (vector<u8>, vector<u8>) {
        (
            serialize_rotation_sigma_proof(&proof.sigma_proof),
            bulletproofs::range_proof_to_bytes(&proof.zkrp_new_balance)
        )
    }

    #[test_only] fun serialize_withdrawal_sigma_proof(proof: &WithdrawalSigmaProof): vector<u8> {
        let bytes = vector[];

        proof.alphas.a1s.for_each_ref(|alpha| {
            bytes.append(ristretto255::scalar_to_bytes(alpha));
        });
        bytes.append(ristretto255::scalar_to_bytes(&proof.alphas.a2));
        bytes.append(ristretto255::scalar_to_bytes(&proof.alphas.a3));
        proof.alphas.a4s.for_each_ref(|alpha| {
            bytes.append(ristretto255::scalar_to_bytes(alpha));
        });

        bytes.append(ristretto255::point_to_bytes(&proof.xs.x1));
        bytes.append(ristretto255::point_to_bytes(&proof.xs.x2));
        proof.xs.x3s.for_each_ref(|x| {
            bytes.append(ristretto255::point_to_bytes(x));
        });
        proof.xs.x4s.for_each_ref(|x| {
            bytes.append(ristretto255::point_to_bytes(x));
        });

        bytes
    }

    #[test_only] fun serialize_transfer_sigma_proof(proof: &TransferSigmaProof): vector<u8> {
        let bytes = vector[];

        proof.alphas.a1s.for_each_ref(|alpha| {
            bytes.append(ristretto255::scalar_to_bytes(alpha));
        });
        bytes.append(ristretto255::scalar_to_bytes(&proof.alphas.a2));
        proof.alphas.a3s.for_each_ref(|alpha| {
            bytes.append(ristretto255::scalar_to_bytes(alpha));
        });
        proof.alphas.a4s.for_each_ref(|alpha| {
            bytes.append(ristretto255::scalar_to_bytes(alpha));
        });
        bytes.append(ristretto255::scalar_to_bytes(&proof.alphas.a5));

        bytes.append(ristretto255::point_to_bytes(&proof.xs.x1));
        proof.xs.x2s.for_each_ref(|x| {
            bytes.append(ristretto255::point_to_bytes(x));
        });
        proof.xs.x3s.for_each_ref(|x| {
            bytes.append(ristretto255::point_to_bytes(x));
        });
        proof.xs.x4s.for_each_ref(|x| {
            bytes.append(ristretto255::point_to_bytes(x));
        });
        bytes.append(ristretto255::point_to_bytes(&proof.xs.x5));
        proof.xs.x6s.for_each_ref(|x| {
            bytes.append(ristretto255::point_to_bytes(x));
        });
        proof.xs.x7s.for_each_ref(|xs| {
            xs.for_each_ref(|x| {
                bytes.append(ristretto255::point_to_bytes(x));
            });
        });

        bytes
    }

    #[test_only] fun serialize_normalization_sigma_proof(proof: &NormalizationSigmaProof): vector<u8> {
        let bytes = vector[];

        proof.alphas.a1s.for_each_ref(|alpha| {
            bytes.append(ristretto255::scalar_to_bytes(alpha));
        });
        bytes.append(ristretto255::scalar_to_bytes(&proof.alphas.a2));
        bytes.append(ristretto255::scalar_to_bytes(&proof.alphas.a3));
        proof.alphas.a4s.for_each_ref(|alpha| {
            bytes.append(ristretto255::scalar_to_bytes(alpha));
        });

        bytes.append(ristretto255::point_to_bytes(&proof.xs.x1));
        bytes.append(ristretto255::point_to_bytes(&proof.xs.x2));
        proof.xs.x3s.for_each_ref(|x| {
            bytes.append(ristretto255::point_to_bytes(x));
        });
        proof.xs.x4s.for_each_ref(|x| {
            bytes.append(ristretto255::point_to_bytes(x));
        });

        bytes
    }

    #[test_only] fun serialize_rotation_sigma_proof(proof: &RotationSigmaProof): vector<u8> {
        let bytes = vector[];

        proof.alphas.a1s.for_each_ref(|alpha| {
            bytes.append(ristretto255::scalar_to_bytes(alpha));
        });
        bytes.append(ristretto255::scalar_to_bytes(&proof.alphas.a2));
        bytes.append(ristretto255::scalar_to_bytes(&proof.alphas.a3));
        bytes.append(ristretto255::scalar_to_bytes(&proof.alphas.a4));
        proof.alphas.a5s.for_each_ref(|alpha| {
            bytes.append(ristretto255::scalar_to_bytes(alpha));
        });

        bytes.append(ristretto255::point_to_bytes(&proof.xs.x1));
        bytes.append(ristretto255::point_to_bytes(&proof.xs.x2));
        bytes.append(ristretto255::point_to_bytes(&proof.xs.x3));
        proof.xs.x4s.for_each_ref(|x| {
            bytes.append(ristretto255::point_to_bytes(x));
        });
        proof.xs.x5s.for_each_ref(|x| {
            bytes.append(ristretto255::point_to_bytes(x));
        });

        bytes
    }

    //
    // Test-only private functions
    //

    #[test_only]
    fun prove_new_balance_range(new_amount: u128, randomness: &vector<Scalar>): RangeProof {
        let new_amount_chunks = confidential_balance::split_into_chunks_u128(new_amount);

        let (proof, _) = bulletproofs::prove_batch_range_pedersen(
            &new_amount_chunks,
            randomness,
            BULLETPROOFS_NUM_BITS,
            BULLETPROOFS_DST);
        proof
    }

    #[test_only]
    fun prove_transfer_amount_range(amount: u64, randomness: &vector<Scalar>): RangeProof {
        let amount_chunks = confidential_balance::split_into_chunks_u64(amount);

        let (proof, _) = bulletproofs::prove_batch_range_pedersen(
            &amount_chunks,
            randomness,
            BULLETPROOFS_NUM_BITS,
            BULLETPROOFS_DST);
        proof
    }

    #[test_only] fun generate_withdrawal_sigma_proof_randomness(): WithdrawalSigmaProofRandomness {
        WithdrawalSigmaProofRandomness {
            x1s: vector::range(0, 8).map(|_| ristretto255::random_scalar()),
            x2: ristretto255::random_scalar(),
            x3: ristretto255::random_scalar(),
            x4s: vector::range(0, 8).map(|_| ristretto255::random_scalar()),
        }
    }

    #[test_only] fun generate_transfer_sigma_proof_randomness(): TransferSigmaProofRandomness {
        TransferSigmaProofRandomness {
            x1s: vector::range(0, 8).map(|_| ristretto255::random_scalar()),
            x2: ristretto255::random_scalar(),
            x3s: vector::range(0, 8).map(|_| ristretto255::random_scalar()),
            x4s: vector::range(0, 4).map(|_| ristretto255::random_scalar()),
            x5: ristretto255::random_scalar(),
        }
    }

    #[test_only] fun generate_normalization_sigma_proof_randomness(): NormalizationSigmaProofRandomness {
        NormalizationSigmaProofRandomness {
            x1s: vector::range(0, 8).map(|_| ristretto255::random_scalar()),
            x2: ristretto255::random_scalar(),
            x3: ristretto255::random_scalar(),
            x4s: vector::range(0, 8).map(|_| ristretto255::random_scalar()),
        }
    }

    #[test_only] fun generate_rotation_sigma_proof_randomness(): RotationSigmaProofRandomness {
        RotationSigmaProofRandomness {
            x1s: vector::range(0, 8).map(|_| ristretto255::random_scalar()),
            x2: ristretto255::random_scalar(),
            x3: ristretto255::random_scalar(),
            x4: ristretto255::random_scalar(),
            x5s: vector::range(0, 8).map(|_| ristretto255::random_scalar()),
        }
    }
}
