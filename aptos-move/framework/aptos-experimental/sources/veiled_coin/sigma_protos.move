/// Package for creating, verifying, serializing & deserializing the $\Sigma$-protocol proofs used in veiled coins.
///
/// ## Preliminaries
///
/// Recall that a $\Sigma$-protocol proof argues knowledge of a *secret* witness $w$ such that an arithmetic relation
/// $R(x; w) = 1$ is satisfied over group and field elements stored in $x$ and $w$.
///
/// Here, $x$ is a public statement known to the verifier (i.e., known to the validators). Importantly, the
/// $\Sigma$-protocol's zero-knowledge property ensures the witness $w$ remains secret.
///
/// ## WithdrawalSubproof: ElGamal-Pedersen equality
///
/// This proof is used to provably convert an ElGamal ciphertext to a Pedersen commitment over which a ZK range proof
/// can be securely computed. Otherwise, knowledge of the ElGamal SK breaks the binding of the 2nd component of the
/// ElGamal ciphertext, making any ZK range proof over it useless.
/// Because the sender cannot, after receiving a fully veiled transaction, compute their balance randomness, their
/// updated balance ciphertext is computed in the relation, which is then linked to the Pedersen commitment of $b$.
///
/// The secret witness $w$ in this relation, known only to the sender of the TXN, consists of:
///  - $b$, sender's new balance, after the withdrawal from their veiled balance
///  - $r$, randomness used to commit to $b$
///  - $sk$, the sender's secret ElGamal encryption key
///
/// (Note that the $\Sigma$-protocol's zero-knowledge property ensures the witness is not revealed.)
///
/// The public statement $x$ in this relation consists of:
///  - $G$, basepoint of a given elliptic curve
///  - $H$, basepoint used for randomness in the Pedersen commitments
///  - $(C_1, C_2)$, ElGamal encryption of the sender's current balance
///  - $c$, Pedersen commitment to $b$ with randomness $r$
///  - $v$, the amount the sender is withdrawing
///  - $Y$, the sender's ElGamal encryption public key
///
/// The relation being proved is as follows:
///
/// ```
/// R(
///     x = [ (C_1, C_2), c, G, H, Y, v]
///     w = [ b, r, sk ]
/// ) = {
///    C_1 - v G = b G + sk C_2
///            c = b G + r H
///            Y = sk G
/// }
/// ```
///
/// ## TransferSubproof: ElGamal-Pedersen equality and ElGamal-ElGamal equality
///
/// This protocol argues two things. First, that the same amount is ElGamal-encrypted for both the sender and recipient.
/// This is needed to correctly withdraw & deposit the same amount during a transfer. Second, that this same amount is
/// committed via Pedersen. Third, that a Pedersen-committed balance is correctly ElGamal encrypted. ZK range proofs
/// are computed over these last two Pedersen commitments, to prevent overflowing attacks on the balance.
///
/// The secret witness $w$ in this relation, known only to the sender of the TXN, consists of:
///  - $v$, amount being transferred
///  - $r$, randomness used to ElGamal-encrypt $v$
///  - $b$, sender's new balance after the transfer occurs
///  - $r_b$, randomness used to Pedersen commit $b$
///  - $sk$, the sender's secret ElGamal encryption key
///
/// The public statement $x$ in this relation consists of:
///  - Public parameters
///    + $G$, basepoint of a given elliptic curve
///    + $H$, basepoint used for randomness in the Pedersen commitments
///  - PKs
///    + $Y$, sender's PK
///    + $Y'$, recipient's PK
///  - Amount encryption & commitment
///    + $(C, D)$, ElGamal encryption of $v$, under the sender's PK, using randomness $r$
///    + $(C', D)$, ElGamal encryption of $v$, under the recipient's PK, using randomness $r$
///    + $c$, Pedersen commitment to $v$ using randomness $r$
///  - New balance encryption & commitment
///    + $(C_1, C_2)$, ElGamal encryption of the sender's *current* balance, under the sender's PK. This is used to
///      compute the sender's updated balance in the relation, as the sender cannot know their balance randomness.
///    + $c'$, Pedersen commitment to $b$ using randomness $r_b$
///
/// The relation being proved is:
/// ```
/// R(
///     x = [ Y, Y', (C, C', D), c, (C_1, C_2), c', G, H ]
///     w = [ v, r, b, r_b, sk ]
/// ) = {
///          C  = v G + r Y
///          C' = v G + r Y'
///          D  = r G
///    C_1 - C  = b G + sk (C_2 - D)
///          c  = v G + r H
///          c' = b G + r_b H
///          Y  = sk G
/// }
/// ```
///
/// A relation similar to this is also described on page 14 of the Zether paper [BAZB20] (just replace  $G$ -> $g$,
/// $C'$ -> $\bar{C}$, $Y$ -> $y$, $Y'$ -> $\bar{y}$, $v$ -> $b^*$). Note that their relation does not include the
/// ElGamal-to-Pedersen conversion parts, as they can do ZK range proofs directly over ElGamal ciphertexts using their
/// $\Sigma$-bullets modification of Bulletproofs.
module aptos_experimental::sigma_protos {
    use std::error;
    use std::option::Option;
    use std::vector;

    use aptos_std::ristretto255_elgamal as elgamal;
    use aptos_std::ristretto255_pedersen as pedersen;
    use aptos_std::ristretto255::{Self, RistrettoPoint, Scalar};

    use aptos_experimental::helpers::cut_vector;

    #[test_only]
    use aptos_experimental::helpers::generate_elgamal_keypair;

    //
    // Errors
    //

    /// The $\Sigma$-protocol proof for withdrawals did not verify.
    const ESIGMA_PROTOCOL_VERIFY_FAILED: u64 = 1;

    //
    // Constants
    //

