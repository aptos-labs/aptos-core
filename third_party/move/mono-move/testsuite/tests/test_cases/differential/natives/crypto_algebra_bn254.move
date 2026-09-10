// RUN: publish
module 0x1::bn254_algebra {
    struct Fr {}
    struct Fq {}
    struct Fq12 {}
    struct G1 {}
    struct G2 {}
    struct Gt {}
    struct FormatFrLsb {}
    struct FormatFrMsb {}
    struct FormatFqLsb {}
    struct FormatFqMsb {}
    struct FormatG1Compr {}
    struct FormatG2Compr {}
    struct FormatGt {}
}

module 0x1::crypto_algebra {
    use 0x1::bn254_algebra::{
        Fq,
        Fq12,
        FormatFqLsb,
        FormatFqMsb,
        FormatFrLsb,
        FormatFrMsb,
        FormatG1Compr,
        FormatG2Compr,
        FormatGt,
        Fr,
        G1,
        G2,
        Gt
    };

    native fun add_internal<S>(handle_1: u64, handle_2: u64): u64;
    native fun div_internal<F>(handle_1: u64, handle_2: u64): (bool, u64);
    native fun double_internal<G>(element_handle: u64): u64;
    native fun downcast_internal<L, S>(handle: u64): (bool, u64);
    native fun from_u64_internal<S>(value: u64): u64;
    native fun eq_internal<S>(handle_1: u64, handle_2: u64): bool;
    native fun inv_internal<F>(handle: u64): (bool, u64);
    native fun mul_internal<F>(handle_1: u64, handle_2: u64): u64;
    native fun multi_pairing_internal<G1, G2, Gt>(
        g1_handles: vector<u64>, g2_handles: vector<u64>
    ): u64;
    native fun multi_scalar_mul_internal<G, S>(
        element_handles: vector<u64>, scalar_handles: vector<u64>
    ): u64;
    native fun neg_internal<F>(handle: u64): u64;
    native fun one_internal<S>(): u64;
    native fun order_internal<G>(): vector<u8>;
    native fun pairing_internal<G1, G2, Gt>(g1_handle: u64, g2_handle: u64): u64;
    native fun scalar_mul_internal<G, S>(element_handle: u64, scalar_handle: u64): u64;
    native fun serialize_internal<S, F>(handle: u64): vector<u8>;
    native fun sqr_internal<G>(handle: u64): u64;
    native fun sub_internal<G>(handle_1: u64, handle_2: u64): u64;
    native fun upcast_internal<S, L>(handle: u64): u64;
    native fun zero_internal<S>(): u64;

    public fun fr_ring_laws(): bool {
        let a = from_u64_internal<Fr>(7);
        let b = from_u64_internal<Fr>(5);
        eq_internal<Fr>(add_internal<Fr>(a, b), from_u64_internal<Fr>(12))
            && eq_internal<Fr>(mul_internal<Fr>(a, b), from_u64_internal<Fr>(35))
            && eq_internal<Fr>(sub_internal<Fr>(add_internal<Fr>(a, b), b), a)
            && eq_internal<Fr>(add_internal<Fr>(a, neg_internal<Fr>(a)), zero_internal<Fr>())
            && eq_internal<Fr>(sqr_internal<Fr>(a), mul_internal<Fr>(a, a))
    }

    // `Fq` is the structure BLS12-381 does not expose.
    public fun fq_field_laws(): bool {
        let a = from_u64_internal<Fq>(7);
        let b = from_u64_internal<Fq>(5);
        let (div_ok, q) = div_internal<Fq>(a, b);
        let (inv_ok, a_inv) = inv_internal<Fq>(a);
        div_ok
            && inv_ok
            && eq_internal<Fq>(mul_internal<Fq>(q, b), a)
            && eq_internal<Fq>(mul_internal<Fq>(a, a_inv), one_internal<Fq>())
            && eq_internal<Fq>(sqr_internal<Fq>(a), mul_internal<Fq>(a, a))
            && eq_internal<Fq>(sub_internal<Fq>(a, a), zero_internal<Fq>())
    }

    public fun fq12_field_laws(): bool {
        let a = from_u64_internal<Fq12>(7);
        let b = from_u64_internal<Fq12>(5);
        let (div_ok, q) = div_internal<Fq12>(a, b);
        let (inv_ok, a_inv) = inv_internal<Fq12>(a);
        div_ok
            && inv_ok
            && eq_internal<Fq12>(mul_internal<Fq12>(q, b), a)
            && eq_internal<Fq12>(mul_internal<Fq12>(a, a_inv), one_internal<Fq12>())
            && eq_internal<Fq12>(sqr_internal<Fq12>(a), mul_internal<Fq12>(a, a))
    }

    public fun fr_order(): vector<u8> {
        order_internal<Fr>()
    }

    // `Fq`'s order is `q`, not `r`; the two must not be confused.
    public fun fq_order(): vector<u8> {
        order_internal<Fq>()
    }

    public fun fr_serialize_both_endians(): (vector<u8>, vector<u8>) {
        let seven = from_u64_internal<Fr>(7);
        (
            serialize_internal<Fr, FormatFrLsb>(seven),
            serialize_internal<Fr, FormatFrMsb>(seven)
        )
    }

    public fun fq_serialize_both_endians(): (vector<u8>, vector<u8>) {
        let seven = from_u64_internal<Fq>(7);
        (
            serialize_internal<Fq, FormatFqLsb>(seven),
            serialize_internal<Fq, FormatFqMsb>(seven)
        )
    }

    public fun g1_generator_serialized(): vector<u8> {
        serialize_internal<G1, FormatG1Compr>(one_internal<G1>())
    }

    public fun g2_generator_serialized(): vector<u8> {
        serialize_internal<G2, FormatG2Compr>(one_internal<G2>())
    }

    public fun gt_generator_serialized(): vector<u8> {
        serialize_internal<Gt, FormatGt>(one_internal<Gt>())
    }

    public fun g1_scalar_mul(): vector<u8> {
        let g = one_internal<G1>();
        let by_scalar = scalar_mul_internal<G1, Fr>(g, from_u64_internal<Fr>(7));
        let by_add =
            add_internal<G1>(
                add_internal<G1>(double_internal<G1>(double_internal<G1>(g)), double_internal<G1>(g)),
                g
            );
        assert!(eq_internal<G1>(by_scalar, by_add), 1);
        serialize_internal<G1, FormatG1Compr>(by_scalar)
    }

    // 3*G + 5*(2*G) == 13*G.
    public fun g1_multi_scalar_mul(): bool {
        let g = one_internal<G1>();
        let two_g = double_internal<G1>(g);
        let combined =
            multi_scalar_mul_internal<G1, Fr>(
                vector[g, two_g],
                vector[from_u64_internal<Fr>(3), from_u64_internal<Fr>(5)]
            );
        eq_internal<G1>(combined, scalar_mul_internal<G1, Fr>(g, from_u64_internal<Fr>(13)))
    }

    public fun g2_multi_scalar_mul(): bool {
        let g = one_internal<G2>();
        let two_g = double_internal<G2>(g);
        let combined =
            multi_scalar_mul_internal<G2, Fr>(
                vector[g, two_g],
                vector[from_u64_internal<Fr>(3), from_u64_internal<Fr>(5)]
            );
        eq_internal<G2>(combined, scalar_mul_internal<G2, Fr>(g, from_u64_internal<Fr>(13)))
    }

    public fun gt_group_laws(): bool {
        let g = one_internal<Gt>();
        let two_g = double_internal<Gt>(g);
        eq_internal<Gt>(add_internal<Gt>(g, zero_internal<Gt>()), g)
            && eq_internal<Gt>(two_g, add_internal<Gt>(g, g))
            && eq_internal<Gt>(add_internal<Gt>(g, neg_internal<Gt>(g)), zero_internal<Gt>())
            && eq_internal<Gt>(sub_internal<Gt>(two_g, g), g)
            && eq_internal<Gt>(scalar_mul_internal<Gt, Fr>(g, from_u64_internal<Fr>(2)), two_g)
    }

    public fun pairing_bilinearity(): bool {
        let a = from_u64_internal<Fr>(7);
        let b = from_u64_internal<Fr>(11);
        let p = one_internal<G1>();
        let q = one_internal<G2>();
        let lhs =
            pairing_internal<G1, G2, Gt>(
                scalar_mul_internal<G1, Fr>(p, a),
                scalar_mul_internal<G2, Fr>(q, b)
            );
        let rhs =
            scalar_mul_internal<Gt, Fr>(
                pairing_internal<G1, G2, Gt>(p, q), mul_internal<Fr>(a, b)
            );
        eq_internal<Gt>(lhs, rhs)
    }

    public fun multi_pairing_matches_naive(): bool {
        let p0 = one_internal<G1>();
        let p1 = scalar_mul_internal<G1, Fr>(p0, from_u64_internal<Fr>(3));
        let q0 = one_internal<G2>();
        let q1 = scalar_mul_internal<G2, Fr>(q0, from_u64_internal<Fr>(5));
        let naive =
            add_internal<Gt>(
                pairing_internal<G1, G2, Gt>(p0, q0),
                pairing_internal<G1, G2, Gt>(p1, q1)
            );
        eq_internal<Gt>(naive, multi_pairing_internal<G1, G2, Gt>(vector[p0, p1], vector[q0, q1]))
    }

    public fun gt_fq12_cast_roundtrip(): bool {
        let gt = one_internal<Gt>();
        let fq12 = upcast_internal<Gt, Fq12>(gt);
        let (in_subgroup, back) = downcast_internal<Fq12, Gt>(fq12);
        in_subgroup && back == gt
    }

    public fun fq12_not_in_gt(): (bool, bool) {
        let seven = from_u64_internal<Fq12>(7);
        let (in_subgroup, back) = downcast_internal<Fq12, Gt>(seven);
        (in_subgroup, back == seven)
    }
}

