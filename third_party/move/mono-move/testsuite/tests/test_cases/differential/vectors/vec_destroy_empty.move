// RUN: publish
module 0x66::vec_destroy_empty {
    use std::vector;

    // `empty()` allocates nothing: null heap pointer.
    public fun destroy_null(): u64 {
        let items = vector::empty<u64>();
        vector::destroy_empty(items);
        7
    }

    // Push-then-pop leaves a non-null pointer to a zero-length vector.
    public fun destroy_after_pop(value: u64): u64 {
        let items = vector::empty<u64>();
        vector::push_back(&mut items, value);
        let popped = vector::pop_back(&mut items);
        vector::destroy_empty(items);
        popped
    }

    public fun destroy_nonempty(value: u64): u64 {
        let items = vector[value, value];
        vector::destroy_empty(items);
        0
    }
}

// RUN: execute 0x66::vec_destroy_empty::destroy_null
// CHECK: results: 7

// RUN: execute 0x66::vec_destroy_empty::destroy_after_pop --args 42
// CHECK: results: 42

// RUN: execute 0x66::vec_destroy_empty::destroy_nonempty --args 9
// CHECK-V1-SUBSTR: VECTOR_OPERATION_ERROR
// CHECK-V2-SUBSTR: VecUnpack: expected 0 elements
