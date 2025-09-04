spec velor_std::secp256k1 {
    spec ecdsa_signature_from_bytes(bytes: vector<u8>): ECDSASignature {
        aborts_if len(bytes) != SIGNATURE_NUM_BYTES;
        ensures result == ECDSASignature { bytes };
    }

    spec ecdsa_raw_public_key_from_64_bytes(bytes: vector<u8>): ECDSARawPublicKey {
        aborts_if len(bytes) != RAW_PUBLIC_KEY_NUM_BYTES;
        ensures result == ECDSARawPublicKey { bytes };
    }

    spec ecdsa_raw_public_key_to_bytes(pk: &ECDSARawPublicKey): vector<u8> {
        aborts_if false;
        ensures result == pk.bytes;
    }

    spec ecdsa_signature_to_bytes(sig: &ECDSASignature): vector<u8> {
        aborts_if false;
        ensures result == sig.bytes;
    }

    spec ecdsa_recover(
        message: vector<u8>,
        recovery_id: u8,
        signature: &ECDSASignature,
    ): Option<ECDSARawPublicKey> {
        aborts_if recovery_id > 3;
        aborts_if ecdsa_recover_internal_abort_condition(message, recovery_id, signature.bytes);
        let pk = spec_ecdsa_recover_internal_result_1(message, recovery_id, signature.bytes);
        let success = spec_ecdsa_recover_internal_result_2(message, recovery_id, signature.bytes);
        ensures success ==> result == std::option::spec_some(ecdsa_raw_public_key_from_64_bytes(pk));
        ensures !success ==> result == std::option::spec_none<ECDSARawPublicKey>();
    }

    spec ecdsa_recover_internal(
        message: vector<u8>,
        recovery_id: u8,
        signature: vector<u8>
    ): (vector<u8>, bool) {
        pragma opaque;
        aborts_if ecdsa_recover_internal_abort_condition(message, recovery_id, signature);
        ensures result_1 == spec_ecdsa_recover_internal_result_1(message, recovery_id, signature);
        ensures result_2 == spec_ecdsa_recover_internal_result_2(message, recovery_id, signature);
        ensures len(result_1) == if (result_2) { RAW_PUBLIC_KEY_NUM_BYTES } else { 0 };
    }

    spec fun ecdsa_recover_internal_abort_condition(message: vector<u8>, recovery_id: u8, signature: vector<u8>): bool;
    spec fun spec_ecdsa_recover_internal_result_1(message: vector<u8>, recovery_id: u8, signature: vector<u8>): vector<u8>;
    spec fun spec_ecdsa_recover_internal_result_2(message: vector<u8>, recovery_id: u8, signature: vector<u8>): bool;
}
