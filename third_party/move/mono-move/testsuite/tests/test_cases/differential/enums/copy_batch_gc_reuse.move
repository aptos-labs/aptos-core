// Copying a struct with an enum field and a vector field. `fill_n` = 1360 dead vectors
// makes the vector's copy the first allocation that does not fit, after the enum's copy
// was made (1359 fits), and two more collections run before the copy is read.

// RUN: publish
module 0x42::copy_batch_gc_reuse {
    use std::vector;

    enum E has copy, drop { V { left: u64, right: u64 } }

    struct Two has copy, drop { pair: E, vals: vector<u64> }

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

    fun fold(two: &Two): u64 {
        match (&two.pair) {
            E::V { left, right } => *left * 10000 + *right * 100 + *vector::borrow(&two.vals, 0) * 10 + *vector::borrow(&two.vals, 2),
        }
    }

    fun poke(two: &mut Two, val: u64) {
        *vector::borrow_mut(&mut two.vals, 2) = val;
        match (&mut two.pair) { E::V { left, right: _ } => *left = val + 1 };
    }

    // Returns the digest and the middle element of the copy and of the original.
    public fun copy_then_reuse(seed: u64, fill_n: u64, scribble_n: u64): (u64, u64, u64) {
        let live = Two { pair: E::V { left: seed, right: seed + 4 }, vals: vector[seed + 1, seed + 2, seed + 3] };
        fill(fill_n);
        let dup = live;
        0x0::test_utils::force_gc();
        scribble(scribble_n);
        let before = fold(&dup);
        poke(&mut dup, 9);
        let digest = (before * 1000000 + fold(&dup)) * 1000000 + fold(&live);
        (digest, *vector::borrow(&dup.vals, 1), *vector::borrow(&live.vals, 1))
    }
}

// Both copies fit: only the forced and the decoy collections run.
// RUN: execute 0x42::copy_batch_gc_reuse::copy_then_reuse --args 7, 1359, 1100 --heap-size 32768
// CHECK: results: 71190101189071190, 9, 9
// CHECK-GC-COUNT: 2

// RUN: execute 0x42::copy_batch_gc_reuse::copy_then_reuse --args 7, 1360, 1100 --heap-size 32768
// CHECK: results: 71190101189071190, 9, 9
// CHECK-GC-COUNT: 3
