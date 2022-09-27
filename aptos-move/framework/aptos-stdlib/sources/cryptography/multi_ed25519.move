/// Exports MultiEd25519 multi-signatures in Move.
/// This module has the exact same interface as the Ed25519 module.

module aptos_std::multi_ed25519 {
    use std::bcs;
    use std::error;
    use std::option::{Self, Option};
    use std::vector;
    use aptos_std::ed25519;

    //
    // Error codes
    //

    /// Wrong number of bytes were given as input when deserializing an Ed25519 public key.
    const E_WRONG_PUBKEY_SIZE: u64 = 1;

    /// Wrong number of bytes were given as input when deserializing an Ed25519 signature.
    const E_WRONG_SIGNATURE_SIZE: u64 = 2;

    //
    // Constants
    //

    /// The identifier of the MultiEd25519 signature scheme, which is used when deriving Aptos authentication keys by hashing
    /// it together with an MultiEd25519 public key.
    const SIGNATURE_SCHEME_ID: u8 = 1;

    /// The size of an individual Ed25519 public key, in bytes.
    /// (A MultiEd25519 public key consists of several of these, plus the threshold.)
    const INDIVIDUAL_PUBLIC_KEY_NUM_BYTES: u64 = 32;

    /// The size of an individual Ed25519 signature, in bytes.
    /// (A MultiEd25519 signature consists of several of these, plus the signer bitmap.)
    const INDIVIDUAL_SIGNATURE_NUM_BYTES: u64 = 64;

    /// When serializing a MultiEd25519 public key, the threshold k will be encoded using this many bytes.
    const THRESHOLD_SIZE_BYTES: u64 = 1;

    /// When serializing a MultiEd25519 signature, the bitmap that indicates the signers will be encoded using this many
    /// bytes.
    const BITMAP_NUM_OF_BYTES: u64 = 4;

    /// Max number of ed25519 public keys allowed in multi-ed25519 keys
    const MAX_NUMBER_OF_PUBLIC_KEYS: u64 = 32;

    //
    // Structs
    //

    /// An *unvalidated*, k out of n MultiEd25519 public key. The `bytes` field contains (1) several chunks of
    /// `ed25519::PUBLIC_KEY_NUM_BYTES` bytes, each encoding a Ed25519 PK, and (2) a single byte encoding the threshold k.
    /// *Unvalidated* means there is no guarantee that the underlying PKs are valid elliptic curve points of non-small
    /// order.
    struct UnvalidatedPublicKey has copy, drop, store {
        bytes: vector<u8>
    }

    /// A *validated* k out of n MultiEd25519 public key. *Validated* means that all the underlying PKs will be
    /// elliptic curve points that are NOT of small-order. It does not necessarily mean they will be prime-order points.
    /// This struct encodes the public key exactly as `UnvalidatedPublicKey`.
    ///
    /// For now, this struct is not used in any verification functions, but it might be in the future.
    struct ValidatedPublicKey has copy, drop, store {
        bytes: vector<u8>
    }

    /// A purported MultiEd25519 multi-signature that can be verified via `verify_signature_strict` or
    /// `verify_signature_strict_t`. The `bytes` field contains (1) several chunks of `ed25519::SIGNATURE_NUM_BYTES`
    /// bytes, each encoding a Ed25519 signature, and (2) a `BITMAP_NUM_OF_BYTES`-byte bitmap encoding the signer
    /// identities.
    struct Signature has copy, drop, store {
        bytes: vector<u8>
    }

    //
    // Functions
    //

    /// Parses the input 32 bytes as an *unvalidated* MultiEd25519 public key.
    public fun new_unvalidated_public_key_from_bytes(bytes: vector<u8>): UnvalidatedPublicKey {
        assert!(vector::length(&bytes) / INDIVIDUAL_PUBLIC_KEY_NUM_BYTES <= MAX_NUMBER_OF_PUBLIC_KEYS, error::invalid_argument(E_WRONG_PUBKEY_SIZE));
        assert!(vector::length(&bytes) % INDIVIDUAL_PUBLIC_KEY_NUM_BYTES == THRESHOLD_SIZE_BYTES, error::invalid_argument(E_WRONG_PUBKEY_SIZE));
        UnvalidatedPublicKey { bytes }
    }

