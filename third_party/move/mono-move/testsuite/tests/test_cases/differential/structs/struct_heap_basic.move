// RUN: publish
module 0x42::struct_heap_basic {
    struct Entry has drop { f0: u64, f1: u64 }

    public fun pack_load_add(): u64 {
        let e = Entry { f0: 42, f1: 100 };
        e.f0 + e.f1
    }
}

// RUN: execute 0x42::struct_heap_basic::pack_load_add
// CHECK: results: 142
