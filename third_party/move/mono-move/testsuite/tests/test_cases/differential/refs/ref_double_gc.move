// RUN: publish
module 0x42::ref_double_gc {
    use std::vector;

    public fun borrow_across_double_gc(): u64 {
        let v = vector::empty<u64>();
        vector::push_back(&mut v, 10);
        vector::push_back(&mut v, 20);
        vector::push_back(&mut v, 30);
        let r = vector::borrow_mut(&mut v, 1);
        0x0::test_utils::forge_gc();
        0x0::test_utils::forge_gc();
        let read = *r;
        *r = 77;
        read * 1000 + *vector::borrow(&v, 1)
    }
}

// RUN: execute 0x42::ref_double_gc::borrow_across_double_gc
// CHECK: results: 20077
// CHECK-GC-COUNT: 2
