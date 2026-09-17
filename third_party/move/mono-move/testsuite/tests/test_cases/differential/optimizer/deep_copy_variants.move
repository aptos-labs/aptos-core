// Copies of heap-owning values must remain independent across mutations,
// loop iterations, and calls. Coverage includes enum payloads and struct fields.

// RUN: publish
module 0x42::deep_copy_variants {
    use std::vector;

    enum Holder has copy, drop {
        One { v: vector<u64> },
        Two { x: u64 },
    }

    struct Boxed has copy, drop { items: vector<u64>, n: u64 }

    fun make_vec(seed: u64): vector<u64> {
        let v = vector::empty<u64>();
        let i = 0;
        while (i < seed) { vector::push_back(&mut v, i); i = i + 1; };
        v
    }

    // Mutating either variant's copied payload must leave the original unchanged.
    fun enum_copy(sel: u64): u64 {
        let e = if (sel == 0) {
            Holder::One { v: vector[1, 2, 3] }
        } else {
            Holder::Two { x: 9 }
        };
        let d = copy e;
        match (&mut d) {
            Holder::One { v } => vector::push_back(v, 99),
            Holder::Two { x } => *x = 0,
        };
        match (&e) {
            Holder::One { v } => vector::length(v),
            Holder::Two { x } => *x + 100,
        }
    }

    // Growing the copied vector field must leave the original unchanged.
    fun struct_vec_copy_realloc(): u64 {
        let b = Boxed { items: vector[1, 2], n: 5 };
        let d = copy b;
        vector::push_back(&mut d.items, 7);
        vector::length(&b.items) * 10 + vector::length(&d.items)
    }

    // Each iteration must copy the unmodified base vector.
    fun copy_in_loop(n: u64): u64 {
        let base = vector[1];
        let acc = 0;
        let i = 0;
        while (i < n) {
            let d = copy base;
            vector::push_back(&mut d, i);
            acc = acc + vector::length(&d);
            i = i + 1;
        };
        acc * 10 + vector::length(&base)
    }

    // The original remains readable after a call borrows the copy.
    fun copy_across_call(x: u64): u64 {
        let v = vector[x, x + 1];
        let d = copy v;
        let l = vector::length(&d);
        *vector::borrow(&v, 0) + l
    }

    // Writes to either vector must leave the other unchanged.
    fun both_mutated(): u64 {
        let v = vector[1, 2, 3];
        let d = copy v;
        *vector::borrow_mut(&mut v, 0) = 10;
        *vector::borrow_mut(&mut d, 1) = 20;
        *vector::borrow(&v, 0) * 1000 + *vector::borrow(&v, 1) * 100
            + *vector::borrow(&d, 0) * 10 + *vector::borrow(&d, 1)
    }

    // Copy of a call result, then grow the copy.
    fun copy_of_call_result(): u64 {
        let v = make_vec(3);
        let d = copy v;
        vector::push_back(&mut d, 77);
        vector::length(&v) * 10 + vector::length(&d)
    }
}

// RUN: execute 0x42::deep_copy_variants::enum_copy --args 0
// CHECK: results: 3

// RUN: execute 0x42::deep_copy_variants::enum_copy --args 1
// CHECK: results: 109

// RUN: execute 0x42::deep_copy_variants::struct_vec_copy_realloc
// CHECK: results: 23

// RUN: execute 0x42::deep_copy_variants::copy_in_loop --args 3
// CHECK: results: 61

// RUN: execute 0x42::deep_copy_variants::copy_across_call --args 5
// CHECK: results: 7

// RUN: execute 0x42::deep_copy_variants::both_mutated
// CHECK: results: 10230

// RUN: execute 0x42::deep_copy_variants::copy_of_call_result
// CHECK: results: 34