    /// Parses the input bytes as a *validated* MultiEd25519 public key.
    public fun new_validated_public_key_from_bytes(bytes: vector<u8>): Option<ValidatedPublicKey> {
        if (vector::length(&bytes) % INDIVIDUAL_PUBLIC_KEY_NUM_BYTES == THRESHOLD_SIZE_BYTES &&
            public_key_validate_internal(bytes)) {
            option::some(ValidatedPublicKey {
                bytes
            })
        } else {
            option::none<ValidatedPublicKey>()
        }
    }

    /// Parses the input bytes as a purported MultiEd25519 multi-signature.
    public fun new_signature_from_bytes(bytes: vector<u8>): Signature {
        assert!(vector::length(&bytes) % INDIVIDUAL_SIGNATURE_NUM_BYTES == BITMAP_NUM_OF_BYTES, error::invalid_argument(E_WRONG_SIGNATURE_SIZE));
        Signature { bytes }
    }

    /// Converts a ValidatedPublicKey to an UnvalidatedPublicKey, which can be used in the strict verification APIs.
    public fun public_key_to_unvalidated(pk: &ValidatedPublicKey): UnvalidatedPublicKey {
        UnvalidatedPublicKey {
            bytes: pk.bytes
        }
    }

    /// Moves a ValidatedPublicKey into an UnvalidatedPublicKey, which can be used in the strict verification APIs.
    public fun public_key_into_unvalidated(pk: ValidatedPublicKey): UnvalidatedPublicKey {
        UnvalidatedPublicKey {
            bytes: pk.bytes
        }
    }

    /// Serializes an UnvalidatedPublicKey struct to 32-bytes.
    public fun unvalidated_public_key_to_bytes(pk: &UnvalidatedPublicKey): vector<u8> {
        pk.bytes
    }

    /// Serializes an ValidatedPublicKey struct to 32-bytes.
    public fun validated_public_key_to_bytes(pk: &ValidatedPublicKey): vector<u8> {
        pk.bytes
    }

    /// Serializes a Signature struct to 64-bytes.
    public fun signature_to_bytes(sig: &Signature): vector<u8> {
        sig.bytes
    }

    /// Takes in an *unvalidated* public key and attempts to validate it.
    /// Returns `Some(ValidatedPublicKey)` if successful and `None` otherwise.
    public fun public_key_validate(pk: &UnvalidatedPublicKey): Option<ValidatedPublicKey> {
        new_validated_public_key_from_bytes(pk.bytes)
    }

    /// Verifies a purported MultiEd25519 `multisignature` under an *unvalidated* `public_key` on the specified `message`.
    /// This call will validate the public key by checking it is NOT in the small subgroup.
    public fun signature_verify_strict(
        multisignature: &Signature,
        public_key: &UnvalidatedPublicKey,
        message: vector<u8>
    ): bool {
        signature_verify_strict_internal(multisignature.bytes, public_key.bytes, message)
    }

    /// This function is used to verify a multi-signature on any BCS-serializable type T. For now, it is used to verify the
    /// proof of private key ownership when rotating authentication keys.
    public fun signature_verify_strict_t<T: drop>(multisignature: &Signature, public_key: &UnvalidatedPublicKey, data: T): bool {
        let encoded = ed25519::new_signed_message(data);

        signature_verify_strict_internal(multisignature.bytes, public_key.bytes, bcs::to_bytes(&encoded))
    }

    /// Derives the Aptos-specific authentication key of the given Ed25519 public key.
    public fun unvalidated_public_key_to_authentication_key(pk: &UnvalidatedPublicKey): vector<u8> {
        public_key_bytes_to_authentication_key(pk.bytes)
    }

    /// Derives the Aptos-specific authentication key of the given Ed25519 public key.
    public fun validated_public_key_to_authentication_key(pk: &ValidatedPublicKey): vector<u8> {
        public_key_bytes_to_authentication_key(pk.bytes)
    }

