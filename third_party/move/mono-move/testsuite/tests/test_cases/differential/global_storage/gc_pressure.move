// RUN: publish
module 0x42::gc_globals {
    struct R has key { v: u64 }

    public fun mutate_force_read(s: signer, a: address, x: u64): u64 {
        move_to(&s, R { v: x });
        let r = borrow_global_mut<R>(a);
        r.v = r.v + 1;
        0x0::test_utils::force_gc();
        borrow_global<R>(a).v
    }
}

// RUN: execute 0x42::gc_globals::mutate_force_read --args 0x42, 0x42, 776 --heap-size 256
// CHECK: results: 777
// CHECK-GC-COUNT: 1