    /// The domain separation tag (DST) used in the Fiat-Shamir transform of our $\Sigma$-protocol.
    const FIAT_SHAMIR_SIGMA_DST : vector<u8> = b"AptosVeiledCoin/WithdrawalSubproofFiatShamir";

    //
    // Structs
    //

    /// A $\Sigma$-protocol used during an unveiled withdrawal (for proving the correct ElGamal encryption of a
    /// Pedersen-committed balance).
    struct WithdrawalSubproof has drop {
        x1: RistrettoPoint,
        x2: RistrettoPoint,
        x3: RistrettoPoint,
        alpha1: Scalar,
        alpha2: Scalar,
        alpha3: Scalar,
    }

    /// A $\Sigma$-protocol proof used during a veiled transfer. This proof encompasses the $\Sigma$-protocol from
    /// `WithdrawalSubproof`.
    struct TransferSubproof has drop {
        x1: RistrettoPoint,
        x2: RistrettoPoint,
        x3: RistrettoPoint,
        x4: RistrettoPoint,
        x5: RistrettoPoint,
        x6: RistrettoPoint,
        x7: RistrettoPoint,
        alpha1: Scalar,
        alpha2: Scalar,
        alpha3: Scalar,
        alpha4: Scalar,
        alpha5: Scalar,
    }

    //
    // Public proof verification functions
    //

    /// Verifies a $\Sigma$-protocol proof necessary to ensure correctness of a veiled transfer.
    ///
    /// Specifically, the proof argues that the same amount $v$ is Pedersen-committed in `comm_amount` and ElGamal-
    /// encrypted in `withdraw_ct` (under `sender_pk`) and in `deposit_ct` (under `recipient_pk`), all three using the
    /// same randomness $r$.
    ///
    /// In addition, it argues that the sender's new balance $b$ committed to by sender_new_balance_comm is the same
    /// as the value encrypted by the ciphertext obtained by subtracting withdraw_ct from sender_curr_balance_ct
    public fun verify_transfer_subproof(
        sender_pk: &elgamal::CompressedPubkey,
        recipient_pk: &elgamal::CompressedPubkey,
        withdraw_ct: &elgamal::Ciphertext,
        deposit_ct: &elgamal::Ciphertext,
        comm_amount: &pedersen::Commitment,
        sender_new_balance_comm: &pedersen::Commitment,
        sender_curr_balance_ct: &elgamal::Ciphertext,
        proof: &TransferSubproof)
    {
        let h = pedersen::randomness_base_for_bulletproof();
        let sender_pk_point = elgamal::pubkey_to_point(sender_pk);
        let recipient_pk_point = elgamal::pubkey_to_point(recipient_pk);
        let (big_c, big_d) = elgamal::ciphertext_as_points(withdraw_ct);
        let (bar_big_c, _) = elgamal::ciphertext_as_points(deposit_ct);
        let c = pedersen::commitment_as_point(comm_amount);
        let (c1, c2) = elgamal::ciphertext_as_points(sender_curr_balance_ct);
        let bar_c = pedersen::commitment_as_point(sender_new_balance_comm);

        // TODO: Can be optimized so we don't re-serialize the proof for Fiat-Shamir
        let rho = fiat_shamir_transfer_subproof_challenge(
            sender_pk, recipient_pk,
            withdraw_ct, deposit_ct, comm_amount,
            sender_curr_balance_ct, sender_new_balance_comm,
            &proof.x1, &proof.x2, &proof.x3, &proof.x4,
            &proof.x5, &proof.x6, &proof.x7);

        let g_alpha2 = ristretto255::basepoint_mul(&proof.alpha2);
        // \rho * D + X1 =? \alpha_2 * g
        let d_acc = ristretto255::point_mul(big_d, &rho);
        ristretto255::point_add_assign(&mut d_acc, &proof.x1);
        assert!(ristretto255::point_equals(&d_acc, &g_alpha2), error::invalid_argument(ESIGMA_PROTOCOL_VERIFY_FAILED));

        let g_alpha1 = ristretto255::basepoint_mul(&proof.alpha1);
        // \rho * C + X2 =? \alpha_1 * g + \alpha_2 * y
        let big_c_acc = ristretto255::point_mul(big_c, &rho);
        ristretto255::point_add_assign(&mut big_c_acc, &proof.x2);
        let y_alpha2 = ristretto255::point_mul(&sender_pk_point, &proof.alpha2);
        ristretto255::point_add_assign(&mut y_alpha2, &g_alpha1);
        assert!(ristretto255::point_equals(&big_c_acc, &y_alpha2), error::invalid_argument(ESIGMA_PROTOCOL_VERIFY_FAILED));

        // \rho * \bar{C} + X3 =? \alpha_1 * g + \alpha_2 * \bar{y}
        let big_bar_c_acc = ristretto255::point_mul(bar_big_c, &rho);
        ristretto255::point_add_assign(&mut big_bar_c_acc, &proof.x3);
        let y_bar_alpha2 = ristretto255::point_mul(&recipient_pk_point, &proof.alpha2);
        ristretto255::point_add_assign(&mut y_bar_alpha2, &g_alpha1);
        assert!(ristretto255::point_equals(&big_bar_c_acc, &y_bar_alpha2), error::invalid_argument(ESIGMA_PROTOCOL_VERIFY_FAILED));

        let g_alpha3 = ristretto255::basepoint_mul(&proof.alpha3);
        // \rho * (C_1 - C) + X_4 =? \alpha_3 * g + \alpha_5 * (C_2 - D)
        let big_c1_acc = ristretto255::point_sub(c1, big_c);
        ristretto255::point_mul_assign(&mut big_c1_acc, &rho);
        ristretto255::point_add_assign(&mut big_c1_acc, &proof.x4);

        let big_c2_acc = ristretto255::point_sub(c2, big_d);
        ristretto255::point_mul_assign(&mut big_c2_acc, &proof.alpha5);
        ristretto255::point_add_assign(&mut big_c2_acc, &g_alpha3);
        assert!(ristretto255::point_equals(&big_c1_acc, &big_c2_acc), error::invalid_argument(ESIGMA_PROTOCOL_VERIFY_FAILED));

        // \rho * c + X_5 =? \alpha_1 * g + \alpha_2 * h
        let c_acc = ristretto255::point_mul(c, &rho);
        ristretto255::point_add_assign(&mut c_acc, &proof.x5);

        let h_alpha2_acc = ristretto255::point_mul(&h, &proof.alpha2);
        ristretto255::point_add_assign(&mut h_alpha2_acc, &g_alpha1);
        assert!(ristretto255::point_equals(&c_acc, &h_alpha2_acc), error::invalid_argument(ESIGMA_PROTOCOL_VERIFY_FAILED));

        // \rho * \bar{c} + X_6 =? \alpha_3 * g + \alpha_4 * h
        let bar_c_acc = ristretto255::point_mul(bar_c, &rho);
        ristretto255::point_add_assign(&mut bar_c_acc, &proof.x6);

        let h_alpha4_acc = ristretto255::point_mul(&h, &proof.alpha4);
        ristretto255::point_add_assign(&mut h_alpha4_acc, &g_alpha3);
        assert!(ristretto255::point_equals(&bar_c_acc, &h_alpha4_acc), error::invalid_argument(ESIGMA_PROTOCOL_VERIFY_FAILED));

        // \rho * Y + X_7 =? \alpha_5 * G
        let y_acc = ristretto255::point_mul(&sender_pk_point, &rho);
        ristretto255::point_add_assign(&mut y_acc, &proof.x7);

        let g_alpha5 = ristretto255::basepoint_mul(&proof.alpha5);
        assert!(ristretto255::point_equals(&y_acc, &g_alpha5), error::invalid_argument(ESIGMA_PROTOCOL_VERIFY_FAILED));
    }

