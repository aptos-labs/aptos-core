// RUN: publish
//
// move_to of a pointer-bearing resource UNDER GC. `WithVec { tag, data }` lays
// the vector pointer at offset 8 of the inline value. The churn loop leaves
// dead garbage (each iteration's vector dies when its slot is reused — GC roots
// here are the static frame layout, not liveness, so only slot reuse frees a
// value). The `--heap-size` is tuned so the loop fits without collecting, then
// the move_to box's `HeapNew` is the allocation that tips the heap over: the GC
// reclaims the garbage and must relocate the child vector pointer (a frame root
// at offset 8 of the resource value) before `HeapMoveTo` copies it into the
// published object. Reading an element back proves the vector survived the
// relocation. `CHECK-GC-COUNT: 1` pins the single collection to the spill.
module 0x42::nested_gc {
    use std::vector;

    struct WithVec has key { tag: u64, data: vector<u64> }

    public fun publish_vec_under_gc(s: signer, a: address, x: u64, iters: u64): u64 {
        let v = vector::empty<u64>();
        vector::push_back(&mut v, x);
        vector::push_back(&mut v, x + 1);
        let i = 0;
        while (i < iters) {
            let junk = vector::empty<u64>();
            vector::push_back(&mut junk, i);
            i = i + 1;
        };
        move_to(&s, WithVec { tag: x + 7, data: v });
        let r = borrow_global<WithVec>(a);
        r.tag + *vector::borrow(&r.data, 1)
    }
}

// RUN: execute 0x42::nested_gc::publish_vec_under_gc --args 0x42, 0x42, 100, 2 --heap-size 152
// CHECK: results: 208
// CHECK-GC-COUNT: 1
