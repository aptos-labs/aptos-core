// RUN: publish --print(micro-ops)
module 0x99::eq {
    use std::vector;

    struct Point has copy, drop { x: u64, y: u64 }
    struct Nested has copy, drop { p: Point, tag: u64 }

    public fun eq_u64(a: u64, b: u64): bool { a == b }
    public fun neq_u64(a: u64, b: u64): bool { a != b }
    public fun eq_u128(a: u128, b: u128): bool { a == b }
    public fun eq_addr(a: address, b: address): bool { a == b }

    public fun eq_struct(ax: u64, ay: u64, bx: u64, by: u64): bool {
        Point { x: ax, y: ay } == Point { x: bx, y: by }
    }

    public fun neq_struct(ax: u64, ay: u64, bx: u64, by: u64): bool {
        Point { x: ax, y: ay } != Point { x: bx, y: by }
    }

    public fun eq_nested(ax: u64, atag: u64, bx: u64, btag: u64): bool {
        Nested { p: Point { x: ax, y: 0 }, tag: atag }
            == Nested { p: Point { x: bx, y: 0 }, tag: btag }
    }

    fun vec2(a: u64, b: u64): vector<u64> {
        let v = vector::empty<u64>();
        vector::push_back(&mut v, a);
        vector::push_back(&mut v, b);
        v
    }

    public fun eq_vec2(a: u64, b: u64, c: u64, d: u64): bool {
        vec2(a, b) == vec2(c, d)
    }

    public fun eq_vec_len(a: u64, b: u64, c: u64): bool {
        let v1 = vec2(a, b);
        let v2 = vector::empty<u64>();
        vector::push_back(&mut v2, c);
        v1 == v2
    }

    public fun eq_ref(a: u64, b: u64): bool {
        &a == &b
    }
}

// --- primitives ---
// RUN: execute 0x99::eq::eq_u64 --args 3, 3
// CHECK: results: true

// RUN: execute 0x99::eq::eq_u64 --args 3, 4
// CHECK: results: false

// RUN: execute 0x99::eq::neq_u64 --args 3, 4
// CHECK: results: true

// RUN: execute 0x99::eq::neq_u64 --args 5, 5
// CHECK: results: false

// RUN: execute 0x99::eq::eq_u128 --args 340282366920938463463374607431768211455, 340282366920938463463374607431768211455
// CHECK: results: true

// RUN: execute 0x99::eq::eq_u128 --args 1, 2
// CHECK: results: false

// RUN: execute 0x99::eq::eq_addr --args 0x1, 0x1
// CHECK: results: true

// RUN: execute 0x99::eq::eq_addr --args 0x1, 0x2
// CHECK: results: false

// --- structs (field-order, lexicographic) ---
// RUN: execute 0x99::eq::eq_struct --args 1, 2, 1, 2
// CHECK: results: true

// RUN: execute 0x99::eq::eq_struct --args 1, 2, 1, 3
// CHECK: results: false

// RUN: execute 0x99::eq::eq_struct --args 1, 2, 9, 2
// CHECK: results: false

// RUN: execute 0x99::eq::neq_struct --args 1, 2, 1, 3
// CHECK: results: true

// --- nested struct (struct containing struct + a u8 field) ---
// RUN: execute 0x99::eq::eq_nested --args 7, 1, 7, 1
// CHECK: results: true

// RUN: execute 0x99::eq::eq_nested --args 7, 1, 7, 2
// CHECK: results: false

// --- vectors (element-wise + length) ---
// RUN: execute 0x99::eq::eq_vec2 --args 1, 2, 1, 2
// CHECK: results: true

// RUN: execute 0x99::eq::eq_vec2 --args 1, 2, 1, 9
// CHECK: results: false

// RUN: execute 0x99::eq::eq_vec_len --args 1, 2, 1
// CHECK: results: false

// --- references (compared by referent) ---
// RUN: execute 0x99::eq::eq_ref --args 4, 4
// CHECK: results: true

// RUN: execute 0x99::eq::eq_ref --args 4, 5
// CHECK: results: false
