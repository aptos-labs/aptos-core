// A small `--heap-size` exercises MonoMove under allocation pressure; V1
// ignores this setting. Live values must survive allocations and dropped
// temporaries. These tests check results without asserting collection counts.

// RUN: publish
module 0x1::gc_stress {
    use std::vector;

    fun make_vec(n: u64): vector<u64> {
        let v = vector::empty<u64>();
        let i = 0;
        while (i < n) { vector::push_back(&mut v, i); i = i + 1; };
        v
    }

    // Allocate-then-drop in a loop under heap pressure.
    public fun churn_with_drops(n: u64): u64 {
        let acc = 0;
        let i = 0;
        while (i < n) {
            let junk = make_vec(i);
            if (vector::length(&junk) > 1000) { acc = acc + 1; };
            acc = acc + i;
            i = i + 1;
        };
        acc
    }

    // One branch allocates a vector; both branches return a scalar.
    public fun slot_reuse(cond: bool): u64 {
        let out = if (cond) {
            let v = vector[1u64, 2, 3];
            vector::length(&v)
        } else {
            42
        };
        out
    }

    fun pair_vec(): (vector<u64>, vector<u64>) { (vector[7], vector[8, 9]) }

    // One of two returned vectors is dropped at the call site.
    public fun drop_half(): u64 {
        let (a, _) = pair_vec();
        *vector::borrow(&a, 0)
    }

    // The live vector must survive repeated allocations of temporary vectors.
    public fun live_across_gc(n: u64): u64 {
        let keep = vector[100u64, 200, 300];
        let i = 0;
        while (i < n) { let _junk = make_vec(4); i = i + 1; };
        *vector::borrow(&keep, 0) + *vector::borrow(&keep, 2)
    }
}

// RUN: execute 0x1::gc_stress::churn_with_drops --args 30 --heap-size 4096
// CHECK: results: 435

// RUN: execute 0x1::gc_stress::slot_reuse --args true --heap-size 4096
// CHECK: results: 3

// RUN: execute 0x1::gc_stress::slot_reuse --args false --heap-size 4096
// CHECK: results: 42

// RUN: execute 0x1::gc_stress::drop_half --heap-size 4096
// CHECK: results: 7

// RUN: execute 0x1::gc_stress::live_across_gc --args 50 --heap-size 4096
// CHECK: results: 400
