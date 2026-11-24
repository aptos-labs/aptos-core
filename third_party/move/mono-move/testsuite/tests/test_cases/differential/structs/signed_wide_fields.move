// Signed integers and u256 as struct fields, enum payloads and generic
// arguments: read, copy, compare, order, and round-trip through BCS.

// RUN: publish
module 0x1::from_bcs {
    public native fun from_bytes<T>(bytes: vector<u8>): T;
}
module 0x42::swf {
    use std::bcs;
    use std::cmp;

    struct S has copy, drop { a: i8, b: i128, c: i16, d: i256, e: u8, f: u256 }
    enum E has copy, drop { A { x: i64 }, B { y: u256, z: i8 } }
    struct Box<T> has copy, drop { t: T }

    fun mk(): S { S { a: -128, b: -170141183460469231731687303715884105728, c: -2, d: -1, e: 255, f: 1 << 255 } }

    // Writes to a copy reach the copy only.
    public fun fields(): (i8, i128, i16, i256, u8, u256, i8, i128, i256, u256) {
        let orig = mk();
        let dup = orig;
        dup.a = 127;
        dup.b = dup.b + 1;
        dup.d = -57896044618658097711785492504343953926634992332820282019728792003956564819968;
        dup.f = dup.f + 5;
        (orig.a, orig.b, orig.c, orig.d, orig.e, orig.f, dup.a, dup.b, dup.d, dup.f)
    }

    public fun compare(): u64 {
        let orig = mk();
        let dup = orig;
        let bits = if (orig == dup) 1 else 0;
        dup.d = 0;
        if (orig != dup) bits = bits | 2;
        if (cmp::compare(&orig, &dup).is_lt()) bits = bits | 4;
        let e1 = E::A { x: -1 };
        let e2 = E::B { y: 0, z: -1 };
        if (cmp::compare(&e1, &e2).is_lt()) bits = bits | 8;
        if (E::B { y: 1 << 200, z: -5 } == E::B { y: 1 << 200, z: -5 }) bits = bits | 16;
        if (cmp::compare(&Box { t: -3i32 }, &Box { t: 2i32 }).is_lt()) bits = bits | 32;
        if (Box { t: (1u256 << 255) } != Box { t: 1 }) bits = bits | 64;
        bits
    }

    public fun enum_payload(): (i64, u256, i8) {
        let values = vector[E::A { x: -9223372036854775808 }, E::B { y: (1 << 255) + 1, z: -128 }];
        let elem = values[0];
        let x_val = match (elem) { E::A { x: x_val } => x_val, E::B { y: _, z: _ } => 0 };
        match (&mut values[1]) { E::B { y: y_ref, z: z_ref } => { *y_ref = *y_ref - 1; *z_ref = *z_ref + 1 }, E::A { x: _ } => () };
        let (y_val, z_val) = match (values[1]) { E::B { y: y_val, z: z_val } => (y_val, z_val), E::A { x: _ } => (0, 0) };
        (x_val, y_val, z_val)
    }

    public fun bcs_round_trip(): (vector<u8>, bool, bool, i256) {
        let orig = mk();
        let bytes = bcs::to_bytes(&orig);
        let back = 0x1::from_bcs::from_bytes<S>(bytes);
        let variant = E::B { y: 3, z: -3 };
        let e_back = 0x1::from_bcs::from_bytes<E>(bcs::to_bytes(&variant));
        let boxed = 0x1::from_bcs::from_bytes<Box<i256>>(bcs::to_bytes(&Box { t: -77i256 }));
        (bcs::to_bytes(&variant), back == orig, e_back == variant, boxed.t)
    }
}

// RUN: execute 0x42::swf::fields
// CHECK: results: -128, -170141183460469231731687303715884105728, -2, -1, 255, 57896044618658097711785492504343953926634992332820282019728792003956564819968, 127, -170141183460469231731687303715884105727, -57896044618658097711785492504343953926634992332820282019728792003956564819968, 57896044618658097711785492504343953926634992332820282019728792003956564819973

// RUN: execute 0x42::swf::compare
// CHECK: results: 127

// RUN: execute 0x42::swf::enum_payload
// CHECK: results: -9223372036854775808, 57896044618658097711785492504343953926634992332820282019728792003956564819968, -127

// RUN: execute 0x42::swf::bcs_round_trip
// CHECK: results: 0x010300000000000000000000000000000000000000000000000000000000000000fd, true, true, -77
