/// Twisted ElGamal encryption over Ristretto255.
/// EK = DK^(-1) * H. Ciphertext: C = v*G + r*H, D = r*EK. Decrypt: v*G = C - DK*D.
module aptos_experimental::ristretto255_twisted_elgamal {
    use aptos_std::ristretto255::{Self, CompressedRistretto, RistrettoPoint};
    #[test_only]
    use std::option::Option;
    #[test_only]
    use aptos_std::ristretto255::{Scalar, random_scalar};

    // === Public functions ===

    /// Returns the compressed generator H used to derive the encryption key as EK = DK^(-1) * H.
    public fun get_encryption_key_basepoint_compressed(): CompressedRistretto {
        ristretto255::basepoint_H_compressed()
    }

    /// Returns the decompressed generator H used to derive the encryption key as EK = DK^(-1) * H.
    public fun get_encryption_key_basepoint(): RistrettoPoint {
        ristretto255::hash_to_point_base()
    }

    // === Test-only functions ===

    #[test_only]
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

    #[test_only]
    public fun generate_twisted_elgamal_keypair(): (Scalar, CompressedRistretto) {
        let dk = random_scalar();
        let ek = pubkey_from_secret_key(&dk);
        (dk, ek.extract())
    }
}
