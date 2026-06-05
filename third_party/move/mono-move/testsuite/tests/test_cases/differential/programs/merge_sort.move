// Recursive merge sort (O(n log n)) with a temp-vector merge. `sort_checksum`
// builds its input via an LCG and folds the sorted result into a checksum.

// RUN: publish
module 0x1::merge_sort {
    use std::vector;

    // LCG parameters; mirrored by `lcg_next`/`LCG_MOD` in src/programs/mod.rs.
    const LCG_MUL: u64 = 1103515245;
    const LCG_INC: u64 = 12345;
    const LCG_MOD: u64 = 1000003;

    public fun merge_sort(v: vector<u64>): vector<u64> {
        let len = vector::length(&v);
        if (len > 1) {
            merge_sort_range(&mut v, 0, len);
        };
        v
    }

    fun merge_sort_range(v: &mut vector<u64>, lo: u64, hi: u64) {
        if (hi - lo <= 1) { return };
        let mid = (lo + hi) / 2;
        merge_sort_range(v, lo, mid);
        merge_sort_range(v, mid, hi);
        merge(v, lo, mid, hi);
    }

    fun merge(v: &mut vector<u64>, lo: u64, mid: u64, hi: u64) {
        let tmp = vector::empty<u64>();
        let i = lo;
        let j = mid;
        while (i < mid && j < hi) {
            let a = *vector::borrow(v, i);
            let b = *vector::borrow(v, j);
            if (a < b) {
                vector::push_back(&mut tmp, a);
                i = i + 1;
            } else {
                vector::push_back(&mut tmp, b);
                j = j + 1;
            };
        };
        while (i < mid) {
            vector::push_back(&mut tmp, *vector::borrow(v, i));
            i = i + 1;
        };
        while (j < hi) {
            vector::push_back(&mut tmp, *vector::borrow(v, j));
            j = j + 1;
        };
        let k = lo;
        let t = 0;
        while (k < hi) {
            *vector::borrow_mut(v, k) = *vector::borrow(&tmp, t);
            k = k + 1;
            t = t + 1;
        };
    }

    /// Build `n` pseudo-random values via an LCG seeded by `seed`, sort them,
    /// and return the positional checksum `sum(i * sorted[i])`.
    public fun sort_checksum(n: u64, seed: u64): u64 {
        let v = vector::empty<u64>();
        let x = seed % LCG_MOD;
        let k = 0;
        while (k < n) {
            x = ((x * LCG_MUL) + LCG_INC) % LCG_MOD;
            vector::push_back(&mut v, x);
            k = k + 1;
        };
        let sorted = merge_sort(v);
        let acc = 0;
        let i = 0;
        while (i < n) {
            acc = acc + i * *vector::borrow(&sorted, i);
            i = i + 1;
        };
        acc
    }
}

// Base cases: empty / single / two-element (exercise the hi-lo<=1 early
// return and the 2-element merge).
// RUN: execute 0x1::merge_sort::sort_checksum --args 0, 42
// CHECK: results: 0
// RUN: execute 0x1::merge_sort::sort_checksum --args 1, 42
// CHECK: results: 0
// RUN: execute 0x1::merge_sort::sort_checksum --args 2, 42
// CHECK: results: 513594
// RUN: execute 0x1::merge_sort::sort_checksum --args 10, 42
// CHECK: results: 28380390
// RUN: execute 0x1::merge_sort::sort_checksum --args 50, 7
// CHECK: results: 887015730
// RUN: execute 0x1::merge_sort::sort_checksum --args 200, 99
// CHECK: results: 13156074317
