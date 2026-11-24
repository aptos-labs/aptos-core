// Vector borrows at the bounds: index equal to the length and u64::MAX, on a never-pushed
// vector, on a drained vector, through `borrow_mut`, on a `vector<u8>` and with index
// syntax. Every out-of-range index fails on both VMs.

// RUN: publish
module 0x66::vec_borrow_bounds {
    use std::vector;

    public fun at(index: u64): u64 {
        let values = vector[10, 20, 30];
        *vector::borrow(&values, index)
    }

    public fun on_null(index: u64): u64 {
        let values = vector::empty<u64>();
        *vector::borrow(&values, index)
    }

    public fun on_drained(index: u64): u64 {
        let values = vector[1u64];
        vector::pop_back(&mut values);
        *vector::borrow(&values, index)
    }

    public fun mut_at(index: u64): u64 {
        let values = vector[10, 20, 30];
        *vector::borrow_mut(&mut values, index) = 99;
        *vector::borrow(&values, 2)
    }

    public fun bytes_at(index: u64): u8 {
        let bytes = b"abc";
        *vector::borrow(&bytes, index)
    }

    public fun index_syntax(index: u64): u64 {
        let values = vector[10, 20, 30];
        values[index]
    }
}

// RUN: execute 0x66::vec_borrow_bounds::at --args 2
// CHECK: results: 30

// RUN: execute 0x66::vec_borrow_bounds::at --args 3
// CHECK-V1-SUBSTR: VECTOR_OPERATION_ERROR
// CHECK-V2-SUBSTR: VecBorrow index out of bounds
// CHECK-ERROR-PARITY

// RUN: execute 0x66::vec_borrow_bounds::at --args 18446744073709551615
// CHECK-V1-SUBSTR: VECTOR_OPERATION_ERROR
// CHECK-V2-SUBSTR: VecBorrow index out of bounds
// CHECK-ERROR-PARITY

// RUN: execute 0x66::vec_borrow_bounds::on_null --args 0
// CHECK-V1-SUBSTR: VECTOR_OPERATION_ERROR
// CHECK-V2-SUBSTR: VecBorrow index out of bounds
// CHECK-ERROR-PARITY

// RUN: execute 0x66::vec_borrow_bounds::on_drained --args 0
// CHECK-V1-SUBSTR: VECTOR_OPERATION_ERROR
// CHECK-V2-SUBSTR: VecBorrow index out of bounds
// CHECK-ERROR-PARITY

// RUN: execute 0x66::vec_borrow_bounds::mut_at --args 2
// CHECK: results: 99

// RUN: execute 0x66::vec_borrow_bounds::mut_at --args 3
// CHECK-V1-SUBSTR: VECTOR_OPERATION_ERROR
// CHECK-V2-SUBSTR: VecBorrow index out of bounds
// CHECK-ERROR-PARITY

// RUN: execute 0x66::vec_borrow_bounds::bytes_at --args 2
// CHECK: results: 99

// RUN: execute 0x66::vec_borrow_bounds::bytes_at --args 3
// CHECK-V1-SUBSTR: VECTOR_OPERATION_ERROR
// CHECK-V2-SUBSTR: VecBorrow index out of bounds
// CHECK-ERROR-PARITY

// RUN: execute 0x66::vec_borrow_bounds::index_syntax --args 3
// CHECK-V1-SUBSTR: VECTOR_OPERATION_ERROR
// CHECK-V2-SUBSTR: VecBorrow index out of bounds
// CHECK-ERROR-PARITY
