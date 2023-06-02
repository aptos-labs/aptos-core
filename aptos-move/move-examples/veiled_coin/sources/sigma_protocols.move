/// Package for creating, verifying, serializing & deserializing the $\Sigma$-protocol proofs used in veiled coins.
///
/// TODO: add remaining tests for `ElGamalToPedSigmaProof`
module veiled_coin::sigma_protocols {
    use std::error;
    use std::option::Option;
    use std::vector;

    use aptos_std::elgamal;
    use aptos_std::pedersen;
    use aptos_std::ristretto255::{Self, RistrettoPoint, Scalar};

    use veiled_coin::helpers::cut_vector;

    #[test_only]
    use aptos_framework::coin;
    #[test_only]
    use veiled_coin::helpers::generate_elgamal_keypair;

    //
    // Errors
    //

    /// $\Sigma$-protocol proof for withdrawals did not verify.
    const ESIGMA_PROTOCOL_VERIFY_FAILED: u64 = 1;

    //
    // Constants
    //

    /// The domain separation tag (DST) used in the Fiat-Shamir transform of our $\Sigma$-protocol.
    const FIAT_SHAMIR_SIGMA_DST : vector<u8> = b"AptosVeiledCoin/WithdrawalProofFiatShamir";

    //
    // Structs
    //

    /// A $\Sigma$-protocol proof used as part of a `UnveiledWithdrawalProof`.
    /// (A more detailed description can be found in `unveil_sigma_protocol_verify`.)
    struct ElGamalToPedSigmaProof<phantom CoinType> has drop {
        x1: RistrettoPoint,
        x2: RistrettoPoint,
        x3: RistrettoPoint,
        alpha1: Scalar,
        alpha2: Scalar,
    }

    /// A $\Sigma$-protocol proof used as part of a `VeiledTransferProof`.
    /// This proof encompasses the $\Sigma$-protocol from `ElGamalToPedSigmaProof`.
    /// (A more detailed description can be found in `verify_withdrawal_sigma_protocol`.)
    struct FullSigmaProof<phantom CoinType> has drop {
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
    }

    //
    // Public proof verification functions
    //

