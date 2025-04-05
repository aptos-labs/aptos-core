module aptos_std::single_key {
    use aptos_std::ed25519;
    use std::hash;
    //
    // Error codes
    //

    /// Wrong number of bytes were given as input when deserializing an Ed25519 public key.
    const E_INVALID_PUBLIC_KEY_TYPE: u64 = 1;

    /// Failed to deserialize the public key.
    const E_FAILED_TO_DESERIALIZE: u64 = 2;

    /// Wrong number of bytes were given as input when deserializing an Ed25519 signature.
    const E_INVALID_SIGNATURE_SCHEME: u64 = 3;

    //
    // Constants
    //

    /// The identifier of the Single Key signature scheme, which is used when deriving Aptos authentication keys by hashing
    /// it together with an Single Key public key.
    const SIGNATURE_SCHEME_ID: u8 = 3;

    /// Scheme identifier for Ed25519 single keys.
    const ED25519_PUBLIC_KEY_TYPE: u8 = 0;

    /// Scheme identifier for SECP256K1 single keys.
    const SECP256K1_PUBLIC_KEY_TYPE: u8 = 1;

    /// Scheme identifier for WebAuthn single keys.
    const WEB_AUTHN_PUBLIC_KEY_TYPE: u8 = 2;

    /// Scheme identifier for Keyless single keys.
    const KEYLESS_PUBLIC_KEY_TYPE: u8 = 3;

    /// Scheme identifier for Federated Keyless single keys.
    const FEDERATED_KEYLESS_PUBLIC_KEY_TYPE: u8 = 4;

    //
    // Structs
    //

    /// An *unvalidated* any public key: not necessarily an elliptic curve point, just a sequence of 32 bytes
    struct UnvalidatedPublicKey has copy, drop, store {
        bytes: vector<u8>
    }

    //
    // Functions
    //

    /// Parses the input bytes as an *unvalidated* single key.  It does check that the first byte is a valid scheme identifier.
    public fun new_unvalidated_public_key_from_bytes(bytes: vector<u8>): UnvalidatedPublicKey {
        let first_byte = bytes[0];
        assert!(first_byte <= 4, std::error::invalid_argument(E_INVALID_PUBLIC_KEY_TYPE));
        UnvalidatedPublicKey { bytes }
    }

    /// Serializes an UnvalidatedPublicKey struct to 32-bytes.
    public fun unvalidated_public_key_to_bytes(pk: &UnvalidatedPublicKey): vector<u8> {
        pk.bytes
    }

    /// Converts an unvalidated Ed25519 public key to an unvalidated single key public key.
    /// We do this by prepending the scheme identifier and the length of the public key (32 bytes or 0x20 in hex) to
    /// the public key bytes.
    public fun from_ed25519_public_key_unvalidated(pk: &ed25519::UnvalidatedPublicKey): UnvalidatedPublicKey {
        let pk_bytes = vector[];
        pk_bytes.push_back(ED25519_PUBLIC_KEY_TYPE);
        pk_bytes.push_back(0x20);
        std::vector::append(&mut pk_bytes, ed25519::unvalidated_public_key_to_bytes(pk));
        UnvalidatedPublicKey {
            bytes: pk_bytes
        }
    }

    /// Derives the Aptos-specific authentication key of the given single key public key.
    public fun unvalidated_public_key_to_authentication_key(pk: &UnvalidatedPublicKey): vector<u8> {
        public_key_bytes_to_authentication_key(pk.bytes)
    }

    /// Derives the Aptos-specific authentication key of the given bytes of a single key public key.
    fun public_key_bytes_to_authentication_key(pk_bytes: vector<u8>): vector<u8> {
        pk_bytes.push_back(SIGNATURE_SCHEME_ID);
        hash::sha3_256(pk_bytes)
    }
}
