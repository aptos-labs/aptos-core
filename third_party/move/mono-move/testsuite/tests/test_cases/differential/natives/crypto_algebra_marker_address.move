// crypto_algebra recognizes marker types only at address 0x1: structure, format and suite
// markers with the same module and struct names at 0x42 abort with 786433 (not implemented),
// while the same calls with the 0x1 markers succeed.

// RUN: publish
module 0x1::bls12381_algebra {
    struct Fr {}
    struct G1 {}
    struct FormatFrLsb {}
    struct FormatG1Uncompr {}
    struct HashG1XmdSha256SswuRo {}
}

module 0x42::bls12381_algebra {
    struct Fr {}
    struct FormatFrLsb {}
    struct HashG1XmdSha256SswuRo {}
}

module 0x1::crypto_algebra {
    use 0x1::bls12381_algebra as bls;
    use 0x42::bls12381_algebra as foreign;

    // RFC 9380 test DST for BLS12381G1_XMD:SHA-256_SSWU_RO_; `suite_control` expects the
    // RFC's point for msg "".
    const DST: vector<u8> = b"QUUX-V01-CS02-with-BLS12381G1_XMD:SHA-256_SSWU_RO_";

    native fun from_u64_internal<S>(value: u64): u64;
    native fun hash_to_internal<S, H>(dst: &vector<u8>, bytes: &vector<u8>): u64;
    native fun serialize_internal<S, F>(handle: u64): vector<u8>;

    public fun structure_control(): vector<u8> {
        serialize_internal<bls::Fr, bls::FormatFrLsb>(from_u64_internal<bls::Fr>(7))
    }

    public fun foreign_structure(): u64 {
        from_u64_internal<foreign::Fr>(7)
    }

    public fun foreign_format(): vector<u8> {
        serialize_internal<bls::Fr, foreign::FormatFrLsb>(from_u64_internal<bls::Fr>(7))
    }

    public fun suite_control(): vector<u8> {
        serialize_internal<bls::G1, bls::FormatG1Uncompr>(
            hash_to_internal<bls::G1, bls::HashG1XmdSha256SswuRo>(&DST, &b"")
        )
    }

    public fun foreign_suite(): u64 {
        hash_to_internal<bls::G1, foreign::HashG1XmdSha256SswuRo>(&DST, &b"")
    }
}

// RUN: execute 0x1::crypto_algebra::structure_control
// CHECK: results: 0x0700000000000000000000000000000000000000000000000000000000000000

// RUN: execute 0x1::crypto_algebra::foreign_structure
// CHECK: aborted: code 786433 in 0x1::crypto_algebra

// RUN: execute 0x1::crypto_algebra::foreign_format
// CHECK: aborted: code 786433 in 0x1::crypto_algebra

// RUN: execute 0x1::crypto_algebra::suite_control
// CHECK: results: 0x052926add2207b76ca4fa57a8734416c8dc95e24501772c814278700eed6d1e4e8cf62d9c09db0fac349612b759e79a108ba738453bfed09cb546dbb0783dbb3a5f1f566ed67bb6be0e8c67e2e81a4cc68ee29813bb7994998f3eae0c9c6a265

// RUN: execute 0x1::crypto_algebra::foreign_suite
// CHECK: aborted: code 786433 in 0x1::crypto_algebra
