// Unsupported type-argument combinations abort with 786433 (not implemented) in add, sub, mul,
// div, neg, inv, eq, from_u64, pairing, multi_pairing, hash_to, scalar_mul and deserialize.
// Each has a supported counterpart in a `*_controls` step that changes only the marker.
// `unread_handles` repeats the handle-taking cases with handles no store holds and still gets
// 786433, so type arguments are checked before handles.

// RUN: publish
module 0x1::bls12381_algebra {
    struct Fr {}
    struct Fq12 {}
    struct G1 {}
    struct G2 {}
    struct Gt {}
    struct FormatFrLsb {}
    struct FormatFq12LscLsb {}
    struct FormatG1Uncompr {}
    struct FormatGt {}
    struct HashG1XmdSha256SswuRo {}
    struct HashG2XmdSha256SswuRo {}
    struct Unknown {}
}

module 0x1::bn254_algebra {
    struct Fr {}
    struct Fq12 {}
    struct G1 {}
    struct G2 {}
    struct Gt {}
    struct FormatG1Compr {}
}

module 0x1::crypto_algebra {
    use 0x1::bls12381_algebra as bls;
    use 0x1::bn254_algebra as bn;

    const BLS_FR_5_LSB: vector<u8> = x"0500000000000000000000000000000000000000000000000000000000000000";
    // r - 5, where r is the BLS12-381 scalar field modulus.
    const BLS_FR_MINUS_5_LSB: vector<u8> = x"fcfffffffefffffffe5bfeff02a4bd5305d8a10908d83933487d9d2953a7ed73";
    const BN_G1_GENERATOR_COMP: vector<u8> = x"0100000000000000000000000000000000000000000000000000000000000000";
    // RFC 9380 test DST for BLS12381G1_XMD:SHA-256_SSWU_RO_; `hash_to_control` expects the
    // RFC's point for msg "".
    const G1_DST: vector<u8> = b"QUUX-V01-CS02-with-BLS12381G1_XMD:SHA-256_SSWU_RO_";

    native fun add_internal<S>(handle_1: u64, handle_2: u64): u64;
    native fun deserialize_internal<S, F>(bytes: &vector<u8>): (bool, u64);
    native fun div_internal<F>(handle_1: u64, handle_2: u64): (bool, u64);
    native fun double_internal<G>(element_handle: u64): u64;
    native fun eq_internal<S>(handle_1: u64, handle_2: u64): bool;
    native fun from_u64_internal<S>(value: u64): u64;
    native fun hash_to_internal<S, H>(dst: &vector<u8>, bytes: &vector<u8>): u64;
    native fun inv_internal<F>(handle: u64): (bool, u64);
    native fun mul_internal<F>(handle_1: u64, handle_2: u64): u64;
    native fun multi_pairing_internal<G1, G2, Gt>(
        g1_handles: vector<u64>, g2_handles: vector<u64>
    ): u64;
    native fun neg_internal<F>(handle: u64): u64;
    native fun one_internal<S>(): u64;
    native fun pairing_internal<G1, G2, Gt>(g1_handle: u64, g2_handle: u64): u64;
    native fun scalar_mul_internal<G, S>(element_handle: u64, scalar_handle: u64): u64;
    native fun serialize_internal<S, F>(handle: u64): vector<u8>;
    native fun sub_internal<G>(handle_1: u64, handle_2: u64): u64;

    fun bits(checks: vector<bool>): vector<u64> {
        checks.map(|check| if (check) 1 else 0)
    }

    fun bls_fr_bytes(handle: u64): vector<u8> {
        serialize_internal<bls::Fr, bls::FormatFrLsb>(handle)
    }

    fun bls_gt_bytes(): vector<u8> {
        serialize_internal<bls::Gt, bls::FormatGt>(one_internal<bls::Gt>())
    }

    // --- arithmetic ---

    // 5 + 5, 5 - 5, -5, 5 == 5 on BLS Fr; Gt handles read as Fq12: gt * gt,
    // gt / gt, gt^-1, and from_u64 on BN Fq12.
    public fun arithmetic_controls(): vector<u64> {
        let five = from_u64_internal<bls::Fr>(5);
        let bls_gt = one_internal<bls::Gt>();
        let bn_gt = one_internal<bn::Gt>();
        let (div_ok, quotient) = div_internal<bn::Fq12>(bn_gt, bn_gt);
        let (inv_ok, inverse) = inv_internal<bls::Fq12>(bls_gt);
        bits(vector[
            bls_fr_bytes(add_internal<bls::Fr>(five, five)) == x"0a00000000000000000000000000000000000000000000000000000000000000",
            bls_fr_bytes(sub_internal<bls::Fr>(five, five)) == x"0000000000000000000000000000000000000000000000000000000000000000",
            bls_fr_bytes(neg_internal<bls::Fr>(five)) == BLS_FR_MINUS_5_LSB,
            eq_internal<bls::Fr>(five, five),
            eq_internal<bls::Gt>(
                mul_internal<bls::Fq12>(bls_gt, bls_gt), double_internal<bls::Gt>(bls_gt)
            ),
            div_ok && eq_internal<bn::Fq12>(quotient, one_internal<bn::Fq12>()),
            inv_ok && eq_internal<bls::Gt>(inverse, neg_internal<bls::Gt>(bls_gt)),
            eq_internal<bn::Fq12>(
                from_u64_internal<bn::Fq12>(7),
                add_internal<bn::Fq12>(from_u64_internal<bn::Fq12>(3), from_u64_internal<bn::Fq12>(4))
            ),
        ])
    }

    public fun add_unknown(): u64 {
        let five = from_u64_internal<bls::Fr>(5);
        add_internal<bls::Unknown>(five, five)
    }

    public fun sub_unknown(): u64 {
        let five = from_u64_internal<bls::Fr>(5);
        sub_internal<bls::Unknown>(five, five)
    }

    public fun neg_unknown(): u64 {
        neg_internal<bls::Unknown>(from_u64_internal<bls::Fr>(5))
    }

    public fun eq_unknown(): bool {
        let five = from_u64_internal<bls::Fr>(5);
        eq_internal<bls::Unknown>(five, five)
    }

    public fun mul_on_gt(): u64 {
        let gt = one_internal<bls::Gt>();
        mul_internal<bls::Gt>(gt, gt)
    }

    public fun div_on_gt(): (bool, u64) {
        let gt = one_internal<bn::Gt>();
        div_internal<bn::Gt>(gt, gt)
    }

    public fun inv_on_gt(): (bool, u64) {
        inv_internal<bls::Gt>(one_internal<bls::Gt>())
    }

    public fun from_u64_on_gt(): u64 {
        from_u64_internal<bn::Gt>(7)
    }

    public fun from_u64_on_g1(): u64 {
        from_u64_internal<bls::G1>(7)
    }

    // --- pairing ---

    // e(g1, g2) is the Gt generator on both curves, through pairing and
    // through a one-pair multi_pairing.
    public fun pairing_controls(): vector<u64> {
        bits(vector[
            eq_internal<bls::Gt>(
                pairing_internal<bls::G1, bls::G2, bls::Gt>(
                    one_internal<bls::G1>(), one_internal<bls::G2>()
                ),
                one_internal<bls::Gt>()
            ),
            eq_internal<bn::Gt>(
                multi_pairing_internal<bn::G1, bn::G2, bn::Gt>(
                    vector[one_internal<bn::G1>()], vector[one_internal<bn::G2>()]
                ),
                one_internal<bn::Gt>()
            ),
        ])
    }

    public fun pairing_mixed_curves(): u64 {
        pairing_internal<bls::G1, bn::G2, bls::Gt>(one_internal<bls::G1>(), one_internal<bls::G2>())
    }

    public fun pairing_foreign_gt(): u64 {
        pairing_internal<bls::G1, bls::G2, bn::Gt>(one_internal<bls::G1>(), one_internal<bls::G2>())
    }

    public fun multi_pairing_mixed_curves(): u64 {
        multi_pairing_internal<bls::G1, bn::G2, bn::Gt>(
            vector[one_internal<bn::G1>()], vector[one_internal<bn::G2>()]
        )
    }

    // Fq12 in place of Gt as the pairing's target group.
    public fun multi_pairing_fq12_target(): u64 {
        multi_pairing_internal<bn::G1, bn::G2, bn::Fq12>(
            vector[one_internal<bn::G1>()], vector[one_internal<bn::G2>()]
        )
    }

    // --- hash_to ---

    public fun hash_to_control(): vector<u8> {
        serialize_internal<bls::G1, bls::FormatG1Uncompr>(
            hash_to_internal<bls::G1, bls::HashG1XmdSha256SswuRo>(&G1_DST, &b"")
        )
    }

    public fun hash_to_g1_with_g2_suite(): u64 {
        hash_to_internal<bls::G1, bls::HashG2XmdSha256SswuRo>(&G1_DST, &b"")
    }

    public fun hash_to_fr(): u64 {
        hash_to_internal<bls::Fr, bls::HashG1XmdSha256SswuRo>(&G1_DST, &b"")
    }

    public fun hash_to_unknown_suite(): u64 {
        hash_to_internal<bls::G1, bls::Unknown>(&G1_DST, &b"")
    }

    // --- scalar_mul ---

    // 3 * g1 on BLS G1 and gt^3 on BN Gt.
    public fun scalar_mul_controls(): vector<u64> {
        let g1 = one_internal<bls::G1>();
        let bn_gt = one_internal<bn::Gt>();
        bits(vector[
            eq_internal<bls::G1>(
                scalar_mul_internal<bls::G1, bls::Fr>(g1, from_u64_internal<bls::Fr>(3)),
                add_internal<bls::G1>(g1, double_internal<bls::G1>(g1))
            ),
            eq_internal<bn::Gt>(
                scalar_mul_internal<bn::Gt, bn::Fr>(bn_gt, from_u64_internal<bn::Fr>(3)),
                add_internal<bn::Gt>(bn_gt, double_internal<bn::Gt>(bn_gt))
            ),
        ])
    }

    // The scalar handle is a BLS Fr element; only the scalar marker is foreign.
    public fun scalar_mul_foreign_scalar_field(): u64 {
        scalar_mul_internal<bls::G1, bn::Fr>(one_internal<bls::G1>(), from_u64_internal<bls::Fr>(3))
    }

    public fun scalar_mul_on_fq12(): u64 {
        scalar_mul_internal<bn::Fq12, bn::Fr>(one_internal<bn::Gt>(), from_u64_internal<bn::Fr>(3))
    }

    // --- deserialize ---

    // Each set of bytes below deserializes under its own format.
    public fun deserialize_controls(): vector<u64> {
        let (fr_ok, fr) = deserialize_internal<bls::Fr, bls::FormatFrLsb>(&BLS_FR_5_LSB);
        let (bn_ok, bn_g1) = deserialize_internal<bn::G1, bn::FormatG1Compr>(&BN_G1_GENERATOR_COMP);
        let gt_bytes = bls_gt_bytes();
        let (gt_ok, gt) = deserialize_internal<bls::Gt, bls::FormatGt>(&gt_bytes);
        let (fq12_ok, _) = deserialize_internal<bls::Fq12, bls::FormatFq12LscLsb>(&gt_bytes);
        bits(vector[
            fr_ok && eq_internal<bls::Fr>(fr, from_u64_internal<bls::Fr>(5)),
            bn_ok && eq_internal<bn::G1>(bn_g1, one_internal<bn::G1>()),
            gt_ok && eq_internal<bls::Gt>(gt, one_internal<bls::Gt>()),
            fq12_ok,
        ])
    }

    public fun deserialize_foreign_format(): (bool, u64) {
        deserialize_internal<bls::G1, bls::FormatFrLsb>(&BLS_FR_5_LSB)
    }

    public fun deserialize_other_curve_format(): (bool, u64) {
        deserialize_internal<bls::G1, bn::FormatG1Compr>(&BN_G1_GENERATOR_COMP)
    }

    public fun deserialize_gt_as_fq12_format(): (bool, u64) {
        let gt_bytes = bls_gt_bytes();
        deserialize_internal<bls::Gt, bls::FormatFq12LscLsb>(&gt_bytes)
    }

    // --- handle lookup order ---

    // Each handle-taking unsupported call above, on handles no store holds.
    public fun unread_handles(case: u64): u64 {
        let missing = 1000;
        let other = 1001;
        if (case == 0) {
            add_internal<bls::Unknown>(missing, other)
        } else if (case == 1) {
            sub_internal<bls::Unknown>(missing, other)
        } else if (case == 2) {
            neg_internal<bls::Unknown>(missing)
        } else if (case == 3) {
            if (eq_internal<bls::Unknown>(missing, other)) 1 else 0
        } else if (case == 4) {
            mul_internal<bls::Gt>(missing, other)
        } else if (case == 5) {
            let (_, quotient) = div_internal<bn::Gt>(missing, other);
            quotient
        } else if (case == 6) {
            let (_, inverse) = inv_internal<bls::Gt>(missing);
            inverse
        } else if (case == 7) {
            pairing_internal<bls::G1, bn::G2, bls::Gt>(missing, other)
        } else if (case == 8) {
            multi_pairing_internal<bn::G1, bn::G2, bn::Fq12>(vector[missing], vector[other])
        } else {
            scalar_mul_internal<bls::G1, bn::Fr>(missing, other)
        }
    }
}