// RUN: execute 0x1::crypto_algebra::fr_ring_laws
// CHECK: results: true

// RUN: execute 0x1::crypto_algebra::fq_field_laws
// CHECK: results: true

// RUN: execute 0x1::crypto_algebra::fq12_field_laws
// CHECK: results: true

// RUN: execute 0x1::crypto_algebra::fr_order
// CHECK: results: 0x010000f093f5e1439170b97948e833285d588181b64550b829a031e1724e6430

// RUN: execute 0x1::crypto_algebra::fq_order
// CHECK: results: 0x47fd7cd8168c203c8dca7168916a81975d588181b64550b829a031e1724e6430

// RUN: execute 0x1::crypto_algebra::fr_serialize_both_endians
// CHECK: results: 0x0700000000000000000000000000000000000000000000000000000000000000, 0x0000000000000000000000000000000000000000000000000000000000000007

// RUN: execute 0x1::crypto_algebra::fq_serialize_both_endians
// CHECK: results: 0x0700000000000000000000000000000000000000000000000000000000000000, 0x0000000000000000000000000000000000000000000000000000000000000007

// RUN: execute 0x1::crypto_algebra::g1_generator_serialized
// CHECK: results: 0x0100000000000000000000000000000000000000000000000000000000000000

// RUN: execute 0x1::crypto_algebra::g2_generator_serialized
// CHECK: results: 0xedf692d95cbdde46ddda5ef7d422436779445c5e66006a42761e1f12efde0018c212f3aeb785e49712e7a9353349aaf1255dfb31b7bf60723a480d9293938e19

