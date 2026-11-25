// Scalar arguments with bit 255 set: the natives clear that bit and use the remaining value
// (5 here) rather than reducing the full 256-bit value mod the group order l or rejecting it.
// Expected values are computed mod l from the cleared value.

// RUN: publish
module 0x1::ristretto255 {
    struct RistrettoPoint has drop {
        handle: u64
    }

    native fun scalar_invert_internal(bytes: vector<u8>): vector<u8>;

    native fun scalar_mul_internal(a_bytes: vector<u8>, b_bytes: vector<u8>): vector<u8>;

    native fun scalar_add_internal(a_bytes: vector<u8>, b_bytes: vector<u8>): vector<u8>;

    native fun scalar_sub_internal(a_bytes: vector<u8>, b_bytes: vector<u8>): vector<u8>;

    native fun scalar_neg_internal(a_bytes: vector<u8>): vector<u8>;

    native fun scalar_from_u64_internal(num: u64): vector<u8>;

    native fun basepoint_mul_internal(scalar: vector<u8>): u64;

    native fun point_compress_internal(point: &RistrettoPoint): vector<u8>;

    // 5 with bit 255 set.
    const HIGH_FIVE: vector<u8> = x"0500000000000000000000000000000000000000000000000000000000000080";
    // 7 with bit 255 set.
    const HIGH_SEVEN: vector<u8> = x"0700000000000000000000000000000000000000000000000000000000000080";

    // 5 + 3 == 8.
    public fun add(): vector<u8> {
        scalar_add_internal(HIGH_FIVE, scalar_from_u64_internal(3))
    }

    // 3 - 5 == l - 2.
    public fun sub(): vector<u8> {
        scalar_sub_internal(scalar_from_u64_internal(3), HIGH_FIVE)
    }

    // 5 * 7 == 35.
    public fun mul(): vector<u8> {
        scalar_mul_internal(HIGH_FIVE, HIGH_SEVEN)
    }

    // -5 == l - 5.
    public fun neg(): vector<u8> {
        scalar_neg_internal(HIGH_FIVE)
    }

    // 5^(l - 2) mod l.
    public fun invert(): vector<u8> {
        scalar_invert_internal(HIGH_FIVE)
    }

    // 5 * B, the RFC 9496 test vector B[5].
    public fun basepoint_mul(): vector<u8> {
        let point = RistrettoPoint { handle: basepoint_mul_internal(HIGH_FIVE) };
        point_compress_internal(&point)
    }
}

// RUN: execute 0x1::ristretto255::add
// CHECK: results: 0x0800000000000000000000000000000000000000000000000000000000000000

// RUN: execute 0x1::ristretto255::sub
// CHECK: results: 0xebd3f55c1a631258d69cf7a2def9de1400000000000000000000000000000010

// RUN: execute 0x1::ristretto255::mul
// CHECK: results: 0x2300000000000000000000000000000000000000000000000000000000000000

// RUN: execute 0x1::ristretto255::neg
// CHECK: results: 0xe8d3f55c1a631258d69cf7a2def9de1400000000000000000000000000000010

// RUN: execute 0x1::ristretto255::invert
// CHECK: results: 0x965d64df9ee036abf7b864ed92cb5f3733333333333333333333333333333303

// RUN: execute 0x1::ristretto255::basepoint_mul
// CHECK: results: 0xe882b131016b52c1d3337080187cf768423efccbb517bb495ab812c4160ff44e
