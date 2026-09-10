// RUN: publish
module 0x1::bls12381_algebra {
    struct Fr {}
    struct Fq12 {}
    struct G1 {}
    struct G2 {}
    struct Gt {}
    struct FormatFrLsb {}
    struct FormatFrMsb {}
    struct FormatG1Compr {}
    struct FormatG1Uncompr {}
    struct FormatG2Compr {}
    struct FormatG2Uncompr {}
    struct FormatGt {}
    struct HashG1XmdSha256SswuRo {}
    struct HashG2XmdSha256SswuRo {}
}

module 0x1::crypto_algebra {
    use 0x1::bls12381_algebra::{
        Fq12,
        FormatFrLsb,
        FormatFrMsb,
        FormatG1Compr,
        FormatG1Uncompr,
        FormatG2Compr,
        FormatG2Uncompr,
        FormatGt,
        Fr,
        G1,
        G2,
        Gt,
        HashG1XmdSha256SswuRo,
        HashG2XmdSha256SswuRo
    };

    const R_SERIALIZED: vector<u8> = x"01000000fffffffffe5bfeff02a4bd5305d8a10908d83933487d9d2953a7ed73";
    const G1_GENERATOR_SERIALIZED_COMP: vector<u8> = x"97f1d3a73197d7942695638c4fa9ac0fc3688c4f9774b905a14e3a3f171bac586c55e83ff97a1aeffb3af00adb22c6bb";
    const G1_GENERATOR_MUL_BY_7_SERIALIZED_COMP: vector<u8> = x"b928f3beb93519eecf0145da903b40a4c97dca00b21f12ac0df3be9116ef2ef27b2ae6bcd4c5bc2d54ef5a70627efcb7";
    const G2_GENERATOR_SERIALIZED_COMP: vector<u8> = x"93e02b6052719f607dacd3a088274f65596bd0d09920b61ab5da61bbdc7f5049334cf11213945d57e5ac7d055d042b7e024aa2b2f08f0a91260805272dc51051c6e47ad4fa403b02b4510b647ae3d1770bac0326a805bbefd48056c8c121bdb8";
    // Test vectors from draft-irtf-cfrg-hash-to-curve-16, suites
    // `BLS12381G1_XMD:SHA-256_SSWU_RO_` and `BLS12381G2_XMD:SHA-256_SSWU_RO_`.
    const G1_DST: vector<u8> = b"QUUX-V01-CS02-with-BLS12381G1_XMD:SHA-256_SSWU_RO_";
    const G2_DST: vector<u8> = b"QUUX-V01-CS02-with-BLS12381G2_XMD:SHA-256_SSWU_RO_";

    native fun add_internal<S>(handle_1: u64, handle_2: u64): u64;
    native fun deserialize_internal<S, F>(bytes: &vector<u8>): (bool, u64);
    native fun div_internal<F>(handle_1: u64, handle_2: u64): (bool, u64);
    native fun double_internal<G>(element_handle: u64): u64;
    native fun downcast_internal<L, S>(handle: u64): (bool, u64);
    native fun from_u64_internal<S>(value: u64): u64;
    native fun eq_internal<S>(handle_1: u64, handle_2: u64): bool;
    native fun hash_to_internal<S, H>(dst: &vector<u8>, bytes: &vector<u8>): u64;
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

    // Group generator scaled by `k`, computed by repeated addition.
    fun repeated_add_g1(k: u64): u64 {
        let acc = zero_internal<G1>();
        let i = 0;
        while (i < k) {
            acc = add_internal<G1>(acc, one_internal<G1>());
            i += 1;
        };
        acc
    }

    public fun fr_ring_laws(): bool {
        let a = from_u64_internal<Fr>(7);
        let b = from_u64_internal<Fr>(5);
        let commutes = eq_internal<Fr>(add_internal<Fr>(a, b), add_internal<Fr>(b, a))
            && eq_internal<Fr>(mul_internal<Fr>(a, b), mul_internal<Fr>(b, a));
        let sub_undoes_add = eq_internal<Fr>(sub_internal<Fr>(add_internal<Fr>(a, b), b), a);
        let neg_is_additive_inverse =
            eq_internal<Fr>(add_internal<Fr>(a, neg_internal<Fr>(a)), zero_internal<Fr>());
        let sqr_is_self_mul = eq_internal<Fr>(sqr_internal<Fr>(a), mul_internal<Fr>(a, a));
        let sum_matches = eq_internal<Fr>(add_internal<Fr>(a, b), from_u64_internal<Fr>(12));
        let product_matches = eq_internal<Fr>(mul_internal<Fr>(a, b), from_u64_internal<Fr>(35));
        commutes
            && sub_undoes_add
            && neg_is_additive_inverse
            && sqr_is_self_mul
            && sum_matches
            && product_matches
    }

    public fun fr_div_and_inv(): bool {
        let a = from_u64_internal<Fr>(35);
        let b = from_u64_internal<Fr>(5);
        let (div_ok, q) = div_internal<Fr>(a, b);
        let (inv_ok, b_inv) = inv_internal<Fr>(b);
        div_ok
            && inv_ok
            && eq_internal<Fr>(q, from_u64_internal<Fr>(7))
            && eq_internal<Fr>(mul_internal<Fr>(b, b_inv), one_internal<Fr>())
            && eq_internal<Fr>(q, mul_internal<Fr>(a, b_inv))
    }

    // A zero divisor is `(false, 0)`, not an abort.
    public fun fr_div_by_zero(): (bool, u64) {
        div_internal<Fr>(from_u64_internal<Fr>(7), zero_internal<Fr>())
    }

    public fun fr_inv_zero(): (bool, u64) {
        inv_internal<Fr>(zero_internal<Fr>())
    }

    public fun fr_order(): vector<u8> {
        order_internal<Fr>()
    }

    // The LSB and MSB formats of the same element differ by a byte reversal.
    public fun fr_serialize_both_endians(): (vector<u8>, vector<u8>) {
        let seven = from_u64_internal<Fr>(7);
        (
            serialize_internal<Fr, FormatFrLsb>(seven),
            serialize_internal<Fr, FormatFrMsb>(seven)
        )
    }

    public fun g1_generator_serialized(): vector<u8> {
        serialize_internal<G1, FormatG1Compr>(one_internal<G1>())
    }

    public fun g2_generator_serialized(): vector<u8> {
        serialize_internal<G2, FormatG2Compr>(one_internal<G2>())
    }

    public fun g1_scalar_mul_matches_repeated_add(): vector<u8> {
        let by_scalar =
            scalar_mul_internal<G1, Fr>(one_internal<G1>(), from_u64_internal<Fr>(7));
        assert!(eq_internal<G1>(by_scalar, repeated_add_g1(7)), 1);
        serialize_internal<G1, FormatG1Compr>(by_scalar)
    }

    public fun g1_double_matches_add(): bool {
        let g = one_internal<G1>();
        eq_internal<G1>(double_internal<G1>(g), add_internal<G1>(g, g))
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
        let expected = scalar_mul_internal<G1, Fr>(g, from_u64_internal<Fr>(13));
        eq_internal<G1>(combined, expected)
    }

    public fun g2_multi_scalar_mul(): bool {
        let g = one_internal<G2>();
        let two_g = double_internal<G2>(g);
        let combined =
            multi_scalar_mul_internal<G2, Fr>(
                vector[g, two_g],
                vector[from_u64_internal<Fr>(3), from_u64_internal<Fr>(5)]
            );
        let expected = scalar_mul_internal<G2, Fr>(g, from_u64_internal<Fr>(13));
        eq_internal<G2>(combined, expected)
    }

    // `Gt` uses multiplicative notation: add is field multiplication, so the
    // group generator is `pairing(G1, G2)` and `zero<Gt>()` is `Fq12::one()`.
    public fun gt_generator_serialized(): vector<u8> {
        let gt = pairing_internal<G1, G2, Gt>(one_internal<G1>(), one_internal<G2>());
        assert!(eq_internal<Gt>(gt, one_internal<Gt>()), 1);
        serialize_internal<Gt, FormatGt>(gt)
    }

    public fun gt_group_laws(): bool {
        let g = one_internal<Gt>();
        let two_g = double_internal<Gt>(g);
        let identity = eq_internal<Gt>(add_internal<Gt>(g, zero_internal<Gt>()), g);
        let doubling = eq_internal<Gt>(two_g, add_internal<Gt>(g, g));
        let inverse = eq_internal<Gt>(
            add_internal<Gt>(g, neg_internal<Gt>(g)), zero_internal<Gt>()
        );
        let subtraction = eq_internal<Gt>(sub_internal<Gt>(two_g, g), g);
        let scaling = eq_internal<Gt>(
            scalar_mul_internal<Gt, Fr>(g, from_u64_internal<Fr>(2)), two_g
        );
        identity && doubling && inverse && subtraction && scaling
    }

    // pairing(a*P, b*Q) == (a*b) * pairing(P, Q).
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

    // multi_pairing([P0, P1], [Q0, Q1]) == pairing(P0, Q0) + pairing(P1, Q1).
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
        let batched = multi_pairing_internal<G1, G2, Gt>(vector[p0, p1], vector[q0, q1]);
        eq_internal<Gt>(naive, batched)
    }

    public fun hash_to_g1_empty_msg(): vector<u8> {
        let point = hash_to_internal<G1, HashG1XmdSha256SswuRo>(&G1_DST, &b"");
        serialize_internal<G1, FormatG1Uncompr>(point)
    }

    public fun hash_to_g1_nonempty_msg(): vector<u8> {
        let point =
            hash_to_internal<G1, HashG1XmdSha256SswuRo>(&G1_DST, &b"abcdef0123456789");
        serialize_internal<G1, FormatG1Uncompr>(point)
    }

    public fun hash_to_g2_empty_msg(): vector<u8> {
        let point = hash_to_internal<G2, HashG2XmdSha256SswuRo>(&G2_DST, &b"");
        serialize_internal<G2, FormatG2Uncompr>(point)
    }

    public fun hash_to_g2_nonempty_msg(): vector<u8> {
        let point =
            hash_to_internal<G2, HashG2XmdSha256SswuRo>(&G2_DST, &b"abcdef0123456789");
        serialize_internal<G2, FormatG2Uncompr>(point)
    }

    // A `Gt` element upcasts to `Fq12` and downcasts back; the handle is the
    // caller's own in both directions, so the round trip is the identity.
    public fun gt_fq12_cast_roundtrip(): bool {
        let gt = one_internal<Gt>();
        let fq12 = upcast_internal<Gt, Fq12>(gt);
        let (in_subgroup, back) = downcast_internal<Fq12, Gt>(fq12);
        in_subgroup && back == gt && eq_internal<Gt>(back, gt)
    }

    // `Fq12` elements outside the `r`-th roots of unity fail the downcast, and
    // the failing case still hands back the caller's handle.
    public fun fq12_not_in_gt(): (bool, bool) {
        let seven = from_u64_internal<Fq12>(7);
        let (in_subgroup, back) = downcast_internal<Fq12, Gt>(seven);
        (in_subgroup, back == seven)
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

    // Serialization allocates a `vector<u8>` on the VM heap while the element
    // store is a Rust-side vector, so the returned bytes must survive GC.
    public fun serialize_survives_gc(rounds: u64): bool {
        let bytes = serialize_internal<G1, FormatG1Compr>(one_internal<G1>());
        let counter = 0;
        while (counter < rounds) {
            let junk = vector[counter, counter, counter, counter];
            counter += junk.length();
        };
        bytes == G1_GENERATOR_SERIALIZED_COMP
    }
}

