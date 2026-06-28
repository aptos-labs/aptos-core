// RUN: publish
module 0x42::ref_nested_vectors_gc {
    use std::vector;

    public fun nested_borrow_across_gc(): u64 {
        let inner0 = vector::empty<u64>();
        vector::push_back(&mut inner0, 100);
        vector::push_back(&mut inner0, 200);
        let inner1 = vector::empty<u64>();
        vector::push_back(&mut inner1, 300);
        vector::push_back(&mut inner1, 400);
        vector::push_back(&mut inner1, 500);
        let outer = vector::empty<vector<u64>>();
        vector::push_back(&mut outer, inner0);
        vector::push_back(&mut outer, inner1);

        let r = vector::borrow_mut(vector::borrow_mut(&mut outer, 1), 2);
        0x0::test_utils::force_gc();
        let read = *r;
        *r = 999;

        let a = *vector::borrow(vector::borrow(&outer, 0), 0);
        let c = *vector::borrow(vector::borrow(&outer, 1), 2);
        read + a + c
    }
}

// RUN: execute 0x42::ref_nested_vectors_gc::nested_borrow_across_gc
// CHECK: results: 1599
// CHECK-GC-COUNT: 1
