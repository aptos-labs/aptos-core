// Popping from an empty vector (never pushed to, or drained) of `u64`, `u8` and vector
// elements: every pop fails on both VMs.

// RUN: publish
module 0x66::vec_pop_empty {
    use std::vector;

    public fun pop_null(): u64 {
        let values = vector::empty<u64>();
        vector::pop_back(&mut values)
    }

    public fun pop_drained(value: u64): u64 {
        let values = vector[value];
        vector::pop_back(&mut values);
        vector::pop_back(&mut values)
    }

    public fun pop_drained_bytes(byte: u8): u8 {
        let bytes = vector[byte];
        vector::pop_back(&mut bytes);
        vector::pop_back(&mut bytes)
    }

    public fun pop_drained_ptr(): u64 {
        let values = vector[vector[1u64]];
        vector::pop_back(&mut values);
        vector::length(&vector::pop_back(&mut values))
    }
}

// RUN: execute 0x66::vec_pop_empty::pop_null
// CHECK-V1-SUBSTR: VECTOR_OPERATION_ERROR
// CHECK-V2-SUBSTR: VecPopBack on empty vector
// CHECK-ERROR-PARITY

// RUN: execute 0x66::vec_pop_empty::pop_drained --args 7
// CHECK-V1-SUBSTR: VECTOR_OPERATION_ERROR
// CHECK-V2-SUBSTR: VecPopBack on empty vector
// CHECK-ERROR-PARITY

// RUN: execute 0x66::vec_pop_empty::pop_drained_bytes --args 7
// CHECK-V1-SUBSTR: VECTOR_OPERATION_ERROR
// CHECK-V2-SUBSTR: VecPopBack on empty vector
// CHECK-ERROR-PARITY

// RUN: execute 0x66::vec_pop_empty::pop_drained_ptr
// CHECK-V1-SUBSTR: VECTOR_OPERATION_ERROR
// CHECK-V2-SUBSTR: VecPopBack on empty vector
// CHECK-ERROR-PARITY