// RUN: execute 0x1::crypto_algebra::fr_ring_laws
// CHECK: results: true

// RUN: execute 0x1::crypto_algebra::fr_div_and_inv
// CHECK: results: true

// RUN: execute 0x1::crypto_algebra::fr_div_by_zero
// CHECK: results: false, 0

// RUN: execute 0x1::crypto_algebra::fr_inv_zero
// CHECK: results: false, 0

// RUN: execute 0x1::crypto_algebra::fr_order
// CHECK: results: 0x01000000fffffffffe5bfeff02a4bd5305d8a10908d83933487d9d2953a7ed73

// RUN: execute 0x1::crypto_algebra::fr_serialize_both_endians
// CHECK: results: 0x0700000000000000000000000000000000000000000000000000000000000000, 0x0000000000000000000000000000000000000000000000000000000000000007

// RUN: execute 0x1::crypto_algebra::g1_generator_serialized
// CHECK: results: 0x97f1d3a73197d7942695638c4fa9ac0fc3688c4f9774b905a14e3a3f171bac586c55e83ff97a1aeffb3af00adb22c6bb

// RUN: execute 0x1::crypto_algebra::g2_generator_serialized
// CHECK: results: 0x93e02b6052719f607dacd3a088274f65596bd0d09920b61ab5da61bbdc7f5049334cf11213945d57e5ac7d055d042b7e024aa2b2f08f0a91260805272dc51051c6e47ad4fa403b02b4510b647ae3d1770bac0326a805bbefd48056c8c121bdb8

