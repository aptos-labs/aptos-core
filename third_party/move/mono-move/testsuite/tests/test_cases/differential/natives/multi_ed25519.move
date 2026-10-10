// Differential test for the multi_ed25519 natives.
//
// Tests return only booleans: each VM generates its own t-of-n keys, so a valid
// roundtrip is `true` on both without comparing key bytes.

// RUN: publish
module 0x1::multi_ed25519 {
    native fun public_key_validate_internal(bytes: vector<u8>): bool;

    native fun public_key_validate_v2_internal(bytes: vector<u8>): bool;

    native fun signature_verify_strict_internal(
        multisignature: vector<u8>,
        public_key: vector<u8>,
        message: vector<u8>,
    ): bool;

    native fun generate_keys_internal(threshold: u8, n: u8): (vector<u8>, vector<u8>);

    native fun sign_internal(sk: vector<u8>, message: vector<u8>): vector<u8>;

    // Sign under a 2-of-3 key and verify: always true.
    public fun sign_verify_roundtrip(): bool {
        let (sk, pk) = generate_keys_internal(2, 3);
        let msg = b"hello multi";
        let sig = sign_internal(sk, msg);
        signature_verify_strict_internal(sig, pk, msg)
    }

    // Verify a signature against a different message: always false.
    public fun verify_wrong_message(): bool {
        let (sk, pk) = generate_keys_internal(2, 3);
        let sig = sign_internal(sk, b"signed message");
        signature_verify_strict_internal(sig, pk, b"other message")
    }

    // A freshly generated 2-of-3 public key validates under both entry points.
    public fun validate_generated_pubkey(): bool {
        let (_, pk) = generate_keys_internal(2, 3);
        public_key_validate_internal(pk)
    }

    public fun validate_v2_generated_pubkey(): bool {
        let (_, pk) = generate_keys_internal(2, 3);
        public_key_validate_v2_internal(pk)
    }

    // The identity point is small-order, so validation rejects it as a sub-key.
    public fun validate_small_order_subkey(): bool {
        public_key_validate_internal(
            x"0100000000000000000000000000000000000000000000000000000000000000"
        )
    }
}

// RUN: execute 0x1::multi_ed25519::sign_verify_roundtrip
// CHECK: results: true

// RUN: execute 0x1::multi_ed25519::verify_wrong_message
// CHECK: results: false

// RUN: execute 0x1::multi_ed25519::validate_generated_pubkey
// CHECK: results: true

// RUN: execute 0x1::multi_ed25519::validate_v2_generated_pubkey
// CHECK: results: true

// RUN: execute 0x1::multi_ed25519::validate_small_order_subkey
// CHECK: results: false
