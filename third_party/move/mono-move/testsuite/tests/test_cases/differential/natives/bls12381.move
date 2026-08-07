// Differential test for the bls12381 natives.

// RUN: publish
module 0x1::bls12381 {
    native fun validate_pubkey_internal(public_key: vector<u8>): bool;

    native fun signature_subgroup_check_internal(signature: vector<u8>): bool;

    native fun verify_normal_signature_internal(
        signature: vector<u8>,
        public_key: vector<u8>,
        message: vector<u8>,
    ): bool;

    native fun verify_signature_share_internal(
        signature_share: vector<u8>,
        public_key: vector<u8>,
        message: vector<u8>,
    ): bool;

    native fun verify_multisignature_internal(
        multisignature: vector<u8>,
        agg_public_key: vector<u8>,
        message: vector<u8>,
    ): bool;

    native fun verify_proof_of_possession_internal(
        public_key: vector<u8>,
        proof_of_possession: vector<u8>,
    ): bool;

    native fun generate_keys_internal(): (vector<u8>, vector<u8>);

    native fun sign_internal(sk: vector<u8>, msg: vector<u8>): vector<u8>;

    native fun generate_proof_of_possession_internal(sk: vector<u8>): vector<u8>;

    // Sign and verify a normal signature (subgroup-checks the key): always true.
    public fun verify_normal_roundtrip(): bool {
        let (sk, pk) = generate_keys_internal();
        let msg = b"hello bls";
        let sig = sign_internal(sk, msg);
        verify_normal_signature_internal(sig, pk, msg)
    }

    // Verify a normal signature against a different message: always false.
    public fun verify_wrong_message(): bool {
        let (sk, pk) = generate_keys_internal();
        let sig = sign_internal(sk, b"signed message");
        verify_normal_signature_internal(sig, pk, b"other message")
    }

    // A signature share verifies without a key subgroup check: always true.
    public fun verify_share_roundtrip(): bool {
        let (sk, pk) = generate_keys_internal();
        let msg = b"share";
        let sig = sign_internal(sk, msg);
        verify_signature_share_internal(sig, pk, msg)
    }

    // A single signature is a valid 1-key multisignature: always true.
    public fun verify_multisignature_single(): bool {
        let (sk, pk) = generate_keys_internal();
        let msg = b"multi";
        let sig = sign_internal(sk, msg);
        verify_multisignature_internal(sig, pk, msg)
    }

    // A freshly generated public key validates (passes the subgroup check).
    public fun validate_generated_pubkey(): bool {
        let (_, pk) = generate_keys_internal();
        validate_pubkey_internal(pk)
    }

    // A real signature passes the subgroup check.
    public fun signature_subgroup_check_valid(): bool {
        let (sk, _) = generate_keys_internal();
        let sig = sign_internal(sk, b"subgroup");
        signature_subgroup_check_internal(sig)
    }

    // A generated proof-of-possession verifies against its public key.
    public fun pop_roundtrip(): bool {
        let (sk, pk) = generate_keys_internal();
        let pop = generate_proof_of_possession_internal(sk);
        verify_proof_of_possession_internal(pk, pop)
    }

    // Wrong-length inputs return false (do not abort).
    public fun validate_wrong_length(): bool {
        validate_pubkey_internal(x"00")
    }

    public fun verify_pop_wrong_length(): bool {
        verify_proof_of_possession_internal(x"00", x"00")
    }
}

// RUN: execute 0x1::bls12381::verify_normal_roundtrip
// CHECK: results: true

// RUN: execute 0x1::bls12381::verify_wrong_message
// CHECK: results: false

// RUN: execute 0x1::bls12381::verify_share_roundtrip
// CHECK: results: true

// RUN: execute 0x1::bls12381::verify_multisignature_single
// CHECK: results: true

// RUN: execute 0x1::bls12381::validate_generated_pubkey
// CHECK: results: true

// RUN: execute 0x1::bls12381::signature_subgroup_check_valid
// CHECK: results: true

// RUN: execute 0x1::bls12381::pop_roundtrip
// CHECK: results: true

// RUN: execute 0x1::bls12381::validate_wrong_length
// CHECK: results: false

// RUN: execute 0x1::bls12381::verify_pop_wrong_length
// CHECK: results: false
