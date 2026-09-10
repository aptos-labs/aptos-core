// RUN: publish --print(bytecode,stackless)
module 0x42::copy_prop_deep_copy_bypass {
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

    fun deep_copy_bypass(): u64 {
        let v = make(1);
        let d = copy v;
        let pin = vector::length(&d); // borrow pins `d` in a home slot
        let len = mutate_first(d);
        *vector::borrow(&v, 0) * 100 + len + pin
    }
}

// RUN: execute 0x42::copy_prop_deep_copy_bypass::deep_copy_bypass
// CHECK: results: 106
