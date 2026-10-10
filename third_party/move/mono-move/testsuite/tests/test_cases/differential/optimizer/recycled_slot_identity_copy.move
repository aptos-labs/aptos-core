// RUN: publish --print(stackless)
module 0x42::recycled_slot_identity_copy {
    use std::vector;

    struct Wrap has drop { v: vector<u64> }

    fun make(seed: u64): vector<u64> {
        let items = vector::empty<u64>();
        vector::push_back(&mut items, seed);
        vector::push_back(&mut items, seed + 1);
        vector::push_back(&mut items, seed + 2);
        items
    }

    fun consume_len(items: vector<u64>): u64 {
        vector::length(&items)
    }

    fun assign_through_call(seed: u64): u64 {
        let h = make(seed);
        let len;
        (h, len) = (make(seed + 10), consume_len(h));
        let wrapped = Wrap { v: copy h };
        *vector::borrow_mut(&mut wrapped.v, 0) = 99;
        *vector::borrow(&h, 0) * 100 + len
    }
}

// RUN: execute 0x42::recycled_slot_identity_copy::assign_through_call --args 1
// CHECK: results: 1103
