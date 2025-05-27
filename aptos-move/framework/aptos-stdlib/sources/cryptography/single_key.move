/// This module implements Single Key representations of public keys.
/// It is used to represent public keys for the Ed25519, SECP256K1, WebAuthn, and Keyless schemes in a unified way.

module aptos_std::single_key {
    use aptos_std::bcs_stream::{Self, deserialize_u8};
    use aptos_std::ed25519;
    use aptos_std::keyless;
    use aptos_std::secp256k1;
    use aptos_std::secp256r1;
    use aptos_std::bcs;
    use aptos_std::federated_keyless;
    use std::error;
    use std::hash;

    // Error codes
    //

    /// Unrecognized public key type.
    const E_INVALID_PUBLIC_KEY_TYPE: u64 = 1;

    /// There are extra bytes in the input when deserializing a Single Key public key.
    const E_INVALID_SINGLE_KEY_EXTRA_BYTES: u64 = 2;

    //
    // Constants
    //

    /// The identifier of the Single Key signature scheme, which is used when deriving Aptos authentication keys by hashing
    /// it together with an Single Key public key.
    const SIGNATURE_SCHEME_ID: u8 = 2;

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

    enum AnyPublicKey has copy, drop, store {
        Ed25519{pk: ed25519::UnvalidatedPublicKey},
        Secp256k1Ecdsa{pk: secp256k1::ECDSARawPublicKey},
        Secp256r1Ecdsa{pk: secp256r1::ECDSARawPublicKey},
        Keyless{pk: keyless::PublicKey},
        FederatedKeyless{pk: federated_keyless::PublicKey}
    }

    //
    // Functions
    //

    /// Parses the input bytes as a AnyPublicKey. The public key bytes are not guaranteed to be a valid
    /// representation of a point on its corresponding curve if applicable.
    /// It does check that the bytes deserialize into a well-formed public key for the given scheme.
    public fun new_public_key_from_bytes(bytes: vector<u8>): AnyPublicKey {
        let stream = bcs_stream::new(bytes);
        let pk = deserialize_any_public_key(&mut stream);
        assert!(!bcs_stream::has_remaining(&mut stream), error::invalid_argument(E_INVALID_SINGLE_KEY_EXTRA_BYTES));
        pk
    }

    /// Deserializes a Single Key public key from a BCS stream.
    public fun deserialize_any_public_key(stream: &mut bcs_stream::BCSStream): AnyPublicKey {
        let scheme_id = bcs_stream::deserialize_u8(stream);
        let pk: AnyPublicKey;
        if (scheme_id == ED25519_PUBLIC_KEY_TYPE) {
            let public_key_bytes = bcs_stream::deserialize_vector(stream, |x| deserialize_u8(x));
            pk = AnyPublicKey::Ed25519{pk: ed25519::new_unvalidated_public_key_from_bytes(public_key_bytes)}
        } else if (scheme_id == SECP256K1_PUBLIC_KEY_TYPE) {
            let public_key_bytes = bcs_stream::deserialize_vector(stream, |x| deserialize_u8(x));
            pk = AnyPublicKey::Secp256k1Ecdsa{pk: secp256k1::ecdsa_raw_public_key_from_64_bytes(public_key_bytes)};
        } else if (scheme_id == WEB_AUTHN_PUBLIC_KEY_TYPE) {
            let public_key_bytes = bcs_stream::deserialize_vector(stream, |x| deserialize_u8(x));
            pk = AnyPublicKey::Secp256r1Ecdsa{pk: secp256r1::ecdsa_raw_public_key_from_64_bytes(public_key_bytes)};
        } else if (scheme_id == KEYLESS_PUBLIC_KEY_TYPE) {
            pk = AnyPublicKey::Keyless{pk: keyless::deserialize_public_key(stream)};
        } else if (scheme_id == FEDERATED_KEYLESS_PUBLIC_KEY_TYPE) {
            pk = AnyPublicKey::FederatedKeyless{pk: federated_keyless::deserialize_public_key(stream)}
        } else {
            abort error::invalid_argument(E_INVALID_PUBLIC_KEY_TYPE);
        };
        pk
    }

    /// Returns true if the public key is a keyless or federated keyless public key.
    public fun is_keyless_or_federated_keyless_public_key(pk: &AnyPublicKey): bool {
        match (pk) {
            AnyPublicKey::Keyless { .. } => true,
            AnyPublicKey::FederatedKeyless { .. } => true,
            _ => false
        }
    }

    /// Converts an unvalidated Ed25519 public key to an AnyPublicKey.
    public fun from_ed25519_public_key_unvalidated(pk: ed25519::UnvalidatedPublicKey): AnyPublicKey {
        AnyPublicKey::Ed25519 { pk }
    }

    /// Gets the authentication key for the AnyPublicKey.
    public fun to_authentication_key(self: &AnyPublicKey): vector<u8> {
        let pk_bytes = bcs::to_bytes(self);
        pk_bytes.push_back(SIGNATURE_SCHEME_ID);
        hash::sha3_256(pk_bytes)
    }
}
