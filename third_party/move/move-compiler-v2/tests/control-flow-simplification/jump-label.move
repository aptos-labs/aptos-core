module 0x42::test {
    use std::option::{Self, Option};
    use std::vector;

    //
    // Error codes
    //

    /// Wrong number of bytes were given as input when deserializing an Ed25519 public key.
    const E_WRONG_PUBKEY_SIZE: u64 = 1;

    /// Wrong number of bytes were given as input when deserializing an Ed25519 signature.
    const E_WRONG_SIGNATURE_SIZE: u64 = 2;

    /// The threshold must be in the range `[1, n]`, where n is the total number of signers.
    const E_INVALID_THRESHOLD_OR_NUMBER_OF_SIGNERS: u64 = 3;

    /// The native functions have not been rolled out yet.
    const E_NATIVE_FUN_NOT_AVAILABLE: u64 = 4;

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
	/// Checks that the serialized format of a t-out-of-n MultiEd25519 PK correctly encodes 1 <= n <= 32 sub-PKs.
    /// (All `ValidatedPublicKey` objects are guaranteed to pass this check.)
    /// Returns the threshold t <= n of the PK.
    public fun check_and_get_threshold(bytes: vector<u8>): Option<u8> {
        let len = vector::length(&bytes);
        if (len == 0) {
            return option::none<u8>()
        };

        let threshold_num_of_bytes = len % INDIVIDUAL_PUBLIC_KEY_NUM_BYTES;
        let num_of_keys = len / INDIVIDUAL_PUBLIC_KEY_NUM_BYTES;
        let threshold_byte = *vector::borrow(&bytes, len - 1);

        if (num_of_keys == 0 || num_of_keys > MAX_NUMBER_OF_PUBLIC_KEYS || threshold_num_of_bytes != 1) {
            return option::none<u8>()
        } else if (threshold_byte == 0 || threshold_byte > (num_of_keys as u8)) {
            return option::none<u8>()
        } else {
            return option::some(threshold_byte)
        }
    }
}
