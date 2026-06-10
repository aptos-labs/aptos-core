// RUN: publish
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