// RUN: execute 0x1::crypto_algebra::g1_scalar_mul_matches_repeated_add
// CHECK: results: 0xb928f3beb93519eecf0145da903b40a4c97dca00b21f12ac0df3be9116ef2ef27b2ae6bcd4c5bc2d54ef5a70627efcb7

// RUN: execute 0x1::crypto_algebra::g1_double_matches_add
// CHECK: results: true

// RUN: execute 0x1::crypto_algebra::g1_multi_scalar_mul
// CHECK: results: true

// RUN: execute 0x1::crypto_algebra::g2_multi_scalar_mul
// CHECK: results: true

// RUN: execute 0x1::crypto_algebra::gt_generator_serialized
// CHECK: results: 0xb68917caaa0543a808c53908f694d1b6e7b38de90ce9d83d505ca1ef1b442d2727d7d06831d8b2a7920afc71d8eb50120f17a0ea982a88591d9f43503e94a8f1abaf2e4589f65aafb7923c484540a868883432a5c60e75860b11e5465b1c9a08873ec29e844c1c888cb396933057ffdd541b03a5220eda16b2b3a6728ea678034ce39c6839f20397202d7c5c44bb68134f93193cec215031b17399577a1de5ff1f5b0666bdd8907c61a7651e4e79e0372951505a07fa73c25788db6eb8023519a5aa97b51f1cad1d43d8aabbff4dc319c79a58cafc035218747c2f75daf8f2fb7c00c44da85b129113173d4722f5b201b6b4454062e9ea8ba78c5ca3cadaf7238b47bace5ce561804ae16b8f4b63da4645b8457a93793cbd64a7254f150781019de87ee42682940f3e70a88683d512bb2c3fb7b2434da5dedbb2d0b3fb8487c84da0d5c315bdd69c46fb05d23763f2191aabd5d5c2e12a10b8f002ff681bfd1b2ee0bf619d80d2a795eb22f2aa7b85d5ffb671a70c94809f0dafc5b73ea2fb0657bae23373b4931bc9fa321e8848ef78894e987bff150d7d671aee30b3931ac8c50e0b3b0868effc38bf48cd24b4b811a2995ac2a09122bed9fd9fa0c510a87b10290836ad06c8203397b56a78e9a0c61c77e56ccb4f1bc3d3fcaea7550f3503efe30f2d24f00891cb45620605fcfaa4292687b3a7db7c1c0554a93579e889a121fd8f72649b2402996a084d2381c5043166673b3849e4fd1e7ee4af24aa8ed443f56dfd6b68ffde4435a92cd7a4ac3bc77e1ad0cb728606cf08bf6386e5410f

