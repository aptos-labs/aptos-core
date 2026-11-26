// Copies of two vectors passed as a call's first two arguments must survive a collection
// forced while the third argument is evaluated. The golden pins the lowering.

// RUN: publish --print(micro-ops)
module 0x1::two_xfer_roots_force_gc {
    use std::vector;

    fun firsts(left: vector<u64>, right: vector<u64>, offset: u64): (u64, u64) {
        (*vector::borrow(&left, 0) + offset, *vector::borrow(&right, 0) + offset)
    }

    public fun run(left: u64, right: u64): (u64, u64, u64) {
        let v1 = vector[left];
        let v2 = vector[right];
        let (left_first, right_first) = firsts(copy v1, copy v2, { 0x0::test_utils::force_gc(); 0 });
        (left_first, right_first, *vector::borrow(&v1, 0) * 100 + *vector::borrow(&v2, 0))
    }
}

// RUN: execute 0x1::two_xfer_roots_force_gc::run --args 11, 22
// CHECK: results: 11, 22, 1122
// CHECK-GC-COUNT: 1
