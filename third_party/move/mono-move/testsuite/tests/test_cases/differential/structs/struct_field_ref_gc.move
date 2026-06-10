// RUN: publish
module 0x42::struct_field_ref_gc {
    struct Entry has drop { key: u64, value: u64 }

    public fun field_ref_across_gc(): u64 {
        let e = Entry { key: 7, value: 13 };
        let r = &mut e.value;
        0x0::test_utils::forge_gc();
        let read = *r;
        *r = 21;
        read * 1000000 + e.value * 1000 + e.key
    }
}

// RUN: execute 0x42::struct_field_ref_gc::field_ref_across_gc
// CHECK: results: 13021007
// CHECK-GC-COUNT: 1
