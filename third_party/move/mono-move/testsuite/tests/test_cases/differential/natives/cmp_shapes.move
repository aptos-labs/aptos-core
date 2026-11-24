// `cmp::compare` on signed integers of each width (sign first); unsigned integers, one-element
// u64 vectors, addresses and signers whose low-order bytes order the opposite way to their values,
// and a u8 with its high bit set; vector prefixes; empty vectors, including one emptied by a pop;
// nested vectors; enums (tag first, then payload); Option; and a struct with a u8 field before a
// u64 field. Results encode the ordering as 0 (less), 1 (equal), 2 (greater); a step without
// arguments returns one code per comparison.

// RUN: publish
module 0x1::create_signer {
    public native fun create_signer(addr: address): signer;
}
module 0x42::cmp_shapes {
    use std::cmp::{Self, Ordering};
    use std::option;
    use std::vector;

    enum Shape has drop { Circle { r: u64 }, Rect { w: u64, h: u64 } }

    enum Tagged has drop {
        Num { value: u64 },
        Unit,
        Bytes { bytes: vector<u8> },
    }

    struct Flat has drop { a: u8, b: u64 }

    fun code(ordering: Ordering): u8 {
        if (ordering.is_lt()) 0 else if (ordering.is_eq()) 1 else 2
    }

    fun ord<T>(left: &T, right: &T): u8 { code(cmp::compare(left, right)) }

    public fun i8s(left: i8, right: i8): u8 { code(cmp::compare(&left, &right)) }
    public fun i64s(left: i64, right: i64): u8 { code(cmp::compare(&left, &right)) }
    public fun i128s(left: i128, right: i128): u8 { code(cmp::compare(&left, &right)) }
    public fun i256s(left: i256, right: i256): u8 { code(cmp::compare(&left, &right)) }

    public fun signed(): vector<u8> {
        let neg8: i8 = -128;
        let pos8: i8 = 127;
        let neg16: i16 = -1;
        let neg32: i32 = -2;
        let neg64: i64 = -1;
        let neg128: i128 = -1;
        let min256: i256 = -57896044618658097711785492504343953926634992332820282019728792003956564819968;
        let neg256: i256 = -1;
        vector[
            ord(&neg8, &pos8),
            ord(&pos8, &neg8),
            ord(&neg16, &0i16),
            ord(&neg32, &-1i32),
            ord(&neg64, &1i64),
            ord(&1i64, &neg64),
            ord(&neg128, &neg128),
            ord(&neg128, &170141183460469231731687303715884105727i128),
            ord(&min256, &neg256),
            ord(&neg256, &min256),
        ]
    }

    public fun u16s(left: u16, right: u16): u8 { code(cmp::compare(&left, &right)) }
    public fun u128s(left: u128, right: u128): u8 { code(cmp::compare(&left, &right)) }
    public fun u256s(left: u256, right: u256): u8 { code(cmp::compare(&left, &right)) }

    // The low byte of the smaller operand is larger, so a byte-wise or
    // truncated comparison gives the wrong answer.
    public fun unsigned_widths(): vector<u8> {
        vector[
            ord(&255u8, &1u8),
            ord(&0x01ffu16, &0x02feu16),
            ord(&0x01ffffffu32, &0x02fffffeu32),
            ord(&0x01ffffffffffffffffu128, &0x02fffffffffffffffeu128),
            ord(
                &0x01ffffffffffffffffffffffffffffffffu256,
                &0x02fffffffffffffffffffffffffffffffeu256
            ),
            ord(&340282366920938463463374607431768211455u128, &340282366920938463463374607431768211455u128),
        ]
    }

    public fun addrs(left: address, right: address): u8 { code(cmp::compare(&left, &right)) }

    // Addresses compare as big-endian byte strings.
    public fun addresses(): vector<u8> {
        let s1 = 0x1::create_signer::create_signer(@0x100);
        let s2 = 0x1::create_signer::create_signer(@0xff);
        vector[
            ord(&@0x1, &@0x100),
            ord(&@0x100, &@0xff),
            ord(&@0xcafe, &@0xcafe),
            ord(&s1, &s2),
        ]
    }

    public fun u64_vecs(left: u64, right: u64): u8 { code(cmp::compare(&vector[left], &vector[right])) }

    public fun prefix(count: u64): u8 {
        let short = vector[1u64, 2];
        let long = vector[1u64, 2];
        let index = 0;
        while (index < count) { vector::push_back(&mut long, index); index = index + 1; };
        code(cmp::compare(&short, &long))
    }

    public fun empty_vs_one(value: u8): u8 { code(cmp::compare(&vector::empty<u8>(), &vector[value])) }

    public fun null_vs_drained(value: u64): u8 {
        let drained = vector[value];
        vector::pop_back(&mut drained);
        code(cmp::compare(&vector::empty<u64>(), &drained))
    }

    public fun nested(left: u8, right: u8): u8 {
        code(cmp::compare(&vector[vector[1u8], vector[left]], &vector[vector[1u8], vector[right, 0]]))
    }

    // Equal prefix: the shorter vector is less. An empty vector is less than
    // any non-empty one.
    public fun vector_lengths(): vector<u8> {
        let empty: vector<u64> = vector[];
        vector[
            ord(&vector[1u8, 2], &vector[1u8, 2, 0]),
            ord(&vector[1u8, 2, 0], &vector[1u8, 2]),
            ord(&empty, &vector[0]),
            ord(&empty, &empty),
            ord(&vector[vector[1u64]], &vector[vector[1u64], vector[]]),
            ord(&vector[vector[2u64]], &vector[vector[1u64], vector[]]),
        ]
    }

    public fun shapes(radius: u64, width: u64): u8 {
        code(cmp::compare(&Shape::Rect { w: width, h: 0 }, &Shape::Circle { r: radius }))
    }

    public fun same_variant(w1: u64, h1: u64, w2: u64, h2: u64): u8 {
        code(cmp::compare(&Shape::Rect { w: w1, h: h1 }, &Shape::Rect { w: w2, h: h2 }))
    }

    // The tag decides before any payload; equal tags compare payloads.
    public fun enums(): vector<u8> {
        vector[
            ord(&Tagged::Num { value: 9 }, &Tagged::Unit),
            ord(&Tagged::Unit, &Tagged::Num { value: 9 }),
            ord(&Tagged::Num { value: 1 }, &Tagged::Num { value: 2 }),
            ord(&Tagged::Unit, &Tagged::Unit),
            ord(&Tagged::Bytes { bytes: b"ab" }, &Tagged::Bytes { bytes: b"a" }),
            ord(&Tagged::Bytes { bytes: vector[] }, &Tagged::Num { value: 0 }),
        ]
    }

    public fun none_vs_some(value: u64): u8 { code(cmp::compare(&option::none<u64>(), &option::some(value))) }

    public fun flats(a1: u8, b1: u64, a2: u8, b2: u64): u8 {
        let values = vector[Flat { a: a1, b: b1 }];
        code(cmp::compare(vector::borrow(&values, 0), &Flat { a: a2, b: b2 }))
    }
}

