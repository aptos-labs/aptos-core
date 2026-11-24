// Encodings of the point at infinity in all eight curve formats; serializing the identity
// gives the canonical encoding. BLS12-381 (infinity bit 0x40 of the first byte, plus 0x80 when
// compressed) accepts only the canonical form. BN254 (bit 6 of the last byte) ignores the
// coordinates as long as they are field elements: p - 1 decodes to the identity and
// re-encodes canonically, p is refused, and so is setting bit 7 as well.

// RUN: publish
module 0x1::bls12381_algebra {
    struct G1 {}
    struct G2 {}
    struct FormatG1Compr {}
    struct FormatG1Uncompr {}
    struct FormatG2Compr {}
    struct FormatG2Uncompr {}
}

module 0x1::bn254_algebra {
    struct G1 {}
    struct G2 {}
    struct FormatG1Compr {}
    struct FormatG1Uncompr {}
    struct FormatG2Compr {}
    struct FormatG2Uncompr {}
}

module 0x1::crypto_algebra {
    use 0x1::bls12381_algebra as bls;
    use 0x1::bn254_algebra as bn;

    native fun deserialize_internal<S, F>(bytes: &vector<u8>): (bool, u64);
    native fun eq_internal<S>(handle_1: u64, handle_2: u64): bool;
    native fun serialize_internal<S, F>(handle: u64): vector<u8>;
    native fun zero_internal<S>(): u64;

    // The BN254 base field modulus p, little-endian.
    const BN_P: vector<u8> = x"47fd7cd8168c203c8dca7168916a81975d588181b64550b829a031e1724e6430";

    // `count` zero bytes, except `byte` at `index`.
    fun zeros_with(count: u64, index: u64, byte: u8): vector<u8> {
        let bytes = vector[];
        let position = 0;
        while (position < count) {
            bytes.push_back(if (position == index) byte else 0);
            position += 1;
        };
        bytes
    }

    fun set(bytes: vector<u8>, index: u64, byte: u8): vector<u8> {
        bytes[index] = byte;
        bytes
    }

    // `prefix` padded with zeros to `count` bytes, with the BN254 infinity bit
    // set on the last byte.
    fun bn_infinity_over(prefix: vector<u8>, count: u64): vector<u8> {
        while (prefix.length() < count) prefix.push_back(0);
        prefix[count - 1] = prefix[count - 1] | 0x40;
        prefix
    }

    fun minus_one(bytes: vector<u8>): vector<u8> {
        bytes[0] = bytes[0] - 1;
        bytes
    }

    // 1: decodes to the identity and re-encodes to `bytes` (the canonical
    // encoding); 2: decodes to the identity and re-encodes to `canonical`;
    // 0: refused as (false, 0); 3: anything else.
    fun identity<Group, Format>(bytes: vector<u8>, canonical: vector<u8>): u64 {
        let (ok, handle) = deserialize_internal<Group, Format>(&bytes);
        if (!ok) {
            if (handle == 0) 0 else 3
        } else if (!eq_internal<Group>(handle, zero_internal<Group>())) {
            3
        } else {
            let encoded = serialize_internal<Group, Format>(handle);
            if (encoded == bytes && bytes == canonical) {
                1
            } else if (encoded == canonical) {
                2
            } else {
                3
            }
        }
    }

    public fun bls_serialized(): (vector<u8>, vector<u8>, vector<u8>, vector<u8>) {
        (
            serialize_internal<bls::G1, bls::FormatG1Compr>(zero_internal<bls::G1>()),
            serialize_internal<bls::G1, bls::FormatG1Uncompr>(zero_internal<bls::G1>()),
            serialize_internal<bls::G2, bls::FormatG2Compr>(zero_internal<bls::G2>()),
            serialize_internal<bls::G2, bls::FormatG2Uncompr>(zero_internal<bls::G2>()),
        )
    }

    public fun bn_serialized(): (vector<u8>, vector<u8>, vector<u8>, vector<u8>) {
        (
            serialize_internal<bn::G1, bn::FormatG1Compr>(zero_internal<bn::G1>()),
            serialize_internal<bn::G1, bn::FormatG1Uncompr>(zero_internal<bn::G1>()),
            serialize_internal<bn::G2, bn::FormatG2Compr>(zero_internal<bn::G2>()),
            serialize_internal<bn::G2, bn::FormatG2Uncompr>(zero_internal<bn::G2>()),
        )
    }

    // Per format: the canonical encoding, the same with a nonzero last byte,
    // with the sort bit (0x20) added, and with the compression bit flipped.
    public fun bls_decoded(): vector<u64> {
        let g1c = zeros_with(48, 0, 0xc0);
        let g1u = zeros_with(96, 0, 0x40);
        let g2c = zeros_with(96, 0, 0xc0);
        let g2u = zeros_with(192, 0, 0x40);
        vector[
            identity<bls::G1, bls::FormatG1Compr>(g1c, g1c),
            identity<bls::G1, bls::FormatG1Compr>(set(g1c, 47, 1), g1c),
            identity<bls::G1, bls::FormatG1Compr>(zeros_with(48, 0, 0xe0), g1c),
            identity<bls::G1, bls::FormatG1Compr>(zeros_with(48, 0, 0x40), g1c),
            identity<bls::G1, bls::FormatG1Uncompr>(g1u, g1u),
            identity<bls::G1, bls::FormatG1Uncompr>(set(g1u, 95, 1), g1u),
            identity<bls::G1, bls::FormatG1Uncompr>(zeros_with(96, 0, 0x60), g1u),
            identity<bls::G1, bls::FormatG1Uncompr>(zeros_with(96, 0, 0xc0), g1u),
            identity<bls::G2, bls::FormatG2Compr>(g2c, g2c),
            identity<bls::G2, bls::FormatG2Compr>(set(g2c, 95, 1), g2c),
            identity<bls::G2, bls::FormatG2Compr>(zeros_with(96, 0, 0xe0), g2c),
            identity<bls::G2, bls::FormatG2Compr>(zeros_with(96, 0, 0x40), g2c),
            identity<bls::G2, bls::FormatG2Uncompr>(g2u, g2u),
            identity<bls::G2, bls::FormatG2Uncompr>(set(g2u, 191, 1), g2u),
            identity<bls::G2, bls::FormatG2Uncompr>(zeros_with(192, 0, 0x60), g2u),
            identity<bls::G2, bls::FormatG2Uncompr>(zeros_with(192, 0, 0xc0), g2u),
        ]
    }

    // Per format: the canonical encoding; the infinity bit with the first
    // coordinate (x, or x.c0 for G2) equal to p - 1, then to p; and both flag
    // bits.
    public fun bn_decoded(): vector<u64> {
        let g1c = zeros_with(32, 31, 0x40);
        let g1u = zeros_with(64, 63, 0x40);
        let g2c = zeros_with(64, 63, 0x40);
        let g2u = zeros_with(128, 127, 0x40);
        vector[
            identity<bn::G1, bn::FormatG1Compr>(g1c, g1c),
            identity<bn::G1, bn::FormatG1Compr>(bn_infinity_over(minus_one(BN_P), 32), g1c),
            identity<bn::G1, bn::FormatG1Compr>(bn_infinity_over(BN_P, 32), g1c),
            identity<bn::G1, bn::FormatG1Compr>(zeros_with(32, 31, 0xc0), g1c),
            identity<bn::G1, bn::FormatG1Uncompr>(g1u, g1u),
            identity<bn::G1, bn::FormatG1Uncompr>(bn_infinity_over(minus_one(BN_P), 64), g1u),
            identity<bn::G1, bn::FormatG1Uncompr>(bn_infinity_over(BN_P, 64), g1u),
            identity<bn::G1, bn::FormatG1Uncompr>(zeros_with(64, 63, 0xc0), g1u),
            identity<bn::G2, bn::FormatG2Compr>(g2c, g2c),
            identity<bn::G2, bn::FormatG2Compr>(bn_infinity_over(minus_one(BN_P), 64), g2c),
            identity<bn::G2, bn::FormatG2Compr>(bn_infinity_over(BN_P, 64), g2c),
            identity<bn::G2, bn::FormatG2Compr>(zeros_with(64, 63, 0xc0), g2c),
            identity<bn::G2, bn::FormatG2Uncompr>(g2u, g2u),
            identity<bn::G2, bn::FormatG2Uncompr>(bn_infinity_over(minus_one(BN_P), 128), g2u),
            identity<bn::G2, bn::FormatG2Uncompr>(bn_infinity_over(BN_P, 128), g2u),
            identity<bn::G2, bn::FormatG2Uncompr>(zeros_with(128, 127, 0xc0), g2u),
        ]
    }
}

// RUN: execute 0x1::crypto_algebra::bls_serialized
// CHECK: results: 0xc00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000, 0x400000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000, 0xc00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000, 0x400000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000

// RUN: execute 0x1::crypto_algebra::bn_serialized
// CHECK: results: 0x0000000000000000000000000000000000000000000000000000000000000040, 0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000040, 0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000040, 0x0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000040

// RUN: execute 0x1::crypto_algebra::bls_decoded
// CHECK: results: [1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0]

// RUN: execute 0x1::crypto_algebra::bn_decoded
// CHECK: results: [1, 2, 0, 0, 1, 2, 0, 0, 1, 2, 0, 0, 1, 2, 0, 0]
