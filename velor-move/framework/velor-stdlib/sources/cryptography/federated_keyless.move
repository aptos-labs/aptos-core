/// This module implements the Federated Keyless authentication scheme.

module velor_std::federated_keyless {
    use velor_std::bcs_stream;
    use velor_std::keyless;
    use velor_std::error;

    #[test_only]
    friend velor_std::federated_keyless_tests;

    //
    // Error codes
    //

    /// There are extra bytes in the input when deserializing a Federated Keyless public key.
    const E_INVALID_FEDERATED_KEYLESS_PUBLIC_KEY_EXTRA_BYTES: u64 = 1;

    //
    // Constants
    //

    //
    // Structs
    //

    /// An *unvalidated* any public key: not necessarily an elliptic curve point, just a sequence of 32 bytes
    struct PublicKey has copy, drop, store {
        jwk_address: address,
        keyless_public_key: keyless::PublicKey,
    }

    //
    // Functions
    //

    /// Parses the input bytes into a keyless public key.
    public fun new_public_key_from_bytes(bytes: vector<u8>): PublicKey {
        let stream = bcs_stream::new(bytes);
        let pk = deserialize_public_key(&mut stream);
        assert!(!bcs_stream::has_remaining(&mut stream), error::invalid_argument(E_INVALID_FEDERATED_KEYLESS_PUBLIC_KEY_EXTRA_BYTES));
        pk
    }

    /// Deserializes a Federated Keyless public key from a BCS stream.
    public fun deserialize_public_key(stream: &mut bcs_stream::BCSStream): PublicKey {
        let jwk_address = bcs_stream::deserialize_address(stream);
        let keyless_public_key = keyless::deserialize_public_key(stream);
        PublicKey { keyless_public_key, jwk_address }
    }

    /// Creates a new Federated Keyless public key from a keyless public key and a JWK address.
    public fun new(keyless_public_key: keyless::PublicKey, jwk_address: address): PublicKey {
        PublicKey { keyless_public_key, jwk_address }
    }

    /// Returns the identifier bytes of the public key
    friend fun get_jwk_address(self: &PublicKey): address {
        self.jwk_address
    }

    /// Returns the keyless public key of the public key
    friend fun get_keyless_public_key(self: &PublicKey): keyless::PublicKey {
        self.keyless_public_key
    }
}
