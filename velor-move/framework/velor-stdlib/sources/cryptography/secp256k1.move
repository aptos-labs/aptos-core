/// This module implements ECDSA signatures based on the prime-order secp256k1 ellptic curve (i.e., cofactor is 1).

module velor_std::secp256k1 {
    use std::option::Option;

    /// An error occurred while deserializing, for example due to wrong input size.
    const E_DESERIALIZE: u64 = 1;   // This code must be the same, if ever returned from the native Rust implementation.

    /// Recovery ID needs to be either 0, 1, 2 or 3. If you are recovering from an (r, s, v) Ethereum signature, take its v value and, set the recovery_id as follows: if v == 27, set to 0, if v == 28, set to 1, if v == 37, set to 0, if v == 38, set to 1.
    const E_BAD_RECOVERY_ID: u64 = 2;

    /// The size of a secp256k1-based ECDSA public key, in bytes.
    const RAW_PUBLIC_KEY_NUM_BYTES: u64 = 64;
    //const COMPRESSED_PUBLIC_KEY_SIZE: u64 = 33;

    /// The size of a secp256k1-based ECDSA signature, in bytes.
    const SIGNATURE_NUM_BYTES: u64 = 64;

    /// A 64-byte ECDSA public key.
    struct ECDSARawPublicKey has copy, drop, store {
        bytes: vector<u8>
    }

    /// A 64-byte ECDSA signature.
    struct ECDSASignature has copy, drop, store {
        bytes: vector<u8>
    }

    /// Constructs an ECDSASignature struct from the given 64 bytes.
    public fun ecdsa_signature_from_bytes(bytes: vector<u8>): ECDSASignature {
        assert!(bytes.length() == SIGNATURE_NUM_BYTES, std::error::invalid_argument(E_DESERIALIZE));
        ECDSASignature { bytes }
    }

    /// Constructs an ECDSARawPublicKey struct, given a 64-byte raw representation.
    public fun ecdsa_raw_public_key_from_64_bytes(bytes: vector<u8>): ECDSARawPublicKey {
        assert!(bytes.length() == RAW_PUBLIC_KEY_NUM_BYTES, std::error::invalid_argument(E_DESERIALIZE));
        ECDSARawPublicKey { bytes }
    }

    /// Serializes an ECDSARawPublicKey struct to 64-bytes.
    public fun ecdsa_raw_public_key_to_bytes(pk: &ECDSARawPublicKey): vector<u8> {
        pk.bytes
    }

    /// Serializes an ECDSASignature struct to 64-bytes.
    public fun ecdsa_signature_to_bytes(sig: &ECDSASignature): vector<u8> {
        sig.bytes
    }

    /// Recovers the signer's raw (64-byte) public key from a secp256k1 ECDSA `signature` given the (2-bit) `recovery_id`
    /// and the signed `message` (32 byte digest).
    ///
    /// This recovery algorithm can only be used to check validity of a signature if the signer's public key (or its
    /// hash) is known beforehand. When the algorithm returns a public key `pk`, this means that the signature in
    /// `signature` verified on `message` under that `pk`. But, again, that is only meaningful if `pk` is the "right"
    /// one (e.g., in Ethereum, the "right" `pk` is the one whose hash matches the account's address).
    ///
    /// If you do not understand this nuance, please learn more about ECDSA and pubkey recovery (see
    /// https://alinush.github.io/ecdsa#pubkey-recovery), or you risk writing completely-insecure code.
    ///
    /// Note: This function does not apply any additional hashing on the `message`; it simply passes in the message as
    /// raw bytes to the ECDSA recovery function. (The max allowed size ~32 bytes.)
    ///  + Nonetheless, most applications will first hash the message to be signed. So, typically, `message` here tends
    ///    to be a hash rather than an actual message. Therefore, the developer should be aware of what hash function
    ///    was used for this.
    ///  + In particular, if using this function to verify an Ethereum signature, you will likely have to input
    ///    a keccak256 hash of the message as the `message` parameter.
    public fun ecdsa_recover(
        message: vector<u8>,
        recovery_id: u8,
        signature: &ECDSASignature,
    ): Option<ECDSARawPublicKey> {

        // If recovery ID is not 0 or 1 or 2 or 3, help the caller out by aborting with `E_BAD_RECOVERY_ID`
        if(recovery_id != 0 && recovery_id != 1 && recovery_id != 2 && recovery_id != 3) {
            abort std::error::invalid_argument(E_BAD_RECOVERY_ID);
        };

        let (pk, success) = ecdsa_recover_internal(message, recovery_id, signature.bytes);

        if (success) {
            std::option::some(ecdsa_raw_public_key_from_64_bytes(pk))
        } else {
            std::option::none<ECDSARawPublicKey>()
        }
    }