    /// Verifies the $\Sigma$-protocol proof necessary to ensure correctness of a veiled-to-unveiled transfer.
    ///
    /// Specifically, the proof argues that the same amount $v$ is Pedersen-committed in `sender_new_balance_comm` and
    /// ElGamal-encrypted in the ciphertext obtained by subtracting the ciphertext (vG, 0G) from sender_curr_balance_ct
    public fun verify_withdrawal_subproof(
        sender_pk: &elgamal::CompressedPubkey,
        sender_curr_balance_ct: &elgamal::Ciphertext,
        sender_new_balance_comm: &pedersen::Commitment,
        amount: &Scalar,
        proof: &WithdrawalSubproof)
    {
        let h = pedersen::randomness_base_for_bulletproof();
        let (big_c1, big_c2) = elgamal::ciphertext_as_points(sender_curr_balance_ct);
        let c = pedersen::commitment_as_point(sender_new_balance_comm);
        let sender_pk_point = elgamal::pubkey_to_point(sender_pk);

        let rho = fiat_shamir_withdrawal_subproof_challenge(
            sender_pk,
            sender_curr_balance_ct,
            sender_new_balance_comm,
            amount,
            &proof.x1,
            &proof.x2,
            &proof.x3);

        let g_alpha1 = ristretto255::basepoint_mul(&proof.alpha1);
        // \rho * (C_1 - v * g) + X_1 =? \alpha_1 * g + \alpha_3 * C_2
        let gv = ristretto255::basepoint_mul(amount);
        let big_c1_acc = ristretto255::point_sub(big_c1, &gv);
        ristretto255::point_mul_assign(&mut big_c1_acc, &rho);
        ristretto255::point_add_assign(&mut big_c1_acc, &proof.x1);

        let big_c2_acc = ristretto255::point_mul(big_c2, &proof.alpha3);
        ristretto255::point_add_assign(&mut big_c2_acc, &g_alpha1);
        assert!(ristretto255::point_equals(&big_c1_acc, &big_c2_acc), error::invalid_argument(ESIGMA_PROTOCOL_VERIFY_FAILED));

        // \rho * c + X_2 =? \alpha_1 * g + \alpha_2 * h
        let c_acc = ristretto255::point_mul(c, &rho);
        ristretto255::point_add_assign(&mut c_acc, &proof.x2);

        let h_alpha2_acc = ristretto255::point_mul(&h, &proof.alpha2);
        ristretto255::point_add_assign(&mut h_alpha2_acc, &g_alpha1);
        assert!(ristretto255::point_equals(&c_acc, &h_alpha2_acc), error::invalid_argument(ESIGMA_PROTOCOL_VERIFY_FAILED));

        // \rho * Y + X_3 =? \alpha_3 * g
        let y_acc = ristretto255::point_mul(&sender_pk_point, &rho);
        ristretto255::point_add_assign(&mut y_acc, &proof.x3);

        let g_alpha3 = ristretto255::basepoint_mul(&proof.alpha3);
        assert!(ristretto255::point_equals(&y_acc, &g_alpha3), error::invalid_argument(ESIGMA_PROTOCOL_VERIFY_FAILED));
    }

    //
    // Public deserialization functions
    //

