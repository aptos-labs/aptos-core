spec aptos_std::multi_ed25519 {

    // -----------------------
    // Function specifications
    // -----------------------

    spec new_unvalidated_public_key_from_bytes(bytes: vector<u8>): UnvalidatedPublicKey {
        let length = len(bytes);
        aborts_if length / INDIVIDUAL_PUBLIC_KEY_NUM_BYTES > MAX_NUMBER_OF_PUBLIC_KEYS;
        aborts_if length % INDIVIDUAL_PUBLIC_KEY_NUM_BYTES != THRESHOLD_SIZE_BYTES;
        ensures result == UnvalidatedPublicKey { bytes };
    }

    spec new_validated_public_key_from_bytes(bytes: vector<u8>): Option<ValidatedPublicKey> {
        aborts_if len(bytes) % INDIVIDUAL_PUBLIC_KEY_NUM_BYTES == THRESHOLD_SIZE_BYTES
            && len(bytes) / INDIVIDUAL_PUBLIC_KEY_NUM_BYTES > MAX_NUMBER_OF_PUBLIC_KEYS;
        let cond = len(bytes) % INDIVIDUAL_PUBLIC_KEY_NUM_BYTES == THRESHOLD_SIZE_BYTES
            && spec_public_key_validate_internal(bytes);
        ensures cond ==> result == option::spec_some(ValidatedPublicKey{bytes});
        ensures !cond ==> result == option::spec_none<ValidatedPublicKey>();
    }

    spec new_signature_from_bytes(bytes: vector<u8>): Signature {
        aborts_if len(bytes) % INDIVIDUAL_SIGNATURE_NUM_BYTES != BITMAP_NUM_OF_BYTES;
        ensures result == Signature { bytes };
    }


    // ----------------
    // Native functions
    // ----------------

    spec fun spec_public_key_validate_internal(bytes: vector<u8>): bool;

    spec fun spec_signature_verify_strict_internal(
        multisignature: vector<u8>,
        public_key: vector<u8>,
        message: vector<u8>
    ): bool;

    spec public_key_validate_internal(bytes: vector<u8>): bool {
        pragma opaque;
        aborts_if len(bytes) / INDIVIDUAL_PUBLIC_KEY_NUM_BYTES > MAX_NUMBER_OF_PUBLIC_KEYS;
        ensures result == spec_public_key_validate_internal(bytes);
    }

    spec signature_verify_strict_internal(
        multisignature: vector<u8>,
        public_key: vector<u8>,
        message: vector<u8>
    ): bool {
        pragma opaque;
        aborts_if false;
        ensures result == spec_signature_verify_strict_internal(multisignature, public_key, message);
    }
}
