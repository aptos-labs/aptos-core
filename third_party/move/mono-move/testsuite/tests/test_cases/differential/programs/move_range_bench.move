// Each iteration moves a chunk out of `from` into `to` and then straight back,
// leaving both vectors in steady state (no reallocation after the first
// iteration), so the loop isolates `vector::move_range`'s per-call cost.
// `shuffle_small` moves a 2-element chunk (dispatch-dominated); `shuffle_large`
// a 60-element chunk (copy-dominated). The weighted checksum guards against the
// work being optimized away and catches reordering. The result is independent
// of the iteration count, since every iteration round-trips.

// RUN: publish
module 0x1::move_range_bench {
    use std::vector;

    fun make(n: u64): vector<u64> {
        let v = vector::empty<u64>();
        let i = 0;
        while (i < n) {
            vector::push_back(&mut v, i + 1);
            i = i + 1;
        };
        v
    }

    fun checksum(v: &vector<u64>): u64 {
        let acc = 0;
        let pos = 0;
        let len = vector::length(v);
        while (pos < len) {
            let e = *vector::borrow(v, pos);
            acc = acc + (pos + 1) * e;
            pos = pos + 1;
        };
        acc
    }

    public fun shuffle_small(iters: u64): u64 {
        let from = make(8);
        let to = vector::empty<u64>();
        let i = 0;
        while (i < iters) {
            vector::move_range(&mut from, 3, 2, &mut to, 0);
            vector::move_range(&mut to, 0, 2, &mut from, 3);
            i = i + 1;
        };
        checksum(&from)
    }

    public fun shuffle_large(iters: u64): u64 {
        let from = make(64);
        let to = vector::empty<u64>();
        let i = 0;
        while (i < iters) {
            vector::move_range(&mut from, 2, 60, &mut to, 0);
            vector::move_range(&mut to, 0, 60, &mut from, 2);
            i = i + 1;
        };
        checksum(&from)
    }
}

// RUN: execute 0x1::move_range_bench::shuffle_small --args 0
// CHECK: results: 204
// RUN: execute 0x1::move_range_bench::shuffle_small --args 1
// CHECK: results: 204
// RUN: execute 0x1::move_range_bench::shuffle_small --args 50
// CHECK: results: 204
// RUN: execute 0x1::move_range_bench::shuffle_large --args 0
// CHECK: results: 89440
// RUN: execute 0x1::move_range_bench::shuffle_large --args 1
// CHECK: results: 89440
// RUN: execute 0x1::move_range_bench::shuffle_large --args 50
// CHECK: results: 89440
