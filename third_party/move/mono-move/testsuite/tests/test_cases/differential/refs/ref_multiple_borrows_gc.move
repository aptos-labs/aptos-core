// RUN: publish
module 0x42::ref_multiple_borrows_gc {
    use std::vector;

    public fun two_refs_across_gc(): u64 {
        let va = vector::empty<u64>();
        vector::push_back(&mut va, 10);
        vector::push_back(&mut va, 20);
        let vb = vector::empty<u64>();
        vector::push_back(&mut vb, 30);
        vector::push_back(&mut vb, 40);
        let ra = vector::borrow_mut(&mut va, 1);
        let rb = vector::borrow_mut(&mut vb, 1);
        0x0::test_utils::forge_gc();
        let read = *ra;
        *ra = 55;
        *rb = 66;
        read * 1000000 + *vector::borrow(&va, 1) * 1000 + *vector::borrow(&vb, 1)
    }
}

// RUN: execute 0x42::ref_multiple_borrows_gc::two_refs_across_gc
// CHECK: results: 20055066
// CHECK-GC-COUNT: 1
