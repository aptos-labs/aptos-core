// Differential test for the ristretto255 scalar natives.

// RUN: publish
module 0x1::ristretto255 {
    native fun scalar_is_canonical_internal(s: vector<u8>): bool;

    native fun scalar_invert_internal(bytes: vector<u8>): vector<u8>;

    native fun scalar_from_sha512_internal(sha2_512_input: vector<u8>): vector<u8>;

    native fun scalar_mul_internal(a_bytes: vector<u8>, b_bytes: vector<u8>): vector<u8>;

    native fun scalar_add_internal(a_bytes: vector<u8>, b_bytes: vector<u8>): vector<u8>;

    native fun scalar_sub_internal(a_bytes: vector<u8>, b_bytes: vector<u8>): vector<u8>;

    native fun scalar_neg_internal(a_bytes: vector<u8>): vector<u8>;

    native fun scalar_from_u64_internal(num: u64): vector<u8>;

    native fun scalar_from_u128_internal(num: u128): vector<u8>;

    native fun scalar_reduced_from_32_bytes_internal(bytes: vector<u8>): vector<u8>;

    native fun scalar_uniform_from_64_bytes_internal(bytes: vector<u8>): vector<u8>;

    // scalar_from_u64(0) is the 32-byte zero encoding.
    public fun from_u64_zero(): vector<u8> {
        scalar_from_u64_internal(0)
    }

    // scalar_from_u64(1) is little-endian: a leading 0x01 then zeros.
    public fun from_u64_one(): vector<u8> {
        scalar_from_u64_internal(1)
    }

    public fun from_u64_42(): vector<u8> {
        scalar_from_u64_internal(42)
    }

    // scalar_from_u128 agrees with scalar_from_u64 on small values.
    public fun from_u128_42(): vector<u8> {
        scalar_from_u128_internal(42)
    }

    // 2 + 3 == 5.
    public fun add_small(): vector<u8> {
        scalar_add_internal(scalar_from_u64_internal(2), scalar_from_u64_internal(3))
    }

    // 10 - 3 == 7.
    public fun sub_small(): vector<u8> {
        scalar_sub_internal(scalar_from_u64_internal(10), scalar_from_u64_internal(3))
    }

    // 6 * 7 == 42.
    public fun mul_small(): vector<u8> {
        scalar_mul_internal(scalar_from_u64_internal(6), scalar_from_u64_internal(7))
    }

    // a + (-a) == 0.
    public fun neg_to_zero(): vector<u8> {
        let a = scalar_from_u64_internal(9);
        scalar_add_internal(a, scalar_neg_internal(a))
    }

    // a * a^{-1} == 1.
    public fun invert_roundtrip(): vector<u8> {
        let a = scalar_from_u64_internal(7);
        scalar_mul_internal(a, scalar_invert_internal(a))
    }

    // reduced_from_32_bytes on a small canonical value is that value.
    public fun reduced_small(): vector<u8> {
        scalar_reduced_from_32_bytes_internal(
            x"0500000000000000000000000000000000000000000000000000000000000000",
        )
    }

    public fun from_sha512_fixed(): vector<u8> {
        scalar_from_sha512_internal(b"ristretto255 scalar test")
    }

    public fun uniform_fixed(): vector<u8> {
        scalar_uniform_from_64_bytes_internal(
            x"000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f202122232425262728292a2b2c2d2e2f303132333435363738393a3b3c3d3e3f",
        )
    }

    // A freshly built scalar is canonical.
    public fun is_canonical_valid(): bool {
        scalar_is_canonical_internal(scalar_from_u64_internal(5))
    }

    // A wrong-length input is not canonical (returns false, does not abort).
    public fun is_canonical_wrong_length(): bool {
        scalar_is_canonical_internal(x"00")
    }

    // A 32-byte value above the group order is not canonical.
    public fun is_canonical_too_large(): bool {
        scalar_is_canonical_internal(
            x"ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
        )
    }
}

// RUN: execute 0x1::ristretto255::from_u64_zero
// CHECK: results: 0x0000000000000000000000000000000000000000000000000000000000000000

// RUN: execute 0x1::ristretto255::from_u64_one
// CHECK: results: 0x0100000000000000000000000000000000000000000000000000000000000000

// RUN: execute 0x1::ristretto255::from_u64_42
// CHECK: results: 0x2a00000000000000000000000000000000000000000000000000000000000000

// RUN: execute 0x1::ristretto255::from_u128_42
// CHECK: results: 0x2a00000000000000000000000000000000000000000000000000000000000000

// RUN: execute 0x1::ristretto255::add_small
// CHECK: results: 0x0500000000000000000000000000000000000000000000000000000000000000

// RUN: execute 0x1::ristretto255::sub_small
// CHECK: results: 0x0700000000000000000000000000000000000000000000000000000000000000

// RUN: execute 0x1::ristretto255::mul_small
// CHECK: results: 0x2a00000000000000000000000000000000000000000000000000000000000000

// RUN: execute 0x1::ristretto255::neg_to_zero
// CHECK: results: 0x0000000000000000000000000000000000000000000000000000000000000000

// RUN: execute 0x1::ristretto255::invert_roundtrip
// CHECK: results: 0x0100000000000000000000000000000000000000000000000000000000000000

// RUN: execute 0x1::ristretto255::reduced_small
// CHECK: results: 0x0500000000000000000000000000000000000000000000000000000000000000

// RUN: execute 0x1::ristretto255::from_sha512_fixed
// CHECK: results: 0x794a276e00e3a3af70f3a8a409fa8a34b20a7f01bc5ecc50afa4dac191e80205

// RUN: execute 0x1::ristretto255::uniform_fixed
// CHECK: results: 0x7a3c6282f02d37a05023b60d5428e6cc5961d4c31221937adae0b574e4d07205

// RUN: execute 0x1::ristretto255::is_canonical_valid
// CHECK: results: true

// RUN: execute 0x1::ristretto255::is_canonical_wrong_length
// CHECK: results: false

// RUN: execute 0x1::ristretto255::is_canonical_too_large
// CHECK: results: false
