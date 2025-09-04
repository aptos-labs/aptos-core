spec velor_std::multi_ed25519 {

    // -----------------------
    // Function specifications
    // -----------------------

    spec new_unvalidated_public_key_from_bytes(bytes: vector<u8>): UnvalidatedPublicKey {
        include NewUnvalidatedPublicKeyFromBytesAbortsIf;
        ensures result == UnvalidatedPublicKey { bytes };
    }
    spec schema NewUnvalidatedPublicKeyFromBytesAbortsIf {
        bytes: vector<u8>;
        let length = len(bytes);
        aborts_if length / INDIVIDUAL_PUBLIC_KEY_NUM_BYTES > MAX_NUMBER_OF_PUBLIC_KEYS;
        aborts_if length % INDIVIDUAL_PUBLIC_KEY_NUM_BYTES != THRESHOLD_SIZE_BYTES;
    }

    spec new_validated_public_key_from_bytes(bytes: vector<u8>): Option<ValidatedPublicKey> {
        aborts_if false;
        let cond = len(bytes) % INDIVIDUAL_PUBLIC_KEY_NUM_BYTES == THRESHOLD_SIZE_BYTES
            && spec_public_key_validate_internal(bytes);
        ensures cond ==> result == option::spec_some(ValidatedPublicKey{bytes});
        ensures !cond ==> result == option::spec_none<ValidatedPublicKey>();
    }

    spec new_validated_public_key_from_bytes_v2(bytes: vector<u8>): Option<ValidatedPublicKey> {
        let cond = spec_public_key_validate_v2_internal(bytes);
        ensures cond ==> result == option::spec_some(ValidatedPublicKey{bytes});
        ensures !cond ==> result == option::spec_none<ValidatedPublicKey>();
    }

    spec new_signature_from_bytes(bytes: vector<u8>): Signature {
        include NewSignatureFromBytesAbortsIf;
        ensures result == Signature { bytes };
    }
    spec schema NewSignatureFromBytesAbortsIf {
        bytes: vector<u8>;
        aborts_if len(bytes) % INDIVIDUAL_SIGNATURE_NUM_BYTES != BITMAP_NUM_OF_BYTES;
    }

    spec check_and_get_threshold(bytes: vector<u8>): Option<u8> {
        aborts_if false;
        ensures result == spec_check_and_get_threshold(bytes);
    }

    spec schema PkDivision {
        bytes: vector<u8>;
        result: u8;
        aborts_if len(bytes) / INDIVIDUAL_PUBLIC_KEY_NUM_BYTES > MAX_U8;
        ensures result == len(bytes) / INDIVIDUAL_PUBLIC_KEY_NUM_BYTES;
    }

    spec unvalidated_public_key_num_sub_pks(pk: &UnvalidatedPublicKey): u8 {
        let bytes = pk.bytes;
        include PkDivision;
    }

    spec validated_public_key_num_sub_pks(pk: &ValidatedPublicKey): u8 {
        let bytes = pk.bytes;
        include PkDivision;
    }

    spec unvalidated_public_key_threshold(pk: &UnvalidatedPublicKey): Option<u8> {
        aborts_if false;
        ensures result == spec_check_and_get_threshold(pk.bytes);
    }

    spec validated_public_key_threshold(pk: &ValidatedPublicKey): u8 {
        aborts_if len(pk.bytes) == 0;
        ensures result == pk.bytes[len(pk.bytes) - 1];
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
        ensures (len(bytes) / INDIVIDUAL_PUBLIC_KEY_NUM_BYTES > MAX_NUMBER_OF_PUBLIC_KEYS) ==> (result == false);
        ensures result == spec_public_key_validate_internal(bytes);
    }

    spec public_key_validate_v2_internal(bytes: vector<u8>): bool {
        pragma opaque;
        ensures result == spec_public_key_validate_v2_internal(bytes);
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

    /// # Helper functions

    spec fun spec_check_and_get_threshold(bytes: vector<u8>): Option<u8> {
        let len = len(bytes);
        if (len == 0) {
            option::none<u8>()
        } else {
            let threshold_num_of_bytes = len % INDIVIDUAL_PUBLIC_KEY_NUM_BYTES;
            let num_of_keys = len / INDIVIDUAL_PUBLIC_KEY_NUM_BYTES;
            let threshold_byte = bytes[len - 1];
            if (num_of_keys == 0 || num_of_keys > MAX_NUMBER_OF_PUBLIC_KEYS || len % INDIVIDUAL_PUBLIC_KEY_NUM_BYTES != 1) {
                option::none<u8>()
            } else if (threshold_byte == 0 || threshold_byte > (num_of_keys as u8)) {
                option::none<u8>()
            } else {
                option::spec_some(threshold_byte)
            }
        }
    }

    spec fun spec_signature_verify_strict_internal(
        multisignature: vector<u8>,
        public_key: vector<u8>,
        message: vector<u8>
    ): bool;

    spec fun spec_public_key_validate_internal(bytes: vector<u8>): bool;

    spec fun spec_public_key_validate_v2_internal(bytes: vector<u8>): bool;

    spec fun spec_public_key_bytes_to_authentication_key(pk_bytes: vector<u8>): vector<u8>;

    spec fun spec_signature_verify_strict_t<T>(signature: Signature, public_key: UnvalidatedPublicKey, data: T): bool {
        let encoded = ed25519::new_signed_message<T>(data);
        let message = bcs::serialize(encoded);
        spec_signature_verify_strict_internal(signature.bytes, public_key.bytes, message)
    }
}
