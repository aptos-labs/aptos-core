// RUN: publish
module 0x42::ref_borrow_local_gc {
    use std::vector;

    public fun borrow_local_across_gc(): u64 {
        let x = 42;
        let r = &mut x;
        let v = vector::empty<u64>();
        vector::push_back(&mut v, 7);
        0x0::test_utils::force_gc();
        let read = *r;
        *r = 99;
        read * 1000 + *r + *vector::borrow(&v, 0)
    }
}

// RUN: execute 0x42::ref_borrow_local_gc::borrow_local_across_gc
// CHECK: results: 42106
// CHECK-GC-COUNT: 1
