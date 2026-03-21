/// Twisted ElGamal encryption over Ristretto255.
/// EK = DK^(-1) * H. Ciphertext: C = v*G + r*H, D = r*EK. Decrypt: v*G = C - DK*D.
module aptos_experimental::ristretto255_twisted_elgamal {
    use aptos_std::ristretto255::{Self, CompressedRistretto};
    #[test_only]
    use std::option::Option;
    #[test_only]
    use aptos_std::ristretto255::{RistrettoPoint, Scalar, random_scalar};

    friend aptos_experimental::confidential_asset;
    friend aptos_experimental::sigma_protocol_registration;
    friend aptos_experimental::sigma_protocol_withdraw;
    friend aptos_experimental::sigma_protocol_transfer;
    friend aptos_experimental::sigma_protocol_key_rotation;
    #[test_only]
    friend aptos_experimental::confidential_balance;
    #[test_only]
    friend aptos_experimental::confidential_asset_tests;

    // === Public functions ===

    /// Returns the compressed generator H used to derive the encryption key as EK = DK^(-1) * H.
    public(friend) fun get_encryption_key_basepoint_compressed(): CompressedRistretto {
        ristretto255::basepoint_H_compressed()
    }

    #[test_only]
    /// Returns the decompressed generator H used to derive the encryption key as EK = DK^(-1) * H.
    public(friend) fun get_encryption_key_basepoint(): RistrettoPoint {
        ristretto255::hash_to_point_base()
    }

    // === Test-only functions ===

    #[test_only]
    /// Returns `Some(EK)` where EK = DK^(-1) * H, or `None` if DK is not invertible.
    public(friend) fun pubkey_from_secret_key(dk: &Scalar): Option<CompressedRistretto> {
        let dk_invert = dk.scalar_invert();

        if (dk_invert.is_some()) {
            let point = ristretto255::hash_to_point_base().point_mul(&dk_invert.extract());
            std::option::some(point.point_compress())
        } else {
            std::option::none()
        }
    }

    #[test_only]
    public(friend) fun generate_twisted_elgamal_keypair(): (Scalar, CompressedRistretto) {
        let dk = random_scalar();
        let ek = pubkey_from_secret_key(&dk);
        (dk, ek.extract())
    }
}
