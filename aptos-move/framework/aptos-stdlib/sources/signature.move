/// Contains functions for [ed25519](https://en.wikipedia.org/wiki/EdDSA) digital signatures.
module aptos_std::signature {

    /// Return `true` if the bytes in `public_key` can be parsed as a valid Ed25519 public key.
    /// Returns `false` if `public_key` is not 32 bytes OR is 32 bytes, but does not pass
    /// points-on-curve or small subgroup checks. This function should NOT be needed for most users
    /// since ed25519_verify already does all these checks. We leave it here just in case.
    /// See the Rust `aptos_crypto::Ed25519PublicKey` type for more details.
    /// Does not abort.
    native public fun ed25519_validate_pubkey(public_key: vector<u8>): bool;

    /// Return `true` if the bytes in `public_key` is a valid bls12381 public key and
    /// passed the PoP check.
    /// Return `false` otherwise.
    /// Does not abort.
    native public fun bls12381_validate_pubkey(public_key: vector<u8>, proof_of_possesion: vector<u8>): bool;

    /// Return true if the Ed25519 `signature` on `message` verifies against the Ed25519 public key
    /// `public_key`.
    /// Returns `false` if:
    /// - `signature` is not 64 bytes
    /// - `public_key` is not 32 bytes
    /// - `public_key` does not pass points-on-curve or small subgroup checks,
    /// - `signature` and `public_key` are valid, but the signature on `message` does not verify.
    /// Does not abort.
    native public fun ed25519_verify(
        signature: vector<u8>,
        public_key: vector<u8>,
        message: vector<u8>
    ): bool;

    /// Recovers the signer's public key from a secp256k1 `signature` provided the `recovery_id` and signed
    /// `message` (32 byte digest).
    /// Returns `(public_key, true)` if inputs are valid and `([], false)` if invalid.
    native public fun secp256k1_recover(
        message: vector<u8>,
        recovery_id: u8,
        signature: vector<u8>
    ): (vector<u8>, bool);

    #[test]
    /// Test on a valid signature created using pk = x"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
    fun test_secp256k1() {
        use std::hash;

        let (pk, ok) = secp256k1_recover(
            hash::sha2_256(b"test aptos secp256k1"),
            0,
            x"f7ad936da03f948c14c542020e3c5f4e02aaacd1f20427c11aa6e2fbf8776477646bba0e1a37f9e7c777c423a1d2849baafd7ff6a9930814a43c3f80d59db56f",
        );
        assert!(ok == true, 1);
        assert!(pk == x"4646ae5047316b4230d0086c8acec687f00b1cd9d1dc634f6cb358ac0a9a8ffffe77b4dd0a4bfb95851f3b7355c781dd60f8418fc8a65d14907aff47c903a559", 2);

        // Flipped bits; Signature stays valid
        let (pk, ok) = secp256k1_recover(
            hash::sha2_256(b"test aptos secp256k1"),
            0,
            x"f7ad936da03f948c14c542020e3c5f4e02aaacd1f20427c11aa6e2fbf8776477646bba0e1a37f9e7c7f7c423a1d2849baafd7ff6a9930814a43c3f80d59db56f",
        );
        assert!(ok == true, 3);
        assert!(pk != x"4646ae5047316b4230d0086c8acec687f00b1cd9d1dc634f6cb358ac0a9a8ffffe77b4dd0a4bfb95851f3b7355c781dd60f8418fc8a65d14907aff47c903a559", 4);

        // Flipped bits; Signature becomes invalid
        let (_, ok) = secp256k1_recover(
            hash::sha2_256(b"test aptos secp256k1"),
            0,
            x"ffad936da03f948c14c542020e3c5f4e02aaacd1f20427c11aa6e2fbf8776477646bba0e1a37f9e7c7f7c423a1d2849baafd7ff6a9930814a43c3f80d59db56f",
        );
        assert!(ok == false, 5);
    }
}
