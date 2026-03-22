#[test_only]
/// Shared cryptographic test utilities for the confidential asset protocol.
/// Consolidates test helpers from multiple source modules into a single test-only module.
module aptos_framework::confidential_crypto_test_utils {
    use std::error;
    use std::option::Option;
    use std::vector;
    use aptos_std::ristretto255::{Self, RistrettoPoint, Scalar, CompressedRistretto,
        random_scalar, double_scalar_mul, point_identity_compressed};
    use aptos_std::ristretto255_bulletproofs::{Self as bulletproofs, RangeProof};
    use aptos_framework::confidential_balance::{Self,
        Pending, Available, Balance,
        ConfidentialBalanceRandomness,
    };
    use aptos_framework::confidential_amount::{Self, Amount};
    use aptos_framework::sigma_protocol_utils::points_clone;
    use aptos_framework::sigma_protocol_witness::{Witness, new_secret_witness};


    // ========================================= //
    //     Twisted ElGamal key generation         //
    // ========================================= //

    /// Returns the decompressed generator H used to derive the encryption key as EK = DK^(-1) * H.
    public fun get_encryption_key_basepoint(): RistrettoPoint {
        ristretto255::hash_to_point_base()
    }

    /// Returns `Some(EK)` where EK = DK^(-1) * H, or `None` if DK is not invertible.
    public fun pubkey_from_secret_key(dk: &Scalar): Option<CompressedRistretto> {
        let dk_invert = dk.scalar_invert();

        if (dk_invert.is_some()) {
            let point = ristretto255::hash_to_point_base().point_mul(&dk_invert.extract());
            std::option::some(point.point_compress())
        } else {
            std::option::none()
        }
    }

    public fun generate_twisted_elgamal_keypair(): (Scalar, CompressedRistretto) {
        let dk = random_scalar();
        let ek = pubkey_from_secret_key(&dk);
        (dk, ek.extract())
    }

    // ========================================= //
    //     Balance randomness & encryption        //
    //     (from confidential_balance)            //
    // ========================================= //

    public fun generate_randomness(num_chunks: u64): ConfidentialBalanceRandomness {
        confidential_balance::new_randomness(vector::range(0, num_chunks).map(|_| random_scalar()))
    }

    public fun generate_pending_randomness(): ConfidentialBalanceRandomness {
        generate_randomness(confidential_balance::get_num_pending_chunks())
    }

    public fun generate_available_randomness(): ConfidentialBalanceRandomness {
        generate_randomness(confidential_balance::get_num_available_chunks())
    }

    /// Shared encryption logic: computes (P, R) where P_i = amount_i*G + r_i*H, R_i = r_i*EK.
    public fun encrypt_amount(
        amount_chunks: &vector<Scalar>,
        randomness: &ConfidentialBalanceRandomness,
        ek: &CompressedRistretto,
        num_chunks: u64,
    ): (vector<RistrettoPoint>, vector<RistrettoPoint>) {
        let r = randomness.scalars();
        let ek_point = ek.point_decompress();
        let basepoint_H = get_encryption_key_basepoint();

        let p = vector::range(0, num_chunks).map(|i| {
            double_scalar_mul(
                &amount_chunks[i], &ristretto255::basepoint(),
                &r[i], &basepoint_H
            )
        });
        let r_out = vector::range(0, num_chunks).map(|i| {
            ek_point.point_mul(&r[i])
        });

        (p, r_out)
    }

    /// Creates a new pending balance from an amount using the provided randomness and encryption key.
    public fun new_pending_from_amount(
        amount: u128,
        randomness: &ConfidentialBalanceRandomness,
        ek: &CompressedRistretto
    ): Balance<Pending> {
        let amount_chunks = confidential_balance::split_pending_into_chunks(amount);
        let (p, r) = encrypt_amount(&amount_chunks, randomness, ek, confidential_balance::get_num_pending_chunks());
        confidential_balance::new_pending_from_p_and_r(p, r)
    }

    /// If `auditor_ek` is `Some`, computes R_aud_i = r_i * EK_auditor; otherwise R_aud is empty.
    public fun new_available_from_amount(
        amount: u128,
        randomness: &ConfidentialBalanceRandomness,
        ek: &CompressedRistretto,
        auditor_ek: &Option<CompressedRistretto>
    ): Balance<Available> {
        let num_chunks = confidential_balance::get_num_available_chunks();
        let amount_chunks = confidential_balance::split_available_into_chunks(amount);
        let (p, r) = encrypt_amount(&amount_chunks, randomness, ek, num_chunks);

        let r_aud_components = if (auditor_ek.is_some()) {
            let auditor_ek_point = auditor_ek.borrow().point_decompress();
            let r_scalars = randomness.scalars();
            vector::range(0, num_chunks).map(|i| {
                auditor_ek_point.point_mul(&r_scalars[i])
            })
        } else {
            vector[]
        };

        confidential_balance::new_available_from_p_r_r_aud(p, r, r_aud_components)
    }

    // ========================================= //
    //     Transfer amount construction           //
    //     (from confidential_amount)             //
    // ========================================= //