    /// Verifies a $\Sigma$-protocol proof necessary to ensure correctness of a veiled transfer.
    /// Specifically, this proof proves that ElGamal ciphertexts `withdraw_ct` and `deposit_ct` encrypt the same amount $v$ using the same
    /// randomness $r$, with `sender_pk` and `recipient_pk` respectively. In addition, it proves that ElGamal ciphertext
    /// `sender_updated_balance_ct` and Pedersen commitment `sender_updated_balance_comm` encode the same value $b$
    /// with the same randomness $r$, where the former uses `sender_pk`. It also proves that Pedersen commitment
    /// `transfer_value` encodes the same value $v$ as `withdraw_ct` and `deposit_ct`, with the same randomness `r`.
    /// These Pedersen commitments are needed to ensure that the range proofs done elsewhere on the left part of the
    /// ElGamal ciphertexts cannot be forged by a user with their secret key, by providing binding for $v$ and $b$.
    ///
    /// # Cryptographic details
    ///
    /// The proof argues knowledge of a witness $w$ such that a specific relation $R(x; w)$ is satisfied, for a public
    /// statement $x$ known to the verifier (i.e., known to the validators). We describe this relation below.
    ///
    /// The secret witness $w$ in this relation, known only to the sender of the TXN, consists of:
    ///  - $v$, the amount being transferred
    ///  - $r$, the ElGamal encryption randomness used to encrypt $v$
    ///  - $b$, the sender's new balance after the transfer occurs
    ///  - $r_b$, the ElGamal encryption randomness used to encrypt $b$
    ///
    /// (Note that the $\Sigma$-protocol's zero-knowledge property ensures the witness is not revealed.)
    ///
    /// The public statement $x$ in this relation consists of:
    ///  - $G$, the basepoint of a given elliptic curve
    ///  = $H$, the basepoint used for randomness in the Pedersen commitments
    ///  - $Y$, the sender's PK
    ///  - $Y'$, the recipient's PK
    ///  - $(C, D)$, the ElGamal encryption of $v$ using randomness $r$ under the sender's PK
    ///  - $(C', D)$, the ElGamal encryption of $v$ using randomness $r$ under the recipient's PK
    ///  - $c$, the Pedersen commitment to $v$ using randomness $r$
    ///  - $(c1, c2)$, the ElGamal encryption of $b$ using randomness $r_b$
    ///  - $c'$, the Pedersen commitment to $b$ using randomness $r_b$
    ///
    ///
    /// ```
    /// R(
    ///     x = [ Y, Y', (C, C', D), c, (c1, c2), c', G, H]
    ///     w = [ v, r, b, r_b ]
    /// ) = {
    ///     C = v * G + r * Y
    ///     C' = v * G + r * Y'
    ///     D = r * G
    ///     c1 = b * G + r_b * Y
    ///     c2 = r_b * G
    ///     c = b * G + r_b * H
    ///     c' = v * G + r * H
    /// }
    /// ```
    ///
    /// A relation similar to this is also described on page 14 of the Zether paper [BAZB20] (just replace  $G$ -> $g$,
    /// $C'$ -> $\bar{C}$, $Y$ -> $y$, $Y'$ -> $\bar{y}$, $v$ -> $b^*$). Note their relation does not include the
    /// Pedersen commitments as they guarantee the binding property by integrating their Bulletproofs range proofs
    /// into their $\Sigma$ protocol.
    ///
    /// Note also that the equations $C_L - C = b' G + sk (C_R - D)$ and $Y = sk G$ in the Zether paper are enforced
    /// programmatically by this smart contract and so are not needed in our $\Sigma$-protocol.
    public fun full_sigma_protocol_verify<CoinType>(
        sender_pk: &elgamal::CompressedPubkey,
        recipient_pk: &elgamal::CompressedPubkey,
        withdraw_ct: &elgamal::Ciphertext,
        deposit_ct: &elgamal::Ciphertext,
        sender_updated_balance_ct: &elgamal::Ciphertext,
        sender_updated_balance_comm: &pedersen::Commitment,
        transfer_value: &pedersen::Commitment,
        proof: &FullSigmaProof<CoinType>)
    {
        let sender_pk_point = elgamal::pubkey_to_point(sender_pk);
        let recipient_pk_point = elgamal::pubkey_to_point(recipient_pk);
        let (big_c, d) = elgamal::ciphertext_as_points(withdraw_ct);
        let (big_bar_c, _) = elgamal::ciphertext_as_points(deposit_ct);
        let (c1, c2) = elgamal::ciphertext_as_points(sender_updated_balance_ct);
        let c = pedersen::commitment_as_point(sender_updated_balance_comm);
        let bar_c = pedersen::commitment_as_point(transfer_value);
        let h = pedersen::randomness_base_for_bulletproof();

        // TODO: Can be optimized so we don't re-serialize the proof for Fiat-Shamir
        let rho = full_sigma_protocol_fiat_shamir<CoinType>(
            sender_pk, recipient_pk,
            withdraw_ct, deposit_ct,
            sender_updated_balance_ct,
            sender_updated_balance_comm, transfer_value,
            &proof.x1, &proof.x2, &proof.x3, &proof.x4,
            &proof.x5, &proof.x6, &proof.x7);

        let g_alpha2 = ristretto255::basepoint_mul(&proof.alpha2);
        // \rho * D + X1 =? \alpha_2 * g
        let d_acc = ristretto255::point_mul(d, &rho);
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
        let big_bar_c_acc = ristretto255::point_mul(big_bar_c, &rho);
        ristretto255::point_add_assign(&mut big_bar_c_acc, &proof.x3);
        let y_bar_alpha2 = ristretto255::point_mul(&recipient_pk_point, &proof.alpha2);
        ristretto255::point_add_assign(&mut y_bar_alpha2, &g_alpha1);
        assert!(ristretto255::point_equals(&big_bar_c_acc, &y_bar_alpha2), error::invalid_argument(ESIGMA_PROTOCOL_VERIFY_FAILED));

        let g_alpha3 = ristretto255::basepoint_mul(&proof.alpha3);
        // \rho * c_1 + X4 =? \alpha_3 * g + \alpha_4 * y
        let c1_acc = ristretto255::point_mul(c1, &rho);
        ristretto255::point_add_assign(&mut c1_acc, &proof.x4);
        let y_alpha4 = ristretto255::point_mul(&sender_pk_point, &proof.alpha4);
        ristretto255::point_add_assign(&mut y_alpha4, &g_alpha3);
        assert!(ristretto255::point_equals(&c1_acc, &y_alpha4), error::invalid_argument(ESIGMA_PROTOCOL_VERIFY_FAILED));

        let g_alpha4 = ristretto255::basepoint_mul(&proof.alpha4);
        // \rho * c_2 + X5 =? \alpha_4 * g
        let c2_acc = ristretto255::point_mul(c2, &rho);
        ristretto255::point_add_assign(&mut c2_acc, &proof.x5);
        assert!(ristretto255::point_equals(&c2_acc, &g_alpha4), error::invalid_argument(ESIGMA_PROTOCOL_VERIFY_FAILED));

        // \rho * c + X6 =? \alpha_3 * g + \alpha_4 * h
        let c_acc = ristretto255::point_mul(c, &rho);
        ristretto255::point_add_assign(&mut c_acc, &proof.x6);
        let h_alpha4 = ristretto255::point_mul(&h, &proof.alpha4);
        ristretto255::point_add_assign(&mut h_alpha4, &g_alpha3);
        assert!(ristretto255::point_equals(&c_acc, &h_alpha4), error::invalid_argument(ESIGMA_PROTOCOL_VERIFY_FAILED));

        // \rho * \bar{c} + X7 =? \alpha_1 * g + \alpha_2 * h
        let bar_c_acc = ristretto255::point_mul(bar_c, &rho);
        ristretto255::point_add_assign(&mut bar_c_acc, &proof.x7);
        let h_alpha2 = ristretto255::point_mul(&h, &proof.alpha2);
        ristretto255::point_add_assign(&mut h_alpha2, &g_alpha1);
        assert!(ristretto255::point_equals(&bar_c_acc, &h_alpha2), error::invalid_argument(ESIGMA_PROTOCOL_VERIFY_FAILED));
    }

