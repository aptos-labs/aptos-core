// `eq` on every structure of both curves: two handles holding the same value, built different
// ways, are equal, and two different values are not. Fq12 values that share the constant
// coefficient and differ only in the last are unequal on both curves. BN254 Fq subtraction in
// both orders is checked against the expected encodings and against a + (-b).

// RUN: publish
module 0x1::bls12381_algebra {
    struct Fr {}
    struct Fq12 {}
    struct G1 {}
    struct G2 {}
    struct Gt {}
    struct FormatFq12LscLsb {}
}

module 0x1::bn254_algebra {
    struct Fr {}
    struct Fq {}
    struct Fq12 {}
    struct G1 {}
    struct G2 {}
    struct Gt {}
    struct FormatFqLsb {}
    struct FormatFq12LscLsb {}
}

module 0x1::crypto_algebra {
    use 0x1::bls12381_algebra as bls;
    use 0x1::bn254_algebra as bn;

    native fun add_internal<S>(handle_1: u64, handle_2: u64): u64;
    native fun deserialize_internal<S, F>(bytes: &vector<u8>): (bool, u64);
    native fun double_internal<G>(element_handle: u64): u64;
    native fun eq_internal<S>(handle_1: u64, handle_2: u64): bool;
    native fun from_u64_internal<S>(value: u64): u64;
    native fun mul_internal<F>(handle_1: u64, handle_2: u64): u64;
    native fun neg_internal<F>(handle: u64): u64;
    native fun one_internal<S>(): u64;
    native fun serialize_internal<S, F>(handle: u64): vector<u8>;
    native fun sub_internal<G>(handle_1: u64, handle_2: u64): u64;

    fun bits(checks: vector<bool>): vector<u64> {
        checks.map(|check| if (check) 1 else 0)
    }

    // 4 == 2 * 2 and 4 != 5.
    fun field_pair<Field>(): (bool, bool) {
        let four = from_u64_internal<Field>(4);
        let two = from_u64_internal<Field>(2);
        (
            eq_internal<Field>(four, mul_internal<Field>(two, two)),
            eq_internal<Field>(four, from_u64_internal<Field>(5))
        )
    }

    // 2g == g + g and 2g != g, with g the generator.
    fun group_pair<Group>(): (bool, bool) {
        let generator = one_internal<Group>();
        let doubled = double_internal<Group>(generator);
        (
            eq_internal<Group>(doubled, add_internal<Group>(generator, generator)),
            eq_internal<Group>(doubled, generator)
        )
    }

    // For each structure: (equal values, different values).
    public fun bls_eq(): vector<u64> {
        let (fr_same, fr_diff) = field_pair<bls::Fr>();
        let (fq12_same, fq12_diff) = field_pair<bls::Fq12>();
        let (g1_same, g1_diff) = group_pair<bls::G1>();
        let (g2_same, g2_diff) = group_pair<bls::G2>();
        let (gt_same, gt_diff) = group_pair<bls::Gt>();
        bits(vector[
            fr_same, fr_diff, fq12_same, fq12_diff, g1_same, g1_diff, g2_same, g2_diff,
            gt_same, gt_diff,
        ])
    }

    public fun bn_eq(): vector<u64> {
        let (fr_same, fr_diff) = field_pair<bn::Fr>();
        let (fq_same, fq_diff) = field_pair<bn::Fq>();
        let (fq12_same, fq12_diff) = field_pair<bn::Fq12>();
        let (g1_same, g1_diff) = group_pair<bn::G1>();
        let (g2_same, g2_diff) = group_pair<bn::G2>();
        let (gt_same, gt_diff) = group_pair<bn::Gt>();
        bits(vector[
            fr_same, fr_diff, fq_same, fq_diff, fq12_same, fq12_diff, g1_same, g1_diff,
            g2_same, g2_diff, gt_same, gt_diff,
        ])
    }

    // An Fq12 encoding of 12 coefficients of `length` bytes: 7 in the first
    // (constant) coefficient, `last` in the last one, zeros elsewhere.
    fun fq12_bytes(length: u64, last: u8): vector<u8> {
        let bytes = vector[];
        while (bytes.length() < 12 * length) bytes.push_back(0);
        bytes[0] = 7;
        bytes[11 * length] = last;
        bytes
    }

    // (two decodings of the same bytes are equal, elements that differ only
    // in the last coefficient are not).
    fun fq12_last_pair<Field, Format>(length: u64): (bool, bool) {
        let (ok_1, one_last) = deserialize_internal<Field, Format>(&fq12_bytes(length, 1));
        let (ok_2, one_last_again) = deserialize_internal<Field, Format>(&fq12_bytes(length, 1));
        let (ok_3, two_last) = deserialize_internal<Field, Format>(&fq12_bytes(length, 2));
        assert!(ok_1 && ok_2 && ok_3, 1);
        (eq_internal<Field>(one_last, one_last_again), eq_internal<Field>(one_last, two_last))
    }

    // Fq12 values from u64 differ only in the constant coefficient; these
    // share it and differ in the last.
    public fun fq12_last_coefficient(): vector<u64> {
        let (bls_same, bls_diff) = fq12_last_pair<bls::Fq12, bls::FormatFq12LscLsb>(48);
        let (bn_same, bn_diff) = fq12_last_pair<bn::Fq12, bn::FormatFq12LscLsb>(32);
        bits(vector[bls_same, bls_diff, bn_same, bn_diff])
    }

    // 5 - 3 = 2 and 3 - 5 = p - 2 (little-endian), and 5 - 3 == 5 + (-3).
    public fun bn_fq_sub(): (vector<u8>, vector<u8>, bool) {
        let five = from_u64_internal<bn::Fq>(5);
        let three = from_u64_internal<bn::Fq>(3);
        (
            serialize_internal<bn::Fq, bn::FormatFqLsb>(sub_internal<bn::Fq>(five, three)),
            serialize_internal<bn::Fq, bn::FormatFqLsb>(sub_internal<bn::Fq>(three, five)),
            eq_internal<bn::Fq>(
                sub_internal<bn::Fq>(five, three),
                add_internal<bn::Fq>(five, neg_internal<bn::Fq>(three))
            )
        )
    }
}

// RUN: execute 0x1::crypto_algebra::bls_eq
// CHECK: results: [1, 0, 1, 0, 1, 0, 1, 0, 1, 0]

// RUN: execute 0x1::crypto_algebra::bn_eq
// CHECK: results: [1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0]

// RUN: execute 0x1::crypto_algebra::fq12_last_coefficient
// CHECK: results: [1, 0, 1, 0]

// RUN: execute 0x1::crypto_algebra::bn_fq_sub
// CHECK: results: 0x0200000000000000000000000000000000000000000000000000000000000000, 0x45fd7cd8168c203c8dca7168916a81975d588181b64550b829a031e1724e6430, true
