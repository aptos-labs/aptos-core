/// This module provides utilities for Twisted ElGamal encryption over the Ristretto255 curve.
///
/// In Twisted ElGamal, an encryption key (EK) is derived from a decryption key (DK) as:
///   EK = DK^(-1) * H
/// where H is a secondary basepoint (distinct from the primary basepoint G).
///
/// A ciphertext encrypting value `v` with randomness `r` under EK is:
///   C = v * G + r * H  (value component)
///   D = r * EK         (EK component for decryption)
///
/// Decryption: v * G = C - DK * D
module aptos_experimental::ristretto255_twisted_elgamal {
    use aptos_std::ristretto255::{Self, CompressedRistretto, RistrettoPoint};
    #[test_only]
    use std::option::Option;
    #[test_only]
    use aptos_std::ristretto255::Scalar;

    //
    // Public functions
    //

    /// Returns the compressed generator H used to derive the encryption key as EK = DK^(-1) * H.
    public fun get_encryption_key_basepoint_compressed(): CompressedRistretto {
        ristretto255::basepoint_H_compressed()
    }

    /// Returns the decompressed generator H used to derive the encryption key as EK = DK^(-1) * H.
    public fun get_encryption_key_basepoint(): RistrettoPoint {
        ristretto255::hash_to_point_base()
    }

    //
    // Test-only functions
    //

    #[test_only]
    /// Derives an encryption key from a decryption key using the formula EK = DK^(-1) * H.
    /// Returns `Some(CompressedRistretto)` if the DK inversion succeeds, otherwise `None`.
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
    /// Generates a random Twisted ElGamal key pair (DK, EK), where EK = DK^(-1) * H.
    public fun generate_twisted_elgamal_keypair(): (Scalar, CompressedRistretto) {
        let dk = ristretto255::random_scalar();
        let ek = pubkey_from_secret_key(&dk);
        (dk, ek.extract())
    }
}