// RUN: execute 0x1::crypto_algebra::gt_group_laws
// CHECK: results: true

// RUN: execute 0x1::crypto_algebra::pairing_bilinearity
// CHECK: results: true

// RUN: execute 0x1::crypto_algebra::multi_pairing_matches_naive
// CHECK: results: true

// RUN: execute 0x1::crypto_algebra::hash_to_g1_empty_msg
// CHECK: results: 0x052926add2207b76ca4fa57a8734416c8dc95e24501772c814278700eed6d1e4e8cf62d9c09db0fac349612b759e79a108ba738453bfed09cb546dbb0783dbb3a5f1f566ed67bb6be0e8c67e2e81a4cc68ee29813bb7994998f3eae0c9c6a265

// RUN: execute 0x1::crypto_algebra::hash_to_g1_nonempty_msg
// CHECK: results: 0x11e0b079dea29a68f0383ee94fed1b940995272407e3bb916bbf268c263ddd57a6a27200a784cbc248e84f357ce82d9803a87ae2caf14e8ee52e51fa2ed8eefe80f02457004ba4d486d6aa1f517c0889501dc7413753f9599b099ebcbbd2d709

// RUN: execute 0x1::crypto_algebra::hash_to_g2_empty_msg
// CHECK: results: 0x05cb8437535e20ecffaef7752baddf98034139c38452458baeefab379ba13dff5bf5dd71b72418717047f5b0f37da03d0141ebfbdca40eb85b87142e130ab689c673cf60f1a3e98d69335266f30d9b8d4ac44c1038e9dcdd5393faf5c41fb78a12424ac32561493f3fe3c260708a12b7c620e7be00099a974e259ddc7d1f6395c3c811cdd19f1e8dbf3e9ecfdcbab8d60503921d7f6a12805e72940b963c0cf3471c7b2a524950ca195d11062ee75ec076daf2d4bc358c4b190c0c98064fdd92

// RUN: execute 0x1::crypto_algebra::hash_to_g2_nonempty_msg
// CHECK: results: 0x190d119345b94fbd15497bcba94ecf7db2cbfd1e1fe7da034d26cbba169fb3968288b3fafb265f9ebd380512a71c3f2c121982811d2491fde9ba7ed31ef9ca474f0e1501297f68c298e9f4c0028add35aea8bb83d53c08cfc007c1e005723cd00bb5e7572275c567462d91807de765611490205a941a5a6af3b1691bfe596c31225d3aabdf15faff860cb4ef17c7c3be05571a0f8d3c08d094576981f4a3b8eda0a8e771fcdcc8ecceaf1356a6acf17574518acb506e435b639353c2e14827c8

// RUN: execute 0x1::crypto_algebra::gt_fq12_cast_roundtrip
// CHECK: results: true

// RUN: execute 0x1::crypto_algebra::fq12_not_in_gt
// CHECK: results: false, true

// RUN: execute 0x1::crypto_algebra::fq12_field_laws
// CHECK: results: true

// RUN: execute 0x1::crypto_algebra::serialize_survives_gc --args 0 --heap-size 16384
// CHECK: results: true
// CHECK-GC-COUNT: 0

// RUN: execute 0x1::crypto_algebra::serialize_survives_gc --args 40000 --heap-size 16384
// CHECK: results: true
