// The test-only `random_scalar_internal`: only properties of the output are checked (32
// bytes, canonical, times its inverse is one, two draws differ). These fail only for a zero
// or repeated draw, each with probability about 2^-252.

// RUN: publish
module 0x1::ristretto255 {
    native fun random_scalar_internal(): vector<u8>;

    native fun scalar_is_canonical_internal(bytes: vector<u8>): bool;

    native fun scalar_invert_internal(bytes: vector<u8>): vector<u8>;

    native fun scalar_mul_internal(a_bytes: vector<u8>, b_bytes: vector<u8>): vector<u8>;

    public fun properties(): (u64, bool, vector<u8>, bool) {
        let first = random_scalar_internal();
        let second = random_scalar_internal();
        (
            first.length(),
            scalar_is_canonical_internal(first),
            scalar_mul_internal(first, scalar_invert_internal(first)),
            first != second
        )
    }
}

// RUN: execute 0x1::ristretto255::properties
// CHECK: results: 32, true, 0x0100000000000000000000000000000000000000000000000000000000000000, true