// RUN: execute 0x42::cmp_shapes::i8s --args -1, 1
// CHECK: results: 0

// RUN: execute 0x42::cmp_shapes::i64s --args -9223372036854775808, 9223372036854775807
// CHECK: results: 0

// RUN: execute 0x42::cmp_shapes::i64s --args -2, -3
// CHECK: results: 2

// RUN: execute 0x42::cmp_shapes::i128s --args -1, 0
// CHECK: results: 0

// RUN: execute 0x42::cmp_shapes::i256s --args -57896044618658097711785492504343953926634992332820282019728792003956564819968, 1
// CHECK: results: 0

// RUN: execute 0x42::cmp_shapes::signed
// CHECK: results: 0x00020000000201000002

// RUN: execute 0x42::cmp_shapes::u16s --args 256, 2
// CHECK: results: 2

// RUN: execute 0x42::cmp_shapes::u128s --args 18446744073709551616, 1
// CHECK: results: 2

// RUN: execute 0x42::cmp_shapes::u128s --args 18446744073709551616, 18446744073709551615
// CHECK: results: 2

// RUN: execute 0x42::cmp_shapes::u256s --args 340282366920938463463374607431768211456, 340282366920938463463374607431768211455
// CHECK: results: 2

// RUN: execute 0x42::cmp_shapes::unsigned_widths
// CHECK: results: 0x020000000001

// RUN: execute 0x42::cmp_shapes::addrs --args 0x200, 0x1ff
// CHECK: results: 2

// RUN: execute 0x42::cmp_shapes::addresses
// CHECK: results: 0x00020102

// RUN: execute 0x42::cmp_shapes::u64_vecs --args 256, 2
// CHECK: results: 2

// RUN: execute 0x42::cmp_shapes::u64_vecs --args 2, 256
// CHECK: results: 0

// RUN: execute 0x42::cmp_shapes::prefix --args 1
// CHECK: results: 0

// RUN: execute 0x42::cmp_shapes::prefix --args 0
// CHECK: results: 1

// RUN: execute 0x42::cmp_shapes::empty_vs_one --args 0
// CHECK: results: 0

// RUN: execute 0x42::cmp_shapes::null_vs_drained --args 3
// CHECK: results: 1

// RUN: execute 0x42::cmp_shapes::nested --args 5, 5
// CHECK: results: 0

// RUN: execute 0x42::cmp_shapes::nested --args 6, 5
// CHECK: results: 2

// RUN: execute 0x42::cmp_shapes::vector_lengths
// CHECK: results: 0x000200010002

// RUN: execute 0x42::cmp_shapes::shapes --args 100, 1
// CHECK: results: 2

// RUN: execute 0x42::cmp_shapes::same_variant --args 1, 9, 1, 8
// CHECK: results: 2

// RUN: execute 0x42::cmp_shapes::same_variant --args 1, 9, 2, 0
// CHECK: results: 0

// RUN: execute 0x42::cmp_shapes::enums
// CHECK: results: 0x000200010202

// RUN: execute 0x42::cmp_shapes::none_vs_some --args 0
// CHECK: results: 0

// RUN: execute 0x42::cmp_shapes::flats --args 1, 300, 1, 2
// CHECK: results: 2

// RUN: execute 0x42::cmp_shapes::flats --args 2, 0, 1, 300
// CHECK: results: 2

// RUN: execute 0x42::cmp_shapes::flats --args 1, 5, 1, 5
// CHECK: results: 1
