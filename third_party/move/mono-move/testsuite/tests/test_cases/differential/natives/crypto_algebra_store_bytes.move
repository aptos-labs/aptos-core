// Filling the 1048576-byte crypto_algebra element memory. BLS12-381 G2 elements count 288
// bytes each, so 3640 fit and the 3641st aborts; Fr elements count 32 bytes, so 32768 fill it
// exactly and the 32769th aborts. One G2 element after 32759 Fr elements fits exactly; after
// 32760 it aborts. Each step returns the last handle, showing every earlier call succeeded.

// RUN: publish
module 0x1::bls12381_algebra {
    struct Fr {}
    struct G2 {}
}

module 0x1::crypto_algebra {
    use 0x1::bls12381_algebra as bls;

    native fun from_u64_internal<S>(value: u64): u64;
    native fun one_internal<S>(): u64;

    public fun g2_fill(count: u64): u64 {
        let last = 0;
        let stored = 0;
        while (stored < count) {
            last = one_internal<bls::G2>();
            stored += 1;
        };
        last
    }

    public fun fr_fill(count: u64): u64 {
        let last = 0;
        let stored = 0;
        while (stored < count) {
            last = from_u64_internal<bls::Fr>(stored);
            stored += 1;
        };
        last
    }

    public fun g2_after_fr(count: u64): u64 {
        fr_fill(count);
        one_internal<bls::G2>()
    }
}

// RUN: execute 0x1::crypto_algebra::g2_fill --args 3640
// CHECK: results: 3639

// RUN: execute 0x1::crypto_algebra::g2_fill --args 3641
// CHECK: aborted: code 589827 (Algebra context memory 1048576-byte limit exceeded: currently using 1048320 bytes; was asked for 1048608 bytes) in 0x1::crypto_algebra

// RUN: execute 0x1::crypto_algebra::fr_fill --args 32768
// CHECK: results: 32767

// RUN: execute 0x1::crypto_algebra::fr_fill --args 32769
// CHECK: aborted: code 589827 (Algebra context memory 1048576-byte limit exceeded: currently using 1048576 bytes; was asked for 1048608 bytes) in 0x1::crypto_algebra

// RUN: execute 0x1::crypto_algebra::g2_after_fr --args 32759
// CHECK: results: 32759

// RUN: execute 0x1::crypto_algebra::g2_after_fr --args 32760
// CHECK: aborted: code 589827 (Algebra context memory 1048576-byte limit exceeded: currently using 1048320 bytes; was asked for 1048608 bytes) in 0x1::crypto_algebra
