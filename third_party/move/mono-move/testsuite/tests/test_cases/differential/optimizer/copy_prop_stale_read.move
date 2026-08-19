// RUN: publish --print(bytecode,stackless)
module 0x42::copy_prop_stale_read {
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

    // Reads element 0 of an owned vector.
    fun first(items: vector<u64>): u64 {
        *vector::borrow(&items, 0)
    }

    // `v` is consumed (and its buffer mutated in place) before `d` is read
    // by value; the read of `d` must see the pristine copy.
    fun stale_read_after_consume(): u64 {
        let v = make(7);
        let d = copy v;
        let pin = vector::length(&d);
        let len = mutate_first(v);
        first(d) * 100 + len + pin
    }
}

// RUN: execute 0x42::copy_prop_stale_read::stale_read_after_consume
// CHECK: results: 706
