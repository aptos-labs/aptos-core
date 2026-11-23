// crypto_algebra sums, differences and negations on the G1, G2 and Fq12 structures of
// BLS12-381 and BN254 and on BN254 Fq, plus zero, one, the Fq12 orders, and BN254 Fr division
// and inversion. Division and inversion by zero fail on each field structure tried. Results
// are serialized; Fq12 elements and orders are compared through their sha3-256.

// RUN: publish
module 0x1::bls12381_algebra {
    struct Fr {}
    struct Fq12 {}
    struct G1 {}
    struct G2 {}
    struct FormatFrLsb {}
    struct FormatFq12LscLsb {}
    struct FormatG1Compr {}
    struct FormatG2Compr {}
}

module 0x1::bn254_algebra {
    struct Fr {}
    struct Fq {}
    struct Fq12 {}
    struct G1 {}
    struct G2 {}
    struct FormatFrLsb {}
    struct FormatFqLsb {}
    struct FormatFq12LscLsb {}
    struct FormatG1Compr {}
    struct FormatG2Compr {}
}

module 0x1::crypto_algebra {
    use std::hash::sha3_256;
    use 0x1::bls12381_algebra as bls;
    use 0x1::bn254_algebra as bn;

    native fun add_internal<S>(handle_1: u64, handle_2: u64): u64;
    native fun div_internal<F>(handle_1: u64, handle_2: u64): (bool, u64);
    native fun from_u64_internal<S>(value: u64): u64;
    native fun inv_internal<F>(handle: u64): (bool, u64);
    native fun neg_internal<F>(handle: u64): u64;
    native fun one_internal<S>(): u64;
    native fun order_internal<G>(): vector<u8>;
    native fun scalar_mul_internal<G, S>(element_handle: u64, scalar_handle: u64): u64;
    native fun serialize_internal<S, F>(handle: u64): vector<u8>;
    native fun sub_internal<G>(handle_1: u64, handle_2: u64): u64;
    native fun zero_internal<S>(): u64;

    fun bls_g2_bytes(handle: u64): vector<u8> {
        serialize_internal<bls::G2, bls::FormatG2Compr>(handle)
    }

    fun bls_fq12_hash(handle: u64): vector<u8> {
        sha3_256(serialize_internal<bls::Fq12, bls::FormatFq12LscLsb>(handle))
    }

    fun bn_fq12_hash(handle: u64): vector<u8> {
        sha3_256(serialize_internal<bn::Fq12, bn::FormatFq12LscLsb>(handle))
    }

    // g + 3g, g - 3g, -g, and the identity.
    public fun bls_g2(): (vector<u8>, vector<u8>, vector<u8>, vector<u8>) {
        let generator = one_internal<bls::G2>();
        let g3 = scalar_mul_internal<bls::G2, bls::Fr>(generator, from_u64_internal<bls::Fr>(3));
        (
            bls_g2_bytes(add_internal<bls::G2>(generator, g3)),
            bls_g2_bytes(sub_internal<bls::G2>(generator, g3)),
            bls_g2_bytes(neg_internal<bls::G2>(generator)),
            bls_g2_bytes(zero_internal<bls::G2>()),
        )
    }

    // g - 3g and -g.
    public fun bls_g1(): (vector<u8>, vector<u8>) {
        let generator = one_internal<bls::G1>();
        let g3 = scalar_mul_internal<bls::G1, bls::Fr>(generator, from_u64_internal<bls::Fr>(3));
        (
            serialize_internal<bls::G1, bls::FormatG1Compr>(sub_internal<bls::G1>(generator, g3)),
            serialize_internal<bls::G1, bls::FormatG1Compr>(neg_internal<bls::G1>(generator)),
        )
    }

    // 5 + 3, 5 - 3, -5, zero, and the order.
    public fun bls_fq12(): (vector<u8>, vector<u8>, vector<u8>, vector<u8>, vector<u8>) {
        let five = from_u64_internal<bls::Fq12>(5);
        let three = from_u64_internal<bls::Fq12>(3);
        (
            bls_fq12_hash(add_internal<bls::Fq12>(five, three)),
            bls_fq12_hash(sub_internal<bls::Fq12>(five, three)),
            bls_fq12_hash(neg_internal<bls::Fq12>(five)),
            bls_fq12_hash(zero_internal<bls::Fq12>()),
            sha3_256(order_internal<bls::Fq12>()),
        )
    }

    // 5 + 3 and -5.
    public fun bn_fq(): (vector<u8>, vector<u8>) {
        let five = from_u64_internal<bn::Fq>(5);
        let three = from_u64_internal<bn::Fq>(3);
        (
            serialize_internal<bn::Fq, bn::FormatFqLsb>(add_internal<bn::Fq>(five, three)),
            serialize_internal<bn::Fq, bn::FormatFqLsb>(neg_internal<bn::Fq>(five)),
        )
    }

    public fun bn_fq12(): (vector<u8>, vector<u8>, vector<u8>, vector<u8>, vector<u8>) {
        let five = from_u64_internal<bn::Fq12>(5);
        let three = from_u64_internal<bn::Fq12>(3);
        (
            bn_fq12_hash(add_internal<bn::Fq12>(five, three)),
            bn_fq12_hash(sub_internal<bn::Fq12>(five, three)),
            bn_fq12_hash(neg_internal<bn::Fq12>(five)),
            bn_fq12_hash(zero_internal<bn::Fq12>()),
            sha3_256(order_internal<bn::Fq12>()),
        )
    }

    // g - 3g, -g, and the identity.
    public fun bn_g1(): (vector<u8>, vector<u8>, vector<u8>) {
        let generator = one_internal<bn::G1>();
        let g3 = scalar_mul_internal<bn::G1, bn::Fr>(generator, from_u64_internal<bn::Fr>(3));
        (
            serialize_internal<bn::G1, bn::FormatG1Compr>(sub_internal<bn::G1>(generator, g3)),
            serialize_internal<bn::G1, bn::FormatG1Compr>(neg_internal<bn::G1>(generator)),
            serialize_internal<bn::G1, bn::FormatG1Compr>(zero_internal<bn::G1>()),
        )
    }

    // g + 3g, g - 3g, -g, and the identity.
    public fun bn_g2(): (vector<u8>, vector<u8>, vector<u8>, vector<u8>) {
        let generator = one_internal<bn::G2>();
        let g3 = scalar_mul_internal<bn::G2, bn::Fr>(generator, from_u64_internal<bn::Fr>(3));
        (
            serialize_internal<bn::G2, bn::FormatG2Compr>(add_internal<bn::G2>(generator, g3)),
            serialize_internal<bn::G2, bn::FormatG2Compr>(sub_internal<bn::G2>(generator, g3)),
            serialize_internal<bn::G2, bn::FormatG2Compr>(neg_internal<bn::G2>(generator)),
            serialize_internal<bn::G2, bn::FormatG2Compr>(zero_internal<bn::G2>()),
        )
    }

    // 35 / 5, 5^-1, and one.
    public fun bn_fr(): (bool, vector<u8>, bool, vector<u8>, vector<u8>) {
        let (div_ok, quotient) = div_internal<bn::Fr>(
            from_u64_internal<bn::Fr>(35), from_u64_internal<bn::Fr>(5)
        );
        let (inv_ok, inverse) = inv_internal<bn::Fr>(from_u64_internal<bn::Fr>(5));
        (
            div_ok,
            serialize_internal<bn::Fr, bn::FormatFrLsb>(quotient),
            inv_ok,
            serialize_internal<bn::Fr, bn::FormatFrLsb>(inverse),
            serialize_internal<bn::Fr, bn::FormatFrLsb>(one_internal<bn::Fr>()),
        )
    }

    // Division by zero and inversion of zero on BLS12-381 Fq12 and BN254 Fr, Fq and Fq12.
    public fun by_zero(): vector<u8> {
        let (bls_fq12_div, _) = div_internal<bls::Fq12>(
            one_internal<bls::Fq12>(), zero_internal<bls::Fq12>()
        );
        let (bls_fq12_inv, _) = inv_internal<bls::Fq12>(zero_internal<bls::Fq12>());
        let (bn_fr_div, _) = div_internal<bn::Fr>(one_internal<bn::Fr>(), from_u64_internal<bn::Fr>(0));
        let (bn_fr_inv, _) = inv_internal<bn::Fr>(from_u64_internal<bn::Fr>(0));
        let (bn_fq_div, _) = div_internal<bn::Fq>(one_internal<bn::Fq>(), zero_internal<bn::Fq>());
        let (bn_fq_inv, _) = inv_internal<bn::Fq>(zero_internal<bn::Fq>());
        let (bn_fq12_div, _) = div_internal<bn::Fq12>(
            one_internal<bn::Fq12>(), zero_internal<bn::Fq12>()
        );
        let (bn_fq12_inv, _) = inv_internal<bn::Fq12>(zero_internal<bn::Fq12>());
        let ok = vector[
            bls_fq12_div, bls_fq12_inv, bn_fr_div, bn_fr_inv,
            bn_fq_div, bn_fq_inv, bn_fq12_div, bn_fq12_inv,
        ];
        ok.map(|flag| if (flag) 1u8 else 0u8)
    }

    // The u64 handles a failed division and inversion return.
    public fun by_zero_handles(): (u64, u64) {
        let (_, quotient) = div_internal<bn::Fq>(one_internal<bn::Fq>(), zero_internal<bn::Fq>());
        let (_, inverse) = inv_internal<bn::Fq12>(zero_internal<bn::Fq12>());
        (quotient, inverse)
    }
}

