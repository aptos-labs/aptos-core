spec aptos_std::ed25519 {

    // -----------------------
    // Function specifications
    // -----------------------

    spec new_unvalidated_public_key_from_bytes(bytes: vector<u8>): UnvalidatedPublicKey {
        include NewUnvalidatedPublicKeyFromBytesAbortsIf;
        ensures result == UnvalidatedPublicKey { bytes };
    }
    spec schema NewUnvalidatedPublicKeyFromBytesAbortsIf {
        bytes: vector<u8>;
        aborts_if len(bytes) != PUBLIC_KEY_NUM_BYTES;
    }

    spec new_validated_public_key_from_bytes(bytes: vector<u8>): Option<ValidatedPublicKey> {
        aborts_if false;
        let cond = spec_public_key_validate_internal(bytes);
        ensures cond ==> result == option::spec_some(ValidatedPublicKey{bytes});
        ensures !cond ==> result == option::spec_none<ValidatedPublicKey>();
    }

    spec new_signature_from_bytes(bytes: vector<u8>): Signature {
        include NewSignatureFromBytesAbortsIf;
        ensures result == Signature { bytes };
    }
    spec schema NewSignatureFromBytesAbortsIf {
        bytes: vector<u8>;
        aborts_if len(bytes) != SIGNATURE_NUM_BYTES;
    }

    spec public_key_validate(pk: &UnvalidatedPublicKey): Option<ValidatedPublicKey> {
        pragma opaque;
        aborts_if false;
        let cond = spec_public_key_validate_internal(pk.bytes);
        ensures cond ==> result == option::spec_some(ValidatedPublicKey { bytes: pk.bytes });
        ensures !cond ==> result == option::spec_none<ValidatedPublicKey>();
    }

    spec signature_verify_strict(
        signature: &Signature,
        public_key: &UnvalidatedPublicKey,
        message: vector<u8>
    ): bool {
        pragma opaque;
        aborts_if false;
        ensures result == spec_signature_verify_strict_internal(signature.bytes, public_key.bytes, message);
    }

    spec signature_verify_strict_t<T: drop>(
        signature: &Signature,
        public_key: &UnvalidatedPublicKey,
        data: T
    ): bool {
        pragma opaque;
        aborts_if !type_info::spec_is_struct<T>();
        ensures result == spec_signature_verify_strict_t(signature, public_key, data);
    }

    spec new_signed_message<T: drop>(data: T): SignedMessage<T> {
        pragma opaque;
        aborts_if !type_info::spec_is_struct<T>();
        ensures result == SignedMessage<T> { type_info: type_info::type_of<T>(), inner: data };
    }

    spec unvalidated_public_key_to_authentication_key(pk: &UnvalidatedPublicKey): vector<u8> {
        pragma opaque;
        aborts_if false;
        ensures result == spec_public_key_bytes_to_authentication_key(pk.bytes);
    }

    spec validated_public_key_to_authentication_key(pk: &ValidatedPublicKey): vector<u8> {
        pragma opaque;
        aborts_if false;
        ensures result == spec_public_key_bytes_to_authentication_key(pk.bytes);
    }

    spec public_key_bytes_to_authentication_key(pk_bytes: vector<u8>): vector<u8> {
        pragma opaque;
        aborts_if false;
        ensures [abstract] result == spec_public_key_bytes_to_authentication_key(pk_bytes);
    }


    // ----------------
    // Native functions
    // ----------------

    spec public_key_validate_internal(bytes: vector<u8>): bool {
        pragma opaque;
        aborts_if false;
        ensures result == spec_public_key_validate_internal(bytes);
    }

    spec signature_verify_strict_internal(
        signature: vector<u8>,
        public_key: vector<u8>,
        message: vector<u8>)
    : bool {
        pragma opaque;
        aborts_if false;
        ensures result == spec_signature_verify_strict_internal(signature, public_key, message);
    }

    /// # Helper functions

    spec fun spec_signature_verify_strict_internal(
        signature: vector<u8>,
        public_key: vector<u8>,
        message: vector<u8>
    ): bool;

    spec fun spec_public_key_validate_internal(bytes: vector<u8>): bool;

    spec fun spec_public_key_bytes_to_authentication_key(pk_bytes: vector<u8>): vector<u8>;

    spec fun spec_signature_verify_strict_t<T>(signature: Signature, public_key: UnvalidatedPublicKey, data: T): bool {
        let encoded = SignedMessage<T> {
            type_info: type_info::type_of<T>(),
            inner: data,
        };
        let message = bcs::serialize(encoded);
        spec_signature_verify_strict_internal(signature.bytes, public_key.bytes, message)
    }

}
