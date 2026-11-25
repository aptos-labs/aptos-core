// Pushing a vector onto a local `vector<vector<u64>>`. `fill_n` = 1351 dead vectors makes
// growing the outer vector the first allocation that does not fit (1350 fits), and two
// more collections run before the elements are read.

// RUN: publish
module 0x42::push_grow_gc_reuse {
    use std::vector;

    fun fill(count: u64): u64 {
        let acc = 0;
        let idx = 0;
        while (idx < count) {
            let junk = vector[idx];
            acc = acc + vector::length(&junk);
            idx = idx + 1;
        };
        acc
    }

    fun scribble(count: u64): u64 {
        let acc = 0;
        let idx = 0;
        while (idx < count) {
            let junk = vector[idx, idx];
            acc = acc + vector::length(&junk);
            idx = idx + 1;
        };
        acc
    }

    fun inner(seed: u64): vector<u64> {
        let vals = vector::empty<u64>();
        vector::push_back(&mut vals, seed);
        vector::push_back(&mut vals, seed + 1);
        vals
    }

    // Three digits per element: ten times its first value, plus its step and length.
    fun digest(outer: &vector<vector<u64>>): u64 {
        let acc = 0;
        let idx = 0;
        while (idx < vector::length(outer)) {
            let vals = vector::borrow(outer, idx);
            acc = acc * 1000 + *vector::borrow(vals, 0) * 10 + *vector::borrow(vals, 1) - *vector::borrow(vals, 0) + vector::length(vals);
            idx = idx + 1;
        };
        acc
    }

    public fun push_then_reuse(base: u64, fill_n: u64, scribble_n: u64): u64 {
        let outer = vector::empty<vector<u64>>();
        let idx = 0;
        while (idx < 4) {
            vector::push_back(&mut outer, inner(base + idx * 10));
            idx = idx + 1;
        };
        let last = inner(base + 40);
        fill(fill_n);
        vector::push_back(&mut outer, last);
        0x0::test_utils::force_gc();
        scribble(scribble_n);
        digest(&outer)
    }
}

// The growth fits: only the forced and the decoy collections run.
// RUN: execute 0x42::push_grow_gc_reuse::push_then_reuse --args 1, 1350, 1100 --heap-size 32768
// CHECK: results: 13113213313413
// CHECK-GC-COUNT: 2

// RUN: execute 0x42::push_grow_gc_reuse::push_then_reuse --args 1, 1351, 1100 --heap-size 32768
// CHECK: results: 13113213313413
// CHECK-GC-COUNT: 3
