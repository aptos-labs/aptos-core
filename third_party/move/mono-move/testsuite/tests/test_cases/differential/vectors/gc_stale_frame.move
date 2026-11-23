// `keep_big` allocates a 256-byte vector and returns; `fresh` then collects before
// allocating its own 256-byte vector. On heaps of 256 and 511 bytes only one fits, so the
// collection must not treat what `keep_big` left on the stack as a live reference to the
// dead vector. The golden pins that both vectors use the same stack slot.

// RUN: publish --print(micro-ops,frame-layout)
module 0x42::stale_frame {
    use std::vector;

    fun keep_big(value: u64): u64 {
        let big = vector[value, value, value, value, value, value, value, value, value, value, value, value, value, value, value, value, value, value, value, value, value, value, value, value, value, value, value, value, value, value];
        vector::length(&big)
    }

    fun fresh(value: u64): u64 {
        0x0::test_utils::force_gc();
        let next = vector[value, value, value, value, value, value, value, value, value, value, value, value, value, value, value, value, value, value, value, value, value, value, value, value, value, value, value, value, value, value];
        vector::length(&next) + *vector::borrow(&next, 29)
    }

    public fun run(value: u64): u64 {
        let big_len = keep_big(value);
        let fresh_sum = fresh(value);
        big_len * 1000 + fresh_sum
    }
}

// RUN: execute 0x42::stale_frame::run --args 7 --heap-size 256
// CHECK: results: 30037
// CHECK-GC-COUNT: 1

// RUN: execute 0x42::stale_frame::run --args 7 --heap-size 511
// CHECK: results: 30037
// CHECK-GC-COUNT: 1
