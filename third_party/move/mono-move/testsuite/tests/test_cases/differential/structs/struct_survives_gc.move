// RUN: publish
module 0x42::struct_survives_gc {
    struct Entry has drop { f0: u64, f1: u64 }

    public fun struct_across_gc(): u64 {
        let e = Entry { f0: 7, f1: 13 };
        0x0::test_utils::force_gc();
        e.f0 + e.f1
    }
}

// RUN: execute 0x42::struct_survives_gc::struct_across_gc
// CHECK: results: 20
// CHECK-GC-COUNT: 1
