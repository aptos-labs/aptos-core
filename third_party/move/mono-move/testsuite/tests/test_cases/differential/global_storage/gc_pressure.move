// RUN: publish
//
// Global storage under GC pressure (differential analog of the runtime
// `gc_traces_and_relocates_local_heap_writes` test). `move_to` then
// `borrow_global_mut` records a `LocalHeap` write; the loop then allocates and
// drops vectors to exhaust a small heap and force a collection. The GC must
// relocate the resource's `LocalHeap` pointer (via the read-write-set scan),
// so the final `borrow_global` reads the mutated value rather than a stale
// pointer. `--heap-size` drives the collection; `CHECK-GC-COUNT` pins it to
// exactly one.
module 0x42::gc_globals {
    use std::vector;

    struct R has key { v: u64 }

    public fun mutate_churn_read(s: signer, a: address, x: u64, iters: u64): u64 {
        move_to(&s, R { v: x });
        let r = borrow_global_mut<R>(a);
        r.v = r.v + 1;
        let i = 0;
        while (i < iters) {
            let junk = vector::empty<u64>();
            vector::push_back(&mut junk, i);
            i = i + 1;
        };
        borrow_global<R>(a).v
    }
}

// RUN: execute 0x42::gc_globals::mutate_churn_read --args 0x42, 0x42, 776, 8 --heap-size 256
// CHECK: results: 777
// CHECK-GC-COUNT: 1
