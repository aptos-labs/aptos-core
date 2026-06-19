// RUN: publish
module 0x42::ref_survives_gc {
    use std::vector;

    public fun borrow_elem_across_gc(): u64 {
        let v = vector::empty<u64>();
        vector::push_back(&mut v, 100);
        vector::push_back(&mut v, 200);
        vector::push_back(&mut v, 300);
        let r = vector::borrow_mut(&mut v, 2);
        0x0::test_utils::force_gc();
        let read = *r;
        *r = 42;
        read * 1000 + *vector::borrow(&v, 2)
    }
}

// RUN: execute 0x42::ref_survives_gc::borrow_elem_across_gc
// CHECK: results: 300042
// CHECK-GC-COUNT: 1
