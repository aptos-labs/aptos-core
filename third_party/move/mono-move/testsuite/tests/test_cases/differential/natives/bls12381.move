// Differential test for the bls12381 natives.

// RUN: publish
module 0x1::bls12381 {
    struct PublicKeyWithPoP has copy, drop, store {
        bytes: vector<u8>,
    }

    struct Signature has copy, drop, store {
        bytes: vector<u8>,
    }

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

    native fun aggregate_pubkeys_internal(
        public_keys: vector<PublicKeyWithPoP>,
    ): (vector<u8>, bool);

    native fun aggregate_signatures_internal(
        signatures: vector<Signature>,
    ): (vector<u8>, bool);

    native fun verify_aggregate_signature_internal(
        aggsig: vector<u8>,
        public_keys: vector<PublicKeyWithPoP>,
        messages: vector<vector<u8>>,
    ): bool;

    fun pk(bytes: vector<u8>): PublicKeyWithPoP {
        PublicKeyWithPoP { bytes }
    }

    fun sig(bytes: vector<u8>): Signature {
        Signature { bytes }
    }

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

    // Aggregating two real public keys succeeds.
    public fun aggregate_pubkeys_success(): bool {
        let (_, pk0) = generate_keys_internal();
        let (_, pk1) = generate_keys_internal();
        let (_, success) = aggregate_pubkeys_internal(vector[pk(pk0), pk(pk1)]);
        success
    }

    // Aggregating no public keys reports no aggregate.
    public fun aggregate_pubkeys_empty(): bool {
        let (_, success) = aggregate_pubkeys_internal(vector[]);
        success
    }

    // Aggregating two real signatures succeeds.
    public fun aggregate_signatures_success(): bool {
        let (sk0, _) = generate_keys_internal();
        let (sk1, _) = generate_keys_internal();
        let s0 = sign_internal(sk0, b"msg");
        let s1 = sign_internal(sk1, b"msg");
        let (_, success) = aggregate_signatures_internal(vector[sig(s0), sig(s1)]);
        success
    }

    // Aggregating no signatures reports no aggregate.
    public fun aggregate_signatures_empty(): bool {
        let (_, success) = aggregate_signatures_internal(vector[]);
        success
    }

    // Two signers over two distinct messages: the aggregate signature verifies.
    public fun verify_aggregate_roundtrip(): bool {
        let (sk0, pk0) = generate_keys_internal();
        let (sk1, pk1) = generate_keys_internal();
        let m0 = b"message zero";
        let m1 = b"message one";
        let s0 = sign_internal(sk0, m0);
        let s1 = sign_internal(sk1, m1);
        let (aggsig, ok) = aggregate_signatures_internal(vector[sig(s0), sig(s1)]);
        assert!(ok, 0);
        verify_aggregate_signature_internal(aggsig, vector[pk(pk0), pk(pk1)], vector[m0, m1])
    }

    // Verifying the aggregate against a tampered message fails.
    public fun verify_aggregate_wrong_message(): bool {
        let (sk0, pk0) = generate_keys_internal();
        let (sk1, pk1) = generate_keys_internal();
        let s0 = sign_internal(sk0, b"message zero");
        let s1 = sign_internal(sk1, b"message one");
        let (aggsig, ok) = aggregate_signatures_internal(vector[sig(s0), sig(s1)]);
        assert!(ok, 0);
        verify_aggregate_signature_internal(
            aggsig,
            vector[pk(pk0), pk(pk1)],
            vector[b"message zero", b"tampered"],
        )
    }

    // A public-key count that disagrees with the message count fails.
    public fun verify_aggregate_length_mismatch(): bool {
        let (sk0, pk0) = generate_keys_internal();
        let s0 = sign_internal(sk0, b"m");
        let (aggsig, _) = aggregate_signatures_internal(vector[sig(s0)]);
        verify_aggregate_signature_internal(aggsig, vector[pk(pk0)], vector[b"m", b"n"])
    }

    // Aggregate pubkeys + signatures over one shared message, then verify as a
    // multisignature: ties the two aggregate natives to the multisig verifier.
    public fun multisig_roundtrip(): bool {
        let (sk0, pk0) = generate_keys_internal();
        let (sk1, pk1) = generate_keys_internal();
        let msg = b"same message";
        let s0 = sign_internal(sk0, msg);
        let s1 = sign_internal(sk1, msg);
        let (aggsig, ok_s) = aggregate_signatures_internal(vector[sig(s0), sig(s1)]);
        let (aggpk, ok_p) = aggregate_pubkeys_internal(vector[pk(pk0), pk(pk1)]);
        assert!(ok_s && ok_p, 0);
        verify_multisignature_internal(aggsig, aggpk, msg)
    }

    // Mirrors the framework wrapper `aggregate_pubkeys`, which aborts with
    // `std::error::invalid_argument(EZERO_PUBKEYS)` (== 65537) when the native
    // reports no aggregate for empty input.
    public fun aggregate_pubkeys_empty_aborts() {
        let (_, success) = aggregate_pubkeys_internal(vector[]);
        assert!(success, 65537);
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

// RUN: execute 0x1::bls12381::aggregate_pubkeys_success
// CHECK: results: true

// RUN: execute 0x1::bls12381::aggregate_pubkeys_empty
// CHECK: results: false

// RUN: execute 0x1::bls12381::aggregate_signatures_success
// CHECK: results: true

// RUN: execute 0x1::bls12381::aggregate_signatures_empty
// CHECK: results: false

// RUN: execute 0x1::bls12381::verify_aggregate_roundtrip
// CHECK: results: true

// RUN: execute 0x1::bls12381::verify_aggregate_wrong_message
// CHECK: results: false

// RUN: execute 0x1::bls12381::verify_aggregate_length_mismatch
// CHECK: results: false

// RUN: execute 0x1::bls12381::multisig_roundtrip
// CHECK: results: true

// RUN: execute 0x1::bls12381::aggregate_pubkeys_empty_aborts
// CHECK: aborted: code 65537 in 0x1::bls12381
