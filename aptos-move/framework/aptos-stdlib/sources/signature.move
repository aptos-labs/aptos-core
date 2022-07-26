/// Contains functions for [ed25519](https://en.wikipedia.org/wiki/EdDSA) digital signatures and for
/// [Boneh-Lynn-Shacham (BLS) signatures](https://en.wikipedia.org/wiki/BLS_digital_signature)
module aptos_std::signature {
    /// Return `true` if the bytes in `public_key` is a valid bls12381 public key and
    /// passed the PoP check.
    /// Return `false` otherwise.
    /// Does not abort.
    native public fun bls12381_validate_pubkey(public_key: vector<u8>, proof_of_possesion: vector<u8>): bool;
    spec bls12381_validate_pubkey { // TODO: temporary mockup.
        pragma opaque;
    }

    /// Return true if the BLS `signature` on `message` verifiers against the BLS public key `public_key`.
    /// Returns `false` if:
    /// - `signature` is not 96 bytes
    /// - `public_key` is not 48 bytes
    /// - `signature` or `public_key` are not valid: i.e., (1) they are the identity point, or (2) they are not valid
    ///    points on the BLS12-381 elliptic curve or (3) they are not prime-order points.
    /// - `signature` and `public key` are valid but the signature on `message` is not valid.
    /// This function can be used to verify either:
    ///     (1) signature shares for a BLS multisignature scheme or for a BLS aggregate signature scheme,
    ///     (2) BLS multisignatures (for this the `public_key` needs to be aggregated via `bls12381_aggregate_pubkey`).
    /// Does not abort.
    native public fun bls12381_verify_signature(
        signature: vector<u8>,
        public_key: vector<u8>,
        message: vector<u8>
    ): bool;

    /// Return `true` if the bytes in `public_key` can be parsed as a valid Ed25519 public key.
    /// Returns `false` if `public_key` is not 32 bytes OR is 32 bytes, but does not pass
    /// points-on-curve or small subgroup checks. This function should NOT be needed for most users
    /// since ed25519_verify already does all these checks. We leave it here just in case.
    /// See the Rust `aptos_crypto::Ed25519PublicKey` type for more details.
    /// Does not abort.
    native public fun ed25519_validate_pubkey(public_key: vector<u8>): bool;

    /// Return `true` if the bytes in `public_key` is an Ed25519 public key with a valid proof-of-knowledge (PoK)
    /// Return `false` otherwise.
    /// Does not abort.
    native public fun ed25519_verify_proof_of_knowledge(public_key: vector<u8>, proof_of_knowledge: vector<u8>): bool;

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
    /// Test on a valid signature created using sk = x"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
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

    #[test]
    /// Tests verification of a random BLS signature
    fun test_bls12381() {
        // Test case generated by running `cargo test -- bls12381_sample_signature --nocapture` in `crates/aptos-crypto`
        // =============================================================================================================
        // SK:        077c8a56f26259215a4a245373ab6ddf328ac6e00e5ea38d8700efa361bdc58d
        // PK:        94209a296b739577cb076d3bfb1ca8ee936f29b69b7dae436118c4dd1cc26fd43dcd16249476a006b8b949bf022a7858
        // Message:   Hello Aptos!
        // Signature: b01ce4632e94d8c611736e96aa2ad8e0528a02f927a81a92db8047b002a8c71dc2d6bfb94729d0973790c10b6ece446817e4b7543afd7ca9a17c75de301ae835d66231c26a003f11ae26802b98d90869a9e73788c38739f7ac9d52659e1f7cf7

        let ok = bls12381_verify_signature(
            x"b01ce4632e94d8c611736e96aa2ad8e0528a02f927a81a92db8047b002a8c71dc2d6bfb94729d0973790c10b6ece446817e4b7543afd7ca9a17c75de301ae835d66231c26a003f11ae26802b98d90869a9e73788c38739f7ac9d52659e1f7cf7",
            x"94209a296b739577cb076d3bfb1ca8ee936f29b69b7dae436118c4dd1cc26fd43dcd16249476a006b8b949bf022a7858",
            b"Hello Aptos!",
        );

        assert!(ok == true, 1);
    }

    #[test]
    /// Tests verification of a random Ed25519 PoK created for sk = x""
    fun test_ed25519_proof_of_knowledge() {
        // Test case generated by running `cargo test -- sample_proof_of_knowledge --nocapture` in `crates/aptos-crypto`
        // =============================================================================================================
        // SK:  be21984f61831db905757ed6cec2e870c90d3a1966934244f6fe68c313ee6bdd
        // PK:  6ace9cefa74dacff3c6233694c0c686ef13c1a231d0e94f10f0f4afaf9e2364d
        // PoK: 21294fa1dc25be6ebf787b1f847ec66e7a777b61cd83b4faa484a003d10a241efc665c9e30ac2425f184df3a5d8919d275e15a4b0973dff315ca1f1e1f135f0b
        let ok = ed25519_verify_proof_of_knowledge(
            x"6ace9cefa74dacff3c6233694c0c686ef13c1a231d0e94f10f0f4afaf9e2364d",
            x"21294fa1dc25be6ebf787b1f847ec66e7a777b61cd83b4faa484a003d10a241efc665c9e30ac2425f184df3a5d8919d275e15a4b0973dff315ca1f1e1f135f0b"
        );

        assert!(ok == true, 1);
    }
}
