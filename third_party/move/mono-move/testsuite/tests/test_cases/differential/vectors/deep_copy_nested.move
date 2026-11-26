// Copying vectors whose elements hold vectors: `vector<vector<u64>>`, a vector of structs
// with a vector field, and a vector of enums. Each test overwrites an element inside the
// copy and reads both owners, so sharing would change the original's digest. At the
// smaller heap sizes the copy collects after copying the outer vector.

// RUN: publish
module 0x66::dcn {
    use std::vector;

    struct S has copy, drop { tag: u8, v: vector<u64> }

    enum E has copy, drop { A { v: vector<u64> }, B { n: u64 }, C { w: vector<u64>, k: u64 } }

    fun junk(value: u64): u64 {
        let garbage = vector[value, value, value, value];
        vector::length(&garbage)
    }

    fun digest(outer: &vector<vector<u64>>): u64 {
        let acc = 0;
        let index = 0;
        while (index < vector::length(outer)) {
            let inner = vector::borrow(outer, index);
            let inner_index = 0;
            while (inner_index < vector::length(inner)) { acc = acc * 100 + *vector::borrow(inner, inner_index); inner_index = inner_index + 1; };
            index = index + 1;
        };
        acc
    }

    fun enum_digest(elems: &vector<E>): u64 {
        let acc = 0;
        let index = 0;
        while (index < vector::length(elems)) {
            acc = acc * 100 + match (vector::borrow(elems, index)) {
                E::A { v: values } => *vector::borrow(values, 0),
                E::B { n: num } => *num,
                E::C { w: values, k: offset } => *vector::borrow(values, 1) + *offset,
            };
            index = index + 1;
        };
        acc
    }

    public fun nested(first: u64, second: u64): (u64, u64) {
        let src_vec = vector[vector[first, second], vector[3], vector[]];
        junk(first);
        let dup_vec = src_vec;
        *vector::borrow_mut(vector::borrow_mut(&mut dup_vec, 0), 1) = 99;
        *vector::borrow_mut(vector::borrow_mut(&mut dup_vec, 1), 0) = 98;
        (digest(&src_vec), digest(&dup_vec))
    }

    public fun of_structs(value: u64): (u64, u64) {
        let src_vec = vector[S { tag: 1, v: vector[value, value + 1] }, S { tag: 2, v: vector[] }];
        junk(value);
        let dup_vec = src_vec;
        *vector::borrow_mut(&mut vector::borrow_mut(&mut dup_vec, 0).v, 0) = 77;
        vector::borrow_mut(&mut dup_vec, 1).tag = 9;
        let orig = vector::borrow(&src_vec, 0);
        let dup = vector::borrow(&dup_vec, 0);
        (*vector::borrow(&orig.v, 0) * 100 + (vector::borrow(&src_vec, 1).tag as u64),
         *vector::borrow(&dup.v, 0) * 100 + (vector::borrow(&dup_vec, 1).tag as u64))
    }

    public fun of_enums(value: u64): (u64, u64) {
        let src_vec = vector[E::A { v: vector[value] }, E::B { n: 4 }, E::C { w: vector[1, 2], k: 10 }];
        junk(value);
        let dup_vec = src_vec;
        match (vector::borrow_mut(&mut dup_vec, 0)) { E::A { v: values } => *vector::borrow_mut(values, 0) = 66, _ => () };
        match (vector::borrow_mut(&mut dup_vec, 2)) { E::C { w: values, k: _ } => *vector::borrow_mut(values, 1) = 50, _ => () };
        (enum_digest(&src_vec), enum_digest(&dup_vec))
    }
}

// RUN: execute 0x66::dcn::nested --args 1, 2
// CHECK: results: 10203, 19998

// RUN: execute 0x66::dcn::of_structs --args 5
// CHECK: results: 502, 7709

// RUN: execute 0x66::dcn::of_enums --args 5
// CHECK: results: 50412, 660460

// RUN: execute 0x66::dcn::nested --args 1, 2 --heap-size 192
// CHECK: results: 10203, 19998
// CHECK-GC-COUNT: 1

// RUN: execute 0x66::dcn::nested --args 1, 2 --heap-size 239
// CHECK: results: 10203, 19998
// CHECK-GC-COUNT: 1

// RUN: execute 0x66::dcn::nested --args 1, 2 --heap-size 240
// CHECK: results: 10203, 19998
// CHECK-GC-COUNT: 0

// At 160 the copy collects on its first allocation, the outer vector.
// RUN: execute 0x66::dcn::of_structs --args 5 --heap-size 160
// CHECK: results: 502, 7709
// CHECK-GC-COUNT: 1

// RUN: execute 0x66::dcn::of_structs --args 5 --heap-size 207
// CHECK: results: 502, 7709
// CHECK-GC-COUNT: 1

// RUN: execute 0x66::dcn::of_structs --args 5 --heap-size 208
// CHECK: results: 502, 7709
// CHECK-GC-COUNT: 0

// RUN: execute 0x66::dcn::of_enums --args 5 --heap-size 384
// CHECK: results: 50412, 660460
// CHECK-GC-COUNT: 1

// RUN: execute 0x66::dcn::of_enums --args 5 --heap-size 431
// CHECK: results: 50412, 660460
// CHECK-GC-COUNT: 1

// RUN: execute 0x66::dcn::of_enums --args 5 --heap-size 432
// CHECK: results: 50412, 660460
// CHECK-GC-COUNT: 0