// RUN: execute 0x1::crypto_algebra::arithmetic_controls
// CHECK: results: [1, 1, 1, 1, 1, 1, 1, 1]

// RUN: execute 0x1::crypto_algebra::add_unknown
// CHECK: aborted: code 786433 in 0x1::crypto_algebra

// RUN: execute 0x1::crypto_algebra::sub_unknown
// CHECK: aborted: code 786433 in 0x1::crypto_algebra

// RUN: execute 0x1::crypto_algebra::neg_unknown
// CHECK: aborted: code 786433 in 0x1::crypto_algebra

// RUN: execute 0x1::crypto_algebra::eq_unknown
// CHECK: aborted: code 786433 in 0x1::crypto_algebra

// RUN: execute 0x1::crypto_algebra::mul_on_gt
// CHECK: aborted: code 786433 in 0x1::crypto_algebra

// RUN: execute 0x1::crypto_algebra::div_on_gt
// CHECK: aborted: code 786433 in 0x1::crypto_algebra

// RUN: execute 0x1::crypto_algebra::inv_on_gt
// CHECK: aborted: code 786433 in 0x1::crypto_algebra

// RUN: execute 0x1::crypto_algebra::from_u64_on_gt
// CHECK: aborted: code 786433 in 0x1::crypto_algebra