    /// Deserializes and returns an `WithdrawalSubproof` given its byte representation.
    public fun deserialize_withdrawal_subproof(proof_bytes: vector<u8>): Option<WithdrawalSubproof> {
        if (proof_bytes.length() != 192) {
            return std::option::none<WithdrawalSubproof>()
        };

        let x1_bytes = cut_vector<u8>(&mut proof_bytes, 32);
        let x1 = ristretto255::new_point_from_bytes(x1_bytes);
        if (!x1.is_some()) {
            return std::option::none<WithdrawalSubproof>()
        };
        let x1 = x1.extract();

        let x2_bytes = cut_vector<u8>(&mut proof_bytes, 32);
        let x2 = ristretto255::new_point_from_bytes(x2_bytes);
        if (!x2.is_some()) {
            return std::option::none<WithdrawalSubproof>()
        };
        let x2 = x2.extract();

        let x3_bytes = cut_vector<u8>(&mut proof_bytes, 32);
        let x3 = ristretto255::new_point_from_bytes(x3_bytes);
        if (!x3.is_some()) {
            return std::option::none<WithdrawalSubproof>()
        };
        let x3 = x3.extract();

        let alpha1_bytes = cut_vector<u8>(&mut proof_bytes, 32);
        let alpha1 = ristretto255::new_scalar_from_bytes(alpha1_bytes);
        if (!alpha1.is_some()) {
            return std::option::none<WithdrawalSubproof>()
        };
        let alpha1 = alpha1.extract();

        let alpha2_bytes = cut_vector<u8>(&mut proof_bytes, 32);
        let alpha2 = ristretto255::new_scalar_from_bytes(alpha2_bytes);
        if (!alpha2.is_some()) {
            return std::option::none<WithdrawalSubproof>()
        };
        let alpha2 = alpha2.extract();

        let alpha3_bytes = cut_vector<u8>(&mut proof_bytes, 32);
        let alpha3 = ristretto255::new_scalar_from_bytes(alpha3_bytes);
        if (!alpha3.is_some()) {
            return std::option::none<WithdrawalSubproof>()
        };
        let alpha3 = alpha3.extract();

        std::option::some(WithdrawalSubproof {
            x1, x2, x3, alpha1, alpha2, alpha3
        })
    }

    /// Deserializes and returns a `TransferSubproof` given its byte representation.
    public fun deserialize_transfer_subproof(proof_bytes: vector<u8>): Option<TransferSubproof> {
        if (proof_bytes.length() != 384) {
            return std::option::none<TransferSubproof>()
        };

        let x1_bytes = cut_vector<u8>(&mut proof_bytes, 32);
        let x1 = ristretto255::new_point_from_bytes(x1_bytes);
        if (!x1.is_some()) {
            return std::option::none<TransferSubproof>()
        };
        let x1 = x1.extract();

        let x2_bytes = cut_vector<u8>(&mut proof_bytes, 32);
        let x2 = ristretto255::new_point_from_bytes(x2_bytes);
        if (!x2.is_some()) {
            return std::option::none<TransferSubproof>()
        };
        let x2 = x2.extract();

        let x3_bytes = cut_vector<u8>(&mut proof_bytes, 32);
        let x3 = ristretto255::new_point_from_bytes(x3_bytes);
        if (!x3.is_some()) {
            return std::option::none<TransferSubproof>()
        };
        let x3 = x3.extract();

        let x4_bytes = cut_vector<u8>(&mut proof_bytes, 32);
        let x4 = ristretto255::new_point_from_bytes(x4_bytes);
        if (!x4.is_some()) {
            return std::option::none<TransferSubproof>()
        };
        let x4 = x4.extract();

        let x5_bytes = cut_vector<u8>(&mut proof_bytes, 32);
        let x5 = ristretto255::new_point_from_bytes(x5_bytes);
        if (!x5.is_some()) {
            return std::option::none<TransferSubproof>()
        };
        let x5 = x5.extract();

        let x6_bytes = cut_vector<u8>(&mut proof_bytes, 32);
        let x6 = ristretto255::new_point_from_bytes(x6_bytes);
        if (!x6.is_some()) {
            return std::option::none<TransferSubproof>()
        };
        let x6 = x6.extract();

        let x7_bytes = cut_vector<u8>(&mut proof_bytes, 32);
        let x7 = ristretto255::new_point_from_bytes(x7_bytes);
        if (!x7.is_some()) {
            return std::option::none<TransferSubproof>()
        };
        let x7 = x7.extract();

        let alpha1_bytes = cut_vector<u8>(&mut proof_bytes, 32);
        let alpha1 = ristretto255::new_scalar_from_bytes(alpha1_bytes);
        if (!alpha1.is_some()) {
            return std::option::none<TransferSubproof>()
        };
        let alpha1 = alpha1.extract();

        let alpha2_bytes = cut_vector<u8>(&mut proof_bytes, 32);
        let alpha2 = ristretto255::new_scalar_from_bytes(alpha2_bytes);
        if (!alpha2.is_some()) {
            return std::option::none<TransferSubproof>()
        };
        let alpha2 = alpha2.extract();

        let alpha3_bytes = cut_vector<u8>(&mut proof_bytes, 32);
        let alpha3 = ristretto255::new_scalar_from_bytes(alpha3_bytes);
        if (!alpha3.is_some()) {
            return std::option::none<TransferSubproof>()
        };
        let alpha3 = alpha3.extract();

        let alpha4_bytes = cut_vector<u8>(&mut proof_bytes, 32);
        let alpha4 = ristretto255::new_scalar_from_bytes(alpha4_bytes);
        if (!alpha4.is_some()) {
            return std::option::none<TransferSubproof>()
        };
        let alpha4 = alpha4.extract();

        let alpha5_bytes = cut_vector<u8>(&mut proof_bytes, 32);
        let alpha5 = ristretto255::new_scalar_from_bytes(alpha5_bytes);
        if (!alpha5.is_some()) {
            return std::option::none<TransferSubproof>()
        };
        let alpha5 = alpha5.extract();

        std::option::some(TransferSubproof {
            x1, x2, x3, x4, x5, x6, x7, alpha1, alpha2, alpha3, alpha4, alpha5
        })
    }