// RUN: execute 0x1::crypto_algebra::gt_generator_serialized
// CHECK: results: 0x950e879d73631f5eb5788589eb5f7ef8d63e0a28de1ba00dfe4ca9ed3f252b264a8afb8eb4349db466ed1809ea4d7c39bdab7938821f1b0a00a295c72c2de002e01dbdfd0254134efcb1ec877395d25f937719b344adb1a58d129be2d6f2a9132b16a16e8ab030b130e69c69bd20b4c45986e6744a98314b5c1a0f50faa90b04dbaf9ef8aeeee3f50be31c210b598f4752f073987f9d35be8f6770d83f2ffc0af0d18dd9d2dbcdf943825acc12a7a9ddca45e629d962c6bd64908c3930a5541cfe2924dcc5580d5cef7a4bfdec90a91b59926f850d4a7923c01a5a5dbf0f5c094a2b9fb9d415820fa6b40c59bb9eade9c953407b0fc11da350a9d872cad6d3142974ca385854afdf5f583c04231adc5957c8914b6b20dc89660ed7c3bbe7c01d972be2d53ecdb27a1bcc16ac610db95aa7d237c8ff55a898cb88645a0e32530b23d7ebf5dafdd79b0f9c2ac4ba07ce18d3d16cf36e47916c4cae5d08d3afa813972c769e8514533e380c9443b3e1ee5c96fa3a0a73f301b626454721527bf900

// RUN: execute 0x1::crypto_algebra::g1_scalar_mul
// CHECK: results: 0x78e0ffab866b3a9876bd01b8ecc66fcb86936277f425539a758dbbd32e2b0717

// RUN: execute 0x1::crypto_algebra::g1_multi_scalar_mul
// CHECK: results: true

// RUN: execute 0x1::crypto_algebra::g2_multi_scalar_mul
// CHECK: results: true

// RUN: execute 0x1::crypto_algebra::gt_group_laws
// CHECK: results: true

// RUN: execute 0x1::crypto_algebra::pairing_bilinearity
// CHECK: results: true

// RUN: execute 0x1::crypto_algebra::multi_pairing_matches_naive
// CHECK: results: true

// RUN: execute 0x1::crypto_algebra::gt_fq12_cast_roundtrip
// CHECK: results: true

// RUN: execute 0x1::crypto_algebra::fq12_not_in_gt
// CHECK: results: false, true
