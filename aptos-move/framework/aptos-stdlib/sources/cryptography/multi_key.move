module aptos_std::multi_key {
    use std::hash;
    use std::vector;

    //
    // Error codes
    //

    //
    // Constants
    //

    /// The identifier of the MultiEd25519 signature scheme, which is used when deriving Aptos authentication keys by hashing
    /// it together with an MultiEd25519 public key.
    const SIGNATURE_SCHEME_ID: u8 = 3;

    /// An *unvalidated*, k out of n MultiKey public key. The `bytes` field contains (1) a vector of single key public keys and
    /// (2) a single byte encoding the threshold k.
    /// *Unvalidated* means there is no guarantee that the underlying PKs are valid elliptic curve points of non-small
    /// order.  Nor is there a guarantee that it would deserialize correctly (i.e., for Keyless public keys).
    struct UnvalidatedPublicKey has copy, drop, store {
        bytes: vector<u8>
    }

    //
    // Functions
    //

    public fun new_unvalidated_public_key_from_bytes(bytes: vector<u8>): UnvalidatedPublicKey {
        UnvalidatedPublicKey { bytes }
    }

    /// Derives the Aptos-specific authentication key of the given MultiKey public key.
    public fun unvalidated_public_key_to_authentication_key(pk: &UnvalidatedPublicKey): vector<u8> {
        public_key_bytes_to_authentication_key(pk.bytes)
    }

    /// Derives the Aptos-specific authentication key of the given MultiKey public key.
    fun public_key_bytes_to_authentication_key(pk_bytes: vector<u8>): vector<u8> {
        pk_bytes.push_back(SIGNATURE_SCHEME_ID)
        hash::sha3_256(pk_bytes)
    }
}
