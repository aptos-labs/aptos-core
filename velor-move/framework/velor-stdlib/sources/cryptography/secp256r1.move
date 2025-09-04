/// This module implements ECDSA signatures based on the prime-order secp256r1 ellptic curve (i.e., cofactor is 1).

module velor_std::secp256r1 {

    /// An error occurred while deserializing, for example due to wrong input size.
    const E_DESERIALIZE: u64 = 1;   // This code must be the same, if ever returned from the native Rust implementation.

    /// The size of a secp256k1-based ECDSA public key, in bytes.
    const RAW_PUBLIC_KEY_NUM_BYTES: u64 = 64;
    //const COMPRESSED_PUBLIC_KEY_SIZE: u64 = 33;

    /// A 64-byte ECDSA public key.
    struct ECDSARawPublicKey has copy, drop, store {
        bytes: vector<u8>
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
}
