// Right-length encodings that are not valid elements deserialize to (false, 0) rather than
// aborting: BLS12-381 FrMsb and Fq12, every BN254 field format, and all eight curve formats.
// Each refused input follows a control that round-trips and differs in one property: the
// modulus beside modulus - 1 (for Fq12, in the last coefficient); a changed point coordinate
// (off the curve or subgroup, an x with no y, x = p + 3 beside x = 3); or flag bits the format
// forbids. A value reduced instead of refused would re-encode differently and give 2.

// RUN: publish
module 0x1::bls12381_algebra {
    struct Fr {}
    struct Fq12 {}
    struct G1 {}
    struct G2 {}
    struct FormatFrMsb {}
    struct FormatFq12LscLsb {}
    struct FormatG1Compr {}
    struct FormatG1Uncompr {}
    struct FormatG2Compr {}
    struct FormatG2Uncompr {}
}

module 0x1::bn254_algebra {
    struct Fr {}
    struct Fq {}
    struct Fq12 {}
    struct G1 {}
    struct G2 {}
    struct FormatFrLsb {}
    struct FormatFrMsb {}
    struct FormatFqLsb {}
    struct FormatFqMsb {}
    struct FormatFq12LscLsb {}
    struct FormatG1Compr {}
    struct FormatG1Uncompr {}
    struct FormatG2Compr {}
    struct FormatG2Uncompr {}
}

