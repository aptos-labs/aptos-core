// A capturing closure stays usable through collections that reuse the memory it was built
// in. `fill_n` = 1361 makes allocating the captured data the first allocation that does not
// fit, while the new closure is not yet stored anywhere; two more collections run before the
// closure is called.

// RUN: publish
module 0x42::capture_gc_reuse {
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

    // Position-weighted, so every captured element matters.
    fun weigh(vals: vector<u64>, scale: u64): u64 {
        *vector::borrow(&vals, 0) * 10000 + *vector::borrow(&vals, 1) * 100 + *vector::borrow(&vals, 2) + scale * 1000000
    }

    public fun pack_then_reuse(first: u64, second: u64, fill_n: u64, scribble_n: u64): u64 {
        let vals = vector::empty<u64>();
        vector::push_back(&mut vals, first);
        vector::push_back(&mut vals, second);
        vector::push_back(&mut vals, first + second);
        fill(fill_n);
        let weighed = |scale| weigh(vals, scale);
        0x0::test_utils::force_gc();
        scribble(scribble_n);
        weighed(1)
    }
}

// The captured data fits: only the forced and the decoy collections run.
// RUN: execute 0x42::capture_gc_reuse::pack_then_reuse --args 4, 5, 1360, 1100 --heap-size 32768
// CHECK: results: 1040509
// CHECK-GC-COUNT: 2

// RUN: execute 0x42::capture_gc_reuse::pack_then_reuse --args 4, 5, 1361, 1100 --heap-size 32768
// CHECK: results: 1040509
// CHECK-GC-COUNT: 3
