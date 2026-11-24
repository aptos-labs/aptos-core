// crypto_algebra handles number elements of all structures in one sequence. Serializing a
// valid handle at the wrong structure is a VM error: V1 reports
// UNKNOWN_INVARIANT_VIOLATION_ERROR (substring check), MonoMove a native invariant violation.
// Each such step reads handle 1 after creating element 0 at the requested structure; its
// control reads handle 1 at its own structure. `upcast` returns any handle unchanged without
// checking it: one past an empty store, u64::MAX, a BLS Fr handle, and a real Gt handle.

// RUN: publish
module 0x1::bls12381_algebra {
    struct Fr {}
    struct Fq12 {}
    struct G1 {}
    struct Gt {}
    struct FormatFrLsb {}
    struct FormatG1Compr {}
}

module 0x1::bn254_algebra {
    struct Fr {}
    struct Fq12 {}
    struct Gt {}
    struct FormatFrLsb {}
}

module 0x1::crypto_algebra {
    use 0x1::bls12381_algebra as bls;
    use 0x1::bn254_algebra as bn;

    native fun from_u64_internal<S>(value: u64): u64;
    native fun one_internal<S>(): u64;
    native fun serialize_internal<S, F>(handle: u64): vector<u8>;
    native fun upcast_internal<S, L>(handle: u64): u64;

    public fun fr_read_as_fr(): vector<u8> {
        one_internal<bls::G1>();
        serialize_internal<bls::Fr, bls::FormatFrLsb>(from_u64_internal<bls::Fr>(7))
    }

    public fun fr_read_as_g1(): vector<u8> {
        one_internal<bls::G1>();
        serialize_internal<bls::G1, bls::FormatG1Compr>(from_u64_internal<bls::Fr>(7))
    }

    public fun bn_fr_read_as_bn_fr(): vector<u8> {
        from_u64_internal<bn::Fr>(9);
        serialize_internal<bn::Fr, bn::FormatFrLsb>(from_u64_internal<bn::Fr>(7))
    }

    // Both Fr encodings are 32 bytes; only the structure differs.
    public fun bls_fr_read_as_bn_fr(): vector<u8> {
        from_u64_internal<bn::Fr>(9);
        serialize_internal<bn::Fr, bn::FormatFrLsb>(from_u64_internal<bls::Fr>(7))
    }

    public fun upcast_unchecked(): (u64, u64, u64) {
        let past_end = upcast_internal<bls::Gt, bls::Fq12>(12345);
        let max = upcast_internal<bn::Gt, bn::Fq12>(18446744073709551615);
        from_u64_internal<bls::Fr>(9);
        let fr = from_u64_internal<bls::Fr>(7);
        (past_end, max, upcast_internal<bls::Gt, bls::Fq12>(fr))
    }

    // The control: a real Gt element at handle 1.
    public fun upcast_gt(): u64 {
        from_u64_internal<bls::Fr>(9);
        upcast_internal<bls::Gt, bls::Fq12>(one_internal<bls::Gt>())
    }
}

// RUN: execute 0x1::crypto_algebra::fr_read_as_fr
// CHECK: results: 0x0700000000000000000000000000000000000000000000000000000000000000

// RUN: execute 0x1::crypto_algebra::fr_read_as_g1
// CHECK-V1-SUBSTR: VMError with status UNKNOWN_INVARIANT_VIOLATION_ERROR at location Module ModuleId { address: 0000000000000000000000000000000000000000000000000000000000000001, name: Identifier("crypto_algebra") } and message aptos_std::crypto_algebra native abort
// CHECK-V2: error: native function invariant violation: invalid crypto_algebra handle: 1

// RUN: execute 0x1::crypto_algebra::bn_fr_read_as_bn_fr
// CHECK: results: 0x0700000000000000000000000000000000000000000000000000000000000000

// RUN: execute 0x1::crypto_algebra::bls_fr_read_as_bn_fr
// CHECK-V1-SUBSTR: VMError with status UNKNOWN_INVARIANT_VIOLATION_ERROR at location Module ModuleId { address: 0000000000000000000000000000000000000000000000000000000000000001, name: Identifier("crypto_algebra") } and message aptos_std::crypto_algebra native abort
// CHECK-V2: error: native function invariant violation: invalid crypto_algebra handle: 1

// RUN: execute 0x1::crypto_algebra::upcast_unchecked
// CHECK: results: 12345, 18446744073709551615, 1

// RUN: execute 0x1::crypto_algebra::upcast_gt
// CHECK: results: 1
