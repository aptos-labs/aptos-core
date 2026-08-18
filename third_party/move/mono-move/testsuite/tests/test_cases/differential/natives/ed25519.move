// Differential test for the ed25519 natives.
//
// Tests return only booleans: each VM generates its own keypair, so a valid
// roundtrip is `true` on both without comparing key bytes.

// RUN: publish
module 0x1::ed25519 {
    native fun public_key_validate_internal(bytes: vector<u8>): bool;

    native fun signature_verify_strict_internal(
        signature: vector<u8>,
        public_key: vector<u8>,
        message: vector<u8>,
    ): bool;

    native fun generate_keys_internal(): (vector<u8>, vector<u8>);

    native fun sign_internal(sk: vector<u8>, msg: vector<u8>): vector<u8>;

    // Sign a message and verify it under the matching public key: always true.
    public fun sign_verify_roundtrip(): bool {
        let (sk, pk) = generate_keys_internal();
        let msg = b"hello mono-move";
        let sig = sign_internal(sk, msg);
        signature_verify_strict_internal(sig, pk, msg)
    }

    // Verify a signature against a different message: always false.
    public fun verify_wrong_message(): bool {
        let (sk, pk) = generate_keys_internal();
        let sig = sign_internal(sk, b"signed message");
        signature_verify_strict_internal(sig, pk, b"other message")
    }

    // A freshly generated public key validates.
    public fun validate_generated_pubkey(): bool {
        let (_, pk) = generate_keys_internal();
        public_key_validate_internal(pk)
    }

    // A wrong-length key returns false (does not abort).
    public fun validate_wrong_length(): bool {
        public_key_validate_internal(x"00")
    }

    // Wrong-length signature/public key inputs return false (do not abort).
    public fun verify_bad_length_inputs(): bool {
        signature_verify_strict_internal(x"00", x"00", b"msg")
    }
}

// RUN: execute 0x1::ed25519::sign_verify_roundtrip
// CHECK: results: true

// RUN: execute 0x1::ed25519::verify_wrong_message
// CHECK: results: false

// RUN: execute 0x1::ed25519::validate_generated_pubkey
// CHECK: results: true

// RUN: execute 0x1::ed25519::validate_wrong_length
// CHECK: results: false

// RUN: execute 0x1::ed25519::verify_bad_length_inputs
// CHECK: results: false
