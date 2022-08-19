/// Contains functions for:
///
///  1. [Ed25519](https://en.wikipedia.org/wiki/EdDSA#Ed25519) digital signatures: i.e., EdDSA signatures over Edwards25519 curves with co-factor 8
///
module aptos_std::ed25519 {
    use std::bcs;
    use aptos_std::type_info::{Self, TypeInfo};
    use std::option::{Self, Option};

    /// Wrong number of bytes were given as input when deserializing an Ed25519 public key.
    const E_WRONG_PUBKEY_SIZE : u64 = 1;

    /// Wrong number of bytes were given as input when deserializing an Ed25519 signature.
    const E_WRONG_SIGNATURE_SIZE : u64 = 2;

    /// The size of a serialized public key, in bytes.
    const PUBLIC_KEY_NUM_BYTES : u64 = 32;

    /// The size of a serialized signature, in bytes.
    const SIGNATURE_NUM_BYTES : u64 = 64;

    /// A BCS-serializable message, which one can verify signatures on via `verify_signature_t`
    struct SignedMessage<MessageType> has drop {
        type_info: TypeInfo,
        inner: MessageType,
    }

    /// An *unvalidated* Ed25519 public key: not necessarily an elliptic curve point, just a sequence of 32 bytes
    struct UnvalidatedPublicKey has copy, drop, store {
        bytes: vector<u8>
    }

    /// A *validated* Ed25519 public key: not necessarily a prime-order point, could be mixed-order, but will never be
    /// a small-order point.
    ///
    /// For now, this struct is not used in any verification functions, but it might be in the future.
    struct ValidatedPublicKey has copy, drop, store {
        bytes: vector<u8>
    }

    /// A purported Ed25519 signature that can be verified via `verify_signature_strict` or `verify_signature_strict_t`.
    struct Signature has copy, drop, store {
        bytes: vector<u8>
    }

    /// Parses the input 32 bytes as an *unvalidated* Ed25519 public key.
    public fun new_unvalidated_public_key_from_bytes(bytes: vector<u8>): UnvalidatedPublicKey {
        assert!(std::vector::length(&bytes) == PUBLIC_KEY_NUM_BYTES, std::error::invalid_argument(E_WRONG_PUBKEY_SIZE));
        UnvalidatedPublicKey { bytes }
    }

    /// Parses the input 32 bytes as a *validated* Ed25519 public key.
    public fun new_validated_public_key_from_bytes(bytes: vector<u8>): Option<ValidatedPublicKey> {
        if(public_key_validate_internal(bytes)) {
            option::some(ValidatedPublicKey {
                bytes
            })
        } else {
            option::none<ValidatedPublicKey>()
        }
    }

    /// Parses the input 64 bytes as a purported Ed25519 signature.
    public fun new_signature_from_bytes(bytes: vector<u8>): Signature {
        assert!(std::vector::length(&bytes) == SIGNATURE_NUM_BYTES, std::error::invalid_argument(E_WRONG_SIGNATURE_SIZE));
        Signature { bytes }
    }

    /// Converts a ValidatedPublicKey to an UnvalidatedPublicKey, which can be used in the strict verification APIs.
    public fun public_key_to_unvalidated(pk: &ValidatedPublicKey): UnvalidatedPublicKey {
        UnvalidatedPublicKey {
            bytes: pk.bytes
        }
    }

    /// Moves a ValidatedPublicKey into an UnvalidatedPublicKey, which can be used in the strict verification APIs.
    public fun public_key_into_unvalidated(pk: ValidatedPublicKey): UnvalidatedPublicKey {
        UnvalidatedPublicKey {
            bytes: pk.bytes
        }
    }

    /// Serializes an UnvalidatedPublicKey struct to 32-bytes.
    public fun unvalidated_public_key_to_bytes(pk: &UnvalidatedPublicKey): vector<u8> {
        pk.bytes
    }

    /// Serializes an ValidatedPublicKey struct to 32-bytes.
    public fun validated_public_key_to_bytes(pk: &ValidatedPublicKey): vector<u8> {
        pk.bytes
    }

    /// Serializes a Signature struct to 64-bytes.
    public fun signature_to_bytes(sig: &Signature): vector<u8> {
        sig.bytes
    }

    /// Takes in an *unvalidated* public key and attempts to validate it.
    /// Returns `Some(ValidatedPublicKey)` if successful and `None` otherwise.
    public fun public_key_validate(pk: &UnvalidatedPublicKey): Option<ValidatedPublicKey> {
        new_validated_public_key_from_bytes(pk.bytes)
    }

    /// Verifies a purported Ed25519 `signature` under an *unvalidated* `public_key` on the specified `message`.
    /// This call will validate the public key by checking it is NOT in the small subgroup.
    public fun signature_verify_strict(
        signature: &Signature,
        public_key: &UnvalidatedPublicKey,
        message: vector<u8>
    ): bool {
        signature_verify_strict_internal(signature.bytes, public_key.bytes, message)
    }

    /// This function is used to verify a signature on any BCS-serializable type T. For now, it is used to verify the
    /// proof of private key ownership when rotating authentication keys.
    public fun signature_verify_strict_t<T: drop> (signature: &Signature, public_key: &UnvalidatedPublicKey, data: T): bool {
        let encoded = SignedMessage {
            type_info: type_info::type_of<T>(),
            inner: data,
        };

        signature_verify_strict_internal(signature.bytes, public_key.bytes, bcs::to_bytes(&encoded))
    }

    //
    // Native functions
    //

    /// Return `true` if the bytes in `public_key` can be parsed as a valid Ed25519 public key: i.e., it passes
    /// points-on-curve and not-in-small-subgroup checks.
    /// Returns `false` otherwise.
    native fun public_key_validate_internal(bytes: vector<u8>): bool;

    /// Return true if the Ed25519 `signature` on `message` verifies against the Ed25519 `public_key`.
    /// Returns `false` if either:
    /// - `signature` or `public key` are of wrong sizes
    /// - `public_key` does not pass points-on-curve or not-in-small-subgroup checks,
    /// - `signature` does not pass points-on-curve or not-in-small-subgroup checks,
    /// - the signature on `message` does not verify.
    native fun signature_verify_strict_internal(
        signature: vector<u8>,
        public_key: vector<u8>,
        message: vector<u8>
    ): bool;
}