    //
    // Private functions for Fiat-Shamir challenge derivation
    //

    /// Computes a Fiat-Shamir challenge `rho = H(G, H, Y, C_1, C_2, c, x_1, x_2, x_3)` for the `WithdrawalSubproof`
    /// $\Sigma$-protocol.
    fun fiat_shamir_withdrawal_subproof_challenge(
        sender_pk: &elgamal::CompressedPubkey,
        sender_curr_balance_ct: &elgamal::Ciphertext,
        sender_new_balance_comm: &pedersen::Commitment,
        amount: &Scalar,
        x1: &RistrettoPoint,
        x2: &RistrettoPoint,
        x3: &RistrettoPoint): Scalar
    {
        let (c1, c2) = elgamal::ciphertext_as_points(sender_curr_balance_ct);
        let c = pedersen::commitment_as_point(sender_new_balance_comm);
        let y = elgamal::pubkey_to_compressed_point(sender_pk);

        let bytes = vector::empty<u8>();

        bytes.append(FIAT_SHAMIR_SIGMA_DST);
        bytes.append(ristretto255::point_to_bytes(&ristretto255::basepoint_compressed()));
        bytes.append(ristretto255::point_to_bytes(
            &ristretto255::point_compress(&pedersen::randomness_base_for_bulletproof())));
        bytes.append(ristretto255::point_to_bytes(&y));
        bytes.append(ristretto255::point_to_bytes(&ristretto255::point_compress(c1)));
        bytes.append(ristretto255::point_to_bytes(&ristretto255::point_compress(c2)));
        bytes.append(ristretto255::point_to_bytes(&ristretto255::point_compress(c)));
        bytes.append(ristretto255::scalar_to_bytes(amount));
        bytes.append(ristretto255::point_to_bytes(&ristretto255::point_compress(x1)));
        bytes.append(ristretto255::point_to_bytes(&ristretto255::point_compress(x2)));
        bytes.append(ristretto255::point_to_bytes(&ristretto255::point_compress(x3)));

        ristretto255::new_scalar_from_sha2_512(bytes)
    }

    /// Computes a Fiat-Shamir challenge `rho = H(G, H, Y, Y', C, D, c, c_1, c_2, \bar{c}, {X_i}_{i=1}^7)` for the
    /// `TransferSubproof` $\Sigma$-protocol.
    fun fiat_shamir_transfer_subproof_challenge(
        sender_pk: &elgamal::CompressedPubkey,
        recipient_pk: &elgamal::CompressedPubkey,
        withdraw_ct: &elgamal::Ciphertext,
        deposit_ct: &elgamal::Ciphertext,
        comm_amount: &pedersen::Commitment,
        sender_curr_balance_ct: &elgamal::Ciphertext,
        sender_new_balance_comm: &pedersen::Commitment,
        x1: &RistrettoPoint,
        x2: &RistrettoPoint,
        x3: &RistrettoPoint,
        x4: &RistrettoPoint,
        x5: &RistrettoPoint,
        x6: &RistrettoPoint,
        x7: &RistrettoPoint): Scalar
    {
        let y = elgamal::pubkey_to_compressed_point(sender_pk);
        let y_prime = elgamal::pubkey_to_compressed_point(recipient_pk);
        let (big_c, big_d) = elgamal::ciphertext_as_points(withdraw_ct);
        let (big_c_prime, _) = elgamal::ciphertext_as_points(deposit_ct);
        let c = pedersen::commitment_as_point(comm_amount);
        let (c1, c2) = elgamal::ciphertext_as_points(sender_curr_balance_ct);
        let bar_c = pedersen::commitment_as_point(sender_new_balance_comm);

        let bytes = vector::empty<u8>();

        bytes.append(FIAT_SHAMIR_SIGMA_DST);
        bytes.append(ristretto255::point_to_bytes(&ristretto255::basepoint_compressed()));
        bytes.append(ristretto255::point_to_bytes(
            &ristretto255::point_compress(&pedersen::randomness_base_for_bulletproof())));
        bytes.append(ristretto255::point_to_bytes(&y));
        bytes.append(ristretto255::point_to_bytes(&y_prime));
        bytes.append(ristretto255::point_to_bytes(&ristretto255::point_compress(big_c)));
        bytes.append(ristretto255::point_to_bytes(&ristretto255::point_compress(big_c_prime)));
        bytes.append(ristretto255::point_to_bytes(&ristretto255::point_compress(big_d)));
        bytes.append(ristretto255::point_to_bytes(&ristretto255::point_compress(c)));
        bytes.append(ristretto255::point_to_bytes(&ristretto255::point_compress(c1)));
        bytes.append(ristretto255::point_to_bytes(&ristretto255::point_compress(c2)));
        bytes.append(ristretto255::point_to_bytes(&ristretto255::point_compress(bar_c)));
        bytes.append(ristretto255::point_to_bytes(&ristretto255::point_compress(x1)));
        bytes.append(ristretto255::point_to_bytes(&ristretto255::point_compress(x2)));
        bytes.append(ristretto255::point_to_bytes(&ristretto255::point_compress(x3)));
        bytes.append(ristretto255::point_to_bytes(&ristretto255::point_compress(x4)));
        bytes.append(ristretto255::point_to_bytes(&ristretto255::point_compress(x5)));
        bytes.append(ristretto255::point_to_bytes(&ristretto255::point_compress(x6)));
        bytes.append(ristretto255::point_to_bytes(&ristretto255::point_compress(x7)));

        ristretto255::new_scalar_from_sha2_512(bytes)
    }

    //
    // Test-only serialization & proving functions
    //

