// RUN: publish --print(stackless)
module 0x42::coalesce_copy_bypass {
    use std::vector;

    fun make(seed: u64): vector<u64> {
        let items = vector::empty<u64>();
        vector::push_back(&mut items, seed);
        vector::push_back(&mut items, seed + 1);
        vector::push_back(&mut items, seed + 2);
        items
    }

    // Takes ownership and mutates element 0 in place (no realloc).
    fun mutate_first(items: vector<u64>): u64 {
        let owned = items;
        *vector::borrow_mut(&mut owned, 0) = 99;
        vector::length(&owned)
    }

    fun direct_arg_copy(seed: u64): u64 {
        let v = make(seed);
        let len = mutate_first(copy v);
        *vector::borrow(&v, 0) * 100 + len
    }
}

// RUN: execute 0x42::coalesce_copy_bypass::direct_arg_copy --args 1
// CHECK: results: 103
