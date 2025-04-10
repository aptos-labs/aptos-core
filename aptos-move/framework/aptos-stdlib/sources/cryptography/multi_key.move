/// This module implements MultiKey type of public key.
/// A MultiKey public key is a collection of single key public keys and a number representing the number of signatures required to authenticate a transaction.
/// Unlike MultiEd25519, the individual single keys can be of different schemes.
module aptos_std::multi_key {
    use aptos_std::single_key;
    use std::hash;
    use std::error;
    use std::bcs_stream;
    use std::bcs;
    //
    // Error codes
    //

    /// No keys were provided when creating a MultiKey public key.
    const E_INVALID_MULTI_KEY_NO_KEYS: u64 = 1;

    /// The number of keys provided is greater than the maximum allowed.
    const E_INVALID_MULTI_KEY_TOO_MANY_KEYS: u64 = 2;

    /// The number of signatures required is greater than the number of keys provided.
    const E_INVALID_MULTI_KEY_SIGNATURES_REQUIRED: u64 = 3;

    /// There are extra bytes in the input when deserializing a MultiKey public key.
    const E_INVALID_MULTI_KEY_EXTRA_BYTES: u64 = 4;

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
    struct MultiKey has copy, drop, store {
        public_keys: vector<single_key::AnyPublicKey>,
        signatures_required: u8
    }

    //
    // Functions
    //

    public fun new_public_key_from_bytes(bytes: vector<u8>): MultiKey {
        let stream = bcs_stream::new(bytes);
        let pk = deserialize_multi_key(&mut stream);
        assert!(bcs_stream::has_remaining(&mut stream) == false, std::error::invalid_argument(E_INVALID_MULTI_KEY_EXTRA_BYTES));
        pk
    }

    public fun new_multi_key_from_single_keys(single_keys: vector<single_key::AnyPublicKey>, signatures_required: u8): MultiKey {
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
        MultiKey { public_keys: single_keys, signatures_required }
    }

    public fun deserialize_multi_key(stream: &mut bcs_stream::BCSStream): MultiKey {
        let public_keys = bcs_stream::deserialize_vector(stream, |x| single_key::deserialize_any_public_key(x));
        let signatures_required = bcs_stream::deserialize_u8(stream);
        MultiKey { public_keys, signatures_required }
    }

    public fun to_authentication_key(self: &MultiKey): vector<u8> {
        let pk_bytes = bcs::to_bytes(self);
        pk_bytes.push_back(SIGNATURE_SCHEME_ID);
        hash::sha3_256(pk_bytes)
    }

    #[test]
    fun test_construct_multi_key() {
        let pk1 = single_key::new_public_key_from_bytes(x"0020aa9b5e7acc48169fdc3809b614532a5a675cf7d4c80cd4aea732b47e328bda1a");
        let pk2 = single_key::new_public_key_from_bytes(x"0020bd182d6e3f4ad1daf0d94e53daaece63ebd571d8a8e0098a02a4c0b4ecc7c99e");
        let multi_key = new_multi_key_from_single_keys(vector[pk1, pk2], 1);
        let mk_bytes: vector<u8> = x"020020aa9b5e7acc48169fdc3809b614532a5a675cf7d4c80cd4aea732b47e328bda1a0020bd182d6e3f4ad1daf0d94e53daaece63ebd571d8a8e0098a02a4c0b4ecc7c99e01";
        assert!(bcs::to_bytes(&multi_key) == mk_bytes, std::error::invalid_state(1));
    }

    #[test]
    #[expected_failure(abort_code = 0x10003, location = Self)]
    fun test_construct_multi_key_bad_input_signatures_required_too_large() {
        let pk1 = single_key::new_public_key_from_bytes(x"0020aa9b5e7acc48169fdc3809b614532a5a675cf7d4c80cd4aea732b47e328bda1a");
        let pk2 = single_key::new_public_key_from_bytes(x"0020bd182d6e3f4ad1daf0d94e53daaece63ebd571d8a8e0098a02a4c0b4ecc7c99e");
        let _multi_key = new_multi_key_from_single_keys(vector[pk1, pk2], 3);
    }

    #[test]
    #[expected_failure(abort_code = 0x10001, location = Self)]
    fun test_construct_multi_key_bad_input_no_keys() {
        let _multi_key = new_multi_key_from_single_keys(vector[], 1);
    }

    #[test]
    fun test_construct_multi_key_from_bytes() {
        let mk_bytes: vector<u8> = x"020020aa9b5e7acc48169fdc3809b614532a5a675cf7d4c80cd4aea732b47e328bda1a0020bd182d6e3f4ad1daf0d94e53daaece63ebd571d8a8e0098a02a4c0b4ecc7c99e01";
        let multi_key = new_public_key_from_bytes(mk_bytes);
        assert!(bcs::to_bytes(&multi_key) == mk_bytes, std::error::invalid_state(1));
    }

    #[test]
    #[expected_failure(abort_code = 0x10004, location = Self)]
    fun test_construct_multi_key_from_bytes_bad_input_extra_bytes() {
        let mk_bytes: vector<u8> = x"020020aa9b5e7acc48169fdc3809b614532a5a675cf7d4c80cd4aea732b47e328bda1a0020bd182d6e3f4ad1daf0d94e53daaece63ebd571d8a8e0098a02a4c0b4ecc7c99e01";
        mk_bytes.push_back(0x01);
        let _multi_key = new_public_key_from_bytes(mk_bytes);
    }

    #[test]
    fun test_get_authentication_key() {
        let mk_bytes: vector<u8> = x"02031b68747470733a2f2f6163636f756e74732e676f6f676c652e636f6d2086bc0a0a825eb6337ca1e8a3157e490eac8df23d5cef25d9641ad5e7edc1d51400205da515f392de68080051559c9d9898f5feb377f0b0f15d43fd01c98f0a63b0d801";
        let multi_key = new_public_key_from_bytes(mk_bytes);
        assert!(
            multi_key.to_authentication_key() == x"c7ab91daf558b00b1f81207b702349a74029dddfbf0e99d54b3d7675714a61de",
            std::error::invalid_state(1)
        );
    }
}