    /// Verifies a $\Sigma$-protocol proof necessary to ensure correctness of a veiled-to-unveiled transfer.
    /// Specifically, this proof proves that `sender_updated_balance_ct` and `sender_updated_balance_comm` encode the same amount $v$ using the same
    /// randomness $r$, with `sender_pk` being used in `sender_updated_balance_ct`. This is necessary to prevent
    /// the forgery of range proofs, as computing a range proof over the left half of an ElGamal ciphertext allows
    /// a user with their secret key to create range proofs over false values.
    ///
    /// # Cryptographic details
    ///
    /// The proof argues knowledge of a witness $w$ such that a specific relation $R(x; w)$ is satisfied, for a public
    /// statement $x$ known to the verifier (i.e., known to the validators). We describe this relation below.
    ///
    /// The secret witness $w$ in this relation, known only to the sender of the TXN, consists of:
    ///  - $b$, the new veiled balance of the sender after their transaction goes through
    ///  - $r$, ElGamal encryption randomness of the sender's new balance
    ///
    /// (Note that the $\Sigma$-protocol's zero-knowledge property ensures the witness is not revealed.)
    ///
    /// The public statement $x$ in this relation consists of:
    ///  - $G$, the basepoint of a given elliptic curve
    ///  - $Y$, the sender's PK
    ///  - $(c1, c2)$, the ElGamal ecnryption of the sender's updated balance $b$ with updated randomness $r$ after their transaction is sent
    ///  - $c$, the Pedersen commitment to $b$ with randomness $r$, using fixed randomness base $H$
    ///
    /// The previse relation being proved is as follows:
    ///
    /// ```
    /// R(
    ///     x = [ Y, (c1, c2), c, G, H]
    ///     w = [ b, r ]
    /// ) = {
    ///     c1 = r * G
    ///     c2 = b * g + r * y
    ///     c = b * g + r * h
    /// }
    /// ```
    public fun unveil_sigma_protocol_verify<CoinType>(
        sender_pk: &elgamal::CompressedPubkey,
        sender_updated_balance_ct: &elgamal::Ciphertext,
        sender_updated_balance_comm: &pedersen::Commitment,
        proof: &ElGamalToPedSigmaProof<CoinType>)
    {
        let (c1, c2) = elgamal::ciphertext_as_points(sender_updated_balance_ct);
        let c = pedersen::commitment_as_point(sender_updated_balance_comm);
        let h = pedersen::randomness_base_for_bulletproof();
        let sender_pk_point = elgamal::pubkey_to_point(sender_pk);

        let rho = unveil_sigma_protocol_fiat_shamir<CoinType>(
            sender_pk,
            sender_updated_balance_ct,
            sender_updated_balance_comm,
            &proof.x1,
            &proof.x2,
            &proof.x3);

        let g_alpha1 = ristretto255::basepoint_mul(&proof.alpha1);
        // \rho * c_1 + X_1 =? \alpha_1 * g
        let c1_acc = ristretto255::point_mul(c1, &rho);
        ristretto255::point_add_assign(&mut c1_acc, &proof.x1);
        assert!(ristretto255::point_equals(&c1_acc, &g_alpha1), error::invalid_argument(ESIGMA_PROTOCOL_VERIFY_FAILED));

        let g_alpha2 = ristretto255::basepoint_mul(&proof.alpha2);
        // \rho * c_2 + X_2 =? \alpha_2 * g + \alpha_1 * y
        let c2_acc = ristretto255::point_mul(c2, &rho);
        ristretto255::point_add_assign(&mut c2_acc, &proof.x2);
        let y_alpha1 = ristretto255::point_mul(&sender_pk_point, &proof.alpha1);
        ristretto255::point_add_assign(&mut y_alpha1, &g_alpha2);
        assert!(ristretto255::point_equals(&c2_acc, &y_alpha1), error::invalid_argument(ESIGMA_PROTOCOL_VERIFY_FAILED));

        // \rho * c + X_3 =? \alpha_2 * g + \alpha_1 * h
        let c_acc = ristretto255::point_mul(c, &rho);
        ristretto255::point_add_assign(&mut c_acc, &proof.x3);
        let h_alpha1 = ristretto255::point_mul(&h, &proof.alpha1);
        ristretto255::point_add_assign(&mut h_alpha1, &g_alpha2);
        assert!(ristretto255::point_equals(&c_acc, &h_alpha1), error::invalid_argument(ESIGMA_PROTOCOL_VERIFY_FAILED));
    }

    //
    // Public deserialization functions
    //

    /// Deserializes and returns an `ElGamalToPedSigmaProof` given its byte representation (see protocol description in
    /// `unveil_sigma_protocol_verify`)
    ///
    /// Elements at the end of the `ElGamalToPedSigmaProof` struct are expected to be at the start of the byte vector, and
    /// serialized using the serialization formats in the `ristretto255` module.
    public fun deserialize_unveil_sigma_proof<CoinType>(proof_bytes: vector<u8>): Option<ElGamalToPedSigmaProof<CoinType>> {
        if (vector::length<u8>(&proof_bytes) != 160) {
            return std::option::none<ElGamalToPedSigmaProof<CoinType>>()
        };

        let x1_bytes = cut_vector<u8>(&mut proof_bytes, 32);
        let x1 = ristretto255::new_point_from_bytes(x1_bytes);
        if (!std::option::is_some<RistrettoPoint>(&x1)) {
            return std::option::none<ElGamalToPedSigmaProof<CoinType>>()
        };
        let x1 = std::option::extract<RistrettoPoint>(&mut x1);

        let x2_bytes = cut_vector<u8>(&mut proof_bytes, 32);
        let x2 = ristretto255::new_point_from_bytes(x2_bytes);
        if (!std::option::is_some<RistrettoPoint>(&x2)) {
            return std::option::none<ElGamalToPedSigmaProof<CoinType>>()
        };
        let x2 = std::option::extract<RistrettoPoint>(&mut x2);

        let x3_bytes = cut_vector<u8>(&mut proof_bytes, 32);
        let x3 = ristretto255::new_point_from_bytes(x3_bytes);
        if (!std::option::is_some<RistrettoPoint>(&x3)) {
            return std::option::none<ElGamalToPedSigmaProof<CoinType>>()
        };
        let x3 = std::option::extract<RistrettoPoint>(&mut x3);

        let alpha1_bytes = cut_vector<u8>(&mut proof_bytes, 32);
        let alpha1 = ristretto255::new_scalar_from_bytes(alpha1_bytes);
        if (!std::option::is_some(&alpha1)) {
            return std::option::none<ElGamalToPedSigmaProof<CoinType>>()
        };
        let alpha1 = std::option::extract(&mut alpha1);

        let alpha2_bytes = cut_vector<u8>(&mut proof_bytes, 32);
        let alpha2 = ristretto255::new_scalar_from_bytes(alpha2_bytes);
        if (!std::option::is_some(&alpha2)) {
            return std::option::none<ElGamalToPedSigmaProof<CoinType>>()
        };
        let alpha2 = std::option::extract(&mut alpha2);

        std::option::some(ElGamalToPedSigmaProof {
            x1, x2, x3, alpha1, alpha2
        })
    }