// RUN: execute 0x1::crypto_algebra::from_u64_on_g1
// CHECK: aborted: code 786433 in 0x1::crypto_algebra

// RUN: execute 0x1::crypto_algebra::pairing_controls
// CHECK: results: [1, 1]

// RUN: execute 0x1::crypto_algebra::pairing_mixed_curves
// CHECK: aborted: code 786433 in 0x1::crypto_algebra

// RUN: execute 0x1::crypto_algebra::pairing_foreign_gt
// CHECK: aborted: code 786433 in 0x1::crypto_algebra

// RUN: execute 0x1::crypto_algebra::multi_pairing_mixed_curves
// CHECK: aborted: code 786433 in 0x1::crypto_algebra

// RUN: execute 0x1::crypto_algebra::multi_pairing_fq12_target
// CHECK: aborted: code 786433 in 0x1::crypto_algebra

// RUN: execute 0x1::crypto_algebra::hash_to_control
// CHECK: results: 0x052926add2207b76ca4fa57a8734416c8dc95e24501772c814278700eed6d1e4e8cf62d9c09db0fac349612b759e79a108ba738453bfed09cb546dbb0783dbb3a5f1f566ed67bb6be0e8c67e2e81a4cc68ee29813bb7994998f3eae0c9c6a265

// RUN: execute 0x1::crypto_algebra::hash_to_g1_with_g2_suite
// CHECK: aborted: code 786433 in 0x1::crypto_algebra

