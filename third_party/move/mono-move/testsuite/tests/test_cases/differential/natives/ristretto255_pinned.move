// ristretto255 outputs pinned to fixed values: hash-to-point results, scalar reduction of
// inputs at or above the group order l (reduce(2^256 - 1) is the aptos_std test vector),
// u64::MAX and u128::MAX as scalars, inversion of zero, subtraction that wraps, the canonical
// check at exactly l, failed decompression (handle u64::MAX), and the 10000-point limit.

// RUN: publish
module 0x1::ristretto255 {
    struct RistrettoPoint has drop {
        handle: u64
    }

    const ORDER_ELL: vector<u8> = x"edd3f55c1a631258d69cf7a2def9de1400000000000000000000000000000010";
    const L_PLUS_ONE: vector<u8> = x"eed3f55c1a631258d69cf7a2def9de1400000000000000000000000000000010";
    const ALL_ONES: vector<u8> = x"ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";
    const ZERO: vector<u8> = x"0000000000000000000000000000000000000000000000000000000000000000";
    const ONE: vector<u8> = x"0100000000000000000000000000000000000000000000000000000000000000";
    const TWO: vector<u8> = x"0200000000000000000000000000000000000000000000000000000000000000";

    native fun scalar_is_canonical_internal(bytes: vector<u8>): bool;
    native fun scalar_invert_internal(bytes: vector<u8>): vector<u8>;
    native fun scalar_sub_internal(a_bytes: vector<u8>, b_bytes: vector<u8>): vector<u8>;
    native fun scalar_from_u64_internal(num: u64): vector<u8>;
    native fun scalar_from_u128_internal(num: u128): vector<u8>;
    native fun scalar_reduced_from_32_bytes_internal(bytes: vector<u8>): vector<u8>;
    native fun point_identity_internal(): u64;
    native fun point_is_canonical_internal(bytes: vector<u8>): bool;
    native fun point_decompress_internal(maybe_non_canonical_bytes: vector<u8>): (u64, bool);
    native fun point_compress_internal(point: &RistrettoPoint): vector<u8>;
    native fun new_point_from_sha512_internal(sha2_512_input: vector<u8>): u64;
    native fun new_point_from_64_uniform_bytes_internal(bytes: vector<u8>): u64;

    public fun from_sha512_bytes(): vector<u8> {
        point_compress_internal(&RistrettoPoint {
            handle: new_point_from_sha512_internal(b"ristretto255 point test")
        })
    }

    public fun from_64_uniform_bytes(): vector<u8> {
        point_compress_internal(&RistrettoPoint {
            handle: new_point_from_64_uniform_bytes_internal(
                x"000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f202122232425262728292a2b2c2d2e2f303132333435363738393a3b3c3d3e3f"
            )
        })
    }

    public fun reduced_all_ones(): vector<u8> {
        scalar_reduced_from_32_bytes_internal(ALL_ONES)
    }

    public fun reduced_order(): vector<u8> {
        scalar_reduced_from_32_bytes_internal(ORDER_ELL)
    }

    public fun reduced_order_plus_one(): vector<u8> {
        scalar_reduced_from_32_bytes_internal(L_PLUS_ONE)
    }

    public fun from_u64_max(): vector<u8> {
        scalar_from_u64_internal(18446744073709551615)
    }

    public fun from_u128_max(): vector<u8> {
        scalar_from_u128_internal(340282366920938463463374607431768211455)
    }

    public fun invert_zero(): vector<u8> {
        scalar_invert_internal(ZERO)
    }

    // 1 - 2 = l - 1.
    public fun sub_wraps(): vector<u8> {
        scalar_sub_internal(ONE, TWO)
    }

    // Exactly l (bit 255 clear) is not canonical; l - 1 is.
    public fun canonical_at_order(): (bool, bool) {
        (
            scalar_is_canonical_internal(ORDER_ELL),
            scalar_is_canonical_internal(scalar_sub_internal(ZERO, ONE)),
        )
    }

    public fun point_wrong_length(): (bool, u64, bool) {
        let (handle, ok) = point_decompress_internal(x"00");
        (point_is_canonical_internal(x"00"), handle, ok)
    }

    public fun point_not_on_curve(): (u64, bool) {
        point_decompress_internal(ALL_ONES)
    }

    public fun make_points(count: u64): u64 {
        let index = 0;
        let last = 0;
        while (index < count) {
            last = point_identity_internal();
            index += 1;
        };
        last
    }
}

// RUN: execute 0x1::ristretto255::from_sha512_bytes
// CHECK: results: 0x642965c2f4b0b091c08a4ad71c888f2a402e93cda23aef994e987e2ca3840e3e

// RUN: execute 0x1::ristretto255::from_64_uniform_bytes
// CHECK: results: 0x2e7c4964f91f5f2b074a9bc147ef973c08dbe29683746f979f11358065a2d155

// RUN: execute 0x1::ristretto255::reduced_all_ones
// CHECK: results: 0x1c95988d7431ecd670cf7d73f45befc6feffffffffffffffffffffffffffff0f

// RUN: execute 0x1::ristretto255::reduced_order
// CHECK: results: 0x0000000000000000000000000000000000000000000000000000000000000000

// RUN: execute 0x1::ristretto255::reduced_order_plus_one
// CHECK: results: 0x0100000000000000000000000000000000000000000000000000000000000000

// RUN: execute 0x1::ristretto255::from_u64_max
// CHECK: results: 0xffffffffffffffff000000000000000000000000000000000000000000000000

// RUN: execute 0x1::ristretto255::from_u128_max
// CHECK: results: 0xffffffffffffffffffffffffffffffff00000000000000000000000000000000

// RUN: execute 0x1::ristretto255::invert_zero
// CHECK: results: 0x0000000000000000000000000000000000000000000000000000000000000000

// RUN: execute 0x1::ristretto255::sub_wraps
// CHECK: results: 0xecd3f55c1a631258d69cf7a2def9de1400000000000000000000000000000010

// RUN: execute 0x1::ristretto255::canonical_at_order
// CHECK: results: false, true

// RUN: execute 0x1::ristretto255::point_wrong_length
// CHECK: results: false, 18446744073709551615, false

// RUN: execute 0x1::ristretto255::point_not_on_curve
// CHECK: results: 18446744073709551615, false

// RUN: execute 0x1::ristretto255::make_points --args 10000
// CHECK: results: 9999

// RUN: execute 0x1::ristretto255::make_points --args 10001
// CHECK: aborted: code 589828 (Too many points created: 10000, limit is 10000) in 0x1::ristretto255
