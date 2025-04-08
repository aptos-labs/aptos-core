/// This module implements MultiKey type of public key.

module aptos_std::multi_key {
    use std::hash;
    use std::error;
    use aptos_std::single_key;
    //
    // Error codes
    //

    /// No keys were provided when creating a MultiKey public key.
    const E_INVALID_MULTI_KEY_NO_KEYS: u64 = 1;

    /// The number of keys provided is greater than the maximum allowed.
    const E_INVALID_MULTI_KEY_TOO_MANY_KEYS: u64 = 2;

    /// The number of signatures required is greater than the number of keys provided.
    const E_INVALID_MULTI_KEY_SIGNATURES_REQUIRED: u64 = 3;

    //
    // Constants
    //

    /// The identifier of the MultiEd25519 signature scheme, which is used when deriving Aptos authentication keys by hashing
    /// it together with an MultiEd25519 public key.
    const SIGNATURE_SCHEME_ID: u8 = 3;

    /// Max number of ed25519 public keys allowed in multi-ed25519 keys
    const MAX_NUMBER_OF_PUBLIC_KEYS: u64 = 32;

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

    public fun new_unvalidated_public_key_from_single_keys(single_keys: vector<single_key::UnvalidatedPublicKey>, signatures_required: u8): UnvalidatedPublicKey {
        let num_keys = single_keys.length();
        assert!(
            num_keys > 0,
            error::invalid_argument(E_INVALID_MULTI_KEY_NO_KEYS)
        );
        assert!(
            num_keys <= MAX_NUMBER_OF_PUBLIC_KEYS,
            error::invalid_argument(E_INVALID_MULTI_KEY_TOO_MANY_KEYS)
        );
        assert!(
            (signatures_required as u64) <= num_keys,
            error::invalid_argument(E_INVALID_MULTI_KEY_SIGNATURES_REQUIRED)
        );
        let bytes = vector[num_keys as u8];
        for (i in 0..single_keys.length()) {
            bytes.append(single_key::unvalidated_public_key_to_bytes(&single_keys[i]));
        };
        bytes.push_back(signatures_required);
        UnvalidatedPublicKey { bytes }
    }

    /// Serializes an UnvalidatedPublicKey struct to byte vec.
    public fun unvalidated_public_key_to_bytes(pk: &UnvalidatedPublicKey): vector<u8> {
        pk.bytes
    }

    /// Derives the Aptos-specific authentication key of the given MultiKey public key.
    public fun unvalidated_public_key_to_authentication_key(pk: &UnvalidatedPublicKey): vector<u8> {
        public_key_bytes_to_authentication_key(pk.bytes)
    }

    /// Derives the Aptos-specific authentication key of the given MultiKey public key.
    fun public_key_bytes_to_authentication_key(pk_bytes: vector<u8>): vector<u8> {
        pk_bytes.push_back(SIGNATURE_SCHEME_ID);
        hash::sha3_256(pk_bytes)
    }

    #[test]
    fun test_construct_multi_key() {
        let pk1 = single_key::new_unvalidated_public_key_from_bytes(x"002222");
        let pk2 = single_key::new_unvalidated_public_key_from_bytes(x"021111");
        let multi_key = new_unvalidated_public_key_from_single_keys(vector[pk1, pk2], 1);

        let mk_bytes: vector<u8> = x"0200222202111101";
        assert!(multi_key.bytes == mk_bytes, std::error::invalid_state(1));
    }

    #[test]
    fun test_get_authentication_key() {
        let mk_bytes: vector<u8> = x"02031b68747470733a2f2f6163636f756e74732e676f6f676c652e636f6d2086bc0a0a825eb6337ca1e8a3157e490eac8df23d5cef25d9641ad5e7edc1d51400205da515f392de68080051559c9d9898f5feb377f0b0f15d43fd01c98f0a63b0d801";
        let multi_key = new_unvalidated_public_key_from_bytes(mk_bytes);
        assert!(
            unvalidated_public_key_to_authentication_key(&multi_key) == x"c7ab91daf558b00b1f81207b702349a74029dddfbf0e99d54b3d7675714a61de",
            std::error::invalid_state(1)
        );
    }
}
