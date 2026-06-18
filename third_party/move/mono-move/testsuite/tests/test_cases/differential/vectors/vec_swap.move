// RUN: publish
module 0x66::vec_swap {
    use std::vector;

    // Digit-pair-encodes the elements (each < 100) for readback.
    fun encode(items: &vector<u64>): u64 {
        let total = 0;
        let pos = 0;
        while (pos < vector::length(items)) {
            total = total * 100 + *vector::borrow(items, pos);
            pos = pos + 1;
        };
        total
    }

    public fun swap_encode(idx_a: u64, idx_b: u64): u64 {
        let items = vector[11, 22, 33, 44];
        vector::swap(&mut items, idx_a, idx_b);
        encode(&items)
    }

    public fun swap_empty(idx_a: u64, idx_b: u64): u64 {
        let items = vector::empty<u64>();
        vector::swap(&mut items, idx_a, idx_b);
        vector::length(&items)
    }

    public fun swap_remove_encode(idx: u64): u64 {
        let items = vector[11, 22, 33, 44];
        let last_idx = vector::length(&items) - 1;
        vector::swap(&mut items, idx, last_idx);
        let removed = vector::pop_back(&mut items);
        removed * 1000000 + encode(&items)
    }

    public fun swap_bytes_encode(idx_a: u64, idx_b: u64): u64 {
        let bytes = vector[1u8, 2u8, 3u8, 4u8];
        vector::swap(&mut bytes, idx_a, idx_b);
        let total = 0u64;
        let pos = 0;
        while (pos < vector::length(&bytes)) {
            total = total * 100 + (*vector::borrow(&bytes, pos) as u64);
            pos = pos + 1;
        };
        total
    }

    public fun swap_wide(first: u256, second: u256): u256 {
        let items = vector[first, second];
        vector::swap(&mut items, 0, 1);
        *vector::borrow(&items, 0) * 1000000 + *vector::borrow(&items, 1)
    }
}

// First/last swap.
// RUN: execute 0x66::vec_swap::swap_encode --args 0, 3
// CHECK: results: 44223311

// Adjacent elements.
// RUN: execute 0x66::vec_swap::swap_encode --args 1, 2
// CHECK: results: 11332244

// Operand order is irrelevant.
// RUN: execute 0x66::vec_swap::swap_encode --args 3, 0
// CHECK: results: 44223311

// In-bounds swap(i, i) is a no-op.
// RUN: execute 0x66::vec_swap::swap_encode --args 2, 2
// CHECK: results: 11223344

// Out-of-bounds swap(i, i) must still error.
// RUN: execute 0x66::vec_swap::swap_encode --args 5, 5
// CHECK-V1-SUBSTR: VECTOR_OPERATION_ERROR
// CHECK-V2-SUBSTR: VecSwap index out of bounds

// First index out of bounds, second in bounds.
// RUN: execute 0x66::vec_swap::swap_encode --args 4, 1
// CHECK-V1-SUBSTR: VECTOR_OPERATION_ERROR
// CHECK-V2-SUBSTR: VecSwap index out of bounds

// Second index out of bounds, first in bounds.
// RUN: execute 0x66::vec_swap::swap_encode --args 1, 4
// CHECK-V1-SUBSTR: VECTOR_OPERATION_ERROR
// CHECK-V2-SUBSTR: VecSwap index out of bounds

// Swap on an empty (null-pointer) vector.
// RUN: execute 0x66::vec_swap::swap_empty --args 0, 0
// CHECK-V1-SUBSTR: VECTOR_OPERATION_ERROR
// CHECK-V2-SUBSTR: VecSwap index out of bounds

// swap_remove middle: last element moves into the hole.
// RUN: execute 0x66::vec_swap::swap_remove_encode --args 1
// CHECK: results: 22114433

// swap_remove first.
// RUN: execute 0x66::vec_swap::swap_remove_encode --args 0
// CHECK: results: 11442233

// swap_remove last: degenerate self-swap, order preserved.
// RUN: execute 0x66::vec_swap::swap_remove_encode --args 3
// CHECK: results: 44112233

// 1-byte elements: [1, 2, 3, 4] -> [4, 2, 3, 1].
// RUN: execute 0x66::vec_swap::swap_bytes_encode --args 0, 3
// CHECK: results: 4020301

// 1-byte elements, adjacent: [1, 2, 3, 4] -> [1, 3, 2, 4].
// RUN: execute 0x66::vec_swap::swap_bytes_encode --args 1, 2
// CHECK: results: 1030204

// 32-byte elements: [5, 9] -> [9, 5].
// RUN: execute 0x66::vec_swap::swap_wide --args 5, 9
// CHECK: results: 9000005