    /// Deserializes and returns a `SigmaProof` given its byte representation (see protocol description in
    /// `sigma_protocol_verify`)
    ///
    /// Elements at the end of the `SigmaProof` struct are expected to be at the start  of the byte vector, and
    /// serialized using the serialization formats in the `ristretto255` module.
    public fun deserialize_full_sigma_proof<CoinType>(proof_bytes: vector<u8>): Option<FullSigmaProof<CoinType>> {
        if (vector::length<u8>(&proof_bytes) != 352) {
            return std::option::none<FullSigmaProof<CoinType>>()
        };

        let x1_bytes = cut_vector<u8>(&mut proof_bytes, 32);
        let x1 = ristretto255::new_point_from_bytes(x1_bytes);
        if (!std::option::is_some<RistrettoPoint>(&x1)) {
            return std::option::none<FullSigmaProof<CoinType>>()
        };
        let x1 = std::option::extract<RistrettoPoint>(&mut x1);

        let x2_bytes = cut_vector<u8>(&mut proof_bytes, 32);
        let x2 = ristretto255::new_point_from_bytes(x2_bytes);
        if (!std::option::is_some<RistrettoPoint>(&x2)) {
            return std::option::none<FullSigmaProof<CoinType>>()
        };
        let x2 = std::option::extract<RistrettoPoint>(&mut x2);

        let x3_bytes = cut_vector<u8>(&mut proof_bytes, 32);
        let x3 = ristretto255::new_point_from_bytes(x3_bytes);
        if (!std::option::is_some<RistrettoPoint>(&x3)) {
            return std::option::none<FullSigmaProof<CoinType>>()
        };
        let x3 = std::option::extract<RistrettoPoint>(&mut x3);

        let x4_bytes = cut_vector<u8>(&mut proof_bytes, 32);
        let x4 = ristretto255::new_point_from_bytes(x4_bytes);
        if (!std::option::is_some<RistrettoPoint>(&x4)) {
            return std::option::none<FullSigmaProof<CoinType>>()
        };
        let x4 = std::option::extract<RistrettoPoint>(&mut x4);

        let x5_bytes = cut_vector<u8>(&mut proof_bytes, 32);
        let x5 = ristretto255::new_point_from_bytes(x5_bytes);
        if (!std::option::is_some<RistrettoPoint>(&x5)) {
            return std::option::none<FullSigmaProof<CoinType>>()
        };
        let x5 = std::option::extract<RistrettoPoint>(&mut x5);

        let x6_bytes = cut_vector<u8>(&mut proof_bytes, 32);
        let x6 = ristretto255::new_point_from_bytes(x6_bytes);
        if (!std::option::is_some<RistrettoPoint>(&x6)) {
            return std::option::none<FullSigmaProof<CoinType>>()
        };
        let x6 = std::option::extract<RistrettoPoint>(&mut x6);

        let x7_bytes = cut_vector<u8>(&mut proof_bytes, 32);
        let x7 = ristretto255::new_point_from_bytes(x7_bytes);
        if (!std::option::is_some<RistrettoPoint>(&x7)) {
            return std::option::none<FullSigmaProof<CoinType>>()
        };
        let x7 = std::option::extract<RistrettoPoint>(&mut x7);

        let alpha1_bytes = cut_vector<u8>(&mut proof_bytes, 32);
        let alpha1 = ristretto255::new_scalar_from_bytes(alpha1_bytes);
        if (!std::option::is_some(&alpha1)) {
            return std::option::none<FullSigmaProof<CoinType>>()
        };
        let alpha1 = std::option::extract(&mut alpha1);

        let alpha2_bytes = cut_vector<u8>(&mut proof_bytes, 32);
        let alpha2 = ristretto255::new_scalar_from_bytes(alpha2_bytes);
        if (!std::option::is_some(&alpha2)) {
            return std::option::none<FullSigmaProof<CoinType>>()
        };
        let alpha2 = std::option::extract(&mut alpha2);

        let alpha3_bytes = cut_vector<u8>(&mut proof_bytes, 32);
        let alpha3 = ristretto255::new_scalar_from_bytes(alpha3_bytes);
        if (!std::option::is_some(&alpha3)) {
            return std::option::none<FullSigmaProof<CoinType>>()
        };
        let alpha3 = std::option::extract(&mut alpha3);

        let alpha4_bytes = cut_vector<u8>(&mut proof_bytes, 32);
        let alpha4 = ristretto255::new_scalar_from_bytes(alpha4_bytes);
        if (!std::option::is_some(&alpha4)) {
            return std::option::none<FullSigmaProof<CoinType>>()
        };
        let alpha4 = std::option::extract(&mut alpha4);

        std::option::some(FullSigmaProof {
            x1, x2, x3, x4, x5, x6, x7, alpha1, alpha2, alpha3, alpha4
        })
    }

