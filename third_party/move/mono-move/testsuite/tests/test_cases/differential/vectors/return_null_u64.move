// Returning a never-pushed `vector<u64>` from `vector::empty` and from an empty literal,
// alone and between other return values next to a non-empty one. `all_kinds` returns
// every primitive type (mostly at extreme values), `vector<u8>`, `String`, and a drained
// `vector<u64>` next to a never-pushed one.

// RUN: publish
module 0x42::return_null_u64 {
    use std::string::{Self, String};
    use std::vector;

    public fun from_empty(): vector<u64> {
        vector::empty<u64>()
    }

    public fun from_literal(): vector<u64> {
        vector<u64>[]
    }

    public fun mixed(seed: u64): (u64, vector<u64>, vector<u64>, u64) {
        let full = vector::empty<u64>();
        vector::push_back(&mut full, seed);
        (seed + 1, vector::empty<u64>(), full, seed + 2)
    }

    public fun all_kinds(seed: u64): (
        bool, bool, u8, u16, u32, u64, u128, u256, i8, i16, i32, i64, i128, i256, address,
        vector<u8>, String, vector<u64>, vector<u64>
    ) {
        let bytes = x"00ff";
        vector::push_back(&mut bytes, (seed as u8));
        let drained = vector[seed, seed + 1];
        vector::pop_back(&mut drained);
        vector::pop_back(&mut drained);
        (
            seed == 7,
            seed == 8,
            255,
            65535,
            4294967295,
            seed,
            340282366920938463463374607431768211455,
            115792089237316195423570985008687907853269984665640564039457584007913129639935,
            -128,
            -32768,
            -2147483648,
            -9223372036854775808,
            -170141183460469231731687303715884105728,
            -57896044618658097711785492504343953926634992332820282019728792003956564819968,
            @0xcafe,
            bytes,
            string::utf8(b"move"),
            drained,
            vector::empty<u64>(),
        )
    }
}

// RUN: execute 0x42::return_null_u64::from_empty
// CHECK: results: []

// RUN: execute 0x42::return_null_u64::from_literal
// CHECK: results: []

// RUN: execute 0x42::return_null_u64::mixed --args 7
// CHECK: results: 8, [], [7], 9

// RUN: execute 0x42::return_null_u64::all_kinds --args 7
// CHECK: results: true, false, 255, 65535, 4294967295, 7, 340282366920938463463374607431768211455, 115792089237316195423570985008687907853269984665640564039457584007913129639935, -128, -32768, -2147483648, -9223372036854775808, -170141183460469231731687303715884105728, -57896044618658097711785492504343953926634992332820282019728792003956564819968, 0xcafe, 0x00ff07, "move", [], []