// RUN: execute 0x1::crypto_algebra::bls_g2
// CHECK: results: 0x870227d3f13684fdb7ce31b8065ba3acb35f7bde6fe2ddfefa359f8b35d08a9ab9537b43e24f4ffb720b5a0bda2a82f20e7a30979a8853a077454eb63b8dcee75f106221b262886bb8e01b0abb043368da82f60899cc1412e33e4120195fc557, 0x8a4edef9c1ed7f729f520e47730a124fd70662a904ba1074728114d1031e1572c6c886f6b57ec72a6178288c47c335771638533957d540a9d2370f17cc7ed5863bc0b995b8825e0ee1ea1e1e4d00dbae81f14b0bf3611b78c952aacab827a053, 0xb3e02b6052719f607dacd3a088274f65596bd0d09920b61ab5da61bbdc7f5049334cf11213945d57e5ac7d055d042b7e024aa2b2f08f0a91260805272dc51051c6e47ad4fa403b02b4510b647ae3d1770bac0326a805bbefd48056c8c121bdb8, 0xc00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000

// RUN: execute 0x1::crypto_algebra::bls_g1
// CHECK: results: 0x8572cbea904d67468808c8eb50a9450c9721db309128012543902d0ac358a62ae28f75bb8f1c7c42c39a8c5529bf0f4e, 0xb7f1d3a73197d7942695638c4fa9ac0fc3688c4f9774b905a14e3a3f171bac586c55e83ff97a1aeffb3af00adb22c6bb

