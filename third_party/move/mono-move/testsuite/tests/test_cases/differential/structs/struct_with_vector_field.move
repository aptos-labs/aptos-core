// RUN: publish
module 0x42::struct_with_vector_field {
    use std::vector;

    struct Counter has drop { tag: u64, items: vector<u64> }

    public fun vector_field_across_gc(): u64 {
        let items = vector::empty<u64>();
        vector::push_back(&mut items, 10);
        vector::push_back(&mut items, 20);
        vector::push_back(&mut items, 30);
        let c = Counter { tag: 999, items };
        0x0::test_utils::forge_gc();
        let sum =
            *vector::borrow(&c.items, 0) + *vector::borrow(&c.items, 1) + *vector::borrow(&c.items, 2);
        c.tag * 1000 + sum
    }
}

// RUN: execute 0x42::struct_with_vector_field::vector_field_across_gc
// CHECK: results: 999060
// CHECK-GC-COUNT: 1
