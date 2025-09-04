/// This module implements the Keyless authentication scheme.

module velor_std::keyless {
    use velor_std::bcs_stream::{Self, deserialize_u8};
    use std::error;
    use std::string::{Self, String};
    friend velor_std::federated_keyless;

    #[test_only]
    friend velor_std::keyless_tests;
    #[test_only]
    friend velor_std::federated_keyless_tests;

    // Error codes
    //

    /// There are extra bytes in the input when deserializing a Keyless public key.
    const E_INVALID_KEYLESS_PUBLIC_KEY_EXTRA_BYTES: u64 = 1;

    /// The length of the identifier commitment bytes in a Keyless public key is invalid.
    const E_INVALID_ID_COMMITMENT_BYTES_LENGTH: u64 = 2;

    /// The length of the issuer string in a Keyless public key is invalid.
    const E_INVALID_ISSUER_UTF8_BYTES_LENGTH: u64 = 3;

    //
    // Constants
    //

    /// The length of the identifier commitment bytes in a Keyless public key.
    const ID_COMMITMENT_BYTES_LENGTH: u64 = 32;

    /// The maximum length of the issuer string in bytes in a Keyless public key.
    const MAX_ISSUER_UTF8_BYTES_LENGTH: u64 = 120;

    //
    // Structs
    //

    /// An *unvalidated* any public key: not necessarily an elliptic curve point, just a sequence of 32 bytes
    struct PublicKey has copy, drop, store {
        iss: String,
        idc: vector<u8>
    }

    //
    // Functions
    //

    /// Parses the input bytes into a keyless public key.
    public fun new_public_key_from_bytes(bytes: vector<u8>): PublicKey {
        let stream = bcs_stream::new(bytes);
        let key = deserialize_public_key(&mut stream);
        assert!(!bcs_stream::has_remaining(&mut stream), error::invalid_argument(E_INVALID_KEYLESS_PUBLIC_KEY_EXTRA_BYTES));
        key
    }

    /// Deserializes a keyless public key from a BCS stream.
    public fun deserialize_public_key(stream: &mut bcs_stream::BCSStream): PublicKey {
        let iss = bcs_stream::deserialize_string(stream);
        let idc = bcs_stream::deserialize_vector(stream, |x| deserialize_u8(x));
        new(iss, idc)
    }

    /// Creates a new keyless public key from an issuer string and an identifier bytes.
    public fun new(iss: String, idc: vector<u8>): PublicKey {
        assert!(string::bytes(&iss).length() <= MAX_ISSUER_UTF8_BYTES_LENGTH, error::invalid_argument(E_INVALID_ISSUER_UTF8_BYTES_LENGTH));
        assert!(idc.length() == ID_COMMITMENT_BYTES_LENGTH, error::invalid_argument(E_INVALID_ID_COMMITMENT_BYTES_LENGTH));
        PublicKey { iss, idc }
    }

    /// Returns the issuer string of the public key
    friend fun get_iss(self: &PublicKey): String {
        self.iss
    }

    /// Returns the identifier bytes of the public key
    friend fun get_idc(self: &PublicKey): vector<u8> {
        self.idc
    }
}