    /// Derives the Aptos-specific authentication key of the given Ed25519 public key.
    fun public_key_bytes_to_authentication_key(pk_bytes: vector<u8>): vector<u8> {
        std::vector::push_back(&mut pk_bytes, SIGNATURE_SCHEME_ID);
        std::hash::sha3_256(pk_bytes)
    }

    //
    // Native functions
    //

    /// Return `true` if the bytes in `public_key` can be parsed as a valid MultiEd25519 public key: i.e., all underlying
    /// PKs pass point-on-curve and not-in-small-subgroup checks.
    /// Returns `false` otherwise.
    native fun public_key_validate_internal(bytes: vector<u8>): bool;

    /// Return true if the MultiEd25519 `multisignature` on `message` verifies against the MultiEd25519 `public_key`.
    /// Returns `false` if either:
    /// - The PKs in `public_key` do not all pass points-on-curve or not-in-small-subgroup checks,
    /// - The signatures in `multisignature` do not all pass points-on-curve or not-in-small-subgroup checks,
    /// - the `multisignature` on `message` does not verify.
    native fun signature_verify_strict_internal(
        multisignature: vector<u8>,
        public_key: vector<u8>,
        message: vector<u8>
    ): bool;

    //
    // Test cases
    //

    #[test]
    fun test_multisigs_verify() {
        // Generated via `cargo test -- test_sample_multisig --nocapture --ignored` in aptos-crypto/
        let msg = b"Hello Aptos!";
        //let ks = vector[1, 1, 2, 2, 3, 15, ]; // the thresholds, implicitly encoded in the public keys
        let ns = vector[1, 2, 2, 3, 10, 32, ];
        let pks = vector[
            x"20fdbac9b10b7587bba7b5bc163bce69e796d71e4ed44c10fcb4488689f7a14401",
            x"20fdbac9b10b7587bba7b5bc163bce69e796d71e4ed44c10fcb4488689f7a14475e4174dd58822548086f17b037cecb0ee86516b7d13400a80c856b4bdaf7fe101",
            x"20fdbac9b10b7587bba7b5bc163bce69e796d71e4ed44c10fcb4488689f7a14475e4174dd58822548086f17b037cecb0ee86516b7d13400a80c856b4bdaf7fe102",
            x"20fdbac9b10b7587bba7b5bc163bce69e796d71e4ed44c10fcb4488689f7a14475e4174dd58822548086f17b037cecb0ee86516b7d13400a80c856b4bdaf7fe1631c1541f3a4bf44d4d897061564aa8495d766f6191a3ff61562003f184b8c6502",
            x"20fdbac9b10b7587bba7b5bc163bce69e796d71e4ed44c10fcb4488689f7a14475e4174dd58822548086f17b037cecb0ee86516b7d13400a80c856b4bdaf7fe1631c1541f3a4bf44d4d897061564aa8495d766f6191a3ff61562003f184b8c65beada06126c78d98b4a1a69f6ee6189694f0f4751538da824f1adc8b14a1b5621c70c891a634681890df6fc4aa3b94d2100ba15a07c78a17908b6e32df190d4b1f7af5f1b0911ff4681d70e218f0dab399aa82366d65d765f2fb6acf61012f4840f4bb1fab3cc0e4fd912a2be61535f95dc30568fe9046c2aec60b55149232b6cb2ee1fdd39ab7ad7047fd2edb7c37f8fd6ea6a5a8b8009d2710036624c0937fd5a781494d2bf1a174ddffde1e02cb8881cff6dab70e61cbdef393deac0ce63909977148265c9527268549fe153b813dc3b3db58e29cabb85e68a454f734686303",
            x"20fdbac9b10b7587bba7b5bc163bce69e796d71e4ed44c10fcb4488689f7a14475e4174dd58822548086f17b037cecb0ee86516b7d13400a80c856b4bdaf7fe1631c1541f3a4bf44d4d897061564aa8495d766f6191a3ff61562003f184b8c65beada06126c78d98b4a1a69f6ee6189694f0f4751538da824f1adc8b14a1b5621c70c891a634681890df6fc4aa3b94d2100ba15a07c78a17908b6e32df190d4b1f7af5f1b0911ff4681d70e218f0dab399aa82366d65d765f2fb6acf61012f4840f4bb1fab3cc0e4fd912a2be61535f95dc30568fe9046c2aec60b55149232b6cb2ee1fdd39ab7ad7047fd2edb7c37f8fd6ea6a5a8b8009d2710036624c0937fd5a781494d2bf1a174ddffde1e02cb8881cff6dab70e61cbdef393deac0ce63909977148265c9527268549fe153b813dc3b3db58e29cabb85e68a454f7346863e32bbd493a9bfe127a20d95e98ab5593d15f5e1452ccebc6c529a7e05ec478c4a464066e617bcda466fcb09b4e77df86868db733b8f1169b663b14be16c22e950c288926cc2321d3dc2f7c17e525253ab46f4270ac1981861fc2bb9c09b02478ca04eef49a32901fe69d016e5e78c03110d2e4fe770ef560dedefb9bd11ce66eb5aba182715c0871496dcbe1dfb9816546c9232e036f8deeee7de716922f191c16cf820c21a447ec77c4eaa4802c0b5d73793cabe685896c029891d10bd98d8a5ee107cad9cabb0ffc46764a54894781ab616290608df1e89f86027bd55c73cb7438e6d500ba33f4257b92235265dbfb7d92bde4f43c2bdf7951ba73d813424ead048b3d81f2bda558d8f0e9df17a3d4a64bef906b7b4d8266ca0afba3216141ea453424ae00f94f14be249589ac7e30118c1a9cb0dff48a36b85b4a7caba24aafbbbe560496ffda9d51778273b439702816865c85852ac18ef8194aa2f55fdcca2afa54c73d73df6aea1ee2e5327021e34dde89ae8c3fe0361e8bbc81c32609aa659f4aa40217d7c63ec0ec4e35450ebc0660472e876d4e58d4b3a24892ef91e35ca64740a3d0cf7185bb24fa00b9bad89b71c5fcc19fb360e16cd8336da682d6e60b53a838d7838867d75e0c6d4840efbc0f19d8f6da258b31a969f8aab8f1b02584ee0ed899c50919aba7e929d17f3752db0e72cd3b6f55680715ce1152a330c475fa8c0cfff4b0d1d4efe701092f901bd086c140cf3d95e17be2f1e990c88d6b9d9d57b8bf3ba7151fb613cc39cfea2dc93d11933a9d9f86ba94ea6f504c5d79c0467665e02df41828856899e368de5a2388beb55d1d7dd9044c63e7132ae2eab0c5918b3b2cb82fa355c2f11a8466cf860d19b13e2275dbd1fc26cc6a5afdec11af57de751e89b8854b18d0313144b35caacf4d3f558112ef8baaf5ad2563a3f7459797861f02bece6aa2578f5b740dd05c17a6aedbc7594b277dd853bf0f",
        ];
        let sigs = vector[
            x"3040d9951e95f0a810b035453631045f6a3d887405bca808a305f3f2f729bf82044506ca122e9613191c2f378f4dac8868ddee3cd16c1c627c6e6f2abaf8660f80000000",
            x"3040d9951e95f0a810b035453631045f6a3d887405bca808a305f3f2f729bf82044506ca122e9613191c2f378f4dac8868ddee3cd16c1c627c6e6f2abaf8660f80000000",
            x"3040d9951e95f0a810b035453631045f6a3d887405bca808a305f3f2f729bf82044506ca122e9613191c2f378f4dac8868ddee3cd16c1c627c6e6f2abaf8660fac5697ef58896ed50ab4f1bbea62b2340743a685761a2569f3eecb80fe54432d6cb7f81a2e84ca9dce845b68970a26d8968ec72ec8bec29fda1a9559ebd06b01c0000000",
            x"3040d9951e95f0a810b035453631045f6a3d887405bca808a305f3f2f729bf82044506ca122e9613191c2f378f4dac8868ddee3cd16c1c627c6e6f2abaf8660fac5697ef58896ed50ab4f1bbea62b2340743a685761a2569f3eecb80fe54432d6cb7f81a2e84ca9dce845b68970a26d8968ec72ec8bec29fda1a9559ebd06b01c0000000",
            x"3040d9951e95f0a810b035453631045f6a3d887405bca808a305f3f2f729bf82044506ca122e9613191c2f378f4dac8868ddee3cd16c1c627c6e6f2abaf8660fac5697ef58896ed50ab4f1bbea62b2340743a685761a2569f3eecb80fe54432d6cb7f81a2e84ca9dce845b68970a26d8968ec72ec8bec29fda1a9559ebd06b019c5c562321b21a2c1a9c8352fd5748fb0985eca40736be075f973ed10b681a2fc17d4254ee3b313b87456c146d67c8316fd9f2df9902f3ea29a8b14bf1092607e0000000",
            x"3040d9951e95f0a810b035453631045f6a3d887405bca808a305f3f2f729bf82044506ca122e9613191c2f378f4dac8868ddee3cd16c1c627c6e6f2abaf8660fac5697ef58896ed50ab4f1bbea62b2340743a685761a2569f3eecb80fe54432d6cb7f81a2e84ca9dce845b68970a26d8968ec72ec8bec29fda1a9559ebd06b019c5c562321b21a2c1a9c8352fd5748fb0985eca40736be075f973ed10b681a2fc17d4254ee3b313b87456c146d67c8316fd9f2df9902f3ea29a8b14bf1092607ad2076e059350e1d38813ddcefaa0f1056662105f6ea81a70341b4ebe36d8cf6ce5519e2c48ec8c81c462aac3e67345eda388e37b610bfd07147c9d4f187e6036c546e05978b496e46474893455fb624d4e00293f208a15762905d7bb7ebaf8de40c9615558179f5059fbb1f6bf41b25c533b1763fb9c80774028d0d1093640c9dbfdbf64a362914f350a45733d94baef60ddc3336a51750172df5c0533e196cb702a7c73a76f95424bd0ac610b228638283e497ef71967d99861530c1ffc80b06f145a86943f328bc9653cfe258fd3427d58700d6ce06d702787598a0b844fc472d9ab7bd07ae7d7ae80e645c1c67ec019992f718e8df11371ba7dfa1818603d24150a257e4c8b5d08b8611eb4335fb4db0d9f877e238e885afb70f477a2fe6975531d1a99c95c8b461ad03020bbb848f8b9b2bbb7546757e47e29f8fb0530c33840f2179501324ee3cc47519721a055e34069410626c10bc22f5ba080802724389c9dd89bc4ec77c3d24ab29c829a04106d14c432815adcac5fd56abccc50a0f5e8dea85f5c2e50052916f4ad263a00c2061cf4cfd4b0c08b3094ec3694056f1f22944c29e3c2bff41dd1df8d3af22ba2d06d1aa67c8895f2c62b84a3d7001c83811ad32be1f388480c67ae115072b19acd964bc7e5b236ce3a5c4581e7d58ef678a8486e6a9b25eeaaddf962d57d8ba03310f725fce96aba2943f64172d0e8f1d8133df1d55ae33eddbf6d92f8325887a6e859d850239e625a06dc73de808f47592ab7c02c20890bf72f88281ad204205aea85b60df984d044b644ef43c058a954195ad168c17a02468c97fa69908484994e3bd52e17df84ad4c38006de9f89310ed14606f8984a7c6fec6a4215ad14bc46e8759905c9031cea7c72e88906c0cc90cf713937e7bc53833116df149e99407d93a4854ed4f3e337310b9cb48d5041bdb75b256083d8329a3dce3aa4da1dc4d670e56898337e9078633fbe130b38e054ee5dc96dbe59f41d2e92cb72d9a4de44719596ea676d4ff50f7e62e78ee110a7347a4512ecde1134173aed804cdc79ddc7a41b410cd67d2e7095621800fffe0000",
        ];

        let num = vector::length(&ns);
        let i = 0;
        while (i < num) {
            let pk_bytes = *vector::borrow(&pks, i);
            let sig_bytes = *vector::borrow(&sigs, i);

            let pk = new_validated_public_key_from_bytes(pk_bytes);
            let pk = std::option::extract(&mut pk);
            let pk = public_key_into_unvalidated(pk);
            let sig = new_signature_from_bytes(sig_bytes);

            assert!(signature_verify_strict(&sig, &pk, msg), 1);

            i = i + 1;
        }
    }
}
