/// This module implements Single Key representations of public keys.
/// It is used to represent public keys for the Ed25519, SECP256K1, WebAuthn, and Keyless schemes in a unified way.

module aptos_std::keyless {
    use aptos_std::bcs_stream::{Self, deserialize_u8};
    use std::string::{String, utf8};
    use std::bcs;
    friend aptos_std::federated_keyless;

    
    // Error codes
    //

    /// There are extra bytes in the input when deserializing a Keyless public key.
    const E_INVALID_KEYLESS_PUBLIC_KEY_EXTRA_BYTES: u64 = 1;

    //
    // Constants
    //

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
        assert!(bcs_stream::has_remaining(&mut stream) == false, std::error::invalid_argument(E_INVALID_KEYLESS_PUBLIC_KEY_EXTRA_BYTES));
        key
    }

    public fun deserialize_public_key(stream: &mut bcs_stream::BCSStream): PublicKey {
        let iss = bcs_stream::deserialize_string(stream);
        let idc = bcs_stream::deserialize_vector(stream, |x| deserialize_u8(x));
        PublicKey { iss, idc }
    }

    public fun new(iss: String, idc: vector<u8>): PublicKey {
        PublicKey { iss, idc }
    }

    /// Returns the issuer string of the public key
    public fun get_iss(self: &PublicKey): String {
        self.iss
    }

    /// Returns the identifier bytes of the public key
    public fun get_idc(self: &PublicKey): vector<u8> {
        self.idc
    }

    #[test]
    fun test_deserialize_public_key() {
        let bytes: vector<u8> = x"1b68747470733a2f2f6163636f756e74732e676f6f676c652e636f6d2086bc0a0a825eb6337ca1e8a3157e490eac8df23d5cef25d9641ad5e7edc1d514";
        let pk = new_public_key_from_bytes(bytes);
        assert!(
            bcs::to_bytes(&pk) == bytes,
            std::error::invalid_state(1)
        );
        assert!(
            pk.iss == utf8(b"https://accounts.google.com"),
            std::error::invalid_state(2)
        );
        assert!(
            pk.idc == x"86bc0a0a825eb6337ca1e8a3157e490eac8df23d5cef25d9641ad5e7edc1d514",
            std::error::invalid_state(3)
        );
    }
}