// RUN: execute 0x1::crypto_algebra::hash_to_fr
// CHECK: aborted: code 786433 in 0x1::crypto_algebra

// RUN: execute 0x1::crypto_algebra::hash_to_unknown_suite
// CHECK: aborted: code 786433 in 0x1::crypto_algebra

// RUN: execute 0x1::crypto_algebra::scalar_mul_controls
// CHECK: results: [1, 1]

// RUN: execute 0x1::crypto_algebra::scalar_mul_foreign_scalar_field
// CHECK: aborted: code 786433 in 0x1::crypto_algebra

// RUN: execute 0x1::crypto_algebra::scalar_mul_on_fq12
// CHECK: aborted: code 786433 in 0x1::crypto_algebra

// RUN: execute 0x1::crypto_algebra::deserialize_controls
// CHECK: results: [1, 1, 1, 1]

// RUN: execute 0x1::crypto_algebra::deserialize_foreign_format
// CHECK: aborted: code 786433 in 0x1::crypto_algebra

// RUN: execute 0x1::crypto_algebra::deserialize_other_curve_format
// CHECK: aborted: code 786433 in 0x1::crypto_algebra

// RUN: execute 0x1::crypto_algebra::deserialize_gt_as_fq12_format
// CHECK: aborted: code 786433 in 0x1::crypto_algebra

// RUN: execute 0x1::crypto_algebra::unread_handles --args 0
// CHECK: aborted: code 786433 in 0x1::crypto_algebra

// RUN: execute 0x1::crypto_algebra::unread_handles --args 1
// CHECK: aborted: code 786433 in 0x1::crypto_algebra

// RUN: execute 0x1::crypto_algebra::unread_handles --args 2
// CHECK: aborted: code 786433 in 0x1::crypto_algebra

// RUN: execute 0x1::crypto_algebra::unread_handles --args 3
// CHECK: aborted: code 786433 in 0x1::crypto_algebra

// RUN: execute 0x1::crypto_algebra::unread_handles --args 4
// CHECK: aborted: code 786433 in 0x1::crypto_algebra

// RUN: execute 0x1::crypto_algebra::unread_handles --args 5
// CHECK: aborted: code 786433 in 0x1::crypto_algebra

// RUN: execute 0x1::crypto_algebra::unread_handles --args 6
// CHECK: aborted: code 786433 in 0x1::crypto_algebra

// RUN: execute 0x1::crypto_algebra::unread_handles --args 7
// CHECK: aborted: code 786433 in 0x1::crypto_algebra

// RUN: execute 0x1::crypto_algebra::unread_handles --args 8
// CHECK: aborted: code 786433 in 0x1::crypto_algebra

// RUN: execute 0x1::crypto_algebra::unread_handles --args 9
// CHECK: aborted: code 786433 in 0x1::crypto_algebra
