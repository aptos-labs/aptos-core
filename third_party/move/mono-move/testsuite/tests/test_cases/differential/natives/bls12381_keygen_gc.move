// A collection forced inside bls12381's test-only key generator, between its 32-byte secret
// key and 48-byte public key allocations; `keygen_fill` shows the heap is sized so the second
// allocation is the one that collects. The later steps add two more collections before the
// keys are read (and, in the last, used to sign and verify).

// RUN: publish
module 0x1::bls12381 {
    native fun generate_keys_internal(): (vector<u8>, vector<u8>);
    native fun sign_internal(sk: vector<u8>, msg: vector<u8>): vector<u8>;
    native fun verify_normal_signature_internal(
        signature: vector<u8>,
        public_key: vector<u8>,
        message: vector<u8>,
    ): bool;

    public fun keys(): (vector<u8>, vector<u8>) {
        generate_keys_internal()
    }

    public fun verify(sk: vector<u8>, pk: vector<u8>): bool {
        let msg = b"m";
        verify_normal_signature_internal(sign_internal(sk, msg), pk, msg)
    }
}
module 0x42::main {
    const KEY_SIZED: vector<u8> = x"0000000000000000000000000000000000000000000000000000000000000000";

    fun churn(rounds: u64) {
        let counter = 0;
        while (counter < rounds) {
            let junk = vector[counter, counter, counter, counter];
            counter = counter + junk.length();
        };
    }

    public fun keygen(rounds: u64): (u64, u64, bool) {
        churn(rounds);
        let (sk, pk) = 0x1::bls12381::keys();
        (sk.length(), pk.length(), sk != pk)
    }

    public fun keygen_fill(rounds: u64): u64 {
        churn(rounds);
        let first = KEY_SIZED;
        first.length()
    }

    public fun keygen_churned(rounds: u64, more: u64): (u64, u64, bool) {
        churn(rounds);
        let (sk, pk) = 0x1::bls12381::keys();
        churn(more);
        (sk.length(), pk.length(), sk != pk)
    }

    public fun keygen_verified(rounds: u64, more: u64): (u64, u64, bool) {
        churn(rounds);
        let (sk, pk) = 0x1::bls12381::keys();
        churn(more);
        (sk.length(), pk.length(), 0x1::bls12381::verify(sk, pk))
    }
}

// RUN: execute 0x42::main::keygen_fill --args 2724 --heap-size 32768
// CHECK: results: 32
// CHECK-GC-COUNT: 0

// RUN: execute 0x42::main::keygen --args 2724 --heap-size 32768
// CHECK: results: 32, 48, true
// CHECK-GC-COUNT: 1

// RUN: execute 0x42::main::keygen_churned --args 2724, 6800 --heap-size 32768
// CHECK: results: 32, 48, true
// CHECK-GC-COUNT: 3

// RUN: execute 0x42::main::keygen_verified --args 2724, 6800 --heap-size 32768
// CHECK: results: 32, 48, true
// CHECK-GC-COUNT: 3
