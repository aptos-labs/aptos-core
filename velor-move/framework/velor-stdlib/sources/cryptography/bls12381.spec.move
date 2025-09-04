spec velor_std::bls12381 {

    spec public_key_from_bytes {
        aborts_if false;
        ensures spec_validate_pubkey_internal(bytes) ==> (std::option::spec_is_some(result) && std::option::spec_borrow(result).bytes == bytes);
        ensures !spec_validate_pubkey_internal(bytes) ==> std::option::spec_is_none(result);
    }

    spec public_key_from_bytes_with_pop {
        pragma opaque;
        aborts_if false;
        ensures spec_verify_proof_of_possession_internal(pk_bytes, pop.bytes) ==> (std::option::spec_is_some(result) && std::option::spec_borrow(result).bytes == pk_bytes);
        ensures !spec_verify_proof_of_possession_internal(pk_bytes, pop.bytes) ==> std::option::spec_is_none(result);
        ensures [abstract] result == spec_public_key_from_bytes_with_pop(pk_bytes, pop);
    }

    spec aggregate_pubkeys {
        let bytes = spec_aggregate_pubkeys_internal_1(public_keys);
        let success = spec_aggregate_pubkeys_internal_2(public_keys);
        aborts_if !success;
        ensures result.bytes == bytes;
    }

    spec aggregate_pubkeys_internal {
        pragma opaque;
        aborts_if [abstract] false; //TODO: check the aborts_if condition in the native implementation
        ensures result_1 == spec_aggregate_pubkeys_internal_1(public_keys);
        ensures result_2 == spec_aggregate_pubkeys_internal_2(public_keys);
    }

    spec aggregate_signatures {
        aborts_if false;
        let bytes = spec_aggregate_signatures_internal_1(signatures);
        let success = spec_aggregate_signatures_internal_2(signatures);
        ensures success ==> (std::option::spec_is_some(result) && std::option::spec_borrow(result).bytes == bytes);
        ensures !success ==> std::option::spec_is_none(result);
    }

    spec aggregate_signatures_internal {
        pragma opaque;
        aborts_if [abstract] false;
        ensures result_1 == spec_aggregate_signatures_internal_1(signatures);
        ensures result_2 == spec_aggregate_signatures_internal_2(signatures);
    }

    spec aggr_or_multi_signature_from_bytes {
        aborts_if len(bytes) != SIGNATURE_SIZE;
        ensures result.bytes == bytes;
    }

    spec validate_pubkey_internal {
        pragma opaque;
        aborts_if [abstract] false;
        ensures result == spec_validate_pubkey_internal(public_key);
    }

    spec aggr_or_multi_signature_subgroup_check {
        aborts_if false;
        ensures result == spec_signature_subgroup_check_internal(signature.bytes);
    }

    spec signature_subgroup_check_internal {
        pragma opaque;
        aborts_if [abstract] false;
        ensures result == spec_signature_subgroup_check_internal(signature);
    }

    spec verify_aggregate_signature {
        aborts_if false;
        ensures result == spec_verify_aggregate_signature_internal(aggr_sig.bytes, public_keys, messages);
    }

    spec verify_aggregate_signature_internal {
        pragma opaque;
        aborts_if [abstract] false;
        ensures result == spec_verify_aggregate_signature_internal(aggsig, public_keys, messages);
    }

    spec verify_multisignature {
        aborts_if false;
        ensures result == spec_verify_multisignature_internal(multisig.bytes, aggr_public_key.bytes, message);
    }

    spec verify_multisignature_internal {
        pragma opaque;
        aborts_if [abstract] false;
        ensures result == spec_verify_multisignature_internal(multisignature, agg_public_key, message);
    }

    spec verify_normal_signature {
        aborts_if false;
        ensures result == spec_verify_normal_signature_internal(signature.bytes, public_key.bytes, message);
    }

    spec verify_normal_signature_internal {
        pragma opaque;
        aborts_if [abstract] false;
        ensures result == spec_verify_normal_signature_internal(signature, public_key, message);
    }

    spec verify_proof_of_possession_internal {
        pragma opaque;
        aborts_if [abstract] false;
        ensures result == spec_verify_proof_of_possession_internal(public_key, proof_of_possesion);
    }

    spec verify_signature_share {
        aborts_if false;
        ensures result == spec_verify_signature_share_internal(signature_share.bytes, public_key.bytes, message);
    }

    spec verify_signature_share_internal {
        pragma opaque;
        aborts_if [abstract] false;
        ensures result == spec_verify_signature_share_internal(signature_share, public_key, message);
    }

    /// # Helper functions

    spec fun spec_aggregate_pubkeys_internal_1(public_keys: vector<PublicKeyWithPoP>): vector<u8>;

    spec fun spec_public_key_from_bytes_with_pop(pk_bytes: vector<u8>, pop: ProofOfPossession): Option<PublicKeyWithPoP>;

    spec fun spec_aggregate_pubkeys_internal_2(public_keys: vector<PublicKeyWithPoP>): bool;

    spec fun spec_aggregate_signatures_internal_1(signatures: vector<Signature>): vector<u8>;

    spec fun spec_aggregate_signatures_internal_2(signatures: vector<Signature>): bool;

    spec fun spec_validate_pubkey_internal(public_key: vector<u8>): bool;

    spec fun spec_signature_subgroup_check_internal(signature: vector<u8>): bool;

    spec fun spec_verify_aggregate_signature_internal(
        aggsig: vector<u8>,
        public_keys: vector<PublicKeyWithPoP>,
        messages: vector<vector<u8>>,
    ): bool;

    spec fun spec_verify_multisignature_internal(
        multisignature: vector<u8>,
        agg_public_key: vector<u8>,
        message: vector<u8>
    ): bool;

    spec fun spec_verify_normal_signature_internal(
        signature: vector<u8>,
        public_key: vector<u8>,
        message: vector<u8>
    ): bool;

    spec fun spec_verify_proof_of_possession_internal(
        public_key: vector<u8>,
        proof_of_possesion: vector<u8>
    ): bool;

    spec fun spec_verify_signature_share_internal(
        signature_share: vector<u8>,
        public_key: vector<u8>,
        message: vector<u8>
    ): bool;


}
