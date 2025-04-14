/// This module implements the Federated Keyless authentication scheme.

module aptos_std::federated_keyless {
    use aptos_std::bcs_stream;
    use aptos_std::keyless;
    use aptos_std::error;

    #[test_only]
    use aptos_std::bcs;
    #[test_only]
    use std::string::{utf8};

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

    #[test]
    fun test_deserialize_public_key() {
        // The bytes below represent a Federated Keyless public key that looks like
        // federated_keyless::PublicKey {
        //     jwk_address: @0xaa9b5e7acc48169fdc3809b614532a5a675cf7d4c80cd4aea732b47e328bda1a,
        //     keyless_public_key: keyless::PublicKey {
        //         iss: "https://accounts.google.com",
        //         idc: "0x86bc0a0a825eb6337ca1e8a3157e490eac8df23d5cef25d9641ad5e7edc1d514"
        //     }
        // }
        //
        let bytes = x"aa9b5e7acc48169fdc3809b614532a5a675cf7d4c80cd4aea732b47e328bda1a1b68747470733a2f2f6163636f756e74732e676f6f676c652e636f6d2086bc0a0a825eb6337ca1e8a3157e490eac8df23d5cef25d9641ad5e7edc1d514";
        let pk = new_public_key_from_bytes(bytes);
        assert!(
            bcs::to_bytes(&pk) == bytes,
        );
        assert!(
            pk.keyless_public_key.get_iss() == utf8(b"https://accounts.google.com"),
        );
        assert!(
            pk.keyless_public_key.get_idc() == x"86bc0a0a825eb6337ca1e8a3157e490eac8df23d5cef25d9641ad5e7edc1d514",
        );
        assert!(
            pk.jwk_address == @0xaa9b5e7acc48169fdc3809b614532a5a675cf7d4c80cd4aea732b47e328bda1a,
        );
    }
}