    #[test_only]
    /// Proves the $\Sigma$-protocol used for veiled-to-unveiled coin transfers.
    /// See top-level comments for a detailed description of the $\Sigma$-protocol
    public fun prove_withdrawal(
        sender_sk: &Scalar,
        sender_pk: &elgamal::CompressedPubkey,
        sender_curr_balance_ct: &elgamal::Ciphertext,
        sender_new_balance_comm: &pedersen::Commitment,
        new_balance_val: &Scalar,
        amount_val: &Scalar,
        new_balance_comm_rand: &Scalar): WithdrawalSubproof
    {
        let x1 = ristretto255::random_scalar();
        let x2 = ristretto255::random_scalar();
        let x3 = ristretto255::random_scalar();
        let h = pedersen::randomness_base_for_bulletproof();
        let (_, c2) = elgamal::ciphertext_as_points(sender_curr_balance_ct);

        let g_x1 = ristretto255::basepoint_mul(&x1);
        // X1 <- x1 * g + x3 * C2
        let big_x1 = ristretto255::point_mul(c2, &x3);
        ristretto255::point_add_assign(&mut big_x1, &g_x1);

        // X2 <- x1 * g + x2 * h
        let big_x2 = ristretto255::point_mul(&h, &x2);
        ristretto255::point_add_assign(&mut big_x2, &g_x1);

        // X3 <- x3 * g
        let big_x3 = ristretto255::basepoint_mul(&x3);

        let rho = fiat_shamir_withdrawal_subproof_challenge(
            sender_pk,
            sender_curr_balance_ct,
            sender_new_balance_comm,
            amount_val,
            &big_x1,
            &big_x2,
            &big_x3);

        // X3 <- x3 * g
        let big_x3 = ristretto255::basepoint_mul(&x3);

        // alpha1 <- x1 + rho * b
        let alpha1 = ristretto255::scalar_mul(&rho, new_balance_val);
        ristretto255::scalar_add_assign(&mut alpha1, &x1);

        // alpha2 <- x2 + rho * r'
        let alpha2 = ristretto255::scalar_mul(&rho, new_balance_comm_rand);
        ristretto255::scalar_add_assign(&mut alpha2, &x2);

        // alpha3 <- x3 + rho * sk
        let alpha3 = ristretto255::scalar_mul(&rho, sender_sk);
        ristretto255::scalar_add_assign(&mut alpha3, &x3);

        WithdrawalSubproof {
            x1: big_x1,
            x2: big_x2,
            x3: big_x3,
            alpha1,
            alpha2,
            alpha3,
        }
    }

    #[test_only]
    /// Proves the $\Sigma$-protocol used for veiled coin transfers.
    /// See top-level comments for a detailed description of the $\Sigma$-protocol
    public fun prove_transfer(
        sender_pk: &elgamal::CompressedPubkey,
        sender_sk: &Scalar,
        recipient_pk: &elgamal::CompressedPubkey,
        withdraw_ct: &elgamal::Ciphertext,
        deposit_ct: &elgamal::Ciphertext,
        comm_amount: &pedersen::Commitment,
        sender_curr_balance_ct: &elgamal::Ciphertext,
        sender_new_balance_comm: &pedersen::Commitment,
        amount_rand: &Scalar,
        amount_val: &Scalar,
        new_balance_comm_rand: &Scalar,
        new_balance_val: &Scalar): TransferSubproof
    {
        let x1 = ristretto255::random_scalar();
        let x2 = ristretto255::random_scalar();
        let x3 = ristretto255::random_scalar();
        let x4 = ristretto255::random_scalar();
        let x5 = ristretto255::random_scalar();
        let source_pk_point = elgamal::pubkey_to_point(sender_pk);
        let recipient_pk_point = elgamal::pubkey_to_point(recipient_pk);
        let h = pedersen::randomness_base_for_bulletproof();
        let (_, c2) = elgamal::ciphertext_as_points(sender_curr_balance_ct);
        let (_, d) = elgamal::ciphertext_as_points(withdraw_ct);

        // X1 <- x2 * g
        let big_x1 = ristretto255::basepoint_mul(&x2);

        let g_x1 = ristretto255::basepoint_mul(&x1);
        // X2 <- x1 * g + x2 * y
        let big_x2 = ristretto255::point_mul(&source_pk_point, &x2);
        ristretto255::point_add_assign(&mut big_x2, &g_x1);

        // X3 <- x1 * g + x2 * \bar{y}
        let big_x3 = ristretto255::point_mul(&recipient_pk_point, &x2);
        ristretto255::point_add_assign(&mut big_x3, &g_x1);

        let g_x3 = ristretto255::basepoint_mul(&x3);
        // X4 <- x3 * g + x5 * (C_2 - D)
        let big_x4 = ristretto255::point_sub(c2, d);
        ristretto255::point_mul_assign(&mut big_x4, &x5);
        ristretto255::point_add_assign(&mut big_x4, &g_x3);

        // X5 <- x1 * g + x2 * h
        let big_x5 = ristretto255::point_mul(&h, &x2);
        ristretto255::point_add_assign(&mut big_x5, &g_x1);

        // X6 <- x3 * g + x4 * h
        let big_x6 = ristretto255::point_mul(&h, &x4);
        ristretto255::point_add_assign(&mut big_x6, &g_x3);

        // X7 <- x5 * g
        let big_x7 = ristretto255::basepoint_mul(&x5);

        let rho = fiat_shamir_transfer_subproof_challenge(
            sender_pk, recipient_pk,
            withdraw_ct, deposit_ct, comm_amount,
            sender_curr_balance_ct, sender_new_balance_comm,
            &big_x1, &big_x2, &big_x3, &big_x4,
            &big_x5, &big_x6, &big_x7);

        // alpha_1 <- x1 + rho * v
        let alpha1 = ristretto255::scalar_mul(&rho, amount_val);
        ristretto255::scalar_add_assign(&mut alpha1, &x1);

        // alpha_2 <- x2 + rho * r
        let alpha2 = ristretto255::scalar_mul(&rho, amount_rand);
        ristretto255::scalar_add_assign(&mut alpha2, &x2);

        // alpha_3 <- x3 + rho * b
        let alpha3 = ristretto255::scalar_mul(&rho, new_balance_val);
        ristretto255::scalar_add_assign(&mut alpha3, &x3);

        // alpha_4 <- x4 + rho * r'
        let alpha4 = ristretto255::scalar_mul(&rho, new_balance_comm_rand);
        ristretto255::scalar_add_assign(&mut alpha4, &x4);

        // alpha5 <- x5 + rho * sk
        let alpha5 = ristretto255::scalar_mul(&rho, sender_sk);
        ristretto255::scalar_add_assign(&mut alpha5, &x5);

        TransferSubproof {
            x1: big_x1,
            x2: big_x2,
            x3: big_x3,
            x4: big_x4,
            x5: big_x5,
            x6: big_x6,
            x7: big_x7,
            alpha1,
            alpha2,
            alpha3,
            alpha4,
            alpha5,
        }
    }

