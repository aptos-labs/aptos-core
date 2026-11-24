// A copied vector passed as a call's first argument must stay live while the second
// argument allocates a new vector; on the 256-byte heap the loop collects repeatedly. The
// golden pins the lowering.

// RUN: publish --print(micro-ops)
module 0x1::xfer_arg_live_across_alloc {
    use std::vector;

    fun combine(left: vector<u64>, right: vector<u64>): u64 {
        *vector::borrow(&left, 0) + vector::length(&right) + *vector::borrow(&right, 2)
    }

    public fun run(value: u64, iters: u64): u64 {
        let held = vector[value];
        let acc = 0;
        let index = 0;
        while (index < iters) {
            acc = acc + combine(copy held, vector[index, index, index]);
            index = index + 1
        };
        acc
    }
}

// RUN: execute 0x1::xfer_arg_live_across_alloc::run --args 5, 100
// CHECK: results: 5750

// RUN: execute 0x1::xfer_arg_live_across_alloc::run --args 5, 100 --heap-size 256
// CHECK: results: 5750
// CHECK-GC-COUNT: 33
