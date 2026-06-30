// RUN: publish
module 0x42::ref_cross_frame_gc {
    use std::vector;

    fun write_through_ref(r: &mut u64) {
        0x0::test_utils::force_gc();
        *r = 77;
    }

    public fun cross_frame_gc(): u64 {
        let v = vector::empty<u64>();
        vector::push_back(&mut v, 10);
        vector::push_back(&mut v, 20);
        vector::push_back(&mut v, 30);
        write_through_ref(vector::borrow_mut(&mut v, 1));
        *vector::borrow(&v, 0) * 1000000 + *vector::borrow(&v, 1) * 1000 + *vector::borrow(&v, 2)
    }
}

// RUN: execute 0x42::ref_cross_frame_gc::cross_frame_gc
// CHECK: results: 10077030
// CHECK-GC-COUNT: 1