    //
    // Private functions for Fiat-Shamir challenge derivation
    //


    /// Computes the challenge value as `rho = H(g, h, y, c_1, c_2, c, X_1, X_2, X_3)
    /// for the $\Sigma$-protocol from `verify_withdrawal_sigma_protocol` using the Fiat-Shamir transform. The notation
    /// used above is from the Zether [BAZB20] paper.
    fun unveil_sigma_protocol_fiat_shamir<CoinType>(
        sender_pk: &elgamal::CompressedPubkey,
        sender_updated_balance_ct: &elgamal::Ciphertext,
        sender_updated_balance_comm: &pedersen::Commitment,
        x1: &RistrettoPoint,
        x2: &RistrettoPoint,
        x3: &RistrettoPoint): Scalar
    {
        let (c1, c2) = elgamal::ciphertext_as_points(sender_updated_balance_ct);
        let c = pedersen::commitment_as_point(sender_updated_balance_comm);

        let hash_input = vector::empty<u8>();

        let basepoint_bytes = ristretto255::point_to_bytes(&ristretto255::basepoint_compressed());
        vector::append<u8>(&mut hash_input, basepoint_bytes);

        let h_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(&pedersen::randomness_base_for_bulletproof()));
        vector::append<u8>(&mut hash_input, h_bytes);

        let y = elgamal::pubkey_to_compressed_point(sender_pk);
        let y_bytes = ristretto255::point_to_bytes(&y);
        vector::append<u8>(&mut hash_input, y_bytes);