    #[test_only]
    /// Given a $\Sigma$-protocol proof for veiled-to-unveiled transfers, serializes it into byte form.
    public fun serialize_withdrawal_subproof(proof: &WithdrawalSubproof): vector<u8> {
        // Reverse-iterates through the fields of the `WithdrawalSubproof` struct, serializes each field, and appends
        // it into a vector of bytes which is returned at the end.
        let x1_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(&proof.x1));
        let x2_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(&proof.x2));
        let x3_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(&proof.x3));
        let alpha1_bytes = ristretto255::scalar_to_bytes(&proof.alpha1);
        let alpha2_bytes = ristretto255::scalar_to_bytes(&proof.alpha2);
        let alpha3_bytes = ristretto255::scalar_to_bytes(&proof.alpha3);

        let bytes = vector::empty<u8>();
        bytes.append(alpha3_bytes);
        bytes.append(alpha2_bytes);
        bytes.append(alpha1_bytes);
        bytes.append(x3_bytes);
        bytes.append(x2_bytes);
        bytes.append(x1_bytes);

        bytes
    }

    #[test_only]
    /// Given a $\Sigma$-protocol proof, serializes it into byte form.
    public fun serialize_transfer_subproof(proof: &TransferSubproof): vector<u8> {
        // Reverse-iterates through the fields of the `TransferSubproof` struct, serializes each field, and appends
        // it into a vector of bytes which is returned at the end.
        let x1_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(&proof.x1));
        let x2_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(&proof.x2));
        let x3_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(&proof.x3));
        let x4_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(&proof.x4));
        let x5_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(&proof.x5));
        let x6_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(&proof.x6));
        let x7_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(&proof.x7));
        let alpha1_bytes = ristretto255::scalar_to_bytes(&proof.alpha1);
        let alpha2_bytes = ristretto255::scalar_to_bytes(&proof.alpha2);
        let alpha3_bytes = ristretto255::scalar_to_bytes(&proof.alpha3);
        let alpha4_bytes = ristretto255::scalar_to_bytes(&proof.alpha4);
        let alpha5_bytes = ristretto255::scalar_to_bytes(&proof.alpha5);

        let bytes = vector::empty<u8>();
        bytes.append(alpha5_bytes);
        bytes.append(alpha4_bytes);
        bytes.append(alpha3_bytes);
        bytes.append(alpha2_bytes);
        bytes.append(alpha1_bytes);
        bytes.append(x7_bytes);
        bytes.append(x6_bytes);
        bytes.append(x5_bytes);
        bytes.append(x4_bytes);
        bytes.append(x3_bytes);
        bytes.append(x2_bytes);
        bytes.append(x1_bytes);

        bytes
    }

    //
    // Sigma proof verification tests
    //

    #[test_only]
    fun verify_transfer_subproof_test(maul_proof: bool)
    {
        // Pick a keypair for the sender, and one for the recipient
        let (sender_sk, sender_pk) = generate_elgamal_keypair();
        let (_, recipient_pk) = generate_elgamal_keypair();

        // Set the transferred amount to 50
        let amount_val = ristretto255::new_scalar_from_u32(50);
        let amount_rand = ristretto255::random_scalar();

        // Encrypt the amount under the sender's PK
        let withdraw_ct = elgamal::new_ciphertext_with_basepoint(&amount_val, &amount_rand, &sender_pk);
        // Encrypt the amount under the recipient's PK
        let deposit_ct = elgamal::new_ciphertext_with_basepoint(&amount_val, &amount_rand, &recipient_pk);
        // Commit to the amount
        let comm_amount = pedersen::new_commitment_for_bulletproof(&amount_val, &amount_rand);

        // Set sender's new balance after the transaction to 100
        let curr_balance_val = ristretto255::new_scalar_from_u32(150);
        let new_balance_val = ristretto255::new_scalar_from_u32(100);
        let new_balance_rand = ristretto255::random_scalar();
        let curr_balance_ct = elgamal::new_ciphertext_with_basepoint(&curr_balance_val, &new_balance_rand, &sender_pk);

        let new_balance_comm = pedersen::new_commitment_for_bulletproof(&new_balance_val, &new_balance_rand);

        let sigma_proof = prove_transfer(
            &sender_pk,
            &sender_sk,
            &recipient_pk,
            &withdraw_ct,           // withdrawn amount, encrypted under sender PK
            &deposit_ct,            // deposited amount, encrypted under recipient PK (same plaintext as `withdraw_ct`)
            &comm_amount,            // commitment to transfer amount to prevent range proof forgery
            &curr_balance_ct,    // sender's balance before the transaction goes through, encrypted under sender PK
            &new_balance_comm,  // commitment to sender's balance to prevent range proof forgery
            &amount_rand,           // encryption randomness for `withdraw_ct` and `deposit_ct`
            &amount_val,            // transferred amount
            &new_balance_rand,  // encryption randomness for updated balance ciphertext
            &new_balance_val,   // sender's balance after the transfer
        );

        if (maul_proof) {
            // This should fail the proof verification below
            let random_point = ristretto255::random_point();
            sigma_proof.x1 = random_point;
        };

        verify_transfer_subproof(
            &sender_pk,
            &recipient_pk,
            &withdraw_ct,
            &deposit_ct,
            &comm_amount,
            &new_balance_comm,
            &curr_balance_ct,
            &sigma_proof
        );
    }

    #[test]
    fun verify_transfer_subproof_succeeds_test() {
        verify_transfer_subproof_test(false);
    }

    #[test]
    #[expected_failure(abort_code = 0x10001, location = Self)]
    fun verify_transfer_subproof_fails_test()
    {
        verify_transfer_subproof_test(true);
    }

    #[test_only]
    fun verify_withdrawal_subproof_test(maul_proof: bool)
    {
        // Pick a keypair for the sender
        let (sender_sk, sender_pk) = generate_elgamal_keypair();

        // Set the transferred amount to 50
        let curr_balance = ristretto255::new_scalar_from_u32(100);
        let new_balance = ristretto255::new_scalar_from_u32(75);
        let amount_withdrawn = ristretto255::new_scalar_from_u32(25);
        let rand = ristretto255::random_scalar();

        // Encrypt the amount under the sender's PK
        let curr_balance_ct = elgamal::new_ciphertext_with_basepoint(&curr_balance, &rand, &sender_pk);
        // Commit to the amount
        let new_balance_comm = pedersen::new_commitment_for_bulletproof(&new_balance, &rand);

        let sigma_proof = prove_withdrawal(
            &sender_sk,
            &sender_pk,
            &curr_balance_ct,
            &new_balance_comm,
            &new_balance,
            &amount_withdrawn,
            &rand,
        );

        if (maul_proof) {
            // This should fail the proof verification below
            let random_point = ristretto255::random_point();
            sigma_proof.x1 = random_point;
        };

        verify_withdrawal_subproof(
            &sender_pk,
            &curr_balance_ct,
            &new_balance_comm,
            &amount_withdrawn,
            &sigma_proof
        );
    }

    #[test]
    fun verify_withdrawal_subproof_succeeds_test() {
        verify_withdrawal_subproof_test(false);
    }

    #[test]
    #[expected_failure(abort_code = 0x10001, location = Self)]
    fun verify_withdrawal_subproof_fails_test() {
        verify_withdrawal_subproof_test(true);
    }

    //
    // Sigma proof deserialization tests
    //

    #[test]
    fun serialize_transfer_subproof_test()
    {
        let (sender_sk, sender_pk) = generate_elgamal_keypair();
        let amount_val = ristretto255::new_scalar_from_u32(50);
        let (_, recipient_pk) = generate_elgamal_keypair();
        let amount_rand = ristretto255::random_scalar();
        let withdraw_ct = elgamal::new_ciphertext_with_basepoint(&amount_val, &amount_rand, &sender_pk);
        let deposit_ct = elgamal::new_ciphertext_with_basepoint(&amount_val, &amount_rand, &recipient_pk);
        let comm_amount = pedersen::new_commitment_for_bulletproof(&amount_val, &amount_rand);
        let curr_balance_val = ristretto255::new_scalar_from_u32(150);
        let new_balance_val = ristretto255::new_scalar_from_u32(100);
        let new_balance_rand = ristretto255::random_scalar();
        let curr_balance_ct = elgamal::new_ciphertext_with_basepoint(&curr_balance_val, &new_balance_rand, &sender_pk);
        let new_balance_comm = pedersen::new_commitment_for_bulletproof(&new_balance_val, &new_balance_rand);

        let sigma_proof = prove_transfer(
            &sender_pk,
            &sender_sk,
            &recipient_pk,
            &withdraw_ct,
            &deposit_ct,
            &comm_amount,
            &curr_balance_ct,
            &new_balance_comm,
            &amount_rand,
            &amount_val,
            &new_balance_rand,
            &new_balance_val);

        let sigma_proof_bytes = serialize_transfer_subproof(&sigma_proof);

        let deserialized_proof = deserialize_transfer_subproof(sigma_proof_bytes).extract();

        assert!(ristretto255::point_equals(&sigma_proof.x1, &deserialized_proof.x1), 1);
        assert!(ristretto255::point_equals(&sigma_proof.x2, &deserialized_proof.x2), 1);
        assert!(ristretto255::point_equals(&sigma_proof.x3, &deserialized_proof.x3), 1);
        assert!(ristretto255::point_equals(&sigma_proof.x4, &deserialized_proof.x4), 1);
        assert!(ristretto255::point_equals(&sigma_proof.x5, &deserialized_proof.x5), 1);
        assert!(ristretto255::point_equals(&sigma_proof.x6, &deserialized_proof.x6), 1);
        assert!(ristretto255::scalar_equals(&sigma_proof.alpha1, &deserialized_proof.alpha1), 1);
        assert!(ristretto255::scalar_equals(&sigma_proof.alpha2, &deserialized_proof.alpha2), 1);
        assert!(ristretto255::scalar_equals(&sigma_proof.alpha3, &deserialized_proof.alpha3), 1);
        assert!(ristretto255::scalar_equals(&sigma_proof.alpha4, &deserialized_proof.alpha4), 1);
        assert!(ristretto255::scalar_equals(&sigma_proof.alpha5, &deserialized_proof.alpha5), 1);
    }
}
