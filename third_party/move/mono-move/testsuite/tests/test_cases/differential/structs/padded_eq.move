// Equality of structs with padding compares fields, not padding bytes. `dirty` runs with a
// different seed before each `build`, so p1 and p2 are built over different leftover stack
// contents and their padding differs; the golden pins the frame layouts that make this so.
// The second module checks equality and ordering of vectors of padded structs and of enums
// across variants of different sizes.

// RUN: publish --print(micro-ops,frame-layout)
module 0x42::padded_eq {
    struct P has copy, drop { a: u8, b: u64, c: u8 }

    fun dirty(seed: u64): u64 {
        let w1 = seed ^ 0x0101010101010101;
        let w2 = seed ^ 0x0202020202020202;
        let w3 = seed ^ 0x0303030303030303;
        let w4 = seed ^ 0x0404040404040404;
        let w5 = seed ^ 0x0505050505050505;
        let w6 = seed ^ 0x0606060606060606;
        w1 ^ w2 ^ w3 ^ w4 ^ w5 ^ w6 ^ seed
    }

    fun build(first: u8, middle: u64, last: u8): P { P { a: first, b: middle, c: last } }

    public fun equal_after_dirty(): bool {
        dirty(0xa0a0a0a0a0a0a0a0);
        let p1 = build(1, 2, 3);
        dirty(0x5050505050505050);
        let p2 = build(1, 2, 3);
        p1 == p2
    }

    public fun ref_equal_after_dirty(): bool {
        dirty(0xa0a0a0a0a0a0a0a0);
        let p1 = build(1, 2, 3);
        dirty(0x5050505050505050);
        let p2 = build(1, 2, 3);
        &p1 == &p2
    }

    public fun differ_after_dirty(): bool {
        dirty(0xa0a0a0a0a0a0a0a0);
        let p1 = build(1, 2, 3);
        dirty(0x5050505050505050);
        let p2 = build(1, 2, 4);
        p1 == p2
    }
}

// RUN: publish
module 0x42::padded_shapes {
    use std::cmp;
    use std::vector;

    struct P has copy, drop, store { a: u8, b: u64, c: u8 }
    enum E has copy, drop, store { X { v: u8 }, Y, Z { w: u128, t: u8 } }
    struct Q has copy, drop, store { tag: u8, p: P, e: E, s: i16 }

    fun build_q(tag: u8, value: u64): Q {
        Q { tag, p: P { a: tag, b: value, c: tag }, e: E::Z { w: (value as u128), t: tag }, s: -1 }
    }

    public fun vectors_of_padded(count: u64): u64 {
        let v1 = vector[];
        let v2 = vector[];
        let index1 = 0;
        while (index1 < count) {
            vector::push_back(&mut v1, build_q((index1 as u8), index1 * 3));
            index1 = index1 + 1;
        };
        0x0::test_utils::force_gc();
        let index2 = 0;
        while (index2 < count) {
            let byte = (index2 as u8);
            vector::push_back(&mut v2, Q { tag: byte, p: P { a: byte, b: index2 * 3, c: byte }, e: E::Z { w: ((index2 * 3) as u128), t: byte }, s: -1 });
            index2 = index2 + 1;
        };
        let bits = 0;
        if (v1 == v2) bits = bits | 1;
        if (cmp::compare(&v1, &v2).is_eq()) bits = bits | 2;
        vector::borrow_mut(&mut v2, count - 1).p.c = 0;
        if (v1 != v2) bits = bits | 4;
        if (cmp::compare(&v1, &v2).is_gt()) bits = bits | 8;
        bits
    }

    // Variants of different payload sizes compare unequal; same variant, same payload, equal.
    public fun enum_variants(): u64 {
        let variant_x = E::X { v: 1 };
        let variant_y = E::Y;
        let variant_z = E::Z { w: 1, t: 1 };
        let bits = 0;
        if (variant_x != variant_y) bits = bits | 1;
        if (variant_y != variant_z) bits = bits | 2;
        if (variant_x == E::X { v: 1 }) bits = bits | 4;
        if (variant_y == E::Y) bits = bits | 8;
        if (variant_z != E::Z { w: 1, t: 2 }) bits = bits | 16;
        if (cmp::compare(&variant_x, &variant_z).is_lt()) bits = bits | 32;
        if (cmp::compare(&variant_z, &variant_y).is_gt()) bits = bits | 64;
        if (&variant_x == &E::X { v: 1 }) bits = bits | 128;
        bits
    }
}

// RUN: execute 0x42::padded_eq::equal_after_dirty
// CHECK: results: true

// RUN: execute 0x42::padded_eq::ref_equal_after_dirty
// CHECK: results: true

// RUN: execute 0x42::padded_eq::differ_after_dirty
// CHECK: results: false

// RUN: execute 0x42::padded_shapes::vectors_of_padded --args 6 --heap-size 8192
// CHECK: results: 15
// CHECK-GC-COUNT: 1

// RUN: execute 0x42::padded_shapes::enum_variants
// CHECK: results: 255
