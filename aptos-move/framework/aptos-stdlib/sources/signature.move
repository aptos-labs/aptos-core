/// Contains functions for:
///
///  1. [Ed25519](https://en.wikipedia.org/wiki/EdDSA#Ed25519) digital signatures
///
///  2. ECDSA digital signatures over secp256k1 elliptic curves
///
///  3. The minimum-pubkey-size variant of [Boneh-Lynn-Shacham (BLS) signatures](https://en.wikipedia.org/wiki/BLS_digital_signature),
///     where public keys are BLS12-381 elliptic-curve points in $\mathbb{G}_1$ and signatures are in $\mathbb{G}_2$,
///     as per the [IETF BLS draft standard](https://datatracker.ietf.org/doc/html/draft-irtf-cfrg-bls-signature#section-2.1)
module aptos_std::signature {
    use std::option::Option;

    /// Given a vector of serialized public keys, combines them into an aggregated public key which can be used to verify
    /// multisignatures using `bls12381_verify_signature`.
    /// Returns 'None' if no public keys are given as input.
    /// This function assumes that the caller verified all public keys have a valid proof-of-possesion (PoP) using
    /// `bls12381_verify_proof_of_possession`.
    /// Does not abort.
    native public fun bls12381_aggregate_pop_verified_pubkeys(
        public_keys: vector<vector<u8>>,
    ): Option<vector<u8>>;

    // TODO: implement remaining BLS functions:
    // native public fun bls12381_aggregate_signatures(signatures: vector<vector<u8>>): Option<vector<u8>>;
    // native public fun bls12381_verify_aggregate_signature(
    //        signature: vector<u8>,
    //        public_keys: vector<vector<u8>>,
    //        messages: vector<vector<u8>>,
    //    ): bool;

    /// Return `true` if the bytes in `public_key` are a valid bls12381 public key: it is in the prime-order subgroup
    /// and it is different from the identity group element.
    /// Return `false` otherwise.
    /// Does not abort.
    native public fun bls12381_validate_pubkey(public_key: vector<u8>): bool;

