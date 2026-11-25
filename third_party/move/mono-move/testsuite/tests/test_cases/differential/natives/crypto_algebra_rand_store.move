// `rand_insecure_internal` on every structure: each sample is stored and serializes to its
// structure's encoded size (values are random and not checked). Also: an unknown structure
// aborts, and creating elements in a loop until the 1048576-byte element memory limit is hit
// aborts with 589827, for random Fq12 samples and for Fq12 and BN254 G2 elements.

// RUN: publish
module 0x1::bls12381_algebra {
    struct Fr {}
    struct Fq12 {}
    struct G1 {}
    struct G2 {}
    struct Gt {}
    struct Unknown {}
    struct FormatFrLsb {}
    struct FormatFq12LscLsb {}
    struct FormatG1Compr {}
    struct FormatG2Compr {}
    struct FormatGt {}
}

module 0x1::bn254_algebra {
    struct Fr {}
    struct Fq {}
    struct Fq12 {}
    struct G1 {}
    struct G2 {}
    struct Gt {}
    struct FormatFrLsb {}
    struct FormatFqLsb {}
    struct FormatFq12LscLsb {}
    struct FormatG1Compr {}
    struct FormatG2Compr {}
    struct FormatGt {}
}

module 0x1::crypto_algebra {
    use 0x1::bls12381_algebra as bls;
    use 0x1::bn254_algebra as bn;

    native fun from_u64_internal<S>(value: u64): u64;
    native fun one_internal<S>(): u64;
    native fun rand_insecure_internal<S>(): u64;
    native fun serialize_internal<S, F>(handle: u64): vector<u8>;

    // The serialized size of each structure's sample.
    public fun bls_samples(): vector<u64> {
        vector[
            serialize_internal<bls::Fr, bls::FormatFrLsb>(rand_insecure_internal<bls::Fr>()).length(),
            serialize_internal<bls::Fq12, bls::FormatFq12LscLsb>(rand_insecure_internal<bls::Fq12>()).length(),
            serialize_internal<bls::G1, bls::FormatG1Compr>(rand_insecure_internal<bls::G1>()).length(),
            serialize_internal<bls::G2, bls::FormatG2Compr>(rand_insecure_internal<bls::G2>()).length(),
            serialize_internal<bls::Gt, bls::FormatGt>(rand_insecure_internal<bls::Gt>()).length(),
        ]
    }

    public fun bn_samples(): vector<u64> {
        vector[
            serialize_internal<bn::Fr, bn::FormatFrLsb>(rand_insecure_internal<bn::Fr>()).length(),
            serialize_internal<bn::Fq, bn::FormatFqLsb>(rand_insecure_internal<bn::Fq>()).length(),
            serialize_internal<bn::Fq12, bn::FormatFq12LscLsb>(rand_insecure_internal<bn::Fq12>()).length(),
            serialize_internal<bn::G1, bn::FormatG1Compr>(rand_insecure_internal<bn::G1>()).length(),
            serialize_internal<bn::G2, bn::FormatG2Compr>(rand_insecure_internal<bn::G2>()).length(),
            serialize_internal<bn::Gt, bn::FormatGt>(rand_insecure_internal<bn::Gt>()).length(),
        ]
    }

    public fun rand_unknown(): u64 {
        rand_insecure_internal<bls::Unknown>()
    }

    public fun rand_over_limit(): u64 {
        let last = 0;
        let index = 0;
        while (index < 100000) {
            last = rand_insecure_internal<bls::Fq12>();
            index += 1;
        };
        last
    }

    public fun fq12_over_limit(): u64 {
        let last = 0;
        let index = 0;
        while (index < 100000) {
            last = from_u64_internal<bls::Fq12>(index);
            index += 1;
        };
        last
    }

    public fun g2_over_limit(): u64 {
        let last = 0;
        let index = 0;
        while (index < 100000) {
            last = one_internal<bn::G2>();
            index += 1;
        };
        last
    }
}

// RUN: execute 0x1::crypto_algebra::bls_samples
// CHECK: results: [32, 576, 48, 96, 576]

// RUN: execute 0x1::crypto_algebra::bn_samples
// CHECK: results: [32, 32, 384, 32, 64, 384]

// RUN: execute 0x1::crypto_algebra::rand_unknown
// CHECK: aborted: code 655376 (Not implemented) in 0x1::crypto_algebra

// RUN: execute 0x1::crypto_algebra::rand_over_limit
// CHECK: aborted: code 589827 in 0x1::crypto_algebra

// RUN: execute 0x1::crypto_algebra::fq12_over_limit
// CHECK: aborted: code 589827 (Algebra context memory 1048576-byte limit exceeded: currently using 1048320 bytes; was asked for 1048896 bytes) in 0x1::crypto_algebra

// RUN: execute 0x1::crypto_algebra::g2_over_limit
// CHECK: aborted: code 589827 (Algebra context memory 1048576-byte limit exceeded: currently using 1048512 bytes; was asked for 1048704 bytes) in 0x1::crypto_algebra