        let c1_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(c1));
        vector::append<u8>(&mut hash_input, c1_bytes);

        let c2_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(c2));
        vector::append<u8>(&mut hash_input, c2_bytes);

        let c_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(c));
        vector::append<u8>(&mut hash_input, c_bytes);

        let x_1_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(x1));
        vector::append<u8>(&mut hash_input, x_1_bytes);

        let x_2_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(x2));
        vector::append<u8>(&mut hash_input, x_2_bytes);

        let x_3_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(x3));
        vector::append<u8>(&mut hash_input, x_3_bytes);

        vector::append<u8>(&mut hash_input, FIAT_SHAMIR_SIGMA_DST);

        ristretto255::new_scalar_from_sha2_512(hash_input)
    }


    /// TODO: explain the challenge derivation as a function of the parameters
    /// Computes the challenge value as `rho = H(g, y, \bar{y}, h, C, D, \bar{C}, c_1, c_2, c, \bar{c}, {X_i}_{i=1}^7)`
    /// for the $\Sigma$-protocol from `verify_withdrawal_sigma_protocol` using the Fiat-Shamir transform. The notation
    /// used above is from the Zether [BAZB20] paper.
    fun full_sigma_protocol_fiat_shamir<CoinType>(
        sender_pk: &elgamal::CompressedPubkey,
        recipient_pk: &elgamal::CompressedPubkey,
        withdraw_ct: &elgamal::Ciphertext,
        deposit_ct: &elgamal::Ciphertext,
        sender_updated_balance: &elgamal::Ciphertext,
        balance: &pedersen::Commitment,
        transfer_value: &pedersen::Commitment,
        x1: &RistrettoPoint,
        x2: &RistrettoPoint,
        x3: &RistrettoPoint,
        x4: &RistrettoPoint,
        x5: &RistrettoPoint,
        x6: &RistrettoPoint,
        x7: &RistrettoPoint): Scalar
    {
        let (big_c, d) = elgamal::ciphertext_as_points(withdraw_ct);
        let (big_bar_c, _) = elgamal::ciphertext_as_points(deposit_ct);
        let (c1, c2) = elgamal::ciphertext_as_points(sender_updated_balance);
        let c = pedersen::commitment_as_point(balance);
        let bar_c = pedersen::commitment_as_point(transfer_value);

        let hash_input = vector::empty<u8>();

        let basepoint_bytes = ristretto255::point_to_bytes(&ristretto255::basepoint_compressed());
        vector::append<u8>(&mut hash_input, basepoint_bytes);

        let h_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(&pedersen::randomness_base_for_bulletproof()));
        vector::append<u8>(&mut hash_input, h_bytes);

        let y = elgamal::pubkey_to_compressed_point(sender_pk);
        let y_bytes = ristretto255::point_to_bytes(&y);
        vector::append<u8>(&mut hash_input, y_bytes);

        let y_bar = elgamal::pubkey_to_compressed_point(recipient_pk);
        let y_bar_bytes = ristretto255::point_to_bytes(&y_bar);
        vector::append<u8>(&mut hash_input, y_bar_bytes);

        let big_c_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(big_c));
        vector::append<u8>(&mut hash_input, big_c_bytes);

        let d_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(d));
        vector::append<u8>(&mut hash_input, d_bytes);

        let bar_c_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(bar_c));
        vector::append<u8>(&mut hash_input, bar_c_bytes);

        let c1_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(c1));
        vector::append<u8>(&mut hash_input, c1_bytes);

        let c2_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(c2));
        vector::append<u8>(&mut hash_input, c2_bytes);

        let c_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(c));
        vector::append<u8>(&mut hash_input, c_bytes);

        let big_bar_c_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(big_bar_c));
        vector::append<u8>(&mut hash_input, big_bar_c_bytes);

        let x_1_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(x1));
        vector::append<u8>(&mut hash_input, x_1_bytes);

        let x_2_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(x2));
        vector::append<u8>(&mut hash_input, x_2_bytes);

        let x_3_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(x3));
        vector::append<u8>(&mut hash_input, x_3_bytes);

        let x_4_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(x4));
        vector::append<u8>(&mut hash_input, x_4_bytes);

        let x_5_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(x5));
        vector::append<u8>(&mut hash_input, x_5_bytes);

        let x_6_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(x6));
        vector::append<u8>(&mut hash_input, x_6_bytes);

        let x_7_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(x7));
        vector::append<u8>(&mut hash_input, x_7_bytes);

        vector::append<u8>(&mut hash_input, FIAT_SHAMIR_SIGMA_DST);

        ristretto255::new_scalar_from_sha2_512(hash_input)
    }

    //
    // Test-only serialization & proving functions
    //

    #[test_only]
    /// Proves the $\Sigma$-protocol used for veiled-to-unveiled coin transfers.
    /// See `unveil_sigma_protocol_verify` for a detailed description of the $\Sigma$-protocol
    public fun unveil_sigma_protocol_prove<CoinType>(
        sender_pk: &elgamal::CompressedPubkey,
        sender_updated_balance_ct: &elgamal::Ciphertext,
        sender_updated_balance_comm: &pedersen::Commitment,
        updated_balance_rand: &Scalar,
        updated_balance_val: &Scalar): ElGamalToPedSigmaProof<CoinType>
    {
        let x1 = ristretto255::random_scalar();
        let x2 = ristretto255::random_scalar();
        let source_pk_point = elgamal::pubkey_to_point(sender_pk);
        let h = pedersen::randomness_base_for_bulletproof();

        // X1 <- x1 * g
        let big_x1 = ristretto255::basepoint_mul(&x1);

        let g_x2 = ristretto255::basepoint_mul(&x2);
        // X2 <- x2 * g + x1 * y
        let big_x2 = ristretto255::point_mul(&source_pk_point, &x1);
        ristretto255::point_add_assign(&mut big_x2, &g_x2);

        // X3 <- x2 * g + x1 * h
        let big_x3 = ristretto255::point_mul(&h, &x1);
        ristretto255::point_add_assign(&mut big_x3, &g_x2);

        let rho = unveil_sigma_protocol_fiat_shamir<CoinType>(
            sender_pk,
            sender_updated_balance_ct,
            sender_updated_balance_comm,
            &big_x1,
            &big_x2,
            &big_x3);

        // alpha_1 <- x1 + rho * r
        let alpha1 = ristretto255::scalar_mul(&rho, updated_balance_rand);
        ristretto255::scalar_add_assign(&mut alpha1, &x1);

        // alpha2 <- x2 + rho * b
        let alpha2 = ristretto255::scalar_mul(&rho, updated_balance_val);
        ristretto255::scalar_add_assign(&mut alpha2, &x2);

        ElGamalToPedSigmaProof {
            x1: big_x1,
            x2: big_x2,
            x3: big_x3,
            alpha1,
            alpha2,
        }
    }

    #[test_only]
    /// Proves the $\Sigma$-protocol used for veiled coin transfers.
    /// See `full_sigma_protocol_verify` for a detailed description of the $\Sigma$-protocol
    public fun full_sigma_protocol_prove<CoinType>(
        sender_pk: &elgamal::CompressedPubkey,
        recipient_pk: &elgamal::CompressedPubkey,
        withdraw_ct: &elgamal::Ciphertext,
        deposit_ct: &elgamal::Ciphertext,
        sender_updated_balance: &elgamal::Ciphertext,
        balance: &pedersen::Commitment,
        transfer_value: &pedersen::Commitment,
        amount_rand: &Scalar,
        amount_val: &Scalar,
        updated_balance_rand: &Scalar,
        updated_balance_val: &Scalar): FullSigmaProof<CoinType>
    {
        let x1 = ristretto255::random_scalar();
        let x2 = ristretto255::random_scalar();
        let x3 = ristretto255::random_scalar();
        let x4 = ristretto255::random_scalar();
        let source_pk_point = elgamal::pubkey_to_point(sender_pk);
        let recipient_pk_point = elgamal::pubkey_to_point(recipient_pk);
        let h = pedersen::randomness_base_for_bulletproof();

        // X1 <- x2 * g
        let big_x1 = ristretto255::basepoint_mul(&x2);

        // X2 <- x1 * g + x2 * y
        let big_x2 = ristretto255::basepoint_mul(&x1);
        let source_pk_x2 = ristretto255::point_mul(&source_pk_point, &x2);
        ristretto255::point_add_assign(&mut big_x2, &source_pk_x2);

        // X3 <- x1 * g + x2 * \bar{y}
        let big_x3 = ristretto255::basepoint_mul(&x1);
        let recipient_pk_x2 = ristretto255::point_mul(&recipient_pk_point, &x2);
        ristretto255::point_add_assign(&mut big_x3, &recipient_pk_x2);

        // X4 <- x3 * g + x4 * y
        let big_x4 = ristretto255::basepoint_mul(&x3);
        let source_pk_x4 = ristretto255::point_mul(&source_pk_point, &x4);
        ristretto255::point_add_assign(&mut big_x4, &source_pk_x4);

        // X5 <- x4 * g
        let big_x5 = ristretto255::basepoint_mul(&x4);

        // X6 <- x3 * g + x4 * h
        let big_x6 = ristretto255::basepoint_mul(&x3);
        let h_x4 = ristretto255::point_mul(&h, &x4);
        ristretto255::point_add_assign(&mut big_x6, &h_x4);

        // X7 <- x1 * g + x2 * h
        let big_x7 = ristretto255::basepoint_mul(&x1);
        let h_x2 = ristretto255::point_mul(&h, &x2);
        ristretto255::point_add_assign(&mut big_x7, &h_x2);


        let rho = full_sigma_protocol_fiat_shamir<CoinType>(
            sender_pk, recipient_pk,
            withdraw_ct, deposit_ct,
            sender_updated_balance,
            balance, transfer_value,
            &big_x1, &big_x2, &big_x3, &big_x4,
            &big_x5, &big_x6, &big_x7);

        // alpha_1 <- x1 + rho * v
        let alpha1 = ristretto255::scalar_mul(&rho, amount_val);
        ristretto255::scalar_add_assign(&mut alpha1, &x1);

        // alpha_2 <- x2 + rho * r
        let alpha2 = ristretto255::scalar_mul(&rho, amount_rand);
        ristretto255::scalar_add_assign(&mut alpha2, &x2);

        // alpha_3 <- x3 + rho * b
        let alpha3 = ristretto255::scalar_mul(&rho, updated_balance_val);
        ristretto255::scalar_add_assign(&mut alpha3, &x3);

        // alpha_4 <- x4 + rho * r_b
        let alpha4 = ristretto255::scalar_mul(&rho, updated_balance_rand);
        ristretto255::scalar_add_assign(&mut alpha4, &x4);

        FullSigmaProof {
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
        }
    }

    #[test_only]
    /// Given a $\Sigma$-protocol proof for veiled-to-unveiled transfers, serializes it into byte form.
    /// Elements at the end of the `ElGamalToPedSigmaProof` struct are placed into the vector first,
    /// using the serialization formats in the `ristretto255` module.
    public fun serialize_unveil_sigma_proof<CoinType>(proof: &ElGamalToPedSigmaProof<CoinType>): vector<u8> {
        let x1_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(&proof.x1));
        let x2_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(&proof.x2));
        let x3_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(&proof.x3));
        let alpha1_bytes = ristretto255::scalar_to_bytes(&proof.alpha1);
        let alpha2_bytes = ristretto255::scalar_to_bytes(&proof.alpha2);

        let bytes = vector::empty<u8>();
        vector::append<u8>(&mut bytes, alpha2_bytes);
        vector::append<u8>(&mut bytes, alpha1_bytes);
        vector::append<u8>(&mut bytes, x3_bytes);
        vector::append<u8>(&mut bytes, x2_bytes);
        vector::append<u8>(&mut bytes, x1_bytes);

        bytes
    }

    #[test_only]
    /// Given a $\Sigma$-protocol proof, serializes it into byte form.
    /// Elements at the end of the `SigmaProof` struct are placed into the vector first,
    /// using the serialization formats in the `ristretto255` module.
    public fun serialize_full_sigma_proof<CoinType>(proof: &FullSigmaProof<CoinType>): vector<u8> {
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

        let bytes = vector::empty<u8>();
        vector::append<u8>(&mut bytes, alpha4_bytes);
        vector::append<u8>(&mut bytes, alpha3_bytes);
        vector::append<u8>(&mut bytes, alpha2_bytes);
        vector::append<u8>(&mut bytes, alpha1_bytes);
        vector::append<u8>(&mut bytes, x7_bytes);
        vector::append<u8>(&mut bytes, x6_bytes);
        vector::append<u8>(&mut bytes, x5_bytes);
        vector::append<u8>(&mut bytes, x4_bytes);
        vector::append<u8>(&mut bytes, x3_bytes);
        vector::append<u8>(&mut bytes, x2_bytes);
        vector::append<u8>(&mut bytes, x1_bytes);

        bytes
    }

    //
    // Full sigma proof verification tests
    //

    #[test]
    fun full_sigma_proof_verify_test()
    {
        // Pick a keypair for the sender, and one for the recipient
        let (_, sender_pk) = generate_elgamal_keypair();
        let (_, recipient_pk) = generate_elgamal_keypair();

        // Set the transferred amount to 50
        let amount_val = ristretto255::new_scalar_from_u32(50);
        let amount_rand = ristretto255::random_scalar();
        // Encrypt the amount under the sender's PK
        let withdraw_ct = elgamal::new_ciphertext_with_basepoint(&amount_val, &amount_rand, &sender_pk);

        // Encrypt the amount under the recipient's PK
        let deposit_ct = elgamal::new_ciphertext_with_basepoint(&amount_val, &amount_rand, &recipient_pk);

        let value_comm = pedersen::new_commitment_for_bulletproof(&amount_val, &amount_rand);

        // Set sender's new balance after the transaction to 100
        let updated_balance_val = ristretto255::new_scalar_from_u32(100);
        let updated_balance_rand = ristretto255::random_scalar();
        let updated_balance_ct = elgamal::new_ciphertext_with_basepoint(&updated_balance_val, &updated_balance_rand, &sender_pk);

        let updated_balance_comm = pedersen::new_commitment_for_bulletproof(&updated_balance_val, &updated_balance_rand);

        let sigma_proof = full_sigma_protocol_prove<coin::FakeMoney>(
            &sender_pk,
            &recipient_pk,
            &withdraw_ct,           // withdrawn amount, encrypted under sender PK
            &deposit_ct,            // deposited amount, encrypted under recipient PK (same plaintext as `withdraw_ct`)
            &updated_balance_ct,    // sender's balance after the transaction goes through, encrypted under sender PK
            &updated_balance_comm,  // commitment to sender's balance to prevent range proof forgery
            &value_comm,            // commitment to transfer amount to prevent range proof forgery
            &amount_rand,           // encryption randomness for `withdraw_ct` and `deposit_ct`
            &amount_val,            // transferred amount
            &updated_balance_rand,  // encryption randomness for updated balance ciphertext
            &updated_balance_val,   // sender's balance after the transfer
        );

        full_sigma_protocol_verify(
            &sender_pk,
            &recipient_pk,
            &withdraw_ct,
            &deposit_ct,
            &updated_balance_ct,
            &updated_balance_comm,
            &value_comm,
            &sigma_proof
        );
    }

    #[test]
    #[expected_failure(abort_code = 0x10001, location = Self)]
    fun full_sigma_proof_verify_fails_test()
    {
        let (_, source_pk) = generate_elgamal_keypair();
        let transfer_val = ristretto255::new_scalar_from_u32(50);
        let (_, dest_pk) = generate_elgamal_keypair();
        let transfer_rand = ristretto255::random_scalar();
        let withdraw_ct = elgamal::new_ciphertext_with_basepoint(&transfer_val, &transfer_rand, &source_pk);
        let deposit_ct = elgamal::new_ciphertext_with_basepoint(&transfer_val, &transfer_rand, &dest_pk);
        let value_comm = pedersen::new_commitment_for_bulletproof(&transfer_val, &transfer_rand);
        let updated_balance_val = ristretto255::new_scalar_from_u32(100);
        let updated_balance_rand = ristretto255::random_scalar();
        let updated_balance_ct = elgamal::new_ciphertext_with_basepoint(&updated_balance_val, &updated_balance_rand, &source_pk);

        let updated_balance_comm = pedersen::new_commitment_for_bulletproof(&updated_balance_val, &updated_balance_rand);

        //let (transfer_range_proof, transfer_comm) = bulletproofs::prove_range_pedersen(&transfer_val, &transfer_rand, MAX_BITS_IN_VALUE, VEILED_COIN_DST);

        let sigma_proof = full_sigma_protocol_prove<coin::FakeMoney>(&source_pk, &dest_pk, &withdraw_ct, &deposit_ct, &updated_balance_ct, &updated_balance_comm, &value_comm, &transfer_rand, &transfer_val, &updated_balance_rand, &updated_balance_val);

        let random_point = ristretto255::random_point();
        sigma_proof.x1 = random_point;

        full_sigma_protocol_verify(&source_pk, &dest_pk, &withdraw_ct, &deposit_ct, &updated_balance_ct, &updated_balance_comm, &value_comm, &sigma_proof);
    }

    //
    // Sigma proof deserialization tests
    //

    #[test]
    fun full_sigma_proof_serialize_test()
    {
        let (_, source_pk) = generate_elgamal_keypair();
        let transfer_val = ristretto255::new_scalar_from_u32(50);
        let (_, dest_pk) = generate_elgamal_keypair();
        let transfer_rand = ristretto255::random_scalar();
        let withdraw_ct = elgamal::new_ciphertext_with_basepoint(&transfer_val, &transfer_rand, &source_pk);
        let deposit_ct = elgamal::new_ciphertext_with_basepoint(&transfer_val, &transfer_rand, &dest_pk);
        let value_comm = pedersen::new_commitment_for_bulletproof(&transfer_val, &transfer_rand);
        let updated_balance_val = ristretto255::new_scalar_from_u32(100);
        let updated_balance_rand = ristretto255::random_scalar();
        let updated_balance_ct = elgamal::new_ciphertext_with_basepoint(&updated_balance_val, &updated_balance_rand, &source_pk);
        let updated_balance_comm = pedersen::new_commitment_for_bulletproof(&updated_balance_val, &updated_balance_rand);

        let sigma_proof = full_sigma_protocol_prove<coin::FakeMoney>(&source_pk, &dest_pk, &withdraw_ct, &deposit_ct, &updated_balance_ct, &updated_balance_comm, &value_comm, &transfer_rand, &transfer_val, &updated_balance_rand, &updated_balance_val);

        let sigma_proof_bytes = serialize_full_sigma_proof<coin::FakeMoney>(&sigma_proof);

        let deserialized_proof = std::option::extract<FullSigmaProof<coin::FakeMoney>>(&mut deserialize_full_sigma_proof<coin::FakeMoney>(sigma_proof_bytes));

        assert!(ristretto255::point_equals(&sigma_proof.x1, &deserialized_proof.x1), 1);
        assert!(ristretto255::point_equals(&sigma_proof.x2, &deserialized_proof.x2), 1);
        assert!(ristretto255::point_equals(&sigma_proof.x3, &deserialized_proof.x3), 1);
        assert!(ristretto255::point_equals(&sigma_proof.x4, &deserialized_proof.x4), 1);
        assert!(ristretto255::point_equals(&sigma_proof.x5, &deserialized_proof.x5), 1);
        assert!(ristretto255::point_equals(&sigma_proof.x6, &deserialized_proof.x6), 1);
        assert!(ristretto255::point_equals(&sigma_proof.x7, &deserialized_proof.x7), 1);
        assert!(ristretto255::scalar_equals(&sigma_proof.alpha1, &deserialized_proof.alpha1), 1);
        assert!(ristretto255::scalar_equals(&sigma_proof.alpha2, &deserialized_proof.alpha2), 1);
        assert!(ristretto255::scalar_equals(&sigma_proof.alpha3, &deserialized_proof.alpha3), 1);
        assert!(ristretto255::scalar_equals(&sigma_proof.alpha4, &deserialized_proof.alpha4), 1);
    }
}