    /// Return `true` if the bytes in `public_key` are a valid bls12381 public key (as per `bls12381_validate_pubkey`)
    /// *and* has a valid proof-of-possesion (PoP).
    /// Return `false` otherwise.
    /// Does not abort.
    native public fun bls12381_verify_proof_of_possession(public_key: vector<u8>, proof_of_possesion: vector<u8>): bool;
    spec bls12381_verify_proof_of_possession { // TODO: temporary mockup.
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

    /// Recovers the signer's public key from a secp256k1 ECDSA `signature` given the `recovery_id` and the signed
    /// `message` (32 byte digest).
    /// Returns `(public_key, true)` if `signature` is a valid signature on `message` under the recovered `public_key`
    /// and returns `([], false)` otherwise.
    ///
    /// Note that an invalid signature, or a signature from a different message, will result in the recovery of an
    /// incorrect public key. This recovery algorithm can only be used to check validity of a signature if the signer's
    /// public key (or its hash) is known beforehand.
    native public fun secp256k1_ecdsa_recover(
        message: vector<u8>,
        recovery_id: u8,
        signature: vector<u8>
    ): (vector<u8>, bool);

    #[test_only]
    use std::vector;
    #[test_only]
    use std::option;

    #[test]
    /// Test on a valid secp256k1 ECDSA signature created using sk = x"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
    fun test_secp256k1_recover() {
        use std::hash;

        let (pk, ok) = secp256k1_ecdsa_recover(
            hash::sha2_256(b"test aptos secp256k1"),
            0,
            x"f7ad936da03f948c14c542020e3c5f4e02aaacd1f20427c11aa6e2fbf8776477646bba0e1a37f9e7c777c423a1d2849baafd7ff6a9930814a43c3f80d59db56f",
        );
        assert!(ok == true, 1);
        assert!(pk == x"4646ae5047316b4230d0086c8acec687f00b1cd9d1dc634f6cb358ac0a9a8ffffe77b4dd0a4bfb95851f3b7355c781dd60f8418fc8a65d14907aff47c903a559", 2);

        // Flipped bits; Signature stays valid
        let (pk, ok) = secp256k1_ecdsa_recover(
            hash::sha2_256(b"test aptos secp256k1"),
            0,
            x"f7ad936da03f948c14c542020e3c5f4e02aaacd1f20427c11aa6e2fbf8776477646bba0e1a37f9e7c7f7c423a1d2849baafd7ff6a9930814a43c3f80d59db56f",
        );
        assert!(ok == true, 3);
        assert!(pk != x"4646ae5047316b4230d0086c8acec687f00b1cd9d1dc634f6cb358ac0a9a8ffffe77b4dd0a4bfb95851f3b7355c781dd60f8418fc8a65d14907aff47c903a559", 4);

        // Flipped bits; Signature becomes invalid
        let (_, ok) = secp256k1_ecdsa_recover(
            hash::sha2_256(b"test aptos secp256k1"),
            0,
            x"ffad936da03f948c14c542020e3c5f4e02aaacd1f20427c11aa6e2fbf8776477646bba0e1a37f9e7c7f7c423a1d2849baafd7ff6a9930814a43c3f80d59db56f",
        );
        assert!(ok == false, 5);
    }

    #[test]
    /// Tests verification of random BLS proofs-of-possession (PoPs)
    fun test_bls12381_verify_pop() {
        // Test case generated by running `cargo test -- sample_pop --nocapture --include-ignored` in `crates/aptos-crypto`
        // =============================================================================================================

        let pks = vector[
            x"808864c91ae7a9998b3f5ee71f447840864e56d79838e4785ff5126c51480198df3d972e1e0348c6da80d396983e42d7",
            x"8843843c76d167c02842a214c21277bad0bfd83da467cb5cf2d3ee67b2dcc7221b9fafa6d430400164012580e0c34d27",
            x"a23b524d4308d46e43ee8cbbf57f3e1c20c47061ad9c3f915212334ea6532451dd5c01d3d3ada6bea10fe180b2c3b450",
            x"a2aaa3eae1df3fc36365491afa1da5181acbb03801afd1430f04bb3b3eb18036f8b756b3508e4caee04beff50d455d1c",
            x"84985b7e983dbdaddfca1f0b7dad9660bb39fff660e329acec15f69ac48c75dfa5d2df9f0dc320e4e7b7658166e0ac1c",
        ];

        let pops = vector[
            x"ab42afff92510034bf1232a37a0d31bc8abfc17e7ead9170d2d100f6cf6c75ccdcfedbd31699a112b4464a06fd636f3f190595863677d660b4c5d922268ace421f9e86e3a054946ee34ce29e1f88c1a10f27587cf5ec528d65ba7c0dc4863364",
            x"a6da5f2bc17df70ce664cff3e3a3e09d17162e47e652032b9fedc0c772fd5a533583242cba12095602e422e579c5284b1735009332dbdd23430bbcf61cc506ae37e41ff9a1fc78f0bc0d99b6bc7bf74c8f567dfb59079a035842bdc5fa3a0464",
            x"b8eef236595e2eab34d3c1abdab65971f5cfa1988c731ef62bd63c9a9ad3dfc9259f4f183bfffbc8375a38ba62e1c41a11173209705996ce889859bcbb3ddd7faa3c4ea3d8778f30a9ff814fdcfea1fb163d745c54dfb4dcc5a8cee092ee0070",
            x"a03a12fab68ad59d85c15dd1528560eff2c89250070ad0654ba260fda4334da179811d2ecdaca57693f80e9ce977d62011e3b1ee7bb4f7e0eb9b349468dd758f10fc35d54e0d0b8536ca713a77a301944392a5c192b6adf2a79ae2b38912dc98",
            x"8899b294f3c066e6dfb59bc0843265a1ccd6afc8f0f38a074d45ded8799c39d25ee0376cd6d6153b0d4d2ff8655e578b140254f1287b9e9df4e2aecc5b049d8556a4ab07f574df68e46348fd78e5298b7913377cf5bb3cf4796bfc755902bfdd",
        ];

        assert!(std::vector::length(&pks) == std::vector::length(&pops), 1);

        let i = 0;
        while (i < std::vector::length(&pks)) {
            assert!(bls12381_verify_proof_of_possession(*std::vector::borrow(&pks, i), *std::vector::borrow(&pops, i)), 1);

            i = i + 1;
        };
    }

    #[test]
    /// Tests verification of a random BLS signature created using sk = x""
    fun test_bls12381_verify_individual_signature() {
        // Test case generated by running `cargo test -- bls12381_sample_signature --nocapture --include-ignored` in
        // `crates/aptos-crypto`
        // =============================================================================================================
        // SK:        077c8a56f26259215a4a245373ab6ddf328ac6e00e5ea38d8700efa361bdc58d

        let ok = bls12381_verify_signature(
            x"b01ce4632e94d8c611736e96aa2ad8e0528a02f927a81a92db8047b002a8c71dc2d6bfb94729d0973790c10b6ece446817e4b7543afd7ca9a17c75de301ae835d66231c26a003f11ae26802b98d90869a9e73788c38739f7ac9d52659e1f7cf7",
            x"94209a296b739577cb076d3bfb1ca8ee936f29b69b7dae436118c4dd1cc26fd43dcd16249476a006b8b949bf022a7858",
            b"Hello Aptos!",
        );

        assert!(ok == true, 1);
    }

    #[test]
    fun test_bls12381_verify_multisig() {
        // First, make sure if no inputs are given, the function returns None
        // assert!(bls12381_aggregate_pop_verified_pubkeys(vector::empty()) == option::none(), 1);
        let none_opt = bls12381_aggregate_pop_verified_pubkeys(vector::empty());
        assert!(option::is_none(&none_opt), 1);

        // Second, try some test-cases generated by running the following command in `crates/aptos-crypto`:
        //  $ cargo test -- bls12381_sample_aggregate_pk_and_multisig --nocapture --include-ignored
        let pks = vector[
            x"92e201a806af246f805f460fbdc6fc90dd16a18d6accc236e85d3578671d6f6690dde22134d19596c58ce9d63252410a",
            x"ab9df801c6f96ade1c0490c938c87d5bcc2e52ccb8768e1b5d14197c5e8bfa562783b96711b702dda411a1a9f08ebbfa",
            x"b698c932cf7097d99c17bd6e9c9dc4eeba84278c621700a8f80ec726b1daa11e3ab55fc045b4dbadefbeef05c4182494",
            x"934706a8b876d47a996d427e1526ce52c952d5ec0858d49cd262efb785b62b1972d06270b0a7adda1addc98433ad1843",
            x"a4cd352daad3a0651c1998dfbaa7a748e08d248a54347544bfedd51a197e016bb6008e9b8e45a744e1a030cc3b27d2da",
        ];

        let agg_pks = vector[
            x"92e201a806af246f805f460fbdc6fc90dd16a18d6accc236e85d3578671d6f6690dde22134d19596c58ce9d63252410a",
            x"b79ad47abb441d7eda9b220a626df2e4e4910738c5f777947f0213398ecafae044ec0c20d552d1348347e9abfcf3eca1",
            x"b5f5eb6153ab5388a1a76343d714e4a2dcf224c5d0722d1e8e90c6bcead05c573fffe986460bd4000645a655bf52bc60",
            x"b922006ec14c183572a8864c31dc6632dccffa9f9c86411796f8b1b5a93a2457762c8e2f5ef0a2303506c4bca9a4e0bf",
            x"b53df1cfee2168f59e5792e710bf22928dc0553e6531dae5c7656c0a66fc12cb82fbb04863938c953dc901a5a79cc0f3",
        ];

        // The signed message is "Hello, Aptoverse!"
        let multisigs = vector[
            x"ade45c67bff09ae57e0575feb0be870f2d351ce078e8033d847615099366da1299c69497027b77badb226ff1708543cd062597030c3f1553e0aef6c17e7af5dd0de63c1e4f1f9da68c966ea6c1dcade2cdc646bd5e8bcd4773931021ec5be3fd",
            x"964af3d83436f6a9a382f34590c0c14e4454dc1de536af205319ce1ed417b87a2374863d5df7b7d5ed900cf91dffa7a105d3f308831d698c0d74fb2259d4813434fb86425db0ded664ae8f85d02ec1d31734910317d4155cbf69017735900d4d",
            x"b523a31813e771e55aa0fc99a48db716ecc1085f9899ccadb64e759ecb481a2fb1cdcc0b266f036695f941361de773081729311f6a1bca9d47393f5359c8c87dc34a91f5dae335590aacbff974076ad1f910dd81750553a72ccbcad3c8cc0f07",
            x"a945f61699df58617d37530a85e67bd1181349678b89293951ed29d1fb7588b5c12ebb7917dfc9d674f3f4fde4d062740b85a5f4927f5a4f0091e46e1ac6e41bbd650a74dd49e91445339d741e3b10bdeb9bc8bba46833e0011ff91fa5c77bd2",
            x"b627b2cfd8ae59dcf5e58cc6c230ae369985fd096e1bc3be38da5deafcbed7d939f07cccc75383539940c56c6b6453db193f563f5b6e4fe54915afd9e1baea40a297fa7eda74abbdcd4cc5c667d6db3b9bd265782f7693798894400f2beb4637",
        ];

        let i = 0;
        let accum_pk = std::vector::empty<vector<u8>>();
        while (i < std::vector::length(&pks)) {
            std::vector::push_back(&mut accum_pk, *std::vector::borrow(&pks, i));

            let apk = bls12381_aggregate_pop_verified_pubkeys(accum_pk);

            assert!(option::is_some(&apk), 1);

            let apk = option::extract(&mut apk);
            assert!(apk == *std::vector::borrow(&agg_pks, i), 1);

            assert!(bls12381_verify_signature(*std::vector::borrow(&multisigs, i), apk, b"Hello, Aptoverse!"), 1);

            i = i + 1;
        };
    }
}