// RUN: execute 0x1::crypto_algebra::bls_fq12
// CHECK: results: 0xbf6b3a3e00a03e4eab774924373f4c089245649269530895986e73611dd56d8a, 0xb2a49649eaf88247bf16bc6093199babf3fc2861510676d1461b7cb8e03fe070, 0xc8b5210d2cac3625be1bf81a169603b43c5dcff16c79036b1f2381232072533d, 0xbedefee92b077f94c851801e96ea70d4aa6216e98fae64e1306816b0b885d3d8, 0x66f68e26b0b254f74fef675ec9c112c6e7d3c18423a7cb267b62c985bdbc61b1

// RUN: execute 0x1::crypto_algebra::bn_fq
// CHECK: results: 0x0800000000000000000000000000000000000000000000000000000000000000, 0x42fd7cd8168c203c8dca7168916a81975d588181b64550b829a031e1724e6430

// RUN: execute 0x1::crypto_algebra::bn_fq12
// CHECK: results: 0xe70bbb83ecb0634ccaa883de4aaa15f3a432985953ad609aa24b8df7296247d4, 0xd91ece1851de1e7a3d10298034254dab24af3efe3b5d27215a0a98e0fc0e7de9, 0x919a56b5eeb8e824b1db3f729a6a4316ba3c4f9366a7c0fdad947be427748df1, 0xef253a7a4953348fb7ba4a91742f327247bc850258b374e07fd5ead321f76557, 0x3a3aa6a27b7dc89a250daadf108960c696f20505ff64908b1f60efe712f890f4

// RUN: execute 0x1::crypto_algebra::bn_g1
// CHECK: results: 0xd3cf876dc108c2d3a81c8716a91678d9851518685b04859b021a132ee7440683, 0x0100000000000000000000000000000000000000000000000000000000000080, 0x0000000000000000000000000000000000000000000000000000000000000040

// RUN: execute 0x1::crypto_algebra::bn_g2
// CHECK: results: 0x2ce236b4ddb40977fd454c08e570151342c4cc50a86380c3472f6a6ff8dbdd296f237195c2856d5a3f8ca190f6666f2084d776f9935a48a2d57a569e476806a9, 0xb9b3b4620913f849ee2aa6a9cfd35c9d146f3e7c27596cc3e8d311fd3472dc2779ad28398ced57998435d8c63164b86d7033733ab82101b6379bf1b45d203e20, 0xedf692d95cbdde46ddda5ef7d422436779445c5e66006a42761e1f12efde0018c212f3aeb785e49712e7a9353349aaf1255dfb31b7bf60723a480d9293938e99, 0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000040

// RUN: execute 0x1::crypto_algebra::bn_fr
// CHECK: results: true, 0x0700000000000000000000000000000000000000000000000000000000000000, true, 0x676666c6d4fbf3e7062d4acae95caea98b56cd337cb5b949aad9135a94525b13, 0x0100000000000000000000000000000000000000000000000000000000000000

// RUN: execute 0x1::crypto_algebra::by_zero
// CHECK: results: 0x0000000000000000

// RUN: execute 0x1::crypto_algebra::by_zero_handles
// CHECK: results: 0, 0
