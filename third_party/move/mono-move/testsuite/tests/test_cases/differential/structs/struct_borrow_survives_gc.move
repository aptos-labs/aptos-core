// RUN: publish
module 0x42::struct_borrow_survives_gc {
    struct Entry has drop { key: u64, value: u64 }

    public fun borrow_field_across_gc(): u64 {
        let e = Entry { key: 100, value: 200 };
        let r = &mut e.value;
        0x0::test_utils::forge_gc();
        let read = *r;
        read * 1000 + e.key
    }
}

// RUN: execute 0x42::struct_borrow_survives_gc::borrow_field_across_gc
// CHECK: results: 200100
// CHECK-GC-COUNT: 1