    /// Creates an Amount by encrypting `amount_u64` under sender, recipient, effective auditor
    /// (if present), and voluntary auditor keys, all using the same `randomness`.
    public fun new_amount_from_u64(
        amount_u64: u64,
        randomness: &ConfidentialBalanceRandomness,
        compressed_ek_sender: &CompressedRistretto,
        compressed_ek_recip: &CompressedRistretto,
        compressed_ek_eff_aud: &Option<CompressedRistretto>,
        compressed_ek_volun_auds: &vector<CompressedRistretto>,
    ): Amount {
        let amount_sender = new_pending_from_amount(
            amount_u64 as u128, randomness, compressed_ek_sender
        );
        let amount_recip = new_pending_from_amount(
            amount_u64 as u128, randomness, compressed_ek_recip
        );

        let _R_eff_aud = if (compressed_ek_eff_aud.is_some()) {
            let a = new_pending_from_amount(
                amount_u64 as u128, randomness, compressed_ek_eff_aud.borrow()
            );
            points_clone(a.get_R())
        } else {
            vector[]
        };

        let _R_volun_auds = compressed_ek_volun_auds.map_ref(|ek| {
            let a = new_pending_from_amount(amount_u64 as u128, randomness, ek);
            points_clone(a.get_R())
        });

        confidential_amount::new(
            points_clone(amount_sender.get_P()),
            points_clone(amount_sender.get_R()),
            points_clone(amount_recip.get_R()),
            _R_eff_aud, _R_volun_auds
        )
    }

    // ========================================= //
    //     Range proof generation                 //
    //     (from confidential_range_proofs)       //
    // ========================================= //

    public fun prove_range(
        amount_chunks: &vector<Scalar>, randomness: &vector<Scalar>
    ): RangeProof {
        let (proof, _) =
            bulletproofs::prove_batch_range_pedersen(
                amount_chunks,
                randomness,
                confidential_balance::get_chunk_size_bits(),
                aptos_framework::confidential_range_proofs::get_bulletproofs_dst()
            );
        proof
    }

    // ========================================= //
    //     Vector math utilities                  //
    //     (from sigma_protocol_utils)            //
    // ========================================= //

    const E_INTERNAL_INVARIANT_FAILED: u64 = 1;

    public fun decompress_points(compressed: &vector<CompressedRistretto>): vector<RistrettoPoint> {
        compressed.map_ref(|p| p.point_decompress())
    }

    public fun compress_points(points: &vector<RistrettoPoint>): vector<CompressedRistretto> {
        points.map_ref(|p| p.point_compress())
    }

    /// Returns a vector of `n` compressed identity (zero) points.
    public fun compressed_identity_points(n: u64): vector<CompressedRistretto> {
        vector::range(0, n).map(|_| point_identity_compressed())
    }

    /// Adds up two vectors of points `a` and `b` returning a new vector `c` where `c[i] = a[i] + b[i]`.
    public fun add_vec_points(a: &vector<RistrettoPoint>, b: &vector<RistrettoPoint>): vector<RistrettoPoint> {
        assert!(a.length() == b.length(), error::internal(E_INTERNAL_INVARIANT_FAILED));

        let r = vector[];
        a.enumerate_ref(|i, pt| {
            r.push_back(pt.point_add(&b[i]));
        });

        r
    }

    /// Given a vector of Ristretto255 points `a` and a scalar `e`, returns a new vector `c` where `c[i] = e * a[i]`.
    public fun mul_points(a: &vector<RistrettoPoint>, e: &Scalar): vector<RistrettoPoint> {
        a.map_ref(|pt| pt.point_mul(e))
    }

    /// Ensures two vectors of Ristretto255 points are equal.
    public fun equal_vec_points(a: &vector<RistrettoPoint>, b: &vector<RistrettoPoint>): bool {
        let m = a.length();
        assert!(m == b.length(), error::internal(E_INTERNAL_INVARIANT_FAILED));

        vector::range(0, m).all(|i| a[*i].point_equals(&b[*i]))
    }

    /// Adds up two vectors of scalars `a` and `b` returning a new vector `c` where `c[i] = a[i] + b[i]`.
    public fun add_vec_scalars(a: &vector<Scalar>, b: &vector<Scalar>): vector<Scalar> {
        assert!(a.length() == b.length(), error::internal(E_INTERNAL_INVARIANT_FAILED));

        let r = vector[];
        a.enumerate_ref(|i, a_i| {
            r.push_back(a_i.scalar_add(&b[i]));
        });

        r
    }

    /// Given a vector of scalars `a` and a scalar `e`, returns a new vector `c` where `c[i] = e * a[i]`.
    public fun mul_scalars(a: &vector<Scalar>, e: &Scalar): vector<Scalar> {
        a.map_ref(|s| s.scalar_mul(e))
    }

    // ========================================= //
    //     Sigma protocol test helpers            //
    //     (from sigma_protocol_* modules)        //
    // ========================================= //

    /// Returns a size-$k$ random witness. Useful when creating a ZKP during testing.
    public fun random_witness(k: u64): Witness {
        new_secret_witness(vector::range(0, k).map(|_| random_scalar()))
    }

    /// Creates a new registration witness: $(\mathsf{dk})$.
    public fun new_registration_witness(dk: Scalar): Witness {
        new_secret_witness(vector[dk])
    }

    /// Verifies that a balance encrypts `amount` using DK on the given R component.
    public fun check_decrypts_to<T>(
        balance: &Balance<T>, decrypt_R: &vector<RistrettoPoint>,
        dk: &Scalar, amount: u128,
    ): bool {
        let num_chunks = balance.get_P().length();
        let b_powers = confidential_balance::get_b_powers(num_chunks);

        let decrypted_chunks: vector<RistrettoPoint> = vector::range(0, num_chunks).map(|i| {
            balance.get_P()[i].point_sub(&decrypt_R[i].point_mul(dk))
        });

        let combined = aptos_std::ristretto255::multi_scalar_mul(&decrypted_chunks, &b_powers);
        combined.point_equals(&aptos_std::ristretto255::new_scalar_from_u128(amount).basepoint_mul())
    }

}
