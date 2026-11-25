// A collection forced inside multi_ed25519's test-only key generator, between the 65-byte
// secret and public key allocations of a 2-of-2 key; `keygen_fill` shows the heap is sized so
// the second allocation is the one that collects. The later steps add two more collections
// before the keys are read (and, in the last, used to sign and verify).

// RUN: publish
module 0x1::multi_ed25519 {
    native fun generate_keys_internal(threshold: u8, num_keys: u8): (vector<u8>, vector<u8>);
    native fun sign_internal(sk: vector<u8>, message: vector<u8>): vector<u8>;
    native fun signature_verify_strict_internal(
        multisignature: vector<u8>,
        public_key: vector<u8>,
        message: vector<u8>,
    ): bool;

    public fun keys(): (vector<u8>, vector<u8>) {
        generate_keys_internal(2, 2)
    }

    public fun verify(sk: vector<u8>, pk: vector<u8>): bool {
        let msg = b"m";
        signature_verify_strict_internal(sign_internal(sk, msg), pk, msg)
    }
}
module 0x42::main {
    const KEY_SIZED: vector<u8> = x"0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000";

    fun churn(rounds: u64) {
        let counter = 0;
        while (counter < rounds) {
            let junk = vector[counter, counter, counter, counter];
            counter = counter + junk.length();
        };
    }

    public fun keygen(rounds: u64): (u64, u64, bool) {
        churn(rounds);
        let (sk, pk) = 0x1::multi_ed25519::keys();
        (sk.length(), pk.length(), sk != pk)
    }

    public fun keygen_fill(rounds: u64): u64 {
        churn(rounds);
        let first = KEY_SIZED;
        first.length()
    }

    public fun keygen_churned(rounds: u64, more: u64): (u64, u64, bool) {
        churn(rounds);
        let (sk, pk) = 0x1::multi_ed25519::keys();
        churn(more);
        (sk.length(), pk.length(), sk != pk)
    }

    public fun keygen_verified(rounds: u64, more: u64): (u64, u64, bool) {
        churn(rounds);
        let (sk, pk) = 0x1::multi_ed25519::keys();
        churn(more);
        (sk.length(), pk.length(), 0x1::multi_ed25519::verify(sk, pk))
    }
}

// RUN: execute 0x42::main::keygen_fill --args 2720 --heap-size 32768
// CHECK: results: 65
// CHECK-GC-COUNT: 0

// RUN: execute 0x42::main::keygen --args 2720 --heap-size 32768
// CHECK: results: 65, 65, true
// CHECK-GC-COUNT: 1

// RUN: execute 0x42::main::keygen_churned --args 2720, 6800 --heap-size 32768
// CHECK: results: 65, 65, true
// CHECK-GC-COUNT: 3

// RUN: execute 0x42::main::keygen_verified --args 2720, 6800 --heap-size 32768
// CHECK: results: 65, 65, true
// CHECK-GC-COUNT: 3
