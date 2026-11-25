// Degenerate pairing and multi-scalar multiplication (MSM) inputs on BLS12-381 and BN254: an
// empty multi_pairing and any pairing with a G1 or G2 identity give the Gt identity (the Fq12
// one), while e(g1, g2) gives the Gt generator. An empty MSM gives the identity of each of the
// four groups; a one-term MSM of the generator with scalar 1 gives the generator.

// RUN: publish
module 0x1::bls12381_algebra {
    struct Fr {}
    struct G1 {}
    struct G2 {}
    struct Gt {}
    struct FormatG1Compr {}
    struct FormatG2Compr {}
    struct FormatGt {}
}

module 0x1::bn254_algebra {
    struct Fr {}
    struct G1 {}
    struct G2 {}
    struct Gt {}
    struct FormatG1Compr {}
    struct FormatG2Compr {}
    struct FormatGt {}
}

module 0x1::crypto_algebra {
    use 0x1::bls12381_algebra as bls;
    use 0x1::bn254_algebra as bn;

    native fun eq_internal<S>(handle_1: u64, handle_2: u64): bool;
    native fun from_u64_internal<S>(value: u64): u64;
    native fun multi_pairing_internal<G1, G2, Gt>(
        g1_handles: vector<u64>, g2_handles: vector<u64>
    ): u64;
    native fun multi_scalar_mul_internal<G, S>(
        element_handles: vector<u64>, scalar_handles: vector<u64>
    ): u64;
    native fun one_internal<S>(): u64;
    native fun pairing_internal<G1, G2, Gt>(g1_handle: u64, g2_handle: u64): u64;
    native fun serialize_internal<S, F>(handle: u64): vector<u8>;
    native fun zero_internal<S>(): u64;

    fun bits(checks: vector<bool>): vector<u64> {
        checks.map(|check| if (check) 1 else 0)
    }

    // The Fq12 one: coefficient 1 followed by zeros.
    fun fq12_one_bytes(length: u64): vector<u8> {
        let bytes = vector[1u8];
        while (bytes.length() < length) bytes.push_back(0);
        bytes
    }

    // The empty multi_pairing is the Gt identity (by value and by encoding) and not the
    // generator; e(0, g2), e(g1, 0) and a one-pair multi_pairing with the identity are the Gt
    // identity; the one-pair multi_pairing of (g1, g2) and e(g1, g2) are the generator.
    fun pairings<Left, Right, Target, TargetFormat>(gt_length: u64): vector<bool> {
        let g1 = one_internal<Left>();
        let g2 = one_internal<Right>();
        let g1_zero = zero_internal<Left>();
        let g2_zero = zero_internal<Right>();
        let empty = multi_pairing_internal<Left, Right, Target>(vector[], vector[]);
        let identity = zero_internal<Target>();
        let generator = one_internal<Target>();
        vector[
            eq_internal<Target>(empty, identity),
            serialize_internal<Target, TargetFormat>(empty) == fq12_one_bytes(gt_length),
            eq_internal<Target>(empty, generator),
            eq_internal<Target>(pairing_internal<Left, Right, Target>(g1_zero, g2), identity),
            eq_internal<Target>(pairing_internal<Left, Right, Target>(g1, g2_zero), identity),
            eq_internal<Target>(
                multi_pairing_internal<Left, Right, Target>(vector[g1_zero], vector[g2]), identity
            ),
            eq_internal<Target>(
                multi_pairing_internal<Left, Right, Target>(vector[g1], vector[g2]), generator
            ),
            eq_internal<Target>(pairing_internal<Left, Right, Target>(g1, g2), generator),
        ]
    }

    public fun bls_pairings(): vector<u64> {
        bits(pairings<bls::G1, bls::G2, bls::Gt, bls::FormatGt>(576))
    }

    public fun bn_pairings(): vector<u64> {
        bits(pairings<bn::G1, bn::G2, bn::Gt, bn::FormatGt>(384))
    }

    // (empty MSM == identity, 1 * g == g) for one group.
    fun msm_pair<Group, Scalar>(): vector<bool> {
        let generator = one_internal<Group>();
        vector[
            eq_internal<Group>(
                multi_scalar_mul_internal<Group, Scalar>(vector[], vector[]), zero_internal<Group>()
            ),
            eq_internal<Group>(
                multi_scalar_mul_internal<Group, Scalar>(
                    vector[generator], vector[from_u64_internal<Scalar>(1)]
                ),
                generator
            ),
        ]
    }

    public fun empty_msm(): vector<u64> {
        let checks = msm_pair<bls::G1, bls::Fr>();
        checks.append(msm_pair<bls::G2, bls::Fr>());
        checks.append(msm_pair<bn::G1, bn::Fr>());
        checks.append(msm_pair<bn::G2, bn::Fr>());
        bits(checks)
    }

    // The empty MSM's encodings: the identity in each compressed format.
    public fun empty_msm_bytes(): (vector<u8>, vector<u8>, vector<u8>, vector<u8>) {
        (
            serialize_internal<bls::G1, bls::FormatG1Compr>(
                multi_scalar_mul_internal<bls::G1, bls::Fr>(vector[], vector[])
            ),
            serialize_internal<bls::G2, bls::FormatG2Compr>(
                multi_scalar_mul_internal<bls::G2, bls::Fr>(vector[], vector[])
            ),
            serialize_internal<bn::G1, bn::FormatG1Compr>(
                multi_scalar_mul_internal<bn::G1, bn::Fr>(vector[], vector[])
            ),
            serialize_internal<bn::G2, bn::FormatG2Compr>(
                multi_scalar_mul_internal<bn::G2, bn::Fr>(vector[], vector[])
            ),
        )
    }
}

// RUN: execute 0x1::crypto_algebra::bls_pairings
// CHECK: results: [1, 1, 0, 1, 1, 1, 1, 1]

// RUN: execute 0x1::crypto_algebra::bn_pairings
// CHECK: results: [1, 1, 0, 1, 1, 1, 1, 1]

// RUN: execute 0x1::crypto_algebra::empty_msm
// CHECK: results: [1, 1, 1, 1, 1, 1, 1, 1]

// RUN: execute 0x1::crypto_algebra::empty_msm_bytes
// CHECK: results: 0xc00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000, 0xc00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000, 0x0000000000000000000000000000000000000000000000000000000000000040, 0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000040