    //
    // Native functions
    //

    /// Returns `(public_key, true)` if `signature` verifies on `message` under the recovered `public_key`
    /// and returns `([], false)` otherwise.
    native fun ecdsa_recover_internal(
        message: vector<u8>,
        recovery_id: u8,
        signature: vector<u8>
    ): (vector<u8>, bool);

    //
    // Tests
    //

    #[test]
    #[expected_failure(abort_code = 65538, location = Self)]
    /// Tests that bad recovery IDs get rejected
    fun test_bad_ecdsa_recovery_id() {
        let _ = ecdsa_recover(
            b"test velor secp256k1",
            4,
            &ECDSASignature { bytes: x"f7ad936da03f948c14c542020e3c5f4e02aaacd1f20427c11aa6e2fbf8776477646bba0e1a37f9e7c777c423a1d2849baafd7ff6a9930814a43c3f80d59db56f" },
        );
    }

    #[test]
    /// Test on a valid secp256k1 ECDSA signature created using sk = x"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
    fun test_ecdsa_recover() {
        use std::hash;

        let pk = ecdsa_recover(
            hash::sha2_256(b"test velor secp256k1"),
            0,
            &ECDSASignature { bytes: x"f7ad936da03f948c14c542020e3c5f4e02aaacd1f20427c11aa6e2fbf8776477646bba0e1a37f9e7c777c423a1d2849baafd7ff6a9930814a43c3f80d59db56f" },
        );
        assert!(pk.is_some(), 1);
        assert!(
            pk.extract().bytes == x"4646ae5047316b4230d0086c8acec687f00b1cd9d1dc634f6cb358ac0a9a8ffffe77b4dd0a4bfb95851f3b7355c781dd60f8418fc8a65d14907aff47c903a559", 1);

        // Flipped bits; Signature stays valid
        let pk = ecdsa_recover(
            hash::sha2_256(b"test velor secp256k1"),
            0,
            // NOTE: A '7' was flipped to an 'f' here
            &ECDSASignature { bytes: x"f7ad936da03f948c14c542020e3c5f4e02aaacd1f20427c11aa6e2fbf8776477646bba0e1a37f9e7c7f7c423a1d2849baafd7ff6a9930814a43c3f80d59db56f" },
        );
        assert!(pk.is_some(), 1);
        assert!(
            pk.extract().bytes != x"4646ae5047316b4230d0086c8acec687f00b1cd9d1dc634f6cb358ac0a9a8ffffe77b4dd0a4bfb95851f3b7355c781dd60f8418fc8a65d14907aff47c903a559", 1);

        // Flipped bits; Signature becomes invalid
        let pk = ecdsa_recover(
            hash::sha2_256(b"test velor secp256k1"),
            0,
            &ECDSASignature { bytes: x"ffad936da03f948c14c542020e3c5f4e02aaacd1f20427c11aa6e2fbf8776477646bba0e1a37f9e7c7f7c423a1d2849baafd7ff6a9930814a43c3f80d59db56f" },
        );
        assert!(pk.is_none(), 1);
    }
}
