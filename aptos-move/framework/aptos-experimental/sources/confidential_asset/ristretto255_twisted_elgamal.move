/// Twisted ElGamal encryption over Ristretto255.
/// EK = DK^(-1) * H. Ciphertext: C = v*G + r*H, D = r*EK. Decrypt: v*G = C - DK*D.
module aptos_experimental::ristretto255_twisted_elgamal {
    use aptos_std::ristretto255::{Self, CompressedRistretto};

    friend aptos_experimental::confidential_asset;
    friend aptos_experimental::sigma_protocol_registration;
    friend aptos_experimental::sigma_protocol_withdraw;
    friend aptos_experimental::sigma_protocol_transfer;
    friend aptos_experimental::sigma_protocol_key_rotation;
    #[test_only]
    friend aptos_experimental::sigma_protocol_proof_tests;

    // === Public functions ===

    /// Returns the compressed generator H used to derive the encryption key as EK = DK^(-1) * H.
    public(friend) fun get_encryption_key_basepoint_compressed(): CompressedRistretto {
        ristretto255::basepoint_H_compressed()
    }

}
