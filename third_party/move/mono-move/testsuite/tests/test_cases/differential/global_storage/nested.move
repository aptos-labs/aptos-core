// RUN: publish --print(micro-ops,frame-layout)
module 0x42::nested_globals {
    use std::vector;

    struct WithVec has key { tag: u64, data: vector<u64> }

    struct Inner has store, drop { data: vector<u64> }
    struct Outer has key { tag: u64, inner: Inner }

    // move_to a resource embedding a vector, then borrow it back and read the
    // tag plus an element.
    public fun publish_vec_read(s: signer, a: address, x: u64): u64 {
        let v = vector::empty<u64>();
        vector::push_back(&mut v, x);
        vector::push_back(&mut v, x + 1);
        move_to(&s, WithVec { tag: x + 7, data: v });
        let r = borrow_global<WithVec>(a);
        r.tag + *vector::borrow(&r.data, 1)
    }

    // move_to then move_from, returning the tag plus an element of the moved-out
    // vector.
    public fun publish_vec_take(s: signer, a: address, x: u64): u64 {
        let v = vector::empty<u64>();
        vector::push_back(&mut v, x);
        move_to(&s, WithVec { tag: x + 7, data: v });
        let WithVec { tag, data } = move_from<WithVec>(a);
        tag + *vector::borrow(&data, 0)
    }

    // The vector pointer is reached through a nested inline struct, so it sits
    // at offset 8 of `Outer` via `Outer.inner.data`. Exercises box/unbox where
    // type_pointer_offsets must recurse into the inner struct, plus nested field
    // access on the borrow.
    public fun publish_nested_read(s: signer, a: address, x: u64): u64 {
        let v = vector::empty<u64>();
        vector::push_back(&mut v, x);
        vector::push_back(&mut v, x + 1);
        move_to(&s, Outer { tag: x + 7, inner: Inner { data: v } });
        let r = borrow_global<Outer>(a);
        r.tag + *vector::borrow(&r.inner.data, 1)
    }

    // borrow_global_mut a vector-bearing resource and push through the mutable
    // borrow (growing the vector and writing the new pointer back into the
    // stored resource), then read both elements via a fresh borrow.
    public fun mutate_vec_push(s: signer, a: address, x: u64): u64 {
        let v = vector::empty<u64>();
        vector::push_back(&mut v, x);
        move_to(&s, WithVec { tag: 0, data: v });
        let r = borrow_global_mut<WithVec>(a);
        vector::push_back(&mut r.data, x + 1);
        let r2 = borrow_global<WithVec>(a);
        *vector::borrow(&r2.data, 0) + *vector::borrow(&r2.data, 1)
    }
}

// RUN: execute 0x42::nested_globals::publish_vec_read --args 0x42, 0x42, 100
// CHECK: results: 208

// RUN: execute 0x42::nested_globals::publish_vec_take --args 0x7, 0x7, 55
// CHECK: results: 117

// RUN: execute 0x42::nested_globals::publish_nested_read --args 0x42, 0x42, 100
// CHECK: results: 208

// RUN: execute 0x42::nested_globals::mutate_vec_push --args 0x42, 0x42, 100
// CHECK: results: 201