module 0x1::crypto_algebra {
    use 0x1::bls12381_algebra as bls;
    use 0x1::bn254_algebra as bn;

    // Moduli, little-endian: the scalar fields' r and the base fields' p.
    const BLS_R: vector<u8> = x"01000000fffffffffe5bfeff02a4bd5305d8a10908d83933487d9d2953a7ed73";
    const BLS_P: vector<u8> = x"abaafffffffffeb9ffff53b1feffab1e24f6b0f6a0d23067bf1285f3844b7764d7ac4b43b6a71b4b9ae67f39ea11011a";
    const BN_R: vector<u8> = x"010000f093f5e1439170b97948e833285d588181b64550b829a031e1724e6430";
    const BN_P: vector<u8> = x"47fd7cd8168c203c8dca7168916a81975d588181b64550b829a031e1724e6430";

    const BLS_G1_COMPR: vector<u8> = x"97f1d3a73197d7942695638c4fa9ac0fc3688c4f9774b905a14e3a3f171bac586c55e83ff97a1aeffb3af00adb22c6bb";
    const BLS_G1_UNCOMPR: vector<u8> = x"17f1d3a73197d7942695638c4fa9ac0fc3688c4f9774b905a14e3a3f171bac586c55e83ff97a1aeffb3af00adb22c6bb08b3f481e3aaa0f1a09e30ed741d8ae4fcf5e095d5d00af600db18cb2c04b3edd03cc744a2888ae40caa232946c5e7e1";
    const BLS_G2_COMPR: vector<u8> = x"93e02b6052719f607dacd3a088274f65596bd0d09920b61ab5da61bbdc7f5049334cf11213945d57e5ac7d055d042b7e024aa2b2f08f0a91260805272dc51051c6e47ad4fa403b02b4510b647ae3d1770bac0326a805bbefd48056c8c121bdb8";
    const BLS_G2_UNCOMPR: vector<u8> = x"13e02b6052719f607dacd3a088274f65596bd0d09920b61ab5da61bbdc7f5049334cf11213945d57e5ac7d055d042b7e024aa2b2f08f0a91260805272dc51051c6e47ad4fa403b02b4510b647ae3d1770bac0326a805bbefd48056c8c121bdb80606c4a02ea734cc32acd2b02bc28b99cb3e287e85a763af267492ab572e99ab3f370d275cec1da1aaa9075ff05f79be0ce5d527727d6e118cc9cdc6da2e351aadfd9baa8cbdd3a76d429a695160d12c923ac9cc3baca289e193548608b82801";
    const BN_G1_UNCOMPR: vector<u8> = x"01000000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000";
    const BN_G2_COMPR: vector<u8> = x"edf692d95cbdde46ddda5ef7d422436779445c5e66006a42761e1f12efde0018c212f3aeb785e49712e7a9353349aaf1255dfb31b7bf60723a480d9293938e19";
    const BN_G2_UNCOMPR: vector<u8> = x"edf692d95cbdde46ddda5ef7d422436779445c5e66006a42761e1f12efde0018c212f3aeb785e49712e7a9353349aaf1255dfb31b7bf60723a480d9293938e19aa7dfa6601cce64c7bd3430c69e7d1e38f40cb8d8071ab4aeb6d8cdba55ec8125b9722d1dcdaac55f38eb37033314bbc95330c69ad999eec75f05f58d0890609";

    native fun deserialize_internal<S, F>(bytes: &vector<u8>): (bool, u64);
    native fun serialize_internal<S, F>(handle: u64): vector<u8>;

    // 1: decodes and re-encodes to `bytes`; 0: refused as (false, 0);
    // 2: anything else.
    fun decodes<Structure, Format>(bytes: vector<u8>): u64 {
        let (ok, handle) = deserialize_internal<Structure, Format>(&bytes);
        if (ok && serialize_internal<Structure, Format>(handle) == bytes) {
            1
        } else if (!ok && handle == 0) {
            0
        } else {
            2
        }
    }

    // The moduli's least significant bytes are nonzero, so nothing borrows.
    fun minus_one(bytes: vector<u8>): vector<u8> {
        bytes[0] = bytes[0] - 1;
        bytes
    }

    // `bytes` + `value`, where the least significant byte does not overflow.
    fun plus(bytes: vector<u8>, value: u8): vector<u8> {
        bytes[0] = bytes[0] + value;
        bytes
    }

    fun reversed(bytes: vector<u8>): vector<u8> {
        bytes.reverse();
        bytes
    }

    fun xor_at(bytes: vector<u8>, index: u64, mask: u8): vector<u8> {
        bytes[index] = bytes[index] ^ mask;
        bytes
    }

    // `value` as a 32-byte little-endian field element.
    fun small(value: u8): vector<u8> {
        let bytes = vector[value];
        while (bytes.length() < 32) bytes.push_back(0);
        bytes
    }

    // An Fq12 encoding whose first 11 coefficients are zero and whose last is
    // `coefficient`.
    fun fq12_ending_with(coefficient: vector<u8>): vector<u8> {
        let bytes = vector[];
        while (bytes.length() < 11 * coefficient.length()) bytes.push_back(0);
        bytes.append(coefficient);
        bytes
    }

    public fun bls_fields(): vector<u64> {
        vector[
            decodes<bls::Fr, bls::FormatFrMsb>(reversed(minus_one(BLS_R))),
            decodes<bls::Fr, bls::FormatFrMsb>(reversed(BLS_R)),
            decodes<bls::Fq12, bls::FormatFq12LscLsb>(fq12_ending_with(minus_one(BLS_P))),
            decodes<bls::Fq12, bls::FormatFq12LscLsb>(fq12_ending_with(BLS_P)),
        ]
    }

    public fun bn_fields(): vector<u64> {
        vector[
            decodes<bn::Fr, bn::FormatFrLsb>(minus_one(BN_R)),
            decodes<bn::Fr, bn::FormatFrLsb>(BN_R),
            decodes<bn::Fr, bn::FormatFrMsb>(reversed(minus_one(BN_R))),
            decodes<bn::Fr, bn::FormatFrMsb>(reversed(BN_R)),
            decodes<bn::Fq, bn::FormatFqLsb>(minus_one(BN_P)),
            decodes<bn::Fq, bn::FormatFqLsb>(BN_P),
            decodes<bn::Fq, bn::FormatFqMsb>(reversed(minus_one(BN_P))),
            decodes<bn::Fq, bn::FormatFqMsb>(reversed(BN_P)),
            decodes<bn::Fq12, bn::FormatFq12LscLsb>(fq12_ending_with(minus_one(BN_P))),
            decodes<bn::Fq12, bn::FormatFq12LscLsb>(fq12_ending_with(BN_P)),
        ]
    }

    // Per format: the generator, then refused variants. The compression bit
    // is the top bit of the first byte; G2 compressed also has its last byte
    // changed.
    public fun bls_curves(): vector<u64> {
        vector[
            decodes<bls::G1, bls::FormatG1Compr>(BLS_G1_COMPR),
            decodes<bls::G1, bls::FormatG1Compr>(xor_at(BLS_G1_COMPR, 0, 0x80)),
            decodes<bls::G1, bls::FormatG1Uncompr>(BLS_G1_UNCOMPR),
            decodes<bls::G1, bls::FormatG1Uncompr>(xor_at(BLS_G1_UNCOMPR, 0, 0x80)),
            decodes<bls::G2, bls::FormatG2Compr>(BLS_G2_COMPR),
            decodes<bls::G2, bls::FormatG2Compr>(xor_at(BLS_G2_COMPR, 95, 0x01)),
            decodes<bls::G2, bls::FormatG2Compr>(xor_at(BLS_G2_COMPR, 0, 0x80)),
            decodes<bls::G2, bls::FormatG2Uncompr>(BLS_G2_UNCOMPR),
            decodes<bls::G2, bls::FormatG2Uncompr>(xor_at(BLS_G2_UNCOMPR, 0, 0x80)),
        ]
    }

    // BN254 G1 compressed: x = 3 decodes; x = p + 3 is out of range (reduced,
    // it would be 3); x = 4 has no y; x = 1 with the y-sign bit decodes, with
    // both flag bits it is refused. Then each other format: the generator, a
    // changed first byte (G2 only), and both flag bits on the last byte.
    public fun bn_curves(): vector<u64> {
        vector[
            decodes<bn::G1, bn::FormatG1Compr>(small(3)),
            decodes<bn::G1, bn::FormatG1Compr>(plus(BN_P, 3)),
            decodes<bn::G1, bn::FormatG1Compr>(small(4)),
            decodes<bn::G1, bn::FormatG1Compr>(xor_at(small(1), 31, 0x80)),
            decodes<bn::G1, bn::FormatG1Compr>(xor_at(small(1), 31, 0xc0)),
            decodes<bn::G1, bn::FormatG1Uncompr>(BN_G1_UNCOMPR),
            decodes<bn::G1, bn::FormatG1Uncompr>(xor_at(BN_G1_UNCOMPR, 63, 0xc0)),
            decodes<bn::G2, bn::FormatG2Compr>(BN_G2_COMPR),
            decodes<bn::G2, bn::FormatG2Compr>(xor_at(BN_G2_COMPR, 0, 0x01)),
            decodes<bn::G2, bn::FormatG2Compr>(xor_at(BN_G2_COMPR, 63, 0xc0)),
            decodes<bn::G2, bn::FormatG2Uncompr>(BN_G2_UNCOMPR),
            decodes<bn::G2, bn::FormatG2Uncompr>(xor_at(BN_G2_UNCOMPR, 0, 0x01)),
            decodes<bn::G2, bn::FormatG2Uncompr>(xor_at(BN_G2_UNCOMPR, 127, 0xc0)),
        ]
    }
}

// RUN: execute 0x1::crypto_algebra::bls_fields
// CHECK: results: [1, 0, 1, 0]

// RUN: execute 0x1::crypto_algebra::bn_fields
// CHECK: results: [1, 0, 1, 0, 1, 0, 1, 0, 1, 0]

// RUN: execute 0x1::crypto_algebra::bls_curves
// CHECK: results: [1, 0, 1, 0, 1, 0, 0, 1, 0]

// RUN: execute 0x1::crypto_algebra::bn_curves
// CHECK: results: [1, 0, 0, 1, 0, 1, 0, 1, 0, 0, 1, 0, 0]
